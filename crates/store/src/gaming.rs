//! What the gaming check flagged on a step, written down and read back.
//!
//! Its own module rather than more of `write.rs` and `read.rs`, which are the
//! two longest files in the crate. The pair is one subject and reads as one.
//!
//! **The pattern is the finding.** `evidence_suspect` says a step's evidence is
//! not to be trusted; only these rows say what about it, which is the same
//! thing a refusal's citation is to a `gate_failure`.

use core_model::{GamingFlag, GamingPattern, JobId, StepId, Timestamp};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;
use crate::row::{enum_value, string};

impl Store {
    /// Record which patterns one step's evidence tripped, replacing whatever an
    /// earlier pass over the same step wrote.
    ///
    /// Same shape as [`record_step_judgments`](Store::record_step_judgments)
    /// and for the same reason: a step is looked at afresh each time it
    /// submits, and two passes interleaved would read as one pass that found
    /// twice as much.
    ///
    /// **No event.** A flag is a fact Fleet observed, and the escalation it led
    /// to is already a row in the log.
    pub fn record_step_gaming_flags(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        flags: &[GamingFlag],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the gaming record"))
            .map_err(WriteError::Database)?;

        tx.execute(
            "DELETE FROM job_step_gaming_flags WHERE job_id = ?1 AND step_id = ?2",
            (job_id.as_str(), step_id.as_str()),
        )
        .map_err(fault("clearing the previous pass"))
        .map_err(WriteError::Database)?;

        for (ordinal, flag) in flags.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_step_gaming_flags (
                     job_id, step_id, ordinal, pattern, cited, flagged_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    job_id.as_str(),
                    step_id.as_str(),
                    ordinal as i64,
                    flag.pattern.as_wire(),
                    flag.cited,
                    at.as_str(),
                ],
            )
            .map_err(fault("writing a gaming flag"))
            .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the gaming record"))
            .map_err(WriteError::Database)
    }

    /// What the gaming check flagged on each of a Job's steps, in the order it
    /// answered them.
    ///
    /// Read beside the record for the reason `step_checks` is: a flag is not a
    /// field of `core_model::Job`. A step with no rows is absent from the list,
    /// which is what "nothing was flagged" looks like — and it is the ordinary
    /// case, because most steps declare no gaming check at all.
    pub fn step_gaming_flags(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(StepId, Vec<GamingFlag>)>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, pattern, cited FROM job_step_gaming_flags
                 WHERE job_id = ?1 ORDER BY step_id, ordinal",
                job_id,
                "reading gaming flags",
                |row| {
                    let pattern = string(row, "pattern")?;
                    Ok((
                        StepId::new(string(row, "step_id")?),
                        GamingFlag {
                            pattern: enum_value(
                                GamingPattern::from_wire,
                                "job_step_gaming_flags",
                                "pattern",
                                &pattern,
                            )?,
                            cited: string(row, "cited")?,
                        },
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(grouped(rows))
    }
}

/// One list per step, in the order the rows came back. A linear pass rather
/// than a map: the rows are already ordered by step, so a map would be a second
/// index over a sorted list.
fn grouped(rows: Vec<(StepId, GamingFlag)>) -> Vec<(StepId, Vec<GamingFlag>)> {
    let mut grouped: Vec<(StepId, Vec<GamingFlag>)> = Vec::new();
    for (step_id, flag) in rows {
        match grouped.last_mut() {
            Some((last, flags)) if *last == step_id => flags.push(flag),
            _ => grouped.push((step_id, vec![flag])),
        }
    }
    grouped
}
