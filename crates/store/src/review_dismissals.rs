//! The findings a person dismissed from a Job's review, and why. #907.
//!
//! One row per finding for the Job, whichever run of the review raised it, so the next
//! pass is told what was ruled out.

use core_model::{Dismissal, JobId, Timestamp};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 63 — a person's dismissals of a review's findings.
///
/// **Nothing to backfill.** Nothing could dismiss a finding before this.
pub(crate) const V63: &str = r#"
CREATE TABLE job_review_dismissals (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    finding TEXT NOT NULL,
    reason  TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (job_id, finding)
) STRICT;
"#;

impl Store {
    /// Keep a dismissal. Dismissing the same finding again replaces the reason.
    pub fn record_dismissal(
        &mut self,
        job_id: &JobId,
        dismissal: &Dismissal,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_review_dismissals (job_id, finding, reason, at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (job_id, finding)
                 DO UPDATE SET reason = excluded.reason, at = excluded.at",
                rusqlite::params![
                    job_id.as_str(),
                    dismissal.finding,
                    dismissal.reason,
                    at.as_str()
                ],
            )
            .map(|_| ())
            .map_err(fault("writing a dismissal"))
            .map_err(WriteError::Database)
    }

    /// Every finding dismissed on a Job, in the order they were dismissed.
    pub fn dismissals(&self, job_id: &JobId) -> Result<Vec<Dismissal>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading dismissals")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT finding, reason FROM job_review_dismissals
                 WHERE job_id = ?1 ORDER BY at, finding",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| {
                Ok(Dismissal {
                    finding: row.get(0)?,
                    reason: row.get(1)?,
                })
            })
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)
    }
}
