//! A store damaged the way a migration damages one, for the tests that have to
//! recover from it.
//!
//! **Compiled only under the `wreckage` feature**, which nothing but `testkit`
//! turns on. The shipping binary therefore has no method that damages a store,
//! and `rusqlite` stays scoped to this crate rather than reaching the crate
//! whose test needs the damage.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Take the frozen workflow off a stored Job.
    ///
    /// This is the state V7 left four real rows in: present, still naming their
    /// Job and their Manifest, and unrebuildable. Reproducing it is the only
    /// way to test what clears it.
    pub fn unfreeze_a_jobs_workflow(&mut self, job_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE jobs SET workflow = NULL WHERE job_id = ?1",
                (job_id,),
            )
            .map_err(fault("unfreezing a workflow"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}
