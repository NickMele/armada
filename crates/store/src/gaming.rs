//! What the gaming check flagged on a step, written down and read back.
//!
//! Its own module rather than more of `write.rs` and `read.rs`, which are the
//! two longest files in the crate. The pair is one subject and reads as one.
//!
//! **The pattern is the finding.** `evidence_suspect` says a step's evidence is
//! not to be trusted; only these rows say what about it, which is the same
//! thing a refusal's citation is to a `gate_failure`.

use rusqlite::Row;

use core_model::{CitedAt, GamingFlag, GamingPattern, JobId, RepoPath, StepId, Timestamp};

use crate::attempt::attempt_now;
use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value, string};

/// The table these columns are read from, named once. An error carries it as a
/// fixed string, never a value from a row.
const TABLE: &str = "job_step_gaming_flags";

/// Version 24 — where a flag points, beside what it cites.
///
/// Beside the table it changes rather than in `schema.rs`, like
/// [`V22`](crate::judged::V22) and for the same reason: that file is at the
/// 900 lines the gate refuses at.
///
/// **Two nullable columns and no `CHECK`.** `SQLite` cannot add a constraint
/// by `ALTER`, and the invariant one would state — a line is never recorded
/// without the file it is a line of — is already unspeakable one layer up:
/// `core_model::CitedAt` has no constructor taking a line alone. A row with a
/// line and a null `cited_file` reads back as no location at all, which is
/// what a value nothing in the workspace could have written deserves.
///
/// **Nothing is backfilled.** A flag written before this existed kept only the
/// sentence the check wrote, and where in the patch that sentence came from was
/// never recorded anywhere — so the honest answer for every existing row is
/// null, which is V5's rule.
pub(crate) const V24: &str = r#"
ALTER TABLE job_step_gaming_flags ADD COLUMN cited_file TEXT;
ALTER TABLE job_step_gaming_flags ADD COLUMN cited_line INTEGER;
"#;

impl Store {
    /// Record which patterns one run of one step tripped, replacing whatever an
    /// earlier pass **over that same run** wrote.
    ///
    /// Same shape as [`record_step_judgments`](Store::record_step_judgments)
    /// and for the same reason: a step is looked at afresh each time it
    /// submits, and two passes interleaved would read as one pass that found
    /// twice as much. A second *run* of the step is not a second pass over the
    /// same evidence, so its flags are kept beside the first run's — the same
    /// pattern flagged on three runs is what says the Drone did not stop doing
    /// it.
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
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;

        tx.execute(
            "DELETE FROM job_step_gaming_flags
             WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3",
            (job_id.as_str(), step_id.as_str(), attempt.number()),
        )
        .map_err(fault("clearing this run's previous pass"))
        .map_err(WriteError::Database)?;

        for (ordinal, flag) in flags.iter().enumerate() {
            tx.execute(
                "INSERT INTO job_step_gaming_flags (
                     job_id, step_id, attempt, ordinal, pattern, cited, flagged_at,
                     cited_file, cited_line
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    job_id.as_str(),
                    step_id.as_str(),
                    attempt.number(),
                    ordinal as i64,
                    flag.pattern.as_wire(),
                    flag.cited,
                    at.as_str(),
                    flag.at.as_ref().map(|at| at.path().as_str()),
                    flag.at.as_ref().and_then(CitedAt::line),
                ],
            )
            .map_err(fault("writing a gaming flag"))
            .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the gaming record"))
            .map_err(WriteError::Database)
    }

    /// What the gaming check flagged on each of a Job's steps **on its latest
    /// run**, in the order it answered them.
    ///
    /// Read beside the record for the reason `step_checks` is: a flag is not a
    /// field of `core_model::Job`. A step with no rows is absent from the list,
    /// which is what "nothing was flagged" looks like — and it is the ordinary
    /// case, because most steps declare no gaming check at all.
    ///
    /// **The latest run and not every run**, which is what this answered before
    /// a step could have more than one. An escalation and a Board row are about
    /// where the step stands now; the history is
    /// [`step_gaming_flags_every_attempt`](Store::step_gaming_flags_every_attempt).
    pub fn step_gaming_flags(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(StepId, Vec<GamingFlag>)>, LoadJobError> {
        let rows = self
            .collect(
                "SELECT step_id, pattern, cited, cited_file, cited_line
                 FROM job_step_gaming_flags AS f
                 WHERE job_id = ?1
                   AND attempt = (SELECT max(attempt) FROM job_step_gaming_flags
                                  WHERE job_id = f.job_id AND step_id = f.step_id)
                 ORDER BY step_id, ordinal",
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
                            at: cited_at(row)?,
                        },
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(grouped(rows))
    }
}

/// Where a flag points, out of the two columns that hold it.
///
/// **The file decides.** A line with no file beside it is dropped rather than
/// carried, because there is nothing for it to be a line of — see [`V24`] for
/// why no constraint says so instead.
pub(crate) fn cited_at(row: &Row<'_>) -> Result<Option<CitedAt>, RowError> {
    let file: Option<String> = row.get("cited_file").map_err(column(TABLE, "cited_file"))?;
    let line: Option<i64> = row.get("cited_line").map_err(column(TABLE, "cited_line"))?;
    Ok(file.map(|file| {
        let path = RepoPath::new(file);
        match line.and_then(|line| u32::try_from(line).ok()) {
            Some(line) => CitedAt::at_line(path, line),
            None => CitedAt::in_file(path),
        }
    }))
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
