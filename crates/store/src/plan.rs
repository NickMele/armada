//! What a step said its work would be, kept after the slot that held it is
//! gone.
//!
//! `fleet::scope` holds a declaration on the working slot, `Working::now_on`
//! clears it at every step boundary, and a Fleet that restarts loses it. So a
//! finished Job's footprint had no promise left to be measured against. This is
//! the durability [`crate::footprint`] already has, for the other half of that
//! comparison.
//!
//! **Keyed by the run**, derived from the log inside the writing transaction,
//! exactly as [`crate::attempt`] requires: a step worked twice declares twice
//! and no caller supplies a run number. Inside one run a declaration replaces,
//! because the tool's contract is that calling it again corrects the plan.
//!
//! **Two tables, because empty is an answer.** A header with no paths is a step
//! that promised to touch nothing; no header is a step that never promised.
//!
//! **Nothing here computes drift.** Which files fell outside which plan is a
//! pure function of this and the footprint, derived where it is served —
//! [`crate::attempt`]'s rule that a stored derivation is a second record that
//! can disagree. The hazard `#127` guarded against is a reading that opens a
//! worktree; comparing two rows opens nothing.

use core_model::{Attempt, DeclaredPaths, JobId, RepoPath, StepId, Timestamp};

use crate::attempt::attempt_now;
use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::string;

/// Version 18 — what a step declared its work would be.
///
/// Beside the tables it creates rather than in `schema.rs`, for the reason
/// `V17` is: that file sits on the 900 lines the gate refuses at. The order
/// still lives there, which is the part that may not be anywhere else.
///
/// **Nothing to backfill.** No declaration was written down before this table
/// existed, which is what zero rows says — and a Job that finished before it
/// reads as a Job whose steps declared nothing, which is the same sentence a
/// Job whose steps genuinely declared nothing gets. Both are silent rather than
/// counted, which is the answer this record exists to make possible.
pub(crate) const V18: &str = r#"
CREATE TABLE job_step_plans (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL,
    declared_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt)
) STRICT;

-- One row per declared path, in the order the drone named them. A plan with no
-- rows is a step that declared it would touch nothing; a step with no header
-- row above declared nothing at all.
CREATE TABLE job_step_plan_paths (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    path    TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt, ordinal)
) STRICT;
"#;

/// One run of one step's promise, as it was written down.
///
/// **Absent and empty are different sentences**, the same pair
/// [`Footprinted`](crate::Footprinted) draws. No [`DeclaredPlan`] for a step is
/// a step that never declared, which is silent. One with no paths is a step
/// that declared it would touch nothing, which every path in the footprint is
/// outside of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredPlan {
    pub step_id: StepId,
    /// Which run of that step declared it. A step worked twice declares twice.
    pub attempt: Attempt,
    /// When the declaration was taken, as the injected clock read it.
    pub declared_at: Timestamp,
    pub paths: DeclaredPaths,
}

impl Store {
    /// Write down what a step said its work would be, replacing whatever that
    /// same run declared before.
    ///
    /// The run is [derived from the log](Store::step_attempt) inside this
    /// transaction rather than passed in, so a caller cannot file a plan under
    /// a run the history does not have.
    pub fn record_step_plan(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        paths: &DeclaredPaths,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the plan record"))
            .map_err(WriteError::Database)?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;

        tx.execute(
            "DELETE FROM job_step_plan_paths
             WHERE job_id = ?1 AND step_id = ?2 AND attempt = ?3",
            (job_id.as_str(), step_id.as_str(), attempt.number()),
        )
        .map_err(fault("clearing this run's previous plan"))
        .map_err(WriteError::Database)?;

        tx.execute(
            "INSERT INTO job_step_plans (job_id, step_id, attempt, declared_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (job_id, step_id, attempt)
                 DO UPDATE SET declared_at = excluded.declared_at",
            (
                job_id.as_str(),
                step_id.as_str(),
                attempt.number(),
                at.as_str(),
            ),
        )
        .map_err(fault("writing a step's declared plan"))
        .map_err(WriteError::Database)?;

        for (ordinal, path) in paths.paths().iter().enumerate() {
            tx.execute(
                "INSERT INTO job_step_plan_paths (job_id, step_id, attempt, ordinal, path)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    job_id.as_str(),
                    step_id.as_str(),
                    attempt.number(),
                    ordinal as i64,
                    path.as_str(),
                ],
            )
            .map_err(fault("writing a declared path"))
            .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the plan record"))
            .map_err(WriteError::Database)
    }

    /// Every plan this Job's steps declared, **in the order they declared
    /// them**.
    ///
    /// Declaration order rather than step order, because step ids sort
    /// alphabetically and a list that read `handoff` before `implement` would
    /// be claiming a workflow order the wire already carries elsewhere. The
    /// instant is Fleet's own clock and two may legitimately tie, so the key
    /// breaks the tie rather than leaving the order up to the file.
    ///
    /// **The header decides**, and paths are attached to it. Asking the path
    /// rows alone could not tell a step that declared nothing from one that
    /// declared it would touch nothing.
    pub fn step_plans(&self, job_id: &JobId) -> Result<Vec<DeclaredPlan>, LoadJobError> {
        let mut plans: Vec<DeclaredPlan> = self
            .collect(
                "SELECT step_id, attempt, declared_at FROM job_step_plans
                 WHERE job_id = ?1 ORDER BY declared_at, step_id, attempt",
                job_id,
                "reading a job's declared plans",
                |row| {
                    Ok(DeclaredPlan {
                        step_id: StepId::new(string(row, "step_id")?),
                        attempt: attempt_of(row)?,
                        declared_at: Timestamp::from_rfc3339(string(row, "declared_at")?),
                        paths: DeclaredPaths::nothing(),
                    })
                },
            )
            .map_err(LoadJobError::Unreadable)?;

        let declared: Vec<(StepId, Attempt, RepoPath)> = self
            .collect(
                "SELECT step_id, attempt, path FROM job_step_plan_paths
                 WHERE job_id = ?1 ORDER BY step_id, attempt, ordinal",
                job_id,
                "reading a job's declared paths",
                |row| {
                    Ok((
                        StepId::new(string(row, "step_id")?),
                        attempt_of(row)?,
                        RepoPath::new(string(row, "path")?),
                    ))
                },
            )
            .map_err(LoadJobError::Unreadable)?;

        // A linear pass over the plans per path would be quadratic only in the
        // number of runs that declared, which is bounded by the steps a Job
        // has. Building an index over that would be the second structure
        // `attempt::grouped` refuses for the same size of list.
        for (step_id, attempt, path) in declared {
            if let Some(plan) = plans
                .iter_mut()
                .find(|plan| plan.step_id == step_id && plan.attempt == attempt)
            {
                let mut paths = plan.paths.paths().to_vec();
                paths.push(path);
                plan.paths = DeclaredPaths::of(paths);
            }
        }
        Ok(plans)
    }
}

/// The run column, refused rather than read as a first attempt when it is zero.
fn attempt_of(row: &rusqlite::Row<'_>) -> Result<Attempt, RowError> {
    let number: i64 = row
        .get("attempt")
        .map_err(crate::row::column("job_step_plans", "attempt"))?;
    u32::try_from(number)
        .ok()
        .and_then(Attempt::stored)
        .ok_or_else(|| RowError::MalformedColumn {
            table: "job_step_plans",
            column: "attempt",
            detail: "an attempt is one-based and this is not".to_string(),
        })
}
