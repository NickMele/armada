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
//! Where the work goes, whose count the pass is charged to, that the loop comes
//! round and is filed apart from the pass before it, and what the cap does when
//! it is spent.

use adapter_traits::WorktreeSpec;
use core_model::{EvidenceType, JobStatus, StepId, StepState};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::evidence::Call;
use crate::resume::Redirection;
use crate::tests::admitted::{dispatched, started};
use crate::tests::daemon::{
    a_proposal_for, diff_evidence, fittings, manifest, note_evidence, one, worktree_directory,
};
use crate::tests::reviewing::Fixture;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// `two_steps_gated_on_a_person`'s two steps, wired as a loop: `summarise`'s `request_changes` routes
/// back to `implement`, bounded by `cap` passes.
///
/// **Design Plan's shape, in this crate's fixture vocabulary.** The gate is on
/// the last step because that is where a loop's gate is — the step that reads
/// the work is the step that sends it back — and `implement` stays `auto` so
/// the Job walks to it the way a Job does.
fn a_loop_of_two_steps(cap: u32) -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        &format!(
            "version: 1\nworkflow_id: fixture-loop\nname: fixture\nstructure: loop\n\
             steps:\n  - id: implement\n    label: \"Implement\"\n    evidence_type: diff\n    \
             mechanical_checks:\n      - type: diff_nonempty\n    advance_gate: auto\n  - \
             id: summarise\n    label: \"Summarise\"\n    evidence_type: facts_note\n    \
             advance_gate: human_always\n    verdict_routing:\n      \
             request_changes: implement\n    iteration_cap: {cap}\n"
        ),
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture loop did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture loop did not resolve: {refused}"))
}

/// A Fleet running [`a_loop_of_two_steps`].
fn a_fleet_running_a_loop(home: &TempDir, work: FakeWorkProduct, cap: u32) -> Fixture {
    let mut fittings = fittings(home, work);
    fittings.workflows = one(a_loop_of_two_steps(cap));
    crate::daemon::Fleet::assembled(fittings)
}

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
    dispatched(&fleet, job.id()).await.expect("it dispatches");
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

/// **The second pass closes.** A return leaves the gate step `running` — the
/// shape `#263` settled, because a seventh `StepState` breaks the wire and
/// `stopped` means "retries spent" — so the forward walk after a redraft
/// arrives at a step that never left. `running -> running` is the edge that
/// records that arrival, and [`StepTarget::Revisited`] is the only thing that
/// walks it.
///
/// **The row is a boundary, not a change of state.** What turns on it is the
/// attempt: `store::attempt` counts entries into `running`, so without it the
/// second pass's checks, judgments and evidence file under the first pass's
/// ordinal — which is the defect `store::attempt` exists to close, arriving
/// from the other direction.
#[tokio::test]
async fn a_second_pass_reaches_the_gate_and_is_filed_apart_from_the_first() {
    let home = TempDir::new();
    let fleet = a_fleet_running_a_loop(&home, FakeWorkProduct::changed(&["src/log.rs"]), 5);
    let job_id = at_the_loop_s_gate(&fleet, &home).await;

    let sent_back = fleet
        .request_changes(&job_id, &said("draft it again"))
        .await
        .expect("the first return is inside a cap of five");
    assert_eq!(sent_back.current_step_id(), Some(&drafted()));
    // **The return queues and the turn spawns**, since `#456`. The redraft
    // below is a Drone's, so the case has to ask for the Drone.
    started(&fleet, &job_id)
        .await
        .expect("the turn puts a Drone back on the draft");

    // The redraft is submitted, clears its own gate, and the Job walks forward
    // onto the step that sent it back.
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the second draft is reported");
    fleet.turn().await.expect("the draft's gate runs again");

    let round = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        round.current_step_id(),
        Some(&gate()),
        "the loop came round to the step that asked for another draft"
    );
    assert_eq!(
        round.step(&drafted()).map(|step| step.state()),
        Some(StepState::Advanced)
    );
    assert_eq!(
        round.step(&gate()).map(|step| step.state()),
        Some(StepState::Running)
    );

    let store = fleet.store();
    let store = store.lock().await;
    assert_eq!(
        store
            .step_attempt(&job_id, &gate())
            .expect("counted")
            .number(),
        2,
        "the gate is on its second run, so its second reading is filed apart"
    );
    assert_eq!(
        store
            .step_spent(&job_id, &gate())
            .expect("counted")
            .number(),
        1,
        "and on a fresh retry budget, because a re-entry as designed is one"
    );
    assert_eq!(
        store
            .step_iteration(&job_id, &gate())
            .expect("counted")
            .number(),
        2,
        "one loop is one pass — the arrival charges nothing the return charged"
    );
}

/// The whole claim, end to end: two `request_changes` passes and then a cap
/// with nothing left in it. A cap of two buys two passes, so the second
/// verdict is the one that stops.
#[tokio::test]
async fn two_passes_and_then_the_cap_is_spent() {
    let home = TempDir::new();
    let fleet = a_fleet_running_a_loop(&home, FakeWorkProduct::changed(&["src/log.rs"]), 2);
    let job_id = at_the_loop_s_gate(&fleet, &home).await;

    let sent_back = fleet
        .request_changes(&job_id, &said("draft it again"))
        .await
        .expect("the first return is inside a cap of two");
    assert_eq!(sent_back.current_step_id(), Some(&drafted()));
    started(&fleet, &job_id)
        .await
        .expect("the turn puts a Drone back on the draft");

    // Round the loop: the redraft is worked, clears, and the gate is reached a
    // second time with a fresh Drone on it.
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the second draft is reported");
    fleet.turn().await.expect("the draft's gate runs again");
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the second summary is reported");
    fleet.turn().await.expect("the human gate runs again");

    let held = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(
        held.status(),
        JobStatus::AwaitingReview,
        "the second pass is a real pass and reaches a person"
    );

    let stopped = fleet
        .request_changes(&job_id, &said("and again"))
        .await
        .expect("the verdict is answered even where the loop is spent");
    assert_eq!(
        stopped.status(),
        JobStatus::Escalated,
        "two passes is what a cap of two buys, and the third verdict has nowhere to go"
    );
    assert_eq!(
        stopped.step(&gate()).map(|step| step.state()),
        Some(StepState::Stopped)
    );
}

// ------------------------------------------------- the definition that ships

/// The workflow this repository actually ships, resolved against the fixture
/// Manifest — which it needs nothing from, since its only check is
/// `artifact_exists`.
///
/// **Read off disk rather than restated here.** `tests::daemon`'s fixtures are
/// written for the tests that use them; this one is the file a person
/// dispatches, and a test that retyped it would keep passing while the shipped
/// definition lost its loop.
fn design_plan() -> config::ResolvedWorkflow {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.armada/workflows/design-plan.json");
    let text = std::fs::read_to_string(&path).expect("the shipped definition is there");
    let def = config::WorkflowDef::parse(&path, &text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the shipped design plan did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the shipped design plan did not resolve: {refused}"))
}

fn drafting() -> StepId {
    StepId::new("draft")
}

fn presenting() -> StepId {
    StepId::new("present")
}

fn document() -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Document,
        claimed: Claimed("The plan names the migration and its rollback."),
        shown_by: ShownBy("`.armada/artifacts/draft.md`, written this step"),
        not_claimed: NotClaimed(""),
    }
}

/// The draft's whole gate is `artifact_exists`, and it reads the file's size —
/// so the worktree has to hold one with something in it.
fn wrote_the_plan(home: &TempDir, job: &core_model::JobId) {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), job.as_str()).expect("a legal spec");
    let at = std::path::Path::new(&spec.worktree_path()).join(".armada/artifacts");
    std::fs::create_dir_all(&at).expect("a place for the artifact");
    std::fs::write(at.join("draft.md"), "# Plan\n\nMigrate, then backfill.\n")
        .expect("the plan is written");
}

/// **Design Plan, the loop it is described as, driven twice round and stopped.**
///
/// `workflows.toml` calls this Armada's only instantiated loop and said so
/// while the file could not carry one. This is the assertion that the sentence
/// is now true of the definition on disk: two `request_changes` verdicts each
/// send the plan back to `draft`, the gate is reached again after each, and the
/// third has no pass left to buy.
///
/// The cap is the designed five and the test spends three of them, so it
/// asserts the loop rather than the bound — `two_passes_and_then_the_cap_is_spent`
/// is where the arithmetic is pinned.
#[tokio::test]
async fn the_shipped_design_plan_goes_round_twice_and_then_stops() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["docs/plan.md"]));
    fittings.workflows = one(design_plan());
    let fleet = crate::daemon::Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal_for("design the migration", "design_plan"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, job.id());
    wrote_the_plan(&home, job.id());
    let job_id = job.id().clone();
    dispatched(&fleet, &job_id).await.expect("it dispatches");

    for pass in 1..=3 {
        submitted_by_the_one(&fleet, document())
            .await
            .expect("the draft is reported");
        fleet.turn().await.expect("the draft's gate runs");
        submitted_by_the_one(&fleet, document())
            .await
            .expect("the plan is presented");
        fleet.turn().await.expect("the human gate runs");

        let held = fleet.load(&job_id).await.expect("the Job is there");
        assert_eq!(
            held.status(),
            JobStatus::AwaitingReview,
            "pass {pass} did not reach a person"
        );
        assert_eq!(held.current_step_id(), Some(&presenting()));

        let answered = fleet
            .request_changes(&job_id, &said("say what the rollback costs"))
            .await
            .expect("the verdict is answered");
        if pass < 3 {
            assert_eq!(
                answered.current_step_id(),
                Some(&drafting()),
                "pass {pass} did not route back to the draft"
            );
            assert_eq!(
                answered.step(&drafting()).map(|step| step.state()),
                Some(StepState::Running)
            );
            started(&fleet, &job_id)
                .await
                .expect("the turn puts the next pass's Drone on the draft");
        }
    }

    let store = fleet.store();
    let store = store.lock().await;
    assert_eq!(
        store
            .step_iteration(&job_id, &presenting())
            .expect("counted")
            .number(),
        4,
        "three returns, and the gate that made all three is on its fourth pass"
    );
    assert_eq!(
        store
            .step_iteration(&job_id, &drafting())
            .expect("counted")
            .number(),
        1,
        "and the step redone three times has emitted nothing"
    );
    assert_eq!(
        store
            .step_attempt(&job_id, &presenting())
            .expect("counted")
            .number(),
        3,
        "the gate was worked three times, and each reading is filed apart"
    );
    assert_eq!(
        store
            .step_spent(&job_id, &drafting())
            .expect("counted")
            .number(),
        1,
        "no retry budget was spent on any of it: nothing failed"
    );
}
