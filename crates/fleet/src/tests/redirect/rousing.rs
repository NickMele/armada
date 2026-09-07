//! The wait between the send and the Drone's answer, and what ends it.
//!
//! **The Job is not moved on the send**, so a person can tell a Drone that took
//! the advice from one past taking it. What ends the wait is the Drone's own
//! turn and nothing else: a Drone that never wakes leaves the Job honestly
//! escalated, and a progress heartbeat is not a turn — the narrow reading is the
//! whole guard against a Job that flickers back to `running` on a Drone still
//! wedged inside the call it was wedged in when the vigil caught it.
//!
//! The last case is the one with no wait at all. Where a step stopped, both
//! machines move on the send, because a step left `stopped` is one no submission
//! could advance.

use std::time::Duration;

use adapter_traits::DroneEvent;
use core_model::{Actor, EscalationTrigger, JobId, JobStatus, StepId, StepState, TransitionReason};
use store::Moved;
use testkit::FakeHarness;

use crate::tests::redirect::{
    a_drone_that_answers, a_drone_that_never_wakes, a_fleet_with, advice, called, heard, refused,
    stalled, step_state, until_roused, Fixture, IMPLEMENT,
};
use crate::tests::tmp::TempDir;

/// A Drone whose only answer is the harness's progress heartbeat — the
/// `tool_progress` line a long tool emits every thirty seconds, which this
/// vocabulary has no variant for. **A Drone that never stopped working, not one
/// that read anything**, which is why the vigil counts it as not-silent.
fn a_drone_that_only_ticks() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo HEARTBEAT; done",
        ],
    )
    .reading("BUSY", called())
    .reading(
        "HEARTBEAT",
        vec![DroneEvent::Unrecognised {
            kind: String::from("tool_progress"),
        }],
    )
}

/// Who the Job's last status move is recorded against.
async fn last_mover(fleet: &Fixture, job: &JobId) -> Actor {
    fleet
        .store()
        .lock()
        .await
        .events_for(job)
        .expect("the Job's log")
        .iter()
        .rev()
        .find_map(|event| match event.moved() {
            Moved::Job { .. } => Some(event.actor()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        })
        .expect("a Job that has moved at least once")
}

/// The owner's rule: *"if the job is stalled it should be unstalled and back to
/// running if the drone starts turning again"*. The evidence is the Drone's own
/// turn, and the Job waits for it.
#[tokio::test]
async fn the_job_returns_to_running_only_once_the_drone_turns() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    // Turns before anybody says anything move nothing: the watcher is cold on a
    // Job no redirect is outstanding on.
    for _ in 0..5 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
    }
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );

    fleet.redirect(&job, &advice()).await.unwrap();

    let roused = until_roused(&fleet).await;
    assert_eq!(roused.job, job);
    assert_eq!(roused.step, StepId::new(IMPLEMENT));
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "the Drone turned, so the Job is not stalled any more"
    );
    // **The person's move, recorded as theirs.** Fleet chose the instant; the
    // decision to take this Job out of `escalated` was a person's, and the
    // registry says a person is who acts on an escalated Job.
    assert_eq!(
        last_mover(&fleet, &job).await,
        Actor::Human,
        "Fleet did not un-escalate a Job of its own accord"
    );

    // The wait is over: a Drone that keeps answering does not move it again.
    for _ in 0..5 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
    }
}

/// **A redirect landing on a Drone which never wakes leaves the Job honestly
/// escalated.** That is the whole reason it is not moved on the send: a person
/// must be able to tell a Drone that took the advice from one past taking it.
#[tokio::test]
async fn a_drone_that_never_wakes_leaves_the_job_escalated() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_never_wakes());
    let job = stalled(&fleet, &home).await;

    fleet
        .redirect(&job, &advice())
        .await
        .expect("the pipe took the write");

    for _ in 0..40 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert_eq!(
        fleet.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Stalled)),
        "the escalation the vigil wrote still stands, and is still what a person reads"
    );
}

/// **A heartbeat is not a turn.** The narrow reading is the whole guard against
/// a Job that flickers back to `running` on a Drone still wedged inside the
/// call it was wedged in when the vigil caught it.
#[tokio::test]
async fn a_progress_heartbeat_does_not_bring_the_job_back() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_only_ticks());
    let job = stalled(&fleet, &home).await;
    let before = heard(&fleet).await;

    fleet.redirect(&job, &advice()).await.unwrap();

    for _ in 0..40 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // The pipe carried something back, so this is a reading rather than a Drone
    // that was never there.
    assert!(
        heard(&fleet).await > before,
        "the heartbeat never arrived, so nothing was told apart"
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
}

/// **The path that already worked, unchanged.** Where a step stopped, something
/// is frozen underneath the Job, so both machines move on the send and nothing
/// waits: a step left `stopped` is one no submission could advance.
#[tokio::test]
async fn a_step_that_stopped_is_handed_back_on_the_send() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let after = fleet.redirect(&job, &advice()).await.unwrap();

    assert_eq!(after.status(), JobStatus::Running);
    assert_eq!(step_state(&fleet, &job).await, StepState::Running);
    // Nothing is outstanding, so the Drone answering is an ordinary turn rather
    // than the thing the Job was waiting on.
    for _ in 0..20 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}
