//! What a Job is called within its Manifest, and the one column that counts.
//!
//! **The store allocates it because only the store can see what is taken.** A
//! number is a fact about a table — the next one is one past the largest in
//! this Manifest — and nothing above here can answer that without asking. The
//! unique index is what makes the answer true rather than merely intended.
//!
//! Not global. A person works in one repository at a time, and *job 12* is
//! shorter than anything that has to be unique across all of them.
//!
//! `crates/core-model/src/job/handle.rs` turns the number and the title into
//! the handle a branch, a worktree and every path under `.armada/` is named by.

use crate::error::{fault, LoadJobError};
use crate::open::Store;
use core_model::{JobNumber, ManifestId};

/// Version 36 — the number a person calls a Job by.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 lines the gate refuses at.
///
/// **Backfilled, not left null.** Every Job gets a number, including the ones
/// written before this, so the record is total and nothing downstream has to
/// draw a Job that has no name. The order is creation order within each
/// Manifest, with the id breaking a tie — two Jobs of one proposal are minted
/// in the same millisecond, so ties are the ordinary case here rather than the
/// rare one.
///
/// `DEFAULT 0` is what `ALTER TABLE` requires of a `NOT NULL` column and is
/// never a value any row keeps: the `UPDATE` below covers every row, and the
/// index after it would refuse a second 0 in one Manifest if it did not.
pub(crate) const V36: &str = r#"
ALTER TABLE jobs ADD COLUMN number INTEGER NOT NULL DEFAULT 0;

UPDATE jobs SET number = (
    SELECT count(*) FROM jobs AS earlier
    WHERE earlier.owner_manifest_id = jobs.owner_manifest_id
      AND (earlier.created_at < jobs.created_at
           OR (earlier.created_at = jobs.created_at AND earlier.job_id <= jobs.job_id))
);

CREATE UNIQUE INDEX jobs_number_within_a_manifest ON jobs (owner_manifest_id, number);
"#;

impl Store {
    /// The number the next Job in this Manifest takes.
    ///
    /// **Read under the same lock as the insert that uses it**, which is what
    /// makes it an allocation rather than a guess. Two callers reading before
    /// either writes would both be handed the same number, and the unique index
    /// would refuse the second — a refusal is the right failure, and holding
    /// the lock is what stops it happening at all.
    ///
    /// **`max + 1`, never `count + 1`.** A number is not reused when a Job is
    /// forgotten, the way a closed issue keeps its own: counting rows would
    /// hand the next Job a number that had already been a branch, a worktree
    /// and a pull request.
    pub fn next_job_number(&self, manifest: &ManifestId) -> Result<JobNumber, LoadJobError> {
        let largest: i64 = self
            .conn
            .query_row(
                "SELECT coalesce(max(number), 0) FROM jobs WHERE owner_manifest_id = ?1",
                [manifest.as_str()],
                |row| row.get(0),
            )
            .map_err(fault("reading the largest job number"))
            .map_err(LoadJobError::Database)?;
        Ok(JobNumber::carried(largest as u32 + 1))
    }
}
