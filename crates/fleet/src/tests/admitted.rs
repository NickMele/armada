//! The two acts a command used to be one of.
//!
//! **No `Commands` method dispatches**, because a dispatch that ran inside the
//! request that asked for it died when the client stopped waiting —
//! `crate::admitting::Fleet::admit_next` carries the rule, `#428` is the
//! approval and `#456` is the other six. A served Fleet has `keep_turning` to
//! admit what is queued; a case here has no loop, so it says so.
//!
//! **Which of the two a case wants is the whole of reading it.** A case about
//! what a command *does* asserts `queued` and calls nothing here — that is the
//! corrected expectation. A case about what happens *next* — a Drone on the
//! next step, a branch caught up, a note in the opening brief — asks for
//! [`started`], because the Drone it is about is a turn's and no longer the
//! command's.
//!
//! **Its own module rather than one more fixture in `tests::daemon`**, which is
//! at the line the gate refuses a file over.

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The Fleet every case here is built on.
type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// Admit whatever is queued, as the end of a turn does.
///
/// **`admit_next` and not `turn`**, deliberately. A turn also settles, reaps,
/// notices a merge and sweeps for worktrees — a case that only wants a Drone on
/// a Job would be paying for six watchers it is not about, and a scripted
/// `FakeVcs` would be answering questions the case never asked. That the turn
/// admits is proved in `crate::tests::unattended` and `crate::tests::serving`,
/// where it is the subject.
pub async fn admit(fleet: &Fixture) -> Result<Vec<core_model::JobId>, Adrift> {
    fleet.admit_next().await
}

/// Admit, then read `job_id` back — **the half of a command that moved to the
/// turn.**
///
/// The Job that comes back is ordinarily `running`, and it is the same read the
/// command used to do for itself. Where the admission is of some *other* Job —
/// a place freed by a kill or a rejection — this still names the Job the case
/// is about, because that is the one whose status the case goes on to assert.
pub async fn started(
    fleet: &Fixture,
    job_id: &core_model::JobId,
) -> Result<core_model::Job, Adrift> {
    admit(fleet).await?;
    fleet.load(job_id).await
}

/// Approve `job_id` and admit it, and answer with the Job that came back.
pub async fn dispatched(
    fleet: &Fixture,
    job_id: &core_model::JobId,
) -> Result<core_model::Job, Adrift> {
    fleet.approve(job_id).await?;
    started(fleet, job_id).await
}
