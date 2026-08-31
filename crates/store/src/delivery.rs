//! What a finished Job's branch came to, and the three columns it waits in.
//!
//! **The record existed and was thrown away.** `fleet::delivery` has committed,
//! pushed and opened a pull request since before this file, and put the result
//! in a map that `take_delivered` *drains* into the Drone's closing turn. So the
//! one reader was a process about to exit, and a person opening the Job an hour
//! later was told "Fleet does not open one yet" — which had stopped being true.
//!
//! **Columns on `jobs`, not a table.** This is at most one row per Job, written
//! once, and never queried except beside the Job it belongs to. `jobs.branch`
//! is the same shape for the same reason.
//!
//! **The column is the authority for its field**, as [`crate::note`]'s is: no
//! event carries a commit or a URL, so there is nothing to fold and
//! [`crate::read`] reads these straight back.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 21 — what a Job's branch came to after its last step.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 the gate refuses at.
///
/// Three nullable columns and no backfill. A Job written before this has none,
/// and null already means "nothing was recorded" — which is exactly true of
/// every Job that finished while the result was being dropped.
pub(crate) const V21: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_commit TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pushed TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pull_request TEXT;
"#;

/// What a Job's branch came to, as the record holds it.
///
/// **Three independent absences, not one.** A commit with no push is a
/// repository with no remote; a push with no pull request is a forge nothing on
/// this machine can open one against. Both are ordinary, and folding them into
/// one optional would make a surface unable to tell either from a Job that
/// finished before any of this was written down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Delivery {
    /// The commit Fleet wrote over the Job's work.
    pub commit: Option<String>,
    /// What the push came to, in the adapter's own words.
    pub pushed: Option<String>,
    /// The address a person clicks. Absent where none was opened.
    pub pull_request: Option<String>,
}

impl Delivery {
    /// Whether there is anything here worth serving.
    ///
    /// A Job that finished before version 21, and one whose delivery failed
    /// before the commit, are the same answer: nothing to say.
    pub fn is_empty(&self) -> bool {
        self.commit.is_none() && self.pushed.is_none() && self.pull_request.is_none()
    }
}

impl Store {
    /// Write what the Job's branch came to.
    ///
    /// **Once, at the end**, from `fleet::landing` after the delivery it
    /// describes. Every field is written including `None`, for
    /// [`record_redirect_waiting`](Store::record_redirect_waiting)'s reason: a
    /// method that could only set would leave clearing to a second spelling of
    /// the same `UPDATE`, and a redispatched Job delivering again must not
    /// inherit the last run's URL.
    pub fn record_delivery(
        &mut self,
        job_id: &core_model::JobId,
        delivery: &Delivery,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET delivery_commit = ?2, delivery_pushed = ?3, \
                 delivery_pull_request = ?4 WHERE job_id = ?1",
                (
                    job_id.as_str(),
                    delivery.commit.as_deref(),
                    delivery.pushed.as_deref(),
                    delivery.pull_request.as_deref(),
                ),
            )
            .map_err(fault("recording what the branch came to"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Read what a Job's branch came to. Every field absent is every Job that
    /// has not finished, and every Job that finished before version 21.
    pub fn delivery_for(
        &self,
        job_id: &core_model::JobId,
    ) -> Result<Delivery, crate::error::LoadJobError> {
        self.conn
            .query_row(
                "SELECT delivery_commit, delivery_pushed, delivery_pull_request \
                 FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok(Delivery {
                        commit: row.get(0)?,
                        pushed: row.get(1)?,
                        pull_request: row.get(2)?,
                    })
                },
            )
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(Delivery::default()),
                other => Err(crate::error::LoadJobError::Unreadable(
                    crate::error::RowError::Database(fault("reading what the branch came to")(
                        other,
                    )),
                )),
            })
    }
}
