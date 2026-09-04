//! The boundaries nothing lifts, at the gate, on every step.
//!
//! **Separate from `tests::scope` because the reach is what is under test.**
//! That file is about a step's own `evidence_scope` — a boundary the step
//! declared, scoped to the step that declared it, which a Judge may lift. This
//! one is about the tier no step declares and no answer moves, which since
//! `#431` is answered on every step of every workflow whether or not one was
//! declared.
//!
//! The two tiers have two reaches, and that is the design. Keeping the cases
//! apart is how the next person reads which is which.

use adapter_traits::Footprint;
use core_model::{CheckOutcome, RepoPath};
use testkit::{FakeJudge, FakeWorkProduct, Sketch};
use verification::{CheckFailed, Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::tests::gate::{budget, diff_evidence, judged_by, judging, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::scope::{declared, ruled_by, scoped};

/// A step that declares nothing at all — no scope, no Check, no criterion.
/// **The shape of fourteen of the twenty-three shipped steps**, and of every
/// terminal one.
fn declaring_nothing() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "handoff",
        label: "Hand off",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// **The floor, and the whole of `#431`.** A step that declares no
/// `evidence_scope` writes a secret and does not advance.
///
/// Before this the gate read the worktree only where a Check asked for changed
/// paths or the step declared a scope, so on a step like this one nothing
/// looked and the step advanced.
///
/// **The Judge fails if it is called**, which is the second half of the claim:
/// an absolute boundary spends no model call, on a step with a scope or
/// without.
#[tokio::test]
async fn a_step_that_declares_no_scope_still_cannot_write_a_secret() {
    let workflow = declaring_nothing();
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs", ".env.production"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judged_by(FakeJudge::that_fails("a Judge no absolute boundary asks")),
        &keeping_nowhere(),
    )
    .await;

    assert!(!ruling.advanced(), "{ruling:?}");
    let Ruling::Failed { failures, .. } = &ruling else {
        panic!("a boundary nothing lifts fails the step: {ruling:?}");
    };
    let CheckFailed::OutOfBounds { paths } = &failures[0] else {
        panic!("and not as a scope this step never declared: {failures:?}");
    };
    assert_eq!(paths.len(), 1, "the ordinary file is not one: {paths:?}");
    assert_eq!(paths[0].path(), &RepoPath::new(".env.production"));
    assert_eq!(paths[0].because(), "it holds secrets");

    // **The row a person opens.** It is not named `evidence_scope`, because
    // this step has none and a row named after a field it never carried is a
    // sentence nobody can act on.
    let recorded: Vec<(&str, CheckOutcome)> = ruling
        .checks()
        .iter()
        .map(|check| (check.name.as_str(), check.outcome))
        .collect();
    assert_eq!(recorded, vec![("out_of_bounds", CheckOutcome::Failed)]);
    assert_eq!(
        ruling.checks()[0].produced.as_deref(),
        Some("the step reaches `.env.production` — it holds secrets, which nothing here can allow")
    );
}

/// The same floor, and the same words, on a step that *did* declare a scope.
///
/// **One boundary, one row name, one sentence.** The two are reached through
/// different tiers — this one through `InScope::resolved`, over the declaration
/// and the footprint together — and a record that spelled them differently
/// would read as two rules.
#[tokio::test]
async fn the_same_boundary_reads_the_same_way_on_a_step_that_declared_one() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_by(
        &judged_by(FakeJudge::that_fails("a Judge no absolute boundary asks")),
        &workflow,
        Some(&declared(&["src"])),
        &["src/lib.rs", ".env.production"],
    )
    .await;

    assert!(!ruling.advanced(), "{ruling:?}");
    let recorded: Vec<(&str, CheckOutcome)> = ruling
        .checks()
        .iter()
        .map(|check| (check.name.as_str(), check.outcome))
        .collect();
    assert_eq!(recorded, vec![("out_of_bounds", CheckOutcome::Failed)]);
    assert_eq!(
        ruling.checks()[0].produced.as_deref(),
        Some("the step reaches `.env.production` — it holds secrets, which nothing here can allow")
    );
}

/// **What a step with no scope still does not gain: a plan.** The floor is a
/// floor, not a drift check.
///
/// The step here writes a file nobody declared and nobody excluded, and it
/// advances — because there is no declaration to have drifted from, and the
/// only question the gate now asks of it is the one no answer moves.
#[tokio::test]
async fn a_step_that_declares_no_scope_gains_a_floor_and_not_a_drift_check() {
    let workflow = declaring_nothing();
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["crates/somewhere/else.rs"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judged_by(FakeJudge::that_fails("a step with no plan asks no Judge")),
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert!(ruling.checks().is_empty(), "{:?}", ruling.checks());
}

/// **The one directory under `.armada` a Drone is told to write to.**
///
/// Seven shipped workflows name a `mechanical_checks[].target` under
/// `.armada/artifacts/`, and Fleet opens exactly that path at the gate to put
/// in the Judge's brief. A boundary that refused it would refuse the work.
///
/// It is asserted here rather than only in `verification` because the gate is
/// where it would have bitten: this repository ignores `.armada/*`, so the
/// artifact never enters a diff and nothing would have caught it until a
/// repository that does not ignore it ran its first `plan` step.
#[tokio::test]
async fn the_deliverable_a_workflow_asks_for_is_not_a_boundary() {
    let workflow = declaring_nothing();
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&[".armada/artifacts/plan.md"]);

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
}

/// **A step with no scope whose worktree is gone stops rather than advancing.**
///
/// This is what the unconditional reading costs, stated as a claim rather than
/// left as a side effect: a gate that cannot read the worktree cannot say the
/// step touched nothing out of bounds, and `CouldNotDecide` neither advances it
/// nor fails it — it is a person's.
#[tokio::test]
async fn a_step_with_no_scope_whose_worktree_will_not_open_decides_nothing() {
    let workflow = declaring_nothing();
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::refusing("a worktree that is no longer there");

    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(),
        &keeping_nowhere(),
    )
    .await;

    assert!(!ruling.advanced(), "{ruling:?}");
    assert_eq!(
        ruling.undecided().map(|(artifact, _)| artifact),
        Some("the Job's changed files")
    );
}
