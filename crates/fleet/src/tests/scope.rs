//! Evidence bound to what the step actually touched.
//!
//! Two moments and two consequences, which is why they are two groups of
//! cases: at the gate a footprint outside the declaration fails the step, and
//! during the step it is recorded and the Drone may declare again.
//!
//! The third group is the one that matters most and asserts nothing new: a step
//! carrying no `evidence_scope` reads no worktree for a scope it does not have
//! and behaves exactly as it did before any of this existed.

use core_model::{CheckOutcome, DeclaredPaths, RepoPath};
use ipc::mcp::DeclareScope;
use testkit::{FakeWorkProduct, Gate, Scoped, Sketch};
use verification::{CheckFailed, Claimed, NotClaimed, OutsideScope, ShownBy, EVIDENCE_SCOPE};

use crate::adrift::NotDeclared;
use crate::at_step::AtStep;
use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::{rule_on, Ruling};
use crate::tests::daemon::{a_fleet_holding, a_proposal, worktree_directory};
use crate::tests::gate::{budget, diff_evidence, fresh, judging, worktree};
use crate::tests::tmp::TempDir;

fn a_diff_call<'a>() -> Call<'a> {
    Call {
        evidence_type: config::EvidenceType::Diff,
        claimed: Claimed("The plan is written."),
        shown_by: ShownBy("docs/plan.md"),
        not_claimed: NotClaimed(""),
    }
}

/// One step, gated on nothing but its scope, so the only thing that can fail is
/// the scope. A Check would put a second reason in every failure below.
fn scoped(diff_check: bool, exclude: &[&str]) -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: Some(Scoped {
            diff_check,
            at_step_start: true,
            exclude,
        }),
        gaming: None,
    }])
}

fn declared(paths: &[&str]) -> DeclaredPaths {
    DeclaredPaths::of(paths.iter().copied().map(RepoPath::new).collect())
}

async fn ruled_on(
    workflow: &config::ResolvedWorkflow,
    declared: Option<&DeclaredPaths>,
    changed: &[&str],
) -> Ruling {
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(changed);
    rule_on(
        at_step,
        &diff_evidence(),
        declared,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await
}

// ------------------------------------------------------------ at the gate

/// **The recycling this capability is named for.** A step that changed a file
/// belonging to some other step did that step's work, and no model call is
/// needed to see it.
#[tokio::test]
async fn a_step_that_changed_what_it_did_not_declare_does_not_advance() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_on(
        &workflow,
        Some(&declared(&["docs"])),
        &["docs/plan.md", "crates/fleet/src/gate.rs"],
    )
    .await;

    assert!(!ruling.advanced());
    let Ruling::Failed { failures, .. } = &ruling else {
        panic!("a footprint outside the declaration is a gate failure: {ruling:?}");
    };
    assert_eq!(
        failures,
        &vec![CheckFailed::OutOfScope(OutsideScope::Undeclared {
            changed: vec![RepoPath::new("crates/fleet/src/gate.rs")]
        })],
        "the failure names the file, and only the file that was outside"
    );
}

/// The refusal is a row like any other failure, so a person reading the record
/// sees which gate stopped the step rather than only that one did.
#[tokio::test]
async fn the_scope_check_is_written_down_as_a_check_that_failed() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_on(&workflow, Some(&declared(&["docs"])), &["src/lib.rs"]).await;

    let recorded: Vec<(&str, CheckOutcome)> = ruling
        .checks()
        .iter()
        .map(|check| (check.name.as_str(), check.outcome))
        .collect();
    assert_eq!(recorded, vec![(EVIDENCE_SCOPE, CheckOutcome::Failed)]);
    assert_eq!(
        ruling.checks()[0].expected.as_deref(),
        Some("the step changes only what it declared")
    );
}

/// A declared path that nothing touched is **read context**, not drift. A hunk
/// alone often cannot answer whether a test is still meaningful and the rest of
/// the file can, which is what `context_paths` is for.
#[tokio::test]
async fn a_declared_path_that_did_not_change_is_not_a_failure() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_on(
        &workflow,
        Some(&declared(&["src/parser.rs", "src/lexer.rs"])),
        &["src/parser.rs"],
    )
    .await;
    assert!(ruling.advanced(), "{ruling:?}");
}

/// A step that asks for a declaration and never gets one has nothing to be
/// measured against, and an unmeasured step must not pass as a measured one.
#[tokio::test]
async fn a_step_that_declared_nothing_at_all_does_not_advance() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_on(&workflow, None, &["src/lib.rs"]).await;

    assert!(!ruling.advanced());
    assert!(matches!(
        &ruling,
        Ruling::Failed { failures, .. }
            if failures == &vec![CheckFailed::OutOfScope(OutsideScope::NothingDeclared)]
    ));
}

/// **Declared to change nothing is an answer**, and a different one from having
/// declared nothing: it passes on a step that changed nothing and fails the
/// moment anything moves.
#[tokio::test]
async fn declaring_nothing_is_not_the_same_as_never_declaring() {
    let workflow = scoped(true, &[]);
    assert!(ruled_on(&workflow, Some(&DeclaredPaths::nothing()), &[])
        .await
        .advanced());
    assert!(
        !ruled_on(&workflow, Some(&DeclaredPaths::nothing()), &["a.rs"])
            .await
            .advanced()
    );
}

/// `scope_diff_check` is the step's own switch. A step that declares where to
/// look and does not ask for its footprint to be checked has not claimed it
/// stayed there.
#[tokio::test]
async fn a_scope_without_the_footprint_check_measures_no_footprint() {
    let workflow = scoped(false, &[]);
    let ruling = ruled_on(&workflow, Some(&declared(&["docs"])), &["src/lib.rs"]).await;
    assert!(ruling.advanced(), "{ruling:?}");
}

// -------------------------------------------------------- while it runs

/// A Fleet whose one step declares its plan at step start and has its footprint
/// checked, so both halves of the comparison are live.
fn a_watching_fleet(
    home: &TempDir,
    work: FakeWorkProduct,
) -> Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> {
    a_fleet_holding(home, work, scoped(true, &[]), 1)
}

/// **The fourth pain, made mechanical.** A Drone that declared `docs` and is
/// editing `crates` has started the next part's work, and Fleet says so while
/// it is happening rather than at the gate.
#[tokio::test]
async fn a_step_editing_outside_its_plan_is_caught_while_it_runs() {
    let home = TempDir::new();
    let fleet = a_watching_fleet(
        &home,
        FakeWorkProduct::changed(&["docs/plan.md", "crates/fleet/src/gate.rs"]),
    );
    let job = fleet.propose(a_proposal("write the plan")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["docs".to_string()],
        })
        .await
        .expect("the step declares a scope");

    let turned = fleet.turn().await.unwrap();
    let drifting = turned.drifting.expect("the live check saw the edit");
    assert_eq!(drifting.job, *job.id());
    assert_eq!(drifting.step.as_str(), "implement");
    assert_eq!(
        drifting.paths,
        vec![RepoPath::new("crates/fleet/src/gate.rs")]
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        core_model::JobStatus::Running,
        "a live mismatch does not auto-fail — the Drone may declare again"
    );
}

/// Said once. A drift reported every tick is a warning nobody reads, and the
/// second turn has nothing new to say about the same file.
#[tokio::test]
async fn the_same_drift_is_reported_once_and_not_every_turn() {
    let home = TempDir::new();
    let fleet = a_watching_fleet(&home, FakeWorkProduct::changed(&["src/lib.rs"]));
    let job = fleet.propose(a_proposal("write the plan")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();
    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["docs".to_string()],
        })
        .await
        .unwrap();

    assert!(fleet.turn().await.unwrap().drifting.is_some());
    assert!(fleet.turn().await.unwrap().drifting.is_none());
}

/// **The plan that turned out wrong.** Declaring again replaces the plan, which
/// is what makes "a live mismatch does not auto-fail" mean something rather
/// than being a warning with no way out.
#[tokio::test]
async fn declaring_again_replaces_the_plan_and_clears_what_drifted() {
    let home = TempDir::new();
    let fleet = a_watching_fleet(&home, FakeWorkProduct::changed(&["src/lib.rs"]));
    let job = fleet.propose(a_proposal("write the plan")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();
    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["docs".to_string()],
        })
        .await
        .unwrap();
    assert!(fleet.turn().await.unwrap().drifting.is_some());

    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["src".to_string()],
        })
        .await
        .unwrap();
    assert!(
        fleet.turn().await.unwrap().drifting.is_none(),
        "the work is inside the plan now, and the plan is the one that counts"
    );
}

/// Nothing is watched on a step that declares no scope, so a Fleet running an
/// ordinary workflow reads no worktree for it.
#[tokio::test]
async fn a_step_with_no_scope_is_not_watched_and_takes_no_declaration() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&["src/lib.rs"]));
    let job = fleet.propose(a_proposal("fix the thing")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    let refused = fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["src".to_string()],
        })
        .await
        .expect_err("a step that declares no scope takes no declaration");
    assert!(matches!(refused, NotDeclared::StepHasNoScope { .. }));
    assert!(fleet.turn().await.unwrap().drifting.is_none());
}

/// The denylist resolves last and wins over anything the Drone declared — and
/// the Drone is told at declaration time, while it can still fix the plan.
#[tokio::test]
async fn a_declaration_naming_an_excluded_path_is_refused_where_it_is_made() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/lib.rs"]),
        scoped(true, &["secrets"]),
        1,
    );
    let job = fleet.propose(a_proposal("read the keys")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    let refused = fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["secrets/keys.toml".to_string()],
        })
        .await
        .expect_err("the denylist wins");
    assert!(matches!(refused, NotDeclared::Outside(_)));
}

/// A declaration is about one step. Carrying it forward would let the next
/// step's footprint be measured against the last one's promise, which is the
/// recycling this whole capability refuses.
#[tokio::test]
async fn the_plan_does_not_survive_the_step_it_was_declared_for() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["docs/plan.md"]),
        testkit::resolved(&[
            Sketch {
                id: "plan",
                label: "Plan",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: Some(Scoped {
                    diff_check: true,
                    at_step_start: true,
                    exclude: &[],
                }),
                gaming: None,
            },
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: Some(Scoped {
                    diff_check: true,
                    at_step_start: true,
                    exclude: &[],
                }),
                gaming: None,
            },
        ]),
        1,
    );
    let job = fleet.propose(a_proposal("plan then do")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();
    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["docs".to_string()],
        })
        .await
        .unwrap();

    fleet.submit_evidence(a_diff_call()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled, Some(Ruling::Advanced { .. })));

    // The second step inherits nothing. Submitting against it with no
    // declaration of its own fails rather than reusing the first step's.
    fleet.submit_evidence(a_diff_call()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(
            turned.ruled,
            Some(Ruling::Failed { ref failures, .. })
                if failures == &vec![CheckFailed::OutOfScope(OutsideScope::NothingDeclared)]
        ),
        "{:?}",
        turned.ruled
    );
}

// ------------------------------------------------ a step that asked nothing

/// **The whole of what a step with no evidence scope does differently: nothing.**
///
/// It reads no worktree for a scope it does not have, which is what keeps the
/// check cold on every step written before one existed.
#[tokio::test]
async fn a_step_with_no_scope_is_neither_checked_nor_read() {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(
        work.asked().len(),
        1,
        "the worktree was read once, for `diff_nonempty`, and not again for a \
         scope the step does not declare"
    );
}

/// A step with neither a scope nor a Check is still the common shape, and it
/// still advances on evidence alone.
#[tokio::test]
async fn an_ungated_step_with_no_scope_advances_on_evidence_alone() {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::refusing("a repository nobody should have opened");

    let ruling = rule_on(
        at_step,
        &diff_evidence(),
        None,
        Some(&fresh()),
        &[],
        &work,
        budget(),
        &judging(),
    )
    .await;
    assert!(ruling.advanced(), "{ruling:?}");
}
