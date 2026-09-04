//! One Job creating other Jobs, and the three ways it may not.
//!
//! **No shipped workflow grants this yet**, and that is deliberate: the shape
//! that will is a loop, and the loop keys are still refused as deferred. So
//! every fixture here declares `may_dispatch_jobs` itself, and
//! `crates/config/tests/shipped.rs` asserts that nothing in
//! `.armada/workflows/` does.
//!
//! The fixture's dispatching step is its **first**, so a Job that is approved
//! is a Job whose Drone may dispatch. Where a person stands relative to it is
//! the workflow's business and not this module's.

use core_model::{JobId, JobStatus, Origin};
use ipc::mcp::DispatchJob;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_holding_all, a_proposal_for, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// Two steps: one that creates Jobs, and one after it that does not.
///
/// **What the second step is for is not this module's question.** What matters
/// here is that there *is* a step after the dispatch, because that is the one a
/// parent must not be put on while its children are still going.
fn a_dispatching_workflow() -> config::ResolvedWorkflow {
    resolved(
        "version: 1\nworkflow_id: fixture-dispatcher\nname: fixture-dispatcher\n\
         structure: linear\nsteps:\n  - id: split\n    label: \"Split\"\n    \
         evidence_type: facts_note\n    may_dispatch_jobs: true\n    \
         advance_gate: auto\n  - id: after\n    label: \"After\"\n    \
         evidence_type: facts_note\n    advance_gate: auto\n",
    )
}

/// A one-step workflow a child can be created on, so that dispatching does not
/// need the fixture Fleet's own two-step definition.
fn an_ordinary_workflow() -> config::ResolvedWorkflow {
    resolved(
        "version: 1\nworkflow_id: fixture-piece\nname: fixture-piece\n\
         structure: linear\nsteps:\n  - id: do_it\n    label: \"Do it\"\n    \
         evidence_type: facts_note\n    advance_gate: auto\n",
    )
}

fn resolved(text: &str) -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        text,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &crate::tests::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture workflow did not resolve: {refused}"))
}

fn a_fleet_that_dispatches(home: &TempDir) -> Fixture {
    a_fleet_holding_all(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        vec![a_dispatching_workflow(), an_ordinary_workflow()],
    )
}

/// A Fleet with a Job of the dispatching workflow approved and running on its
/// dispatching step.
async fn dispatching(home: &TempDir) -> (Fixture, JobId) {
    let fleet = a_fleet_that_dispatches(home);
    let job = fleet
        .propose(a_proposal_for("the parent", "fixture-dispatcher"))
        .await
        .expect("the proposal lands");
    worktree_directory(home, job.id());
    dispatched(&fleet, job.id()).await.expect("approval lands");
    let id = job.id().clone();
    (fleet, id)
}

fn asking(title: &str, after: &[&str]) -> DispatchJob {
    DispatchJob {
        title: title.to_string(),
        workflow: "fixture-piece".to_string(),
        brief: "what this piece is, from the parent that read the epic".to_string(),
        acceptance_criteria: vec!["it does the thing".to_string()],
        after: after.iter().map(|id| id.to_string()).collect(),
    }
}

/// **The exemption, working.** A child enters at `queued`, names its parent and
/// the step that made it, and nobody approved it.
#[tokio::test]
async fn a_dispatched_child_enters_queued_naming_the_step_that_made_it() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;

    let child = fleet
        .sub_dispatch(&parent, &asking("port the parser", &[]))
        .await
        .expect("the child is created");

    let child = fleet.load(&child).await.expect("the child reads back");
    assert_eq!(child.status(), JobStatus::Queued);
    assert_eq!(child.origin(), Origin::SubDispatched);
    let by = child.dispatched_by().expect("a child names its parent");
    assert_eq!(&by.job_id, &parent);
    assert_eq!(
        by.step_id.as_str(),
        "split",
        "the step is half of `DispatchOrigin`, and it is what a later step reads back"
    );
}

/// The brief the parent wrote reaches the child, because a child briefed from a
/// title alone is one the decomposition was thrown away for.
#[tokio::test]
async fn the_brief_the_parent_wrote_is_what_the_child_is_told() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;

    let child = fleet
        .sub_dispatch(&parent, &asking("port the parser", &[]))
        .await
        .expect("the child is created");

    let child = fleet.load(&child).await.expect("the child reads back");
    assert!(child.facts().as_str().contains("read the epic"));
    assert_eq!(child.acceptance_criteria().len(), 1);
}

/// An edge points at a sibling, by the id this same call handed back.
#[tokio::test]
async fn a_child_waits_for_the_sibling_whose_id_it_names() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;

    let first = fleet
        .sub_dispatch(&parent, &asking("first", &[]))
        .await
        .expect("the first child is created");
    let second = fleet
        .sub_dispatch(&parent, &asking("second", &[first.as_str()]))
        .await
        .expect("the second child is created");

    let second = fleet.load(&second).await.expect("it reads back");
    let edges = second.dependencies();
    assert_eq!(edges.len(), 1);
    assert_eq!(&edges[0].peer, &first);
}

/// **A Drone may not sequence a Job it did not create.** The refusal names the
/// id rather than dropping the edge: a plan whose order was silently discarded
/// runs in the wrong order and looks fine.
#[tokio::test]
async fn an_after_naming_a_job_this_parent_did_not_create_is_refused() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;
    let stranger = fleet
        .propose(a_proposal_for("somebody else's Job", "fixture-piece"))
        .await
        .expect("the proposal lands");

    let refused = fleet
        .sub_dispatch(&parent, &asking("mine", &[stranger.id().as_str()]))
        .await
        .expect_err("a Job this parent did not create cannot be waited on");

    let said = refused.to_string();
    assert!(said.contains(stranger.id().as_str()), "{said}");
    assert!(said.contains("not one of the Jobs you created"), "{said}");
}

/// **The tool is not the ordinary Drone's.** A Job on a workflow that declares
/// no dispatching step is refused where it calls, as well as never being given
/// the tool at its spawn.
#[tokio::test]
async fn a_drone_on_a_workflow_that_dispatches_nothing_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_that_dispatches(&home);
    let job = fleet
        .propose(a_proposal_for("an ordinary Job", "fixture-piece"))
        .await
        .expect("the proposal lands");
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.expect("approval lands");

    let refused = fleet
        .sub_dispatch(job.id(), &asking("work I invented", &[]))
        .await
        .expect_err("an ordinary Job creates no Jobs");

    assert!(
        refused
            .to_string()
            .contains("not part of the task you are on"),
        "{refused}"
    );
}

/// **Depth 1, and it is a missing constructor rather than a cap.** A child is
/// `Origin::SubDispatched`, which has no `TopLevelOrigin`, so there is no
/// arrangement of the call that reaches a grandchild — a person who wants a
/// child epic dispatches it themselves.
///
/// The child is driven all the way to *running on its own dispatching step*
/// before the call is made, which is what makes the refusal about its origin
/// and nothing else: a child left `queued` would be refused for standing on no
/// step, and the test would pass without the property it is named for.
#[tokio::test]
async fn a_sub_dispatched_job_running_on_a_dispatching_step_still_cannot_dispatch() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;
    let mut asked = asking("a child epic", &[]);
    asked.workflow = "fixture-dispatcher".to_string();
    let child = fleet
        .sub_dispatch(&parent, &asked)
        .await
        .expect("a child on the same workflow is a legal thing to create");
    worktree_directory(&home, &child);

    // The parent's step advances, it gives up its slot, and the next turn puts
    // a Drone on the child — which is the whole arrangement this refusal has to
    // survive.
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the parent's evidence is taken");
    fleet.turn().await.expect("the turn runs");
    fleet.turn().await.expect("and admits the child");

    let standing = fleet.load(&child).await.expect("the child reads back");
    assert_eq!(standing.status(), JobStatus::Running);
    assert_eq!(
        standing
            .current_step_id()
            .map(|step| step.as_str().to_string()),
        Some("split".to_string()),
        "it is on the step that would dispatch, if it were allowed to"
    );

    let refused = fleet
        .sub_dispatch(&child, &asking("a grandchild", &[]))
        .await
        .expect_err("a sub-dispatched Job creates nothing");

    assert!(
        refused.to_string().contains("itself created that way"),
        "{refused}"
    );
}

/// **The deadlock, refused.** A parent that has dispatched gives its slot back
/// rather than holding it while the children it is waiting for need one.
#[tokio::test]
async fn a_parent_waiting_for_its_children_gives_up_its_slot() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;
    let child = fleet
        .sub_dispatch(&parent, &asking("one piece", &[]))
        .await
        .expect("the child is created");
    worktree_directory(&home, &child);

    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the evidence is taken");
    fleet.turn().await.expect("the turn runs");

    let standing = fleet.load(&parent).await.expect("the parent reads back");
    assert_eq!(
        standing.status(),
        JobStatus::Queued,
        "a parent waiting on the Jobs it created is queued rather than running"
    );
    assert!(
        !fleet.working_on().await.contains(&parent),
        "and it is holding no slot, which is the whole point"
    );
}

/// The Board says why, in the words the registry gives a `queued` Job — and
/// says nobody put it back, because nobody did.
///
/// **Both halves of the row, in one test, because the pair is the claim.**
/// `resumption` names a person's act; a parent waiting on Jobs it created was
/// re-queued by Fleet. A `Some` here would be the Board telling somebody they
/// pressed something, and it is the shape a fourth `Resumption` variant would
/// have produced — `readmitting::Owed::resumption` holds why there is not one.
#[tokio::test]
async fn a_waiting_parent_reads_as_blocked_and_as_nobody_having_put_it_back() {
    use api::Queries;

    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;
    let child = fleet
        .sub_dispatch(&parent, &asking("one piece", &[]))
        .await
        .expect("the child is created");
    worktree_directory(&home, &child);
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the evidence is taken");
    fleet.turn().await.expect("the turn runs");

    let summary = fleet
        .get_job(ipc::JobId::from(&parent))
        .await
        .expect("the Job reads")
        .job;

    assert_eq!(
        summary.queued_reason.map(|why| why.as_wire()),
        Some("blocked_by_dependency"),
        "the Jobs it created are what it is waiting for, which is what that label says"
    );
    assert_eq!(
        summary.resumption.map(|act| act.as_wire()),
        None,
        "Fleet stood this Drone down; no person put the Job back, so no act names one"
    );
}

/// **The other half of the wait: it ends.** Once the child is terminal the
/// parent is admitted again, and the Drone it gets is on the step after the
/// dispatch, with every child's outcome in its opening brief.
#[tokio::test]
async fn the_parent_comes_back_on_the_next_step_once_its_children_are_done() {
    let home = TempDir::new();
    let (fleet, parent) = dispatching(&home).await;
    let child = fleet
        .sub_dispatch(&parent, &asking("one piece", &[]))
        .await
        .expect("the child is created");
    worktree_directory(&home, &child);

    // The parent dispatches and stands down; the child is admitted and works
    // its one step; the parent is admitted again.
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the parent's evidence is taken");
    fleet.turn().await.expect("the parent stands down");
    fleet.turn().await.expect("the child is admitted");
    submitted_by_the_one(&fleet, note_evidence())
        .await
        .expect("the child's evidence is taken");
    fleet.turn().await.expect("the child finishes");
    fleet.turn().await.expect("the parent is admitted again");

    let standing = fleet.load(&parent).await.expect("the parent reads back");
    assert_eq!(standing.status(), JobStatus::Running);
    assert_eq!(
        standing
            .current_step_id()
            .map(|step| step.as_str().to_string()),
        Some("after".to_string()),
        "the step after the one that dispatched, entered at the spawn and not before"
    );
}

/// **What the Drone after a dispatch is told, rendered.** It is the one thing
/// on a Job a Drone cannot find out for itself: there is no tool that reads the
/// Board and no way to reach another Job's record, so a step asked to act on
/// what its Job created has nothing to act on without it.
#[test]
fn the_dispatched_block_names_every_child_and_where_it_ended() {
    use crate::crossing::Dispatched;

    let rolled = Dispatched::of(&[
        (
            "01CHILDONE".to_string(),
            "port the parser".to_string(),
            "completed_success",
        ),
        (
            "01CHILDTWO".to_string(),
            "widen the schema".to_string(),
            "completed_failed",
        ),
    ])
    .expect("two children render a block");

    let said = rolled.text();
    assert!(said.contains("01CHILDONE"), "{said}");
    assert!(said.contains("port the parser"), "{said}");
    assert!(
        said.contains("completed_failed"),
        "the failures are what the report is for: {said}"
    );
}

/// A parent that created nothing renders no block, rather than a sentence
/// saying there were none — `Crossed`'s rule, and the same argument the empty
/// boundary makes.
#[test]
fn a_parent_that_dispatched_nothing_renders_no_block() {
    assert!(crate::crossing::Dispatched::of(&[]).is_none());
}
