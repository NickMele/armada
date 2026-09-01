//! Forgetting a Job — whole, or not at all.
//!
//! **There is no method here that deletes a row.** [`Store::forget_job`] takes
//! a [`JobId`] and removes the Job with every row beneath it. There is no
//! `delete_event`, no `delete_step` and no predicate a caller supplies, because
//! each of those is a way to leave a Job standing over a history that no longer
//! explains it — and the fold would then read back a Job that never happened.
//! The database holds the same rule from underneath: `job_events` refuses a
//! delete while its Job row exists, per [`schema::MIGRATIONS`]'s fourth entry.
//!
//! **One transaction, with foreign keys deferred inside it.** The Job row goes
//! first so the trigger lets the events go, which violates the foreign key for
//! as long as the transaction is open; `defer_foreign_keys` moves that check to
//! the commit, where both halves are gone. Nothing outside the transaction ever
//! sees the half-forgotten shape.
//!
//! **The tables are asked for, not listed.** "Every row beneath it" is read out
//! of the file's own catalog by [`tables_pointing_at_a_job`] each time, because
//! the three times this was a list in a loop it was a list three tables out of
//! date. See that function for what it cost.
//!
//! [`schema::MIGRATIONS`]: crate::schema::MIGRATIONS

use core_model::JobId;

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::schema::tables_pointing_at_a_job;

/// What forgetting one Job removed.
///
/// Counted rather than assumed: a Job with no events is a Job the log cannot
/// explain, and a caller printing these numbers is what makes that visible
/// instead of silent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Forgotten {
    /// Whether there was a Job row at all. `false` means the id named nothing.
    pub existed: bool,
    pub events: usize,
    pub steps: usize,
    pub write_targets: usize,
    pub manifests: usize,
    /// Rows recording what each declared Check did.
    pub step_checks: usize,
    /// Files handed to the Job at proposal time.
    pub attachments: usize,
    /// What the Judge answered, one row per criterion per run of a step.
    pub step_judgments: usize,
    /// What the gaming check flagged, one row per pattern per run of a step.
    pub step_gaming_flags: usize,
    /// What each run of a step submitted, one row per run.
    pub step_evidence: usize,
    /// The header saying a Job's footprint was recorded. One row, or none.
    pub footprint: usize,
    /// The files that footprint held, one row each.
    pub footprint_files: usize,
    /// The header saying one run of one step declared a plan. One row per run
    /// that declared.
    pub step_plans: usize,
    /// The paths those plans named, one row each.
    pub step_plan_paths: usize,
    /// What each Drone of the Job spent, one row per Drone.
    pub drone_spend: usize,
    /// Rows removed from a table this build has no field for.
    ///
    /// Always zero today, and a test says so. It exists because the delete is
    /// derived from the file's own catalog while these fields are written by
    /// hand: a table added tomorrow is emptied correctly the moment it is
    /// created, and until somebody names it here its rows are counted in a
    /// lump rather than not counted at all. An undercount is the thing this
    /// struct exists to prevent.
    pub other: usize,
}

impl Forgotten {
    /// Where one table's removed rows are counted, or `None` for a table no
    /// field here names.
    ///
    /// Kept as a lookup rather than written into the loop so a test can ask the
    /// same question of every table in the schema and fail on the first one
    /// this does not answer for.
    pub(crate) fn count_of(&mut self, table: &str) -> Option<&mut usize> {
        Some(match table {
            "job_events" => &mut self.events,
            "job_steps" => &mut self.steps,
            "job_write_targets" => &mut self.write_targets,
            "job_manifests" => &mut self.manifests,
            "job_step_checks" => &mut self.step_checks,
            "job_attachments" => &mut self.attachments,
            "job_step_judgments" => &mut self.step_judgments,
            "job_step_gaming_flags" => &mut self.step_gaming_flags,
            "job_step_evidence" => &mut self.step_evidence,
            "job_footprint" => &mut self.footprint,
            "job_footprint_files" => &mut self.footprint_files,
            "job_step_plans" => &mut self.step_plans,
            "job_step_plan_paths" => &mut self.step_plan_paths,
            "job_drone_spend" => &mut self.drone_spend,
            _ => return None,
        })
    }
}

impl Store {
    /// Remove one Job and everything recorded beneath it.
    ///
    /// An id naming no Job is not a failure: it comes back with `existed`
    /// false, because "there is nothing here" is the state the caller asked
    /// for.
    pub fn forget_job(&mut self, job_id: &JobId) -> Result<Forgotten, WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the forget"))
            .map_err(WriteError::Database)?;

        // Scoped to this transaction by SQLite itself, and reset at its end.
        tx.pragma_update(None, "defer_foreign_keys", "ON")
            .map_err(fault("deferring foreign keys"))
            .map_err(WriteError::Database)?;

        let id = job_id.as_str();
        // The Job row first. The append-only trigger below it reads the
        // presence of this row, so nothing else may go before it.
        let existed = tx
            .execute("DELETE FROM jobs WHERE job_id = ?1", (id,))
            .map_err(fault("removing the job row"))
            .map_err(WriteError::Database)?
            > 0;
        let mut removed = Forgotten {
            existed,
            ..Forgotten::default()
        };
        // Read inside the transaction that is about to use it, and read at all
        // rather than listed: see `schema::tables_pointing_at_a_job` for the
        // three tables a list forgot. The names are the file's own catalog
        // entries and cannot be bound as parameters, identifiers never being
        // bindable in SQL.
        let tables = tables_pointing_at_a_job(&tx)
            .map_err(fault("asking which tables point at a job"))
            .map_err(WriteError::Database)?;
        for table in &tables {
            let rows = tx
                .execute(&format!("DELETE FROM {table} WHERE job_id = ?1"), (id,))
                .map_err(fault("removing the rows beneath a job"))
                .map_err(WriteError::Database)?;
            match removed.count_of(table) {
                Some(into) => *into += rows,
                None => removed.other += rows,
            }
        }

        tx.commit()
            .map_err(fault("committing the forget"))
            .map_err(WriteError::Database)?;
        Ok(removed)
    }
}
