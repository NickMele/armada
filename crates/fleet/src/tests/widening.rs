//! A Drone asking the task's own scope to grow, and a Judge answering it.
//!
//! **The property under test is what does not happen.** A widening used to be a
//! second trip to the approval gate, which ended the Drone before anybody
//! answered; here the Job is `running` on both sides of the call and the Drone
//! is still holding its session. Every case below asserts that as well as the
//! answer.
//!
//! The cases that cost nothing come first — the ones refused before a call is
//! made — because each of them is a Judge call not spent, and the assertion
//! that the Judge was never asked is the point of them.

use core_model::{EscalationTrigger, JobStatus, RepoPath, ScopeRevisionOutcome, WriteTargets};
use ipc::mcp::RequestScope;
use testkit::{FakeJudge, FakeWorkProduct, Scoped, Sketch};

use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::widening::NotWidened;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// One step, excluding one directory, so the denylist has something to refuse.
fn workflow(exclude: &[&str]) -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement the fix",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: Some(Scoped {
            diff_check: true,
            at_step_start: true,
            exclude,
            references: &[],
        }),
        gaming: None,
    }])
}

fn asking(paths: &[&str]) -> RequestScope {
    RequestScope {
        paths: paths.iter().map(|path| (*path).to_string()).collect(),
        reason: "the column the fix needs is declared there".to_string(),
    }
}

/// A Fleet with one Job running, whose scope is the paths given.
async fn running(
    home: &TempDir,
    judge: FakeJudge,
    exclude: &[&str],
    scope: Option<&[&str]>,
) -> (Fixture, core_model::JobId) {
    let fleet = a_fleet_judged_by(
        home,
        FakeWorkProduct::changed(&[]),
        workflow(exclude),
        judge,
    );
    let mut proposal = a_proposal("fix the reader");
    proposal.write_targets =
        scope.map(|paths| paths.iter().map(|path| (*path).to_string()).collect());
    let job = fleet.propose(proposal).await.expect("a job");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("it dispatches");
    let id = job.id().clone();
    (fleet, id)
}

/// Ask as the Drone of the one Job being worked.
async fn asked_by_the_one(
    fleet: &Fixture,
    request: &RequestScope,
) -> Result<crate::widening::Widening, NotWidened> {
    let Some(job) = fleet.working_on().await.first().cloned() else {
        return Err(NotWidened::NothingIsWorking);
    };
    fleet.request_scope(&job, request).await
}

// ------------------------------------------------ answered without a call

/// **Null is not empty.** A Job that never said which files it writes has
/// nothing for a path to be outside of, so there is no question to ask — and a
/// request that determined the scope would let a Drone write the whole answer
/// to a question the Job never asked.
#[tokio::test]
async fn a_task_with_no_stated_scope_has_nothing_to_widen_and_spends_nothing() {
    let home = TempDir::new();
    let judge = FakeJudge::with_no_objection();
    let (fleet, _) = running(&home, judge, &[], None).await;

    let why = asked_by_the_one(&fleet, &asking(&["crates/store"]))
        .await
        .expect_err("a scope nobody stated cannot grow");
    assert!(matches!(why, NotWidened::ScopeUndetermined), "{why:?}");
    assert!(
        why.to_string().contains("declare_scope"),
        "it says which tool the Drone wanted: {why}"
    );
}

/// The denylist resolves last and wins over anything declared **and over any
/// model**. A Judge that could excuse one is not a denylist, so no call is made
/// at all.
#[tokio::test]
async fn a_path_the_step_excludes_is_refused_mechanically() {
    let home = TempDir::new();
    let (fleet, _) = running(
        &home,
        FakeJudge::that_fails("a judge that must never be asked"),
        &["secrets"],
        Some(&["crates/fleet"]),
    )
    .await;

    let why = asked_by_the_one(&fleet, &asking(&["secrets/keys.toml"]))
        .await
        .expect_err("the denylist is not a model's to lift");
    assert!(matches!(why, NotWidened::Excluded { .. }), "{why:?}");
}

/// A request for what the task already covers asks for nothing. Recorded as a
/// widening it would read, later, as a step that needed more room.
#[tokio::test]
async fn a_path_already_in_scope_asks_for_nothing() {
    let home = TempDir::new();
    let (fleet, _) = running(
        &home,
        FakeJudge::that_fails("a judge that must never be asked"),
        &[],
        Some(&["crates/fleet"]),
    )
    .await;

    let why = asked_by_the_one(&fleet, &asking(&["crates/fleet/src/gate.rs"]))
        .await
        .expect_err("nothing is being asked for");
    assert!(matches!(why, NotWidened::AlreadyInScope), "{why:?}");
}

// --------------------------------------------------------- the call itself

/// **The capability, and what it does not cost.** The Judge is asked, the scope
/// moves, the history says so — and the Job is `running` on both sides of the
/// call, so the Drone still holds the session it was working in.
#[tokio::test]
async fn a_cleared_request_widens_the_task_and_ends_no_drone() {
    let home = TempDir::new();
    let (fleet, id) = running(
        &home,
        FakeJudge::saying("answer: consistent"),
        &[],
        Some(&["crates/fleet"]),
    )
    .await;

    asked_by_the_one(&fleet, &asking(&["crates/store/src/schema.rs"]))
        .await
        .expect("the judge cleared it");

    let record = fleet.load(&id).await.expect("it loads");
    assert_eq!(
        record.status(),
        JobStatus::Running,
        "a judge call ends nothing"
    );
    assert!(record
        .write_targets()
        .expect("a scope")
        .paths()
        .contains(&RepoPath::new("crates/store/src/schema.rs")));
    let entry = record.scope_revisions().last().expect("an entry");
    assert_eq!(entry.outcome.as_str(), ScopeRevisionOutcome::TOOK);
    assert_eq!(
        entry.rationale, "the column the fix needs is declared there",
        "the Drone's own words are what a person reads"
    );
    assert!(entry.at_step.is_some(), "a widening belongs to a step");
    assert!(
        !fleet.working_on().await.is_empty(),
        "the Drone is still on its step"
    );
}

/// A refusal is the exception, and the exception is what a person is met by.
/// The step stops carrying its verdict, the Job escalates, and the Drone is
/// left alive and idle — which is what a redirect resumes.
#[tokio::test]
async fn a_refused_request_escalates_and_says_why() {
    let home = TempDir::new();
    let (fleet, id) = running(
        &home,
        FakeJudge::saying("answer: inconsistent\nbecause: the schema is step four's work"),
        &[],
        Some(&["crates/fleet"]),
    )
    .await;

    let why = asked_by_the_one(&fleet, &asking(&["crates/store/src/schema.rs"]))
        .await
        .expect_err("the judge refused");
    let said = why.to_string();
    assert!(said.contains("step four's work"), "{said}");
    assert!(said.contains("A person has been asked"), "{said}");

    let record = fleet.load(&id).await.expect("it loads");
    assert_eq!(record.status(), JobStatus::Escalated);
    assert!(!record.status().is_terminal(), "a refusal is answerable");
    let (_, stopped) = record.stopped_on().expect("the step carries the verdict");
    assert_eq!(stopped.trigger(), EscalationTrigger::ScopeRefused);
    let entry = record.scope_revisions().last().expect("an entry");
    assert_eq!(entry.outcome.as_str(), ScopeRevisionOutcome::NOT_TAKEN);
    assert_eq!(
        record.write_targets().map(WriteTargets::paths),
        Some([RepoPath::new("crates/fleet")].as_slice()),
        "a refused request moves nothing"
    );
}

/// **A machine that cannot answer must not produce one, in either direction.**
/// A call that would not run leaves the Job exactly where it was, records
/// nothing, and does not spend the step's one ask.
#[tokio::test]
async fn a_call_that_could_not_be_made_decides_nothing_and_costs_no_ask() {
    let home = TempDir::new();
    let (fleet, id) = running(
        &home,
        FakeJudge::that_fails("the quota"),
        &[],
        Some(&["crates/fleet"]),
    )
    .await;

    let why = asked_by_the_one(&fleet, &asking(&["crates/store/src/schema.rs"]))
        .await
        .expect_err("the call failed");
    assert!(matches!(why, NotWidened::CouldNotAsk { .. }), "{why:?}");

    let record = fleet.load(&id).await.expect("it loads");
    assert_eq!(record.status(), JobStatus::Running);
    assert!(
        record
            .scope_revisions()
            .iter()
            .all(|entry| entry.at_step.is_none()),
        "nothing was recorded against the step"
    );
}

/// One ask per step, counted off the record rather than held on the slot — so
/// it survives a Fleet that restarts and a Drone that is replaced. A step that
/// has already been answered is told to work inside what it has.
#[tokio::test]
async fn a_step_may_ask_once() {
    let home = TempDir::new();
    let (fleet, _) = running(
        &home,
        FakeJudge::saying("answer: consistent"),
        &[],
        Some(&["crates/fleet"]),
    )
    .await;

    asked_by_the_one(&fleet, &asking(&["crates/store/src/schema.rs"]))
        .await
        .expect("the first ask is answered");
    let why = asked_by_the_one(&fleet, &asking(&["crates/api/src/routes.rs"]))
        .await
        .expect_err("the second is not");
    assert!(matches!(why, NotWidened::AlreadyAsked { .. }), "{why:?}");
    assert!(
        why.to_string().contains("not_claimed"),
        "it says what to do instead: {why}"
    );
}

/// The four things the decision is made on reach the call, and the Drone's
/// reason reaches it as an argument rather than as a fact.
#[tokio::test]
async fn the_call_carries_the_step_the_scope_the_paths_and_the_reason() {
    use std::sync::Arc;

    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying("answer: consistent"));
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&[]),
        workflow(&[]),
        Arc::clone(&judge),
    );
    let mut proposal = a_proposal("fix the reader");
    proposal.write_targets = Some(vec!["crates/fleet".to_string()]);
    let job = fleet.propose(proposal).await.expect("a job");
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.expect("it dispatches");

    asked_by_the_one(&fleet, &asking(&["crates/store/src/schema.rs"]))
        .await
        .expect("the judge cleared it");

    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one call, and no panel: {asked:?}");
    let question = &asked[0];
    assert!(question.contains("Implement the fix"), "{question}");
    assert!(question.contains("crates/fleet"), "{question}");
    assert!(
        question.contains("crates/store/src/schema.rs"),
        "{question}"
    );
    assert!(
        question.contains("the column the fix needs is declared there"),
        "{question}"
    );
    assert!(
        question.contains("fix the reader"),
        "the request: {question}"
    );
}
