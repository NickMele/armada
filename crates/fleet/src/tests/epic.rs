//! The workflow whose product is other Jobs, driven off the file that ships.
//!
//! **This is `#215`'s claim as close as a test reaches it**: a person approves
//! one Job naming a milestone, and the work of that milestone is dispatched and
//! gated without them approving each piece. What a test cannot reach is the
//! merge and a Drone deciding anything — the pieces here are asked for by the
//! fixture rather than decided by a model, which is `bug_job.rs`'s own division
//! and for the same reason.
//!
//! **Read off disk rather than restated**, exactly as `tests::looping` reads
//! `design-plan.json`. Every mechanic underneath was already proved against
//! fixtures in `tests::sub_dispatch` and `tests::looping`; what had never been
//! asserted is that the definition a person actually dispatches wires them
//! together — the grant on the step after the gate, the stand-down, the return,
//! and the loop coming round.
//!
//! # Why the fixture writes the artifacts
//!
//! All three steps declare `artifact_exists`, and the check reads the file's
//! size. A fake Drone writes nothing, so the worktree is seeded before each
//! gate. That is the same seam `tests::looping::wrote_the_plan` uses and it is
//! not a weakening: what the check proves is that Fleet opens the declared path,
//! and a test in which nothing is ever written proves it by never running it.

use adapter_traits::WorktreeSpec;
use core_model::{EvidenceType, JobId, JobStatus, Origin, StepId, StepState};
use ipc::mcp::DispatchJob;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::resume::Redirection;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, manifest, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The definition this repository ships, resolved against the fixture Manifest
/// — which it needs nothing from, because it declares no Manifest Check at all.
///
/// **That is a property of the workflow rather than an accident of the
/// fixture.** A workflow whose product is Jobs has no diff to build and no
/// suite to run, so it resolves against any repository's Manifest, including
/// one that declares nothing.
///
/// The roster is the adapter's, because `plan` names a Judge model and a
/// definition naming a model the adapter does not offer is refused at parse.
fn epic() -> config::ResolvedWorkflow {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.armada/workflows/epic.json");
    let text = std::fs::read_to_string(&path).expect("the shipped definition is there");
    let roster = config::Roster::of(adapters::HeadlessAgent::models());
    let def = config::WorkflowDef::parse(&path, &text, &roster)
        .unwrap_or_else(|refused| panic!("the shipped epic did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the shipped epic did not resolve: {refused}"))
}

/// A one-step workflow the children run under, so that a wave needs nothing of
/// the epic's own definition.
fn a_piece() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        "version: 1\nworkflow_id: fixture-piece\nname: fixture-piece\n\
         structure: linear\nsteps:\n  - id: do_it\n    label: \"Do it\"\n    \
         evidence_type: facts_note\n    advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .expect("the child workflow parses");
    config::ResolvedWorkflow::resolve(&def, &manifest()).expect("it resolves")
}

/// A Fleet holding the epic and something for its children to run.
///
/// **The Judge answers rather than failing**, which the default fittings' Judge
/// does not: `plan` carries two criteria, so a Fleet that left the Judge cold
/// would stop at the first gate for a reason that is not this workflow's.
fn a_fleet_running_epics(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["docs/plan.md"]));
    fittings.workflows = [epic(), a_piece()]
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = std::sync::Arc::new(FakeJudge::with_no_objection());
    Fleet::assembled(fittings)
}

fn planning() -> StepId {
    StepId::new("plan")
}

fn dispatching() -> StepId {
    StepId::new("dispatch")
}

fn rolling_up() -> StepId {
    StepId::new("roll_up")
}

/// A file at a path inside a Job's worktree, with something in it — which is
/// the whole of what `artifact_exists` reads.
fn wrote(home: &TempDir, job: &JobId, at: &str, text: &str) {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), job.as_str()).expect("a legal spec");
    let path = std::path::Path::new(&spec.worktree_path()).join(at);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("a place for the artifact");
    std::fs::write(&path, text).expect("the artifact is written");
}

fn document(claim: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Document,
        claimed: Claimed(claim),
        shown_by: ShownBy("the file this part delivered"),
        not_claimed: NotClaimed(""),
    }
}

fn note(claim: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::FactsNote,
        claimed: Claimed(claim),
        shown_by: ShownBy("the file this part delivered"),
        not_claimed: NotClaimed(""),
    }
}

fn asking(title: &str) -> DispatchJob {
    DispatchJob {
        title: title.to_string(),
        workflow: "fixture-piece".to_string(),
        brief: "what this piece is, from the parent that read the epic".to_string(),
        acceptance_criteria: vec!["it does the thing".to_string()],
        after: Vec::new(),
    }
}

fn said(words: &str) -> Redirection {
    Redirection::saying(words).expect("a note with something in it")
}

/// A Job on the epic, approved, with a Drone on `plan` and a plan in its
/// worktree.
async fn planning_a_wave(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet_running_epics(home);
    let job = fleet
        .propose(a_proposal_for("run the Throughput milestone", "epic"))
        .await
        .expect("a Job at the approval gate");
    let id = job.id().clone();
    worktree_directory(home, &id);
    wrote(
        home,
        &id,
        ".armada/artifacts/plan.md",
        "# Wave 1\n\nOne piece.\n",
    );
    dispatched(&fleet, &id).await.expect("it dispatches");
    (fleet, id)
}

/// The plan is submitted and the Job stands at the gate a person answers.
async fn presented_the_plan(fleet: &Fixture) {
    submitted_by_the_one(fleet, document("The split is drawn."))
        .await
        .expect("the plan is reported");
    fleet.turn().await.expect("the plan's gate runs");
}

/// **The approval is the dispatch.** A person answers `plan`, and what that
/// advances into is the step holding the tool.
async fn approved_the_plan(fleet: &Fixture, job: &JobId, home: &TempDir) {
    fleet
        .approve_review(job)
        .await
        .expect("a person approves the plan");
    wrote(
        home,
        job,
        ".armada/artifacts/dispatched.md",
        "# Dispatched\n\nOne Job.\n",
    );
}

/// **The definition a person dispatches loads.** Parsing is not resolving and
/// resolving is not freezing, and the third is what a Job actually runs — a
/// definition that resolved and could not be frozen would fail at Job creation,
/// where the request is already made.
#[test]
fn the_shipped_epic_parses_resolves_and_freezes() {
    let workflow = epic();
    let frozen = workflow.frozen();
    let ids: Vec<&str> = frozen
        .steps()
        .iter()
        .map(|step| step.id().as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["plan", "dispatch", "roll_up"],
        "plan, dispatch, roll up — and `roll_up` is the issue's name for it",
    );
}

/// **The grant is held on one step and withheld on the two around it**, and it
/// is asserted through the call rather than through the toolbelt: a refusal at
/// the call is what a Drone that reached for it anyway would get, and the
/// allowlist and the refusal are built from the same expression.
#[tokio::test]
async fn only_the_dispatching_step_may_create_jobs() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;

    let refused = fleet.sub_dispatch(&job, &asking("too early")).await;
    assert!(
        refused.is_err(),
        "the planning step has no tool and no call: {refused:?}",
    );

    presented_the_plan(&fleet).await;
    approved_the_plan(&fleet, &job, &home).await;
    let standing = fleet.load(&job).await.expect("the Job reads back");
    assert_eq!(standing.current_step_id(), Some(&dispatching()));

    let child = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("the dispatching step creates Jobs");
    let child = fleet.load(&child).await.expect("the child reads back");
    assert_eq!(child.status(), JobStatus::Queued);
    assert_eq!(child.origin(), Origin::SubDispatched);
    assert_eq!(
        child
            .dispatched_by()
            .expect("a child names its parent")
            .step_id
            .as_str(),
        "dispatch",
    );
}

/// **The whole claim in one run.** One approval, a wave dispatched on it, the
/// parent standing down rather than holding a slot its children need, and a
/// fresh Drone on `roll_up` once they are done — with nobody having approved
/// the child.
#[tokio::test]
async fn one_approval_dispatches_a_wave_and_the_parent_returns_to_roll_it_up() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    presented_the_plan(&fleet).await;
    approved_the_plan(&fleet, &job, &home).await;

    let child = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("the wave leaves");
    worktree_directory(&home, &child);

    submitted_by_the_one(&fleet, note("One Job created."))
        .await
        .expect("the dispatch is reported");
    fleet.turn().await.expect("the parent stands down");

    let waiting = fleet.load(&job).await.expect("the parent reads back");
    assert_eq!(
        waiting.status(),
        JobStatus::Queued,
        "a parent waiting on the Jobs it created is queued, not running",
    );
    assert!(
        !fleet.working_on().await.contains(&job),
        "and it is holding no slot, which is what makes the wait not a deadlock",
    );

    fleet.turn().await.expect("the child is admitted");
    submitted_by_the_one(&fleet, note("The piece is done."))
        .await
        .expect("the child reports");
    fleet.turn().await.expect("the child finishes");
    fleet.turn().await.expect("the parent is admitted again");

    let back = fleet.load(&job).await.expect("the parent reads back");
    assert_eq!(back.status(), JobStatus::Running);
    assert_eq!(
        back.current_step_id(),
        Some(&rolling_up()),
        "the step after the one that dispatched, which is where the report is written",
    );
}

/// **The loop closes on the shipped file.** A person reading the roll-up asks
/// for another wave, and the Job re-enters `plan` — which is what makes a
/// milestone plan a hypothesis rather than a slate written at the start.
///
/// The cap is five and this spends one of them, so what is asserted here is the
/// return; `tests::looping::two_passes_and_then_the_cap_is_spent` is where the
/// arithmetic is pinned.
#[tokio::test]
async fn a_roll_up_that_asks_for_another_wave_re_enters_the_plan() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    presented_the_plan(&fleet).await;
    approved_the_plan(&fleet, &job, &home).await;

    let child = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("the wave leaves");
    worktree_directory(&home, &child);
    submitted_by_the_one(&fleet, note("One Job created."))
        .await
        .expect("the dispatch is reported");
    fleet.turn().await.expect("the parent stands down");
    fleet.turn().await.expect("the child is admitted");
    submitted_by_the_one(&fleet, note("The piece is done."))
        .await
        .expect("the child reports");
    fleet.turn().await.expect("the child finishes");
    fleet.turn().await.expect("the parent is admitted again");

    wrote(
        &home,
        &job,
        ".armada/artifacts/roll-up.md",
        "# Wave 1\n\nOne Job, and it landed.\n",
    );
    submitted_by_the_one(&fleet, document("What the wave did."))
        .await
        .expect("the roll-up is reported");
    fleet.turn().await.expect("the human gate runs");

    let held = fleet.load(&job).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(held.current_step_id(), Some(&rolling_up()));

    let round = fleet
        .request_changes(
            &job,
            &said("the next wave takes the two this one made ready"),
        )
        .await
        .expect("another wave is asked for");
    assert_eq!(
        round.current_step_id(),
        Some(&planning()),
        "the loop returns to the step that plans, not to the step that dispatched",
    );
    assert_eq!(
        round.step(&planning()).map(|step| step.state()),
        Some(StepState::Running),
    );
}

/// **The plan step is told what its file becomes**, which is the one thing it
/// cannot find out for itself. A step definition has no field for prose, so
/// this block is `fleet::terms`' and it is keyed off the *next* step's grant —
/// the assertion is here rather than in `tests::terms` because the fixture
/// workflow there declares no dispatching step and could not raise it.
///
/// **The two reasons a piece waits are both named.** Only one of them survives
/// leaving the plan as a dependency edge; the other is held apart by the
/// drawing and by nothing else in Fleet, so a plan that does not tell them
/// apart is one whose reader cannot either.
#[test]
fn the_step_before_the_dispatch_is_told_that_what_it_writes_becomes_jobs() {
    let turn = turn_at(&planning());
    assert!(turn.contains("WHAT THIS PART DECIDES"), "{turn}");
    assert!(
        turn.contains("its own worktree, its own agent and its own spend"),
        "the cost of a piece is the half a plan is written without: {turn}",
    );
    assert!(
        turn.contains("Draw them as well as describing them"),
        "the drawing is what is approved: {turn}",
    );
    assert!(
        turn.contains("would write the same files"),
        "a sequencing edge is not a dependency edge: {turn}",
    );
}

/// **The dispatching step is told the plan is the authority.** The tool's own
/// description says what one call does to the world; what it cannot say is that
/// the decision was already made and read, and that this part is not where it
/// is made again.
#[test]
fn the_dispatching_step_is_told_it_is_carrying_out_a_decision() {
    let turn = turn_at(&dispatching());
    assert!(turn.contains("WHAT THIS PART CREATES"), "{turn}");
    assert!(
        turn.contains("nothing it does not name"),
        "inventing work is the failure this workflow is watched for: {turn}",
    );
    assert!(
        !turn.contains("WHAT THIS PART DECIDES"),
        "one block or the other, never both: {turn}",
    );
}

/// The step after the dispatch gets neither, because it creates nothing and
/// nothing after it does. **Every other step of every other workflow is this
/// case**, which is why the block is conditional at all.
#[test]
fn the_step_after_the_dispatch_is_told_nothing_about_creating_jobs() {
    let turn = turn_at(&rolling_up());
    assert!(!turn.contains("WHAT THIS PART CREATES"), "{turn}");
    assert!(!turn.contains("WHAT THIS PART DECIDES"), "{turn}");
}

/// The opening turn a Drone on one step of the epic is given.
///
/// The Job is `tests::briefing`'s, which runs a different workflow — the brief
/// takes the two separately, and what is under test here is the workflow half.
fn turn_at(step: &StepId) -> String {
    crate::briefing::first_turn(
        &crate::tests::briefing::a_job(),
        epic().frozen(),
        step,
        &crate::crossing::Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string()
}
