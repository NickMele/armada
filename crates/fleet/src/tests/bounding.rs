//! A person's act on an escalated Job does not push Fleet past its own cap.
//!
//! # The hole these close
//!
//! `#50` made `settings.concurrency-cap` a real bound, and `Slots::room` is the
//! whole of it — consulted by `admit_next` and by `queued_reason`. Two acts put
//! a Job back to work without going through admission: `restart_step` and
//! `override_verdict` each opened a slot of their own and spawned into it. So a
//! person could stand on a Fleet already at its cap and add another Drone to
//! it, one restart at a time, and nothing anywhere said so.
//!
//! # What is asserted, and what would pass without meaning anything
//!
//! **The place has to be spent by a different Job across the assertion.** An
//! act that re-queued into an empty Fleet is admitted inline a line later and
//! looks exactly like the act that spawned for itself. Every case here holds
//! one Job's Drone open while the act is taken on a second, so the deferral is
//! observable.
//!
//! And the act must **land** rather than be refused: `#50`'s follow-on chose
//! the edge over refusing the act while the cap is spent, so a person is told
//! the Job is waiting and not told to press the button again later.

use api::Daemon;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepTarget,
    Target,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::daemon::{Fittings, Fleet};
use crate::overruling::Overruling;
use crate::slots::Concurrency;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";
const SUMMARISE: &str = "summarise";

/// Two steps, gated on nothing, so nothing but the acts under test moves a Job
/// — and an override has somewhere to advance to.
fn two_ungated_steps() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: IMPLEMENT,
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: SUMMARISE,
            label: "Summarise",
            evidence_type: Some("facts_note"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// A Fleet that may work exactly one Job, whose Drones hold their input open.
///
/// **One is the point.** With room to spare the deferral and the overrun are
/// indistinguishable from outside.
fn bounded_at_one(home: &TempDir) -> Fixture {
    let mut fittings: Fittings<FakeHarness, FakeVcs, FakeWorkProduct> =
        fittings(home, FakeWorkProduct::changed(&["src/parse.rs"]));
    fittings.workflows = one(two_ungated_steps());
    fittings.judge = std::sync::Arc::new(FakeJudge::that_fails("no model is asked here"));
    fittings.concurrency = Concurrency::of(1);
    Fleet::assembled(fittings)
}

/// Approve a Job and let it take the place, with the worktree its dispatch
/// wants.
async fn working(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet
        .propose(a_proposal(title))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("released to run");
    job.id().clone()
}

/// A Job that ran, stopped at its first step, escalated over it, and whose
/// Drone has gone. **The state all four acts on an escalated Job start from**,
/// reached the way `crate::tests::restarting` reaches it — what is under test
/// here is the bound, not the refusal.
///
/// It has a branch, which is what makes its re-admission a re-admission rather
/// than a fresh dispatch.
async fn escalated_with_no_drone(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = working(fleet, home, title).await;
    let slot = fleet.slot_of(&job).await.expect("its dispatch took a slot");
    let mut held = slot.lock().await;
    fleet
        .stood_down(&job, &mut held)
        .await
        .expect("the Drone ends and its exit is recorded");
    drop(held);

    let record = fleet.load(&job).await.expect("the Job reads");
    let record = fleet
        .move_step(
            &record,
            &StepId::new(IMPLEMENT),
            StepTarget::Stopped(
                StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
            ),
        )
        .await
        .expect("the step stops");
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::GateFailure),
            Actor::Fleet,
        )
        .await
        .expect("the Job escalates over it");
    job
}

/// What the Board says about a Job that has not started.
async fn queued_reason(fleet: &Fixture, job: &JobId) -> Option<String> {
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .job
        .queued_reason
        .map(|why| why.as_wire().to_string())
}

/// One step's state, off the record.
async fn state_of(fleet: &Fixture, job: &JobId, step: &str) -> Option<StepState> {
    fleet
        .load(job)
        .await
        .expect("the Job reads")
        .step(&StepId::new(step))
        .map(|row| row.state())
}

/// **The hole itself.** A restart on a Fleet whose one place is spent does not
/// take a second one.
#[tokio::test]
async fn a_restart_does_not_spend_a_place_the_bound_has_not_got() {
    let home = TempDir::new();
    let fleet = bounded_at_one(&home);
    let stopped = escalated_with_no_drone(&fleet, &home, "make the parser take it").await;
    // Taken *after* the first Job stood down, so it is this Job holding the
    // place across the assertion rather than a leftover.
    let holding = working(&fleet, &home, "the Job in the place").await;
    assert_eq!(fleet.working_on().await, vec![holding.clone()]);

    let after = fleet
        .restart_step(&stopped)
        .await
        .expect("the act lands whatever the cap holds");

    assert_eq!(
        after.status(),
        JobStatus::Queued,
        "the person's decision is taken and the Job waits for room"
    );
    assert_eq!(
        fleet.working_on().await,
        vec![holding],
        "and no second Drone: the bound is one and one Job is being worked"
    );
    assert_eq!(
        queued_reason(&fleet, &stopped).await.as_deref(),
        Some("waiting_on_resources"),
        "a person is told why it is waiting rather than told to press it again"
    );
}

/// The other half: the restarted step is entered when the place frees, and the
/// Drone is put on the step the person named.
#[tokio::test]
async fn the_restarted_step_runs_once_the_place_frees() {
    let home = TempDir::new();
    let fleet = bounded_at_one(&home);
    let stopped = escalated_with_no_drone(&fleet, &home, "make the parser take it").await;
    let holding = working(&fleet, &home, "the Job in the place").await;
    fleet.restart_step(&stopped).await.expect("the act lands");

    assert_eq!(
        state_of(&fleet, &stopped, IMPLEMENT).await,
        Some(StepState::Stopped),
        "the step has not entered `running`, because no Drone is in it yet — \
         `store::attempt` counts entries into `running` as runs"
    );

    fleet.kill_job(&holding).await.expect("the place frees");

    assert_eq!(
        fleet.working_on().await,
        vec![stopped.clone()],
        "admission took the Job a person restarted"
    );
    assert_eq!(
        fleet.load(&stopped).await.expect("the Job reads").status(),
        JobStatus::Running,
    );
    assert_eq!(
        state_of(&fleet, &stopped, IMPLEMENT).await,
        Some(StepState::Running),
        "and the step the person restarted is the one being worked",
    );
}

/// An override on a Fleet at its cap records the person's verdict and waits.
///
/// **The verdict is not deferred and the Drone is** — the two halves of the
/// act land in different places, which is the whole of the design.
#[tokio::test]
async fn an_override_records_the_verdict_now_and_waits_for_a_place() {
    let home = TempDir::new();
    let fleet = bounded_at_one(&home);
    let stopped = escalated_with_no_drone(&fleet, &home, "widen the bound instead").await;
    let holding = working(&fleet, &home, "the Job in the place").await;

    let after = fleet
        .override_verdict(
            &stopped,
            &Overruling::saying("the criterion is mis-stated").expect("a reason"),
        )
        .await
        .expect("the act lands whatever the cap holds");

    assert_eq!(after.status(), JobStatus::Queued);
    assert_eq!(
        state_of(&fleet, &stopped, IMPLEMENT).await,
        Some(StepState::Advanced),
        "the person's verdict is on the record at the moment they gave it"
    );
    assert_eq!(
        state_of(&fleet, &stopped, SUMMARISE).await,
        Some(StepState::NotStarted),
        "and the next step has not opened a run nothing is in"
    );
    assert_eq!(
        fleet.working_on().await,
        vec![holding],
        "no second Drone against a bound of one"
    );
    assert_eq!(
        queued_reason(&fleet, &stopped).await.as_deref(),
        Some("waiting_on_resources"),
    );
}

/// And the next step is the one the freed place goes to.
#[tokio::test]
async fn the_step_after_an_override_runs_once_the_place_frees() {
    let home = TempDir::new();
    let fleet = bounded_at_one(&home);
    let stopped = escalated_with_no_drone(&fleet, &home, "widen the bound instead").await;
    let holding = working(&fleet, &home, "the Job in the place").await;
    fleet
        .override_verdict(
            &stopped,
            &Overruling::saying("the criterion is mis-stated").expect("a reason"),
        )
        .await
        .expect("the act lands");

    fleet.kill_job(&holding).await.expect("the place frees");

    assert_eq!(fleet.working_on().await, vec![stopped.clone()]);
    assert_eq!(
        state_of(&fleet, &stopped, SUMMARISE).await,
        Some(StepState::Running),
        "the Drone is on the step after the one a person cleared"
    );
    assert_eq!(
        state_of(&fleet, &stopped, IMPLEMENT).await,
        Some(StepState::Advanced),
        "and the overridden step stayed where the override put it",
    );
}

/// **The freeze rule was not widened, and this is what says so.**
///
/// Building the `escalated -> queued` edge needed one step move to be legal
/// beneath `escalated` — the override, because it *is* the person's verdict and
/// nothing may make it for them. `core_model::step_machine` admits that one
/// move as a named predicate rather than by adding `escalated` to
/// `ADVANCING_STATUSES`, and the difference between those two changes is
/// exactly this case: every other step move beneath `escalated` is still
/// refused, so nothing can resume, retry or advance a step under a Job that is
/// parked for a person.
#[tokio::test]
async fn only_an_override_moves_a_step_beneath_an_escalated_job() {
    let home = TempDir::new();
    let fleet = bounded_at_one(&home);
    let stopped = escalated_with_no_drone(&fleet, &home, "make the parser take it").await;
    let record = fleet.load(&stopped).await.expect("the Job reads");

    let resumed = fleet
        .move_step(&record, &StepId::new(IMPLEMENT), StepTarget::Running)
        .await;

    assert!(
        matches!(
            resumed,
            Err(crate::Adrift::IllegalStepMove(
                core_model::IllegalStepTransition::StepsAreFrozen { .. }
            ))
        ),
        "a stopped step is not resumed beneath `escalated` — the spawn does \
         that, beneath `running`: {resumed:?}"
    );
    assert_eq!(
        state_of(&fleet, &stopped, IMPLEMENT).await,
        Some(StepState::Stopped),
        "and the refusal moved nothing"
    );
}

/// **A Job a person re-queued cannot be swept back out by `#48`.**
///
/// `strand_dependents` escalates every `queued` Job whose upstream ended badly,
/// and these two acts now leave a Job at `queued` where they used to leave it
/// `running`. That is only safe because a Job that has already run was admitted
/// once, which means every peer it depends on had already reached a terminal
/// the coupling calls clear — and no terminal has an outbound edge.
///
/// Asserted rather than argued: the argument is about a walk over the edge
/// table, and a Job swept off the queue by a turn would undo a person's act
/// silently.
#[tokio::test]
async fn a_turn_does_not_strand_a_job_a_person_put_back_in_the_queue() {
    let home = TempDir::new();
    let fleet = bounded_at_one(&home);
    let stopped = escalated_with_no_drone(&fleet, &home, "make the parser take it").await;
    let holding = working(&fleet, &home, "the Job in the place").await;
    fleet.restart_step(&stopped).await.expect("the act lands");

    let turned = fleet.turn().await.expect("the loop turns");

    assert!(
        turned.stranded.is_empty(),
        "nothing swept the restarted Job off the queue: {:?}",
        turned.stranded
    );
    assert_eq!(
        fleet.load(&stopped).await.expect("the Job reads").status(),
        JobStatus::Queued,
        "it is still waiting for the place, and still a person's decision"
    );
    assert_eq!(fleet.working_on().await, vec![holding]);
}
