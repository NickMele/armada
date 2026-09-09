//! Giving a Job's resources back without giving up its record.
//!
//! **`armada clean` used to reach for [`Store::forget_job`], and that was the
//! defect.** `forget_job` is real deletion — the Job row, `job_events`, every
//! Check, Judgment and piece of Evidence, gone with the worktree it cleaned
//! up beside them. `#69`'s retention promise (never delete `job_events` or
//! Evidence) held everywhere except here. [`Store::retain_job`] is the other
//! half [`forget_job`](crate::Store::forget_job) never had: it clears the one
//! table that is a resource and not a record — [`crate::process`]'s Drone pid,
//! stale the moment the worktree it names is gone — and leaves the Job, its
//! log and its workflow results exactly as they were.
//!
//! **`forget_job` still means what it always meant.** A person who asks Fleet
//! to forget a Job by name still gets real deletion; this is what a sweep
//! reaches for instead, over every Job it is giving disk back for.

use core_model::{JobId, Timestamp};
use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 37 — whether a Job's disk has been given back.
///
/// Beside the change it makes, like [`crate::numbering::V36`]: `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// **Null, and no backfill.** Nothing recorded a reclaim before this column
/// existed, so every row predating it is a Job whose disk — as far as this
/// column can say — still stands, which is the honest reading rather than a
/// guess. `Store::retain_job` is the one writer.
pub(crate) const V37: &str = r#"
ALTER TABLE jobs ADD COLUMN reclaimed_at TEXT;
"#;

/// What keeping one Job's record, while giving its resources back, did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Retained {
    /// Whether there was a Job row at all. `false` means the id named
    /// nothing, the same reading [`crate::Forgotten::existed`] gives.
    pub existed: bool,
    /// Whether a live Drone process row was cleared. `false` on a Job whose
    /// Drone had already left, which is the ordinary shape by the time a
    /// sweep reaches a terminal Job.
    pub drone_process: bool,
}

impl Store {
    /// Reclaim one Job's resources, and touch nothing that is its record but
    /// the mark that says so.
    ///
    /// **Only `job_drone_process` goes, and `reclaimed_at` is stamped.** Every
    /// other table `forget_job` walks — `job_events`, the Checks, the
    /// Judgments, the Evidence, the footprint, the declared plans, the
    /// delivery columns on `jobs` itself — is workflow history, not a
    /// resource a sweep is reclaiming, and stays. An id naming no Job is not
    /// a failure, for `forget_job`'s reason: there is nothing here for either
    /// call to have taken.
    ///
    /// **`at` is the caller's clock reading, not this store's.** Every writer
    /// in this crate takes its instant as an argument rather than reading one
    /// off the machine, so a replay stays deterministic; `fleet::Clock` is
    /// where a reading is actually taken.
    pub fn retain_job(&mut self, job_id: &JobId, at: &Timestamp) -> Result<Retained, WriteError> {
        let id = job_id.as_str();
        let existed = self
            .conn
            .query_row("SELECT 1 FROM jobs WHERE job_id = ?1", (id,), |_| Ok(()))
            .optional()
            .map_err(fault("checking whether a job row exists"))
            .map_err(WriteError::Database)?
            .is_some();
        let drone_process = self
            .conn
            .execute("DELETE FROM job_drone_process WHERE job_id = ?1", (id,))
            .map_err(fault("clearing a Drone's process"))
            .map_err(WriteError::Database)?
            > 0;
        self.conn
            .execute(
                "UPDATE jobs SET reclaimed_at = ?1 WHERE job_id = ?2",
                (at.as_str(), id),
            )
            .map_err(fault("stamping a job as reclaimed"))
            .map_err(WriteError::Database)?;
        Ok(Retained {
            existed,
            drone_process,
        })
    }
}
