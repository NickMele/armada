//! What one Job alone may spend, and the column it is written in.
//!
//! `core_model::Job::cost_cap_micros` says why a Job carries a ceiling of its
//! own at all and why absent means *defer to the tier above*. This is the
//! column and the one write.
//!
//! **The column is the authority for its field**, in exactly the sense
//! `jobs.branch` and `jobs.redirect_waiting` are: no event carries a cap, so
//! there is nothing to fold and [`crate::read`] reads it straight back.

use core_model::Job;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 32 — a cost ceiling for one Job, over the one every Job shares.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 the gate refuses at.
///
/// One nullable column, and nothing is backfilled. Null is what every Job
/// written before this already meant — nobody has decided anything about this
/// Job's money — and a backfill to the setting in force would freeze today's
/// figure onto every historical row, so that changing the setting would stop
/// reaching the Jobs it is supposed to govern.
pub(crate) const V32: &str = r#"
ALTER TABLE jobs ADD COLUMN cost_cap_micros INTEGER;
"#;

impl Store {
    /// Write whatever ceiling the Job is holding, including none.
    ///
    /// **One method for setting it and for clearing it**, like
    /// [`record_redirect_waiting`](Store::record_redirect_waiting) and unlike
    /// [`record_branch`](Store::record_branch): nothing clears a cap today, and
    /// a method that could only set one would leave the clearing to a second
    /// spelling of the same `UPDATE` the day somebody wants it back.
    ///
    /// **No event**, for those two methods' reason: nothing in the log
    /// describes a ceiling, so this column is its field's authority and the
    /// rebuild reads it rather than folding it.
    ///
    /// The figure crosses SQLite as a signed 64-bit integer, which is what that
    /// file has. A cap large enough to overflow it is a number no budget means,
    /// and the act that writes one is bounded long before here.
    pub fn record_cost_cap(&mut self, job: &Job) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET cost_cap_micros = ?2 WHERE job_id = ?1",
                (
                    job.id().as_str(),
                    job.cost_cap_micros().map(|micros| micros as i64),
                ),
            )
            .map_err(fault("recording what this Job alone may spend"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }
        Ok(())
    }
}
