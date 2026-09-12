//! One finished run as its own directory keeps it — **whichever owner's it
//! is** — and the two shapes it is answered in.
//!
//! **One record on disk and two on the wire.** A Job's run and the checkout's
//! are the same act in different trees, and the fields that differ are the few
//! only a Job has. Keeping one shape under `.armada/runs/` is what lets
//! [`records`](super::records), [`in_flight`](super::in_flight) and
//! [`running`](super::running) each know one type; the split is made where the
//! answer leaves Fleet and nowhere before it.
//!
//! **A run written before this file existed reads back unchanged.** Every
//! field it had is here under the same name, and `job_id` is the one that
//! became optional: a record with one is a Job's, a record without one is the
//! checkout's. Nothing had to be migrated and nothing had to be guessed.

use ipc::{ChangedFile, CheckoutRunRecord, Instant, JobId, RunRecord};
use serde::{Deserialize, Serialize};

/// A run's `run.json`. Not an `ipc` type: it is Fleet's own format for its own
/// directory, and the wire shapes below are projections of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) id: String,
    /// The Job whose worktree this ran in. **Absent is the main checkout** —
    /// the one field that says which owner a directory belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) job_id: Option<JobId>,
    pub(crate) name: String,
    pub(crate) command: String,
    /// Job-only: the checkout has no diff of its own to narrow against.
    #[serde(default)]
    pub(crate) narrowed: bool,
    /// Job-only: the checkout has one Manifest and never a frozen second.
    #[serde(default)]
    pub(crate) worktree_version: bool,
    /// Job-only, for [`worktree_version`](Record::worktree_version)'s reason.
    #[serde(default)]
    pub(crate) frozen: bool,
    pub(crate) required: Vec<String>,
    pub(crate) started_at: Instant,
    pub(crate) ended_at: Instant,
    pub(crate) duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i32>,
    pub(crate) expect_exit_code: i64,
    pub(crate) ended: String,
    pub(crate) stopped: bool,
    pub(crate) changed: Vec<ChangedFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) changed_unreadable: Option<String>,
    /// Job-only: no Drone ever works in the main checkout.
    #[serde(default)]
    pub(crate) shared_with_drone: bool,
    /// Where the tree before the run is kept. Absent: nothing to undo from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) snapshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) undone_at: Option<Instant>,
    pub(crate) log: String,
}

impl Record {
    /// This record as the Job's sheet reads it. `None` where the directory is
    /// the checkout's, which is how a run id from one owner reaches no answer
    /// belonging to the other.
    pub(crate) fn of_job(&self) -> Option<RunRecord> {
        Some(RunRecord {
            id: self.id.clone(),
            job_id: self.job_id.clone()?,
            name: self.name.clone(),
            command: self.command.clone(),
            narrowed: self.narrowed,
            worktree_version: self.worktree_version,
            frozen: self.frozen,
            required: self.required.clone(),
            started_at: self.started_at.clone(),
            ended_at: self.ended_at.clone(),
            duration_ms: self.duration_ms,
            exit_code: self.exit_code,
            expect_exit_code: self.expect_exit_code,
            ended: self.ended.clone(),
            stopped: self.stopped,
            changed: self.changed.clone(),
            changed_unreadable: self.changed_unreadable.clone(),
            shared_with_drone: self.shared_with_drone,
            snapshot: self.snapshot.clone(),
            undone_at: self.undone_at.clone(),
            log: self.log.clone(),
        })
    }

    /// This record as the Manifest surface reads it.
    ///
    /// **`undoable` is the snapshot, answered rather than handed over.** What
    /// a surface in the main checkout has to know is whether Undo is there to
    /// offer, because this tree holds a person's own uncommitted work — the
    /// reference it would be restored from is Fleet's business.
    pub(crate) fn of_checkout(&self) -> CheckoutRunRecord {
        CheckoutRunRecord {
            id: self.id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            required: self.required.clone(),
            started_at: self.started_at.clone(),
            ended_at: self.ended_at.clone(),
            duration_ms: self.duration_ms,
            exit_code: self.exit_code,
            expect_exit_code: self.expect_exit_code,
            ended: self.ended.clone(),
            stopped: self.stopped,
            changed: self.changed.clone(),
            changed_unreadable: self.changed_unreadable.clone(),
            undoable: self.snapshot.is_some() && self.undone_at.is_none(),
            undone_at: self.undone_at.clone(),
            log: self.log.clone(),
        }
    }
}

/// A run that has started and not finished, as [`Rehearsals`] holds it.
///
/// [`Record`]'s reasoning one moment earlier: one shape while the run is
/// Fleet's own, projected when it is answered.
///
/// [`Rehearsals`]: super::in_flight::Rehearsals
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Underway {
    pub(crate) id: String,
    pub(crate) name: String,
    /// The command line that is running, narrowed where it was.
    pub(crate) command: String,
    pub(crate) narrowed: bool,
    pub(crate) started_at: Instant,
}

impl Underway {
    pub(crate) fn of_job(&self, job_id: &JobId) -> ipc::RunUnderway {
        ipc::RunUnderway {
            id: self.id.clone(),
            job_id: job_id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            narrowed: self.narrowed,
            started_at: self.started_at.clone(),
        }
    }

    pub(crate) fn of_checkout(&self) -> ipc::CheckoutRunUnderway {
        ipc::CheckoutRunUnderway {
            id: self.id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            started_at: self.started_at.clone(),
        }
    }
}
