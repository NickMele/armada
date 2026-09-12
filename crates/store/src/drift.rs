//! Where a step edited outside its declared plan, kept past the turn that saw
//! it. `fleet::scope`'s live check used to only log this, which is a warning
//! nobody but a log reader ever acted on. A path seen twice keeps its first
//! instant: still touching the same file outside the plan is one fact.

use core_model::{JobId, RepoPath, StepId, Timestamp};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;
use crate::row::string;

/// Version 53 — a step edited outside its declared plan, kept for as long as
/// the Job is. Beside the table it creates, per `crate::report::V17`.
pub(crate) const V53: &str = r#"
CREATE TABLE job_scope_drift (
    job_id        TEXT NOT NULL REFERENCES jobs(job_id),
    step_id       TEXT NOT NULL,
    path          TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    PRIMARY KEY (job_id, path)
) STRICT;
"#;

/// One path a step edited outside the plan it declared, and when that was
/// first seen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeDrift {
    pub step_id: StepId,
    pub path: RepoPath,
    pub first_seen_at: Timestamp,
}

impl Store {
    /// Keep a live drift finding past the turn that saw it. Idempotent: a path
    /// already on record keeps the instant it was first seen at.
    pub fn record_scope_drift(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        paths: &[RepoPath],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        for path in paths {
            self.conn
                .execute(
                    "INSERT INTO job_scope_drift (job_id, step_id, path, first_seen_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (job_id, path) DO NOTHING",
                    (
                        job_id.as_str(),
                        step_id.as_str(),
                        path.as_str(),
                        at.as_str(),
                    ),
                )
                .map_err(fault("recording a step's live drift"))
                .map_err(WriteError::Database)?;
        }
        Ok(())
    }

    /// Every path this Job's steps have edited outside their declared plan,
    /// oldest first.
    pub fn scope_drift(&self, job_id: &JobId) -> Result<Vec<ScopeDrift>, LoadJobError> {
        self.collect(
            "SELECT step_id, path, first_seen_at FROM job_scope_drift
             WHERE job_id = ?1 ORDER BY first_seen_at, path",
            job_id,
            "reading a job's live scope drift",
            |row| {
                Ok(ScopeDrift {
                    step_id: StepId::new(string(row, "step_id")?),
                    path: RepoPath::new(string(row, "path")?),
                    first_seen_at: Timestamp::from_rfc3339(string(row, "first_seen_at")?),
                })
            },
        )
        .map_err(LoadJobError::Unreadable)
    }
}
