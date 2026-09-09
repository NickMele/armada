//! Which comments on a Job's pull request have already been handed to a Drone.
//!
//! # A record, because the forge has no memory of what Armada did
//!
//! A comment stays on a pull request and reads the same on every sweep. So a
//! Job whose Drone ran against three of them and satisfied two meets all three
//! again the next time somebody opens the choice, and the two already worked
//! look exactly like the one that was not.
//!
//! **The forge's own handle is the key**, `adapter_traits::Remark::id`. An
//! author and a time cannot identify a comment and neither can the text: one
//! edited between the reading and the press stops being itself, which is
//! precisely the case this has to survive.
//!
//! **The handle and nothing else is stored.** No author, no time and above all
//! no body — `fleet::under_review` is explicit that a remark's text travels the
//! road ending at a Drone's prompt and does not go through a record to get
//! there.
//!
//! **It points at `jobs`, so `forget_job` sweeps it.** The opposite rule —
//! `commit_checks`, which deliberately has no foreign key — is for a record two
//! Jobs share, and this belongs to exactly one.
use std::collections::BTreeSet;

use core_model::{JobId, Timestamp};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// Version 31 — the comments on a Job's pull request that have reached a Drone.
///
/// Beside the change it makes, like [`V29`](crate::proving::V29): `schema.rs`
/// is at the 900 lines the gate refuses at.
///
/// **The pair is the key**, so writing the same handle twice for one Job is a
/// conflict rather than a second row — the whole content of this table is
/// "already", and a duplicate would be a fact stated twice.
///
/// `remark_id` is the forge's own text and is never parsed. It is compared and
/// nothing else.
pub(crate) const V31: &str = r#"
CREATE TABLE job_remarks_taken_up (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    remark_id   TEXT NOT NULL,
    at          TEXT NOT NULL,
    PRIMARY KEY (job_id, remark_id)
) STRICT;
"#;

impl Store {
    /// Write down that these comments have been handed to a Drone.
    ///
    /// **Called after the Job has moved and never before.** A press can be
    /// refused — no worktree left, a note already waiting, a gate that routed
    /// somewhere else — and a set recorded ahead of the move would burn a
    /// person's comments on an act that did not happen. The other order risks
    /// a crash between the move and this write, which costs one comment offered
    /// twice; the refusals are the common case and the crash is not.
    ///
    /// **No event.** Nothing in the log describes this, for
    /// [`record_redirect_waiting`](Store::record_redirect_waiting)'s reason:
    /// the table is the authority for its own fact and the rebuild reads it
    /// rather than folding it.
    ///
    /// Silent on a handle already written. Taking the same comment up twice is
    /// what the caller refuses; arriving here it would mean the caller's check
    /// and this table disagree, and the row already says what this call came to
    /// say.
    pub fn record_remarks_taken_up(
        &mut self,
        job_id: &JobId,
        remarks: &[String],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let written = self
            .conn
            .transaction()
            .map_err(fault("recording the comments taken up"))
            .map_err(WriteError::Database)?;
        for remark in remarks {
            written
                .execute(
                    "INSERT OR IGNORE INTO job_remarks_taken_up (job_id, remark_id, at) \
                     VALUES (?1, ?2, ?3)",
                    (job_id.as_str(), remark, at.as_str()),
                )
                .map_err(fault("recording one comment taken up"))
                .map_err(WriteError::Database)?;
        }
        written
            .commit()
            .map_err(fault("recording the comments taken up"))
            .map_err(WriteError::Database)
    }

    /// Every comment on this Job's pull request that has already reached a
    /// Drone.
    ///
    /// **A set, because every caller asks the same question of it** — is this
    /// one already spent — and a `Vec` would have each of them write the search.
    /// A Job that has taken none up answers with an empty set, which is the same
    /// answer as a Job with no pull request: neither has spent a comment.
    pub fn remarks_taken_up(&self, job_id: &JobId) -> Result<BTreeSet<String>, LoadJobError> {
        let mut asking = self
            .conn
            .prepare("SELECT remark_id FROM job_remarks_taken_up WHERE job_id = ?1")
            .map_err(unreadable)?;
        let rows = asking
            .query_map([job_id.as_str()], |row| row.get::<_, String>(0))
            .map_err(unreadable)?;
        let mut taken = BTreeSet::new();
        for remark in rows {
            taken.insert(remark.map_err(unreadable)?);
        }
        Ok(taken)
    }
}

fn unreadable(cause: rusqlite::Error) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(fault(
        "reading which comments have already reached a Drone",
    )(cause)))
}
