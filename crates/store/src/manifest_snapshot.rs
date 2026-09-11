//! What the Manifest read at Job creation, kept whole.
//!
//! **Text, not fields.** `armada.yml` is `config`'s to parse and `config` is
//! not this crate's to depend on — `store` sits under it, not beside it — so
//! nothing here knows a Check from a Command. What is kept is the file's own
//! bytes as Fleet read them the moment the Job was created, and re-parsing
//! that text through the same `config::Manifest::parse` any live file goes
//! through is how a caller gets the Job's Commands, Checks and Setup back.
//! **The one property that buys**: a key `config` learns to read tomorrow is
//! captured today, with no second change here to carry it.
//!
//! **Off the [`Job`](core_model::Job) row's own fields, like
//! [`crate::allowing::when_blocked`].** Nothing in `core-model` needs to know
//! this exists — no rule reads it, no transition depends on it — so it is a
//! column queried on its own rather than a field threaded through `NewJob` and
//! every constructor and test fixture that builds one.
//!
//! **`NULL` is a Job written before this column existed**, never a Job that
//! declined a snapshot: every path that creates a Job now sets it in the same
//! breath. A caller reading `None` back is what tells a pre-migration Job from
//! one this crate lost the write for — `crate::fleet`'s to fall back on, this
//! crate's only to report honestly.

use core_model::JobId;

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 46 — the Manifest snapshot column, beside the frozen workflow it
/// sits next to in what a Job carries out of its creation.
///
/// **Nullable, and no `DEFAULT`.** Unlike [`crate::allowing::V44`]'s
/// `when_blocked`, there is no value here a Job that named none could be said
/// to have chosen — the column is either the file as it stood the instant the
/// row was written, or nothing was ever written, and a placeholder string
/// would read as the first when it is the second.
pub(crate) const V46: &str = r#"
ALTER TABLE jobs ADD COLUMN manifest_snapshot TEXT;
"#;

impl Store {
    /// The Manifest's own text, as it stood the instant this Job was created —
    /// or re-created by the one re-snapshot `docs/concepts/drone.md` names, a
    /// person-approved scope revision. `None` is a Job written before this
    /// column existed; every Job created since carries one.
    pub fn manifest_snapshot(&self, job_id: &JobId) -> Result<Option<String>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT manifest_snapshot FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => LoadJobError::NoSuchJob {
                    job_id: job_id.clone(),
                },
                other => LoadJobError::Database(fault("reading a job's manifest snapshot")(other)),
            })
    }

    /// Set what [`manifest_snapshot`](Store::manifest_snapshot) answers.
    ///
    /// **Two callers only, and this crate does not choose between them.** The
    /// dispatch path that just wrote the row calls this once, in the same
    /// breath, to turn the Manifest Fleet resolved into what every later read
    /// sees; a person-approved scope revision calls it again, later, and is
    /// the *only* other caller — `docs/concepts/drone.md`'s one re-snapshot.
    /// Nothing else may, and nothing here refuses a third caller from doing
    /// so: that discipline is `crate::fleet`'s to keep, the same way
    /// `Store::insert_job` trusts its caller for `created_at`.
    pub fn set_manifest_snapshot(
        &mut self,
        job_id: &JobId,
        snapshot: &str,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET manifest_snapshot = ?2 WHERE job_id = ?1",
                (job_id.as_str(), snapshot),
            )
            .map_err(fault("recording a job's manifest snapshot"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }
}
