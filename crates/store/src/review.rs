//! The review Fleet composed at a Job's gate — the same one it writes into a
//! pull request, where this Job has one.
//!
//! **Columns on `jobs`, not a table**, exactly the reason
//! [`crate::delivery::Delivery`]'s own header gives: at most one row per Job,
//! written at the gate and never queried except beside the Job it belongs to.
//!
//! **Written once per gate, and overwritten at the next one.** A Job that
//! passes back through the same or a later gate — a redispatch, a retry — gets
//! a fresh composition each time; nothing here accumulates a history, which is
//! `crate::note`'s rule for the same reason: the review area draws the Job's
//! *current* gate, not an archive of every one it ever stopped at.

use core_model::JobId;

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// Version 47 — the review composed at a Job's gate, in the four parts it is
/// made of.
///
/// Beside the change it makes, like [`V21`](crate::delivery::V21): `schema.rs`
/// is at the 900 lines the gate refuses at.
///
/// Four nullable columns and no backfill, written and cleared together: a Job
/// that has not yet reached a gate, and a Job that reached one before this
/// shipped, are the same absence.
pub(crate) const V47: &str = r#"
ALTER TABLE jobs ADD COLUMN review_why TEXT;
ALTER TABLE jobs ADD COLUMN review_outcome TEXT;
ALTER TABLE jobs ADD COLUMN review_evidence TEXT;
ALTER TABLE jobs ADD COLUMN review_risks TEXT;
"#;

/// The review Fleet composed, one builder's text in the four sections it is
/// assembled from. Headings are not stored — a surface draws its own labels
/// around each part, and the Markdown a pull request carries adds them back at
/// render time. See `fleet::review`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Review {
    /// Why the change was needed — the brief, in the requester's own words.
    pub why: Option<String>,
    /// What the Job's worktree changed, as far as a diff can say it.
    pub outcome: Option<String>,
    /// Every step and every Check that ran against it, with its outcome.
    pub evidence: Option<String>,
    /// What nothing checked, and what the base carries that this Job did not
    /// write.
    pub risks: Option<String>,
}

impl Review {
    /// Whether there is anything here worth serving.
    ///
    /// **All four or none.** [`Store::record_review`] writes every field
    /// together, so a row can never hold one section without the other three —
    /// this is one flag rather than four for the same reason
    /// [`crate::delivery::Delivery::is_empty`] is one.
    pub fn is_empty(&self) -> bool {
        self.why.is_none()
            && self.outcome.is_none()
            && self.evidence.is_none()
            && self.risks.is_none()
    }
}

impl Store {
    /// Write the review Fleet composed at this Job's current gate.
    ///
    /// **All four fields, including `None`**, for
    /// [`Store::record_delivery`](crate::delivery::Delivery)'s reason: a method
    /// that could only set would leave clearing to a second spelling of the
    /// same `UPDATE`, and a Job re-entering an earlier gate on a restart must
    /// not read the far gate's words beside this one's.
    pub fn record_review(&mut self, job_id: &JobId, review: &Review) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET review_why = ?2, review_outcome = ?3, \
                 review_evidence = ?4, review_risks = ?5 WHERE job_id = ?1",
                (
                    job_id.as_str(),
                    review.why.as_deref(),
                    review.outcome.as_deref(),
                    review.evidence.as_deref(),
                    review.risks.as_deref(),
                ),
            )
            .map_err(fault("recording the review composed at this Job's gate"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Read the review Fleet composed at this Job's current gate. Every field
    /// absent is a Job that has not reached a gate yet, and every Job written
    /// before this shipped.
    pub fn review_for(&self, job_id: &JobId) -> Result<Review, LoadJobError> {
        self.conn
            .query_row(
                "SELECT review_why, review_outcome, review_evidence, review_risks \
                 FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok(Review {
                        why: row.get(0)?,
                        outcome: row.get(1)?,
                        evidence: row.get(2)?,
                        risks: row.get(3)?,
                    })
                },
            )
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(Review::default()),
                other => Err(LoadJobError::Unreadable(RowError::Database(fault(
                    "reading the review composed at this Job's gate",
                )(
                    other
                )))),
            })
    }
}
