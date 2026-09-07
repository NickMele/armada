//! Which Job takes the act at all, and which is refused.
//!
//! **What decides is a Drone in the slot and where the Job stands**, and the
//! two are separate questions: a Job escalated with its Drone gone has nobody
//! to tell, and a Job past its step has somebody to tell and nothing left to
//! say to them. Each refusal is paired with the restart it is not, because the
//! two acts stopped sharing a predicate and must not have started to overlap.
//!
//! Two of them ask the wire rather than the enum, because a refusal that is
//! right and reported as a 500 is a refusal nobody can act on.

use std::time::Duration;

use core_model::{Actor, JobStatus, StepState, Target};
use testkit::FakeHarness;

use crate::adrift::Adrift;
use crate::tests::redirect::{
    a_drone_that_answers, a_fleet_with, advice, called, job_moves, refused, stalled, started,
    step_state, Fixture,
};
use crate::tests::tmp::TempDir;

/// A Drone that speaks once and leaves, emptying the slot under an escalation.
///
/// **It leaves after it has been told, and `crate::tests::planted` owns why**:
/// `echo BUSY` alone races `start`'s first write, and a busy machine turns this
/// fixture into a spawn that failed rather than a Drone that left.
fn a_drone_that_leaves() -> FakeHarness {
    crate::tests::planted::a_drone_that_leaves("BUSY").reading("BUSY", called())
}

/// Turn until the slot empties: a Drone that left, reaped.
async fn until_reaped(fleet: &Fixture) {
    for _ in 0..400 {
        fleet.turn().await.expect("a turn");
        if fleet.the_only_slot().await.lock().await.is_none() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never left");
}

/// **The defect, as a case.** A Drone alive on a `stalled` Job takes a
/// redirect, and took nothing before this.
#[tokio::test]
async fn a_stalled_job_with_a_live_drone_admits_a_redirect() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    let after = fleet
        .redirect(&job, &advice())
        .await
        .expect("a Drone that is there can be told something");

    // **Still escalated on the way out of the call.** Nothing about the Job has
    // moved yet, because nothing about the Drone has been shown.
    assert_eq!(after.status(), JobStatus::Escalated);
    assert_eq!(
        step_state(&fleet, &job).await,
        StepState::Running,
        "a Job-level escalation froze no step, so the redirect unfroze none"
    );
}

/// `stalled` can fire with the Drone dead too, and the answer there is the one
/// `adrift` already draws rather than a new one. **And nothing else applies
/// either**: a restart wants a stopped step and a Job-level escalation named
/// none, so what is left is a redispatch or Pilot. Unchanged by this act, and
/// asserted here so the pair is visible in one place.
#[tokio::test]
async fn a_stalled_job_whose_drone_is_gone_refuses_a_redirect() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = stalled(&fleet, &home).await;
    until_reaped(&fleet).await;

    assert!(
        matches!(
            fleet.redirect(&job, &advice()).await,
            Err(Adrift::NoDroneToRedirect { .. })
        ),
        "a redirect needs a session, and there is none"
    );
    assert!(
        matches!(
            fleet.restart_step(&job, None).await,
            Err(Adrift::NoStepStopped { .. })
        ),
        "and a restart needs a stopped step, which a Job-level escalation never named"
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
}

/// **The wire, not just the enum.** `#126`: `NoDroneToRedirect` fell through
/// the daemon's match to the 500 catch-all — an excellent message on a status
/// code that told the caller Fleet broke, when it had correctly refused.
#[tokio::test]
async fn a_redirect_with_no_drone_answers_409_over_the_wire() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = stalled(&fleet, &home).await;
    until_reaped(&fleet).await;

    let refusal = api::Commands::redirect_drone(
        &fleet,
        ipc::JobId::from(&job),
        ipc::Redirection {
            instruction: String::from("read tests/parse.rs first"),
        },
    )
    .await
    .expect_err("no Drone is there to redirect");

    assert!(matches!(refusal, api::Refusal::IllegalMove(_)));
    assert_eq!(refusal.status(), 409);
}

/// The same wire proof for `DroneStillThere`: no existing scenario reaches it,
/// so this is `refused` with a Drone that never leaves the slot, restarted
/// before anything reaps it.
#[tokio::test]
async fn a_restart_with_the_drone_still_there_answers_409_over_the_wire() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let refusal = api::Commands::restart_step(&fleet, ipc::JobId::from(&job), None)
        .await
        .expect_err("the Drone `refused` left in the slot is still there");

    assert!(matches!(refusal, api::Refusal::IllegalMove(_)));
    assert_eq!(refusal.status(), 409);
}

/// **The refusal that had to survive the split.** `interrupted` and
/// `resource_exhausted` have no stopped step *and* no live Drone; loosening the
/// predicate for the act that needed it must not have loosened it for them.
#[tokio::test]
async fn a_job_escalated_with_its_drone_gone_still_refuses_both_acts() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = started(&fleet, &home).await;
    until_reaped(&fleet).await;
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );

    assert!(matches!(
        fleet.redirect(&job, &advice()).await,
        Err(Adrift::NoDroneToRedirect { .. })
    ));
    assert!(matches!(
        fleet.restart_step(&job, None).await,
        Err(Adrift::NoStepStopped { .. })
    ));
}

/// **`#145`, as a case.** A healthy Drone working normally takes a redirect,
/// and refused one until now: `docs/concepts/drone.md` promises all of Redirect,
/// Kill and Pause on a non-escalated Drone, and the one Drone this act could not
/// reach was the one going the wrong way with nothing yet wrong.
///
/// A restart is still refused, and that pairing is the point — the two acts stop
/// sharing a predicate without starting to overlap.
#[tokio::test]
async fn a_healthy_job_takes_a_redirect_and_still_refuses_a_restart() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let moved = job_moves(&fleet, &job).await;

    let after = fleet
        .redirect(&job, &advice())
        .await
        .expect("a Drone that is working can be told something");

    assert_eq!(after.status(), JobStatus::Running);
    assert_eq!(
        step_state(&fleet, &job).await,
        StepState::Running,
        "nothing was frozen, so nothing was unfrozen"
    );
    assert_eq!(
        job_moves(&fleet, &job).await,
        moved,
        "a redirect into a healthy Drone moves the Job nowhere at all"
    );
    assert!(
        matches!(
            fleet.restart_step(&job, None).await,
            Err(Adrift::NotResumable {
                status: JobStatus::Running,
                ..
            })
        ),
        "a restart still wants a Job a person is holding"
    );
}

/// The status is still a real question, and what it catches is a Drone that
/// outlives the status which had one. A Job at `awaiting_review` has been
/// answered by its machine gates; the act there is `request_changes`, whose note
/// waits for the next Drone.
#[tokio::test]
async fn a_job_past_its_step_refuses_a_redirect_even_with_a_drone_in_the_slot() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(&record, Target::AwaitingReview, Actor::Fleet)
        .await
        .unwrap();

    assert!(
        matches!(
            fleet.redirect(&job, &advice()).await,
            Err(Adrift::NotResumable {
                status: JobStatus::AwaitingReview,
                ..
            })
        ),
        "the slot still holds a Drone, and where the Job stands is what decides"
    );
}
