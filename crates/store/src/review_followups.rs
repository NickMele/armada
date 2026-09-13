//! What a person made of a review's findings: a Job queued behind the Job, or an issue. #906.
//!
//! One row per finding and kind for the Job, so the finding can show what it became.

use core_model::{Became, FollowUp, JobId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 66 — what a person made of a review's findings.
///
/// **Nothing to backfill.** Nothing could follow a finding up before this.
pub(crate) const V66: &str = r#"
CREATE TABLE job_review_followups (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    finding TEXT NOT NULL,
    kind    TEXT NOT NULL CHECK (kind IN ('queued', 'issue')),
    target  TEXT NOT NULL,
    at      TEXT NOT NULL,
    PRIMARY KEY (job_id, finding, kind)
) STRICT;
"#;

impl Store {
    /// Keep what a finding became. The same kind again replaces where it points.
    pub fn record_followup(
        &mut self,
        job_id: &JobId,
        followup: &FollowUp,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let (kind, target) = match &followup.became {
            Became::Queued { job } => ("queued", job.as_str().to_string()),
            Became::Issue { url } => ("issue", url.clone()),
        };
        self.conn
            .execute(
                "INSERT INTO job_review_followups (job_id, finding, kind, target, at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (job_id, finding, kind)
                 DO UPDATE SET target = excluded.target, at = excluded.at",
                rusqlite::params![job_id.as_str(), followup.finding, kind, target, at.as_str()],
            )
            .map(|_| ())
            .map_err(fault("writing a follow-up"))
            .map_err(WriteError::Database)
    }

    /// What each finding on a Job became, in the order a person made them.
    pub fn followups(&self, job_id: &JobId) -> Result<Vec<FollowUp>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading follow-ups")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT finding, kind, target FROM job_review_followups
                 WHERE job_id = ?1 ORDER BY at, finding",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| {
                let kind: String = row.get(1)?;
                let target: String = row.get(2)?;
                Ok(FollowUp {
                    finding: row.get(0)?,
                    became: match kind.as_str() {
                        "queued" => Became::Queued {
                            job: JobId::carried(Ulid::carried(target)),
                        },
                        _ => Became::Issue { url: target },
                    },
                })
            })
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)
    }
}
