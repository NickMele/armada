//! A workflow that declares `structure: loop`, run end to end.
//!
//! **The claim `#263` closes on**: a step whose verdict routes backwards
//! re-enters an earlier step, the return increments `iteration_count` rather
//! than `retry_count`, and a step that reaches its `iteration_cap` escalates as
//! `loop_cap`.
//!
//! Every case here reaches the gate through a real dispatch, the way
//! `tests::reviewing` and `tests::sending_back` do. Nothing stands a Job at a
//! gate by hand, because what is under test is whether the loop closes at all.
//!
//! # What each case pins
//!
//! Where the work goes, whose count the pass is charged to, what the cap does
//! when it is spent — and, in the last case, the one thing this wave did not
//! close: what state the *gate* step is left in between passes, which is what
//! stops a second pass reaching it.

use core_model::{IllegalStepTransition, JobStatus, StepId, StepState};
use testkit::FakeWorkProduct;

use crate::resume::Redirection;
use crate::tests::daemon::{
    a_fleet_running_a_loop, a_proposal_for, diff_evidence, note_evidence, worktree_directory,
};
use crate::tests::reviewing::Fixture;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::Adrift;

fn drafted() -> StepId {
    StepId::new("implement")
}

fn gate() -> StepId {
    StepId::new("summarise")
}

fn said(words: &str) -> Redirection {
    Redirection::saying(words).expect("a note with something in it")
}

/// Dispatch, work the first step, work the gate step, and stand at the gate.
async fn at_the_loop_s_gate(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal_for("fix the off-by-one", "fixture-loop"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("it dispatches");
    walked_to_the_gate(fleet).await;
    job.id().clone()
}

/// One pass: the draft is submitted and advances on its own, the gate step is
/// submitted, and the Job holds for a person.
async fn walked_to_the_gate(fleet: &Fixture) {
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    fleet.turn().await.expect("the first step's gate runs");
    submitted_by_the_one(fleet, note_evidence())
        .await
        .expect("the Drone reports its summary");
    fleet.turn().await.expect("the human gate runs");
}

/// **The loop closes.** `request_changes` at a gate that routes backwards puts
/// the Job on the step the workflow names, and the next Drone starts there.
///
/// The contrast is `tests::sending_back`, where the same call on a linear
/// workflow re-queues at the same step. Which of the two happens is the step's
/// own declaration and nothing else.
#[tokio::test]
async fn a_verdict_that_routes_backwards_puts_the_job_on_the_earlier_step() {
    let home = TempDir::new();
    let fleet = a_fleet_running_a_loop(&home, FakeWorkProduct::changed(&["src/log.rs"]), 5);
    let job_id = at_the_loop_s_gate(&fleet, &home).await;
    let held = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(held.current_step_id(), Some(&gate()));

    let sent_back = fleet
        .request_changes(
            &job_id,
            &said("the plan skips the migration — draft it again"),
        )
        .await
        .expect("the loop has room for another pass");

    assert_eq!(
        sent_back.current_step_id(),
        Some(&drafted()),
        "the next Drone goes where the work goes, which is back at the draft"
    );
    assert_eq!(
        sent_back.step(&drafted()).map(|step| step.state()),
        Some(StepState::Running),
        "the step that had advanced is being worked again"
    );
    assert_eq!(
        sent_back.step(&gate()).map(|step| step.state()),
        Some(StepState::Running),
        "and the gate did not move: a step at a human gate stays `running`"
    );
}

/// **Whose count it is.** The pass is charged to the gate that emitted the
/// verdict and not to the step it routes back to — the reading
/// `docs/journeys/triage-queue.md` settles, and the only one that survives two
/// loops sharing a target step.
#[tokio::test]
async fn the_pass_is_charged_to_the_gate_and_not_to_the_step_redone() {
    let home = TempDir::new();
    let fleet = a_fleet_running_a_loop(&home, FakeWorkProduct::changed(&["src/log.rs"]), 5);
    let job_id = at_the_loop_s_gate(&fleet, &home).await;
    fleet
        .request_changes(&job_id, &said("again, with the migration"))
        .await
        .expect("the loop has room");

    let store = fleet.store();
    let store = store.lock().await;
    assert_eq!(
        store
            .step_iteration(&job_id, &gate())
            .expect("counted")
            .number(),
        2,
        "the gate that sent it back is on its second pass"
    );
    assert_eq!(
        store
            .step_iteration(&job_id, &drafted())
            .expect("counted")
            .number(),
        1,
        "and the step being redone has emitted nothing, so its own cap is untouched"
    );
    assert_eq!(
        store
            .step_spent(&job_id, &drafted())
            .expect("counted")
            .number(),
        1,
        "the return opened a fresh retry budget — a redo as designed is not a retry"
    );
    assert_eq!(
        store
            .step_attempt(&job_id, &drafted())
            .expect("counted")
            .number(),
        2,
        "while the coordinate its records are filed under kept climbing"
    );
}

/// **A cap of one makes the first `request_changes` the last.** Nothing failed,
/// which is the whole content of `loop_cap`: the step stops, the Job escalates,
/// and the retry budget was never touched.
#[tokio::test]
async fn a_spent_cap_escalates_as_loop_cap_and_not_as_a_failure() {
    let home = TempDir::new();
    let fleet = a_fleet_running_a_loop(&home, FakeWorkProduct::changed(&["src/log.rs"]), 1);
    let job_id = at_the_loop_s_gate(&fleet, &home).await;

    let stopped = fleet
        .request_changes(&job_id, &said("one more draft, please"))
        .await
        .expect("the verdict is answered even where the loop is spent");

    assert_eq!(stopped.status(), JobStatus::Escalated);
    assert_eq!(
        stopped.step(&gate()).map(|step| step.state()),
        Some(StepState::Stopped),
        "the gate is where the loop ran out, so the gate is the step that stopped"
    );
    assert_eq!(
        stopped.step(&drafted()).map(|step| step.state()),
        Some(StepState::Advanced),
        "and the draft was not re-entered: there was no pass left to enter it for"
    );
    assert_eq!(
        stopped.current_step_id(),
        Some(&gate()),
        "the cursor stays where the Job stopped, which is what a person is shown"
    );
}

/// **The second pass does not close, and this is where it stops.** A loop return
/// re-enters the routed-to step and leaves the *gate* step `running` — which is
/// the shape `#263` settled, because the alternatives were a seventh
/// `StepState` (a wire break, and Bridge to its /v0 lifeboat) and `stopped`,
/// whose registry meaning is "retries spent".
///
/// The consequence was not settled with it. When the redone draft advances,
/// `crate::dispatch` walks forward onto the gate step and finds it already
/// `running`, and there is no self-edge: `advanced -> running` is the loop's
/// own edge and `running -> running` is not an edge at all. So a pass is a
/// return and a redraft, and the *second* arrival at the gate has nowhere to be
/// recorded.
///
/// **What the four states leave** is why this is a decision and not an
/// oversight. `advanced` writes `passed` over a gate the person declined;
/// `stopped` and `retrying` both say something failed and `retrying` needs a
/// trigger there is none of; `not_started` would re-enter cleanly and renders a
/// worked step as never run. Each is a different lie, and which one the record
/// should tell is the owner's.
///
/// Asserted rather than left to be discovered: a refusal named in a test is
/// where the next person starts, and a `#[ignore]` is where they do not.
#[tokio::test]
async fn a_second_pass_cannot_re_enter_the_gate_step_yet() {
    let home = TempDir::new();
    let fleet = a_fleet_running_a_loop(&home, FakeWorkProduct::changed(&["src/log.rs"]), 5);
    let job_id = at_the_loop_s_gate(&fleet, &home).await;

    let sent_back = fleet
        .request_changes(&job_id, &said("draft it again"))
        .await
        .expect("the first return is inside a cap of five");
    assert_eq!(sent_back.current_step_id(), Some(&drafted()));

    // The redraft is submitted and clears its own gate. The forward walk onto
    // the step that sent it back is what has nowhere to go.
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the second draft is reported");
    let refused = fleet.turn().await;
    assert!(
        matches!(
            refused,
            Err(Adrift::IllegalStepMove(IllegalStepTransition::NoSuchEdge {
                from: StepState::Running,
                to: StepState::Running,
                ..
            }))
        ),
        "the gap is the gate step's own state between passes, and it is this one: {refused:?}"
    );
}
