//! What a person reads while a redirect is outstanding, as `GET /jobs/:job_id`
//! serves it.
//!
//! **The wait is the only thing on the wire that says the act happened.** On a
//! healthy Job nothing else moves at all, and on an escalated one the status is
//! the same before and after — so a redirect that is waiting and one that never
//! arrived would be the same Job drawn the same way.
//!
//! It is read here off the wire rather than off the record, because in the
//! record it is nowhere: `get_job` is the read a second window and a reload
//! both make, and a fact Bridge remembered instead would die with the window.

use core_model::{JobId, JobStatus};

use crate::tests::daemon::a_proposal;
use crate::tests::redirect::{
    a_drone_that_answers, a_drone_that_never_wakes, a_fleet_with, advice, job_moves, refused,
    stalled, started, until_roused, Fixture,
};
use crate::tests::tmp::TempDir;

/// One Job, as `GET /jobs/:job_id` serves it. The wire answer and not the
/// record, because the wait is on the first and in the second is nowhere.
async fn detail(fleet: &Fixture, job: &JobId) -> ipc::JobDetail {
    api::Queries::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists")
}

/// **What the screen could not say before.** The instruction went down the pipe,
/// the Job is still escalated, and the wait rides beside that status — read
/// twice, because a reload has to find the same answer the first read did.
#[tokio::test]
async fn a_waiting_redirect_is_on_the_wire_and_a_reread_finds_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_never_wakes());
    let job = stalled(&fleet, &home).await;

    assert!(
        detail(&fleet, &job).await.redirecting.is_none(),
        "nothing has been said to this Drone yet"
    );

    fleet.redirect(&job, &advice()).await.unwrap();

    let waiting = detail(&fleet, &job).await;
    assert!(
        waiting.redirecting.is_some(),
        "the instruction went down the pipe and the Job is still escalated"
    );
    // The Job did not become something else. The wait rides beside the status,
    // as a Judge call in flight rides beside a step's state.
    assert_eq!(waiting.job.status.domain(), JobStatus::Escalated);
    assert_eq!(
        detail(&fleet, &job).await.redirecting,
        waiting.redirecting,
        "a second read of the same Job says the same thing, and says it the same way"
    );
}

/// The two cases read differently, and **what decides is whether a step
/// stopped**: where one had, both machines moved on the send, so the Job is
/// `running` on the way out of the call and there is nothing to wait for.
#[tokio::test]
async fn a_redirect_onto_a_stopped_step_leaves_nothing_waiting() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    fleet.redirect(&job, &advice()).await.unwrap();

    let after = detail(&fleet, &job).await;
    assert_eq!(after.job.status.domain(), JobStatus::Running);
    assert!(
        after.redirecting.is_none(),
        "the step was handed back on the send, so nothing is outstanding"
    );
}

/// **The wait ends where the Job moves.** The Drone turns, the Job goes back to
/// `running`, and the fact about the last act goes with it — a sentence that
/// outlived the wait would be the same ambiguity pointed the other way.
#[tokio::test]
async fn the_wait_is_over_when_the_drone_turns() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    fleet.redirect(&job, &advice()).await.unwrap();
    assert!(detail(&fleet, &job).await.redirecting.is_some());

    until_roused(&fleet).await;

    let after = detail(&fleet, &job).await;
    assert_eq!(after.job.status.domain(), JobStatus::Running);
    assert!(
        after.redirecting.is_none(),
        "the Drone answered, so nothing is waiting on it"
    );
}

/// The wait rides on a healthy Job the same way, and **it is the only thing
/// that says anything happened** — this Job is `running` before the send and
/// `running` after, so a person with no `redirecting` on the wire cannot tell a
/// Drone that was steered from one that was not.
#[tokio::test]
async fn a_healthy_job_carries_the_wait_and_moves_nothing_when_it_is_answered() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let moved = job_moves(&fleet, &job).await;

    fleet.redirect(&job, &advice()).await.unwrap();
    assert!(detail(&fleet, &job).await.redirecting.is_some());

    let roused = until_roused(&fleet).await;
    assert_eq!(roused.job, job);

    let after = detail(&fleet, &job).await;
    assert_eq!(after.job.status.domain(), JobStatus::Running);
    assert!(
        after.redirecting.is_none(),
        "the Drone answered, so nothing is waiting on it"
    );
    assert_eq!(
        job_moves(&fleet, &job).await,
        moved,
        "the answer landed on a Job that was never held for it"
    );
}

/// One Job's wait is not another's. The slot holds one Drone, and a Job opened
/// beside it must not draw somebody else's — the reading `Aloft::on` takes of a
/// Judge call, asked of the same slot.
#[tokio::test]
async fn a_job_that_was_not_redirected_carries_no_wait() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_never_wakes());
    let job = stalled(&fleet, &home).await;
    let other = fleet
        .propose(a_proposal("a second Job nobody has spoken to"))
        .await
        .expect("a proposal")
        .id()
        .clone();

    fleet.redirect(&job, &advice()).await.unwrap();

    assert!(detail(&fleet, &other).await.redirecting.is_none());
}
