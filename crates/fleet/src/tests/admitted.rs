//! The two acts an approval used to be one of.
//!
//! `Fleet::approve` queues and stops there, because a dispatch that ran inside
//! the request that asked for it died when the client stopped waiting —
//! `crate::daemon::Fleet::approve` carries the chain, and `#428` is the issue.
//! A served Fleet has `keep_turning` to admit what is queued; a case here has
//! no loop, so it says so.
//!
//! **Its own module rather than one more fixture in `tests::daemon`**, which is
//! at the line the gate refuses a file over.

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Approve `job_id` and admit it, and answer with the Job that came back.
///
/// **`admit_next` and not `turn`**, deliberately. A turn also settles, reaps,
/// notices a merge and sweeps for worktrees — a case that only wants a Drone on
/// a Job would be paying for six watchers it is not about, and a scripted
/// `FakeVcs` would be answering questions the case never asked. That the turn
/// admits is proved in `crate::tests::unattended` and `crate::tests::serving`,
/// where it is the subject.
pub async fn dispatched(
    fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    job_id: &core_model::JobId,
) -> Result<core_model::Job, Adrift> {
    fleet.approve(job_id).await?;
    fleet.admit_next().await?;
    fleet.load(job_id).await
}
