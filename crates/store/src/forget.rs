//! Forgetting a Job — whole, or not at all.
//!
//! # There is no method here that deletes a row
//!
//! [`Store::forget_job`] takes a [`JobId`] and removes the Job with every row
//! beneath it. There is no `delete_event`, no `delete_step` and no predicate a
//! caller supplies, because each of those is a way to leave a Job standing over
//! a history that no longer explains it — and the fold would then read back a
//! Job that never happened.
//!
//! The database holds the same rule from underneath: `job_events` refuses a
//! delete while its Job row exists. See [`schema::MIGRATIONS`]'s fourth entry.
//!
//! # One transaction, and foreign keys deferred inside it
//!
//! The Job row goes first so the trigger lets the events go, which is a state
//! that violates the foreign key for as long as the transaction is open.
//! `defer_foreign_keys` moves that check to the commit, where both halves are
//! gone. Nothing outside the transaction ever sees the half-forgotten shape.
//!
//! [`schema::MIGRATIONS`]: crate::schema::MIGRATIONS

use core_model::JobId;

use crate::error::{fault, WriteError};
use crate::open::Store;

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
        for (table, into) in [
            ("job_events", &mut removed.events),
            ("job_steps", &mut removed.steps),
            ("job_write_targets", &mut removed.write_targets),
            ("job_manifests", &mut removed.manifests),
            ("job_step_checks", &mut removed.step_checks),
            ("job_attachments", &mut removed.attachments),
        ] {
            *into = tx
                .execute(&format!("DELETE FROM {table} WHERE job_id = ?1"), (id,))
                .map_err(fault("removing the rows beneath a job"))
                .map_err(WriteError::Database)?;
        }

        tx.commit()
            .map_err(fault("committing the forget"))
            .map_err(WriteError::Database)?;
        Ok(removed)
    }
}
