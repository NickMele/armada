//! What the act does to the step underneath the Job, and to what that step has
//! spent.
//!
//! **A redirect is not a restart**, and the two readings that separate them are
//! whether a stopped step is handed back and where the wall clock stands after.
//! A step the gate stopped stays stopped, because handing it back would be a
//! person's sentence silently re-running a step the gate had ended.
//!
//! The clock follows from that. A healthy Drone's step has been running for the
//! whole time the Drone was healthy and a person speaking to it does not make
//! that untrue — and nothing caps how often a person may redirect, so a reading
//! that went back to zero would be a step held under its ceiling for ever by
//! being spoken to. A step that was handed back does start again, because that
//! Drone stood idle at the escalation.

use std::time::Duration;

use core_model::{EscalationTrigger, JobStatus, StepId, StepLevelTrigger, StepState, StepTarget};

use crate::tests::redirect::{
    a_drone_that_answers, a_fleet_with, advice, refused, started, step_state, Fixture, IMPLEMENT,
};
use crate::tests::tmp::TempDir;

/// How long the slot says the step has been running. **The reading the
/// wall-clock tripwire is taken from** — `crate::converging::tripped` asks the
/// slot this question and nothing else, so asking it here is asking what the
/// tripwire would see.
async fn running_for(fleet: &Fixture) -> Duration {
    let now = fleet.now();
    let held = fleet.the_only_slot().await;
    let slot = held.lock().await;
    slot.as_ref()
        .map(|at| at.running_for(&now))
        .expect("a Drone in the slot")
}

/// **The restart this act must never quietly become.** A gate stops a step
/// before the Job escalates over it, so a `running` Job holds a `stopped` step
/// for an instant — and a redirect arriving in that instant must leave it
/// stopped. Handing it back would be a person's sentence silently re-running a
/// step the gate had ended, which is `docs/concepts/job.md`'s
/// *"a redirect that respawns is a restart that threw away the session"* one
/// rung shallower.
#[tokio::test]
async fn a_redirect_does_not_unfreeze_a_step_under_a_job_nobody_escalated() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_step(
            &record,
            &StepId::new(IMPLEMENT),
            StepTarget::Stopped(
                StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
            ),
        )
        .await
        .unwrap();

    let after = fleet.redirect(&job, &advice()).await.unwrap();

    assert_eq!(after.status(), JobStatus::Running);
    assert_eq!(
        step_state(&fleet, &job).await,
        StepState::Stopped,
        "the gate stopped this step and only the gate's own recovery may start it"
    );
}

/// **The clock a redirect must not put back.** A healthy Drone's step has been
/// running for the whole time the Drone was healthy, and a person speaking to
/// it does not make that untrue — so the wall clock the tripwire reads carries
/// straight across the redirect.
///
/// Nothing caps how often a person may redirect, so a reading that went back to
/// zero here would be a step held under its ceiling for ever by being spoken
/// to. The reading is taken off the slot rather than driven to the ceiling
/// because the ceiling is `converging`'s subject and this is about the number
/// it reads.
#[tokio::test]
async fn a_redirect_into_a_healthy_drone_does_not_put_the_step_clock_back() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;

    let before = running_for(&fleet).await;
    assert!(
        before >= Duration::from_secs(5),
        "the step has run long enough for the reading to mean something: {before:?}"
    );
    fleet.redirect(&job, &advice()).await.unwrap();

    let after = running_for(&fleet).await;
    assert!(
        after >= before,
        "the step has run since it started, not since somebody typed: {after:?} < {before:?}"
    );
}

/// **And the stopped step still gets it back**, which is the half that was
/// always right: that Drone stood idle at the escalation, so what the wall
/// clock counts does begin again when a person hands the step over.
#[tokio::test]
async fn a_redirect_onto_a_stopped_step_starts_the_step_clock_again() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let before = running_for(&fleet).await;
    fleet.redirect(&job, &advice()).await.unwrap();

    let after = running_for(&fleet).await;
    assert!(
        after < before,
        "the step's work begins again, so what it spent begins again: {after:?} >= {before:?}"
    );
}
