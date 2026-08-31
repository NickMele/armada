//! The note a boundary is holding, and the column it waits in.
//!
//! `core_model::RedirectWaiting` says why the record holds a person's words at
//! all and why the field is on the Job rather than on a step. This is the
//! column and the two writes.
//!
//! **The column is the authority for its field**, in exactly the sense
//! `jobs.branch` is: no event carries a note, so there is nothing to fold and
//! [`crate::read`] reads it straight back. That is the opposite of
//! `job_steps.assigned_drone`, which is a cache the fold fills.

use core_model::Job;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 20 — a redirect that arrived with no Drone to take it.
///
/// Beside the change it makes, like [`V19`](crate::drone::V19): `schema.rs` is
/// at the 900 the gate refuses at.
///
/// One nullable column, and nothing is backfilled: a Job written before this
/// has no note waiting, which is what null already means.
pub(crate) const V20: &str = r#"
ALTER TABLE jobs ADD COLUMN redirect_waiting TEXT;
"#;

impl Store {
    /// Write whatever note the Job is holding, including none.
    ///
    /// **One method for setting it down and for clearing it**, unlike
    /// [`record_branch`](Store::record_branch), which returns early on `None`
    /// because a branch is never unset. Here `None` is the whole of delivery:
    /// a note is cleared into the Drone that opened with it, and a method that
    /// could only set one would leave the clearing to a second spelling of the
    /// same `UPDATE`.
    ///
    /// **No event**, for `record_branch`'s reason: nothing in the log describes
    /// a note, so the column is this field's authority and the rebuild reads it
    /// rather than folding it.
    pub fn record_redirect_waiting(&mut self, job: &Job) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET redirect_waiting = ?2 WHERE job_id = ?1",
                (
                    job.id().as_str(),
                    job.redirect_waiting().map(|note| note.text()),
                ),
            )
            .map_err(fault("recording the note waiting for the next Drone"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }
        Ok(())
    }
}
