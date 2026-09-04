//! Evidence bound to what the step actually touched.
//!
//! Two moments, and neither fails the step: at the gate a footprint outside the
//! declaration tags the step for a mandatory Judge look, and during the step it
//! is recorded and the Drone may declare again.
//!
//! The third group is the one that matters most: a step carrying no
//! `evidence_scope` is asked for no declaration, drifts from nothing and
//! reaches no Judge about where its work went — and is still answered over the
//! boundaries nothing lifts. That floor is `#431`, and it is the one thing on
//! such a step that is not as it was before any of this existed.

use std::sync::Arc;

use adapter_traits::Footprint;
use core_model::{
    CheckOutcome, CriterionId, DeclaredPaths, EscalationTrigger, JobStatus, JudgeVerdict, RepoPath,
    Timestamp, TransitionReason,
};
use ipc::mcp::DeclareScope;
use testkit::{FakeJudge, FakeWorkProduct, Gate, Scoped, Sketch};
use verification::{
    CheckFailed, Claimed, Lifted, NotClaimed, OutsideScope, Request, ShownBy, DECLARED_PLAN_DRIFT,
};

use crate::at_step::AtStep;
use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::{apply, rule_on, Ruling};
use crate::judging::Judging;
use crate::scope::NotDeclared;
use crate::tests::admitted::dispatched;
use crate::tests::briefing::turns_sent;
use crate::tests::daemon::{a_fleet_holding, a_fleet_judged_by, a_proposal, worktree_directory};
use crate::tests::gate::{
    budget, diff_evidence, judged_by, judged_by_shared, judging, running_job, worktree,
};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::tools::{declared_by_the_one, submitted_by_the_one};

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
pub(super) fn scoped(diff_check: bool, exclude: &[&str]) -> config::ResolvedWorkflow {
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
            references: &[],
        }),
        gaming: None,
    }])
}

pub(super) fn declared(paths: &[&str]) -> DeclaredPaths {
    DeclaredPaths::of(paths.iter().copied().map(RepoPath::new).collect())
}

/// Ruled on by a Judge that must never be asked. Every case below that uses it
/// asserts the tier stayed cold as much as it asserts the ruling.
async fn ruled_on(
    workflow: &config::ResolvedWorkflow,
    declared: Option<&DeclaredPaths>,
    changed: &[&str],
) -> Ruling {
    ruled_by(&judging(), workflow, declared, changed).await
}

pub(super) async fn ruled_by(
    judging: &Judging,
    workflow: &config::ResolvedWorkflow,
    declared: Option<&DeclaredPaths>,
    changed: &[&str],
) -> Ruling {
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(changed);
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        declared,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        judging,
        &keeping_nowhere(),
    )
    .await
}

// ------------------------------------------------------------ at the gate

/// **The mandatory look, on a step that declares no criterion of its own.**
/// `judge.md` gives declared plan drift to the Judge; the step here asks
/// nothing, so the only thing that can have made a call is the drift.
#[tokio::test]
async fn a_step_that_changed_what_it_did_not_declare_reaches_the_judge() {
    let workflow = scoped(true, &[]);
    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = ruled_by(
        &judged_by_shared(Arc::clone(&judge)),
        &workflow,
        Some(&declared(&["docs"])),
        &["docs/plan.md", "crates/fleet/src/gate.rs"],
    )
    .await;

    assert!(
        ruling.advanced(),
        "a step that drifted and whose Judge is content advances: {ruling:?}"
    );
    let asked = judge.asked();
    assert_eq!(asked.len(), 1, "one call, and no panel: {asked:?}");
    assert!(
        asked[0].contains("outside that declaration: crates/fleet/src/gate.rs."),
        "the question names the path that drifted, and only that path — \
         `docs/plan.md` was declared: {}",
        asked[0]
    );
}

/// **Escalated, never terminal.** Job `01M148ZF0D001BYXWN9XWHYGYF` reached
/// `completed_failed` on drift, which `restart_step` cannot come back from. A
/// refusal is a person's to answer.
#[tokio::test]
async fn drift_the_judge_refuses_escalates_rather_than_ending_the_job() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_by(
        &judged_by(FakeJudge::refusing(
            "the step touches only what it declared",
            "it also rewrote an unrelated module",
            "the next step's work is already half done and unreviewed",
        )),
        &workflow,
        Some(&declared(&["docs"])),
        &["docs/plan.md", "crates/fleet/src/gate.rs"],
    )
    .await;

    assert!(!ruling.advanced());
    let Ruling::Refused { refusals, .. } = &ruling else {
        panic!("a drift refusal is a refusal, not a gate failure: {ruling:?}");
    };
    assert_eq!(
        refusals.criteria(),
        vec![&CriterionId::new(DECLARED_PLAN_DRIFT)]
    );
    assert!(
        !ruling.ends_the_drone(),
        "the Drone stays alive and idle, which is what a redirect resumes"
    );
    let moved = apply(
        &running_job(),
        &ruling,
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    )
    .expect("a refusal moves the Job")
    .expect("a legal move");
    assert_eq!(moved.job.status(), JobStatus::Escalated);
    assert!(!moved.job.status().is_terminal(), "drift is answerable");
    assert_eq!(
        moved.event.reason(),
        &TransitionReason::Escalation(EscalationTrigger::GateFailure)
    );
}

/// Drift writes no failed Check row. `CheckOutcome` has no value meaning "seen
/// and did not stop the step", and `Failed` would say the gate failed when it
/// held — the record of the look is the judgment, cited by criterion.
#[tokio::test]
async fn drift_is_recorded_as_a_judgment_rather_than_a_failed_check() {
    let workflow = scoped(true, &[]);
    let ruling = ruled_by(
        &judged_by(FakeJudge::with_no_objection()),
        &workflow,
        Some(&declared(&["docs"])),
        &["src/lib.rs"],
    )
    .await;

    let recorded: Vec<(&str, CheckOutcome)> = ruling
        .checks()
        .iter()
        .map(|check| (check.name.as_str(), check.outcome))
        .collect();
    assert_eq!(recorded, Vec::new());
    let judged = ruling.judged();
    assert_eq!(judged.len(), 1);
    assert_eq!(
        judged[0].criterion_id,
        CriterionId::new(DECLARED_PLAN_DRIFT)
    );
    assert_eq!(judged[0].verdict, JudgeVerdict::Met);
}

/// A declaration the step's own denylist refuses is **not** drift, and stays
/// mechanical: a model that could excuse a denylist is not a denylist. The
/// Judge here would fail if it were asked.
#[tokio::test]
async fn a_declaration_the_denylist_refuses_is_still_a_gate_failure() {
    let workflow = scoped(true, &["secrets"]);
    let ruling = ruled_on(
        &workflow,
        Some(&declared(&["secrets/keys.toml"])),
        &["secrets/keys.toml"],
    )
    .await;

    assert!(
        matches!(
            &ruling,
            Ruling::Failed { failures, .. }
                if failures == &vec![CheckFailed::OutOfScope(OutsideScope::Excluded {
                    declared: vec![RepoPath::new("secrets/keys.toml")]
                })]
        ),
        "{ruling:?}"
    );
}

/// The whole of the defect, through a real Fleet: a Job that drifts stops at
/// `escalated`, from which `restart_step` and Pilot are both reachable, rather
/// than at `completed_failed`, from which neither is.
#[tokio::test]
async fn a_job_that_drifted_is_answerable_rather_than_over() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["docs/plan.md", "protocol-version.toml"]),
        scoped(true, &[]),
        FakeJudge::refusing(
            "the bump is this step's own work",
            "it belongs to the step that adds the operation",
            "two steps' changes land under one review",
        ),
    );
    let job = fleet.propose(a_proposal("write the plan")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["docs".to_string()],
        },
    )
    .await
    .unwrap();

    submitted_by_the_one(&fleet, a_diff_call()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(turned.ruled(), Some(Ruling::Refused { .. })),
        "{:?}",
        turned.ruled()
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Escalated
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
    dispatched(&fleet, job.id()).await.unwrap();

    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["docs".to_string()],
        },
    )
    .await
    .expect("the step declares a scope");

    let turned = fleet.turn().await.unwrap();
    let drifting = turned.drifting().expect("the live check saw the edit");
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
    dispatched(&fleet, job.id()).await.unwrap();
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["docs".to_string()],
        },
    )
    .await
    .unwrap();

    assert!(fleet.turn().await.unwrap().drifting().is_some());
    assert!(fleet.turn().await.unwrap().drifting().is_none());
}

/// **The half that was missing for as long as the check existed.** The finding
/// went to the Job's log, which no Drone reads, so the call that replaces a
/// plan was one the Drone had no reason to make — Job
/// `01M14HZ8ND001FYT6264WZJFPB` drifted, carried on for seven minutes and
/// reached its gate holding a declaration it had outgrown.
///
/// Once per path, and the second path's notice carries only the second path:
/// a Drone told again about a file it has already answered for reads the
/// notice as having been ignored.
#[tokio::test]
async fn a_drifting_drone_is_told_once_per_path_and_not_again() {
    let home = TempDir::new();
    let fleet = a_watching_fleet(&home, FakeWorkProduct::changed(&["src/lib.rs"]));
    let job = fleet.propose(a_proposal("write the plan")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["docs".to_string()],
        },
    )
    .await
    .unwrap();

    assert!(fleet.turn().await.unwrap().drifting().is_some());
    let told = turns_sent(&fleet, 2).await;
    assert!(
        told[1].contains("src/lib.rs") && told[1].contains("call the scope tool again"),
        "the Drone is told which file and what to do about it: {}",
        told[1]
    );

    // Nothing new this turn, so nothing is said. The Drone then edits a second
    // file outside the plan, which is new and is.
    assert!(fleet.turn().await.unwrap().drifting().is_none());
    fleet
        .work()
        .wrote(&[("src/reader.rs", adapter_traits::Change::Modified)]);
    assert!(fleet.turn().await.unwrap().drifting().is_some());

    let told = turns_sent(&fleet, 3).await;
    assert!(told[2].contains("src/reader.rs"), "{}", told[2]);
    assert!(
        !told[2].contains("src/lib.rs"),
        "the turn between said nothing, and this one repeats nothing: {}",
        told[2]
    );
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
    dispatched(&fleet, job.id()).await.unwrap();
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["docs".to_string()],
        },
    )
    .await
    .unwrap();
    assert!(fleet.turn().await.unwrap().drifting().is_some());

    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["src".to_string()],
        },
    )
    .await
    .unwrap();
    assert!(
        fleet.turn().await.unwrap().drifting().is_none(),
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
    dispatched(&fleet, job.id()).await.unwrap();

    let refused = declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["src".to_string()],
        },
    )
    .await
    .expect_err("a step that declares no scope takes no declaration");
    assert!(matches!(refused, NotDeclared::StepHasNoScope { .. }));
    assert!(fleet.turn().await.unwrap().drifting().is_none());
}

/// The denylist resolves last and wins over anything the Drone declared — and
/// the Drone is told at declaration time, while it can still fix the plan.
///
/// **And it is told there is a route.** An ordinary boundary refused with no
/// route named is `#417`: a Drone that reads "not allowed" fails its step or
/// works around it, and a person finds out at the gate.
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
    dispatched(&fleet, job.id()).await.unwrap();

    let refused = declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["secrets/keys.toml".to_string()],
        },
    )
    .await
    .expect_err("the denylist wins");
    assert!(
        matches!(refused, NotDeclared::Excluded { .. }),
        "{refused:?}"
    );
    assert!(
        refused.to_string().contains("request_scope"),
        "it names the route out: {refused}"
    );
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
                    references: &[],
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
                    references: &[],
                }),
                gaming: None,
            },
        ]),
        1,
    );
    let job = fleet.propose(a_proposal("plan then do")).await.unwrap();
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["docs".to_string()],
        },
    )
    .await
    .unwrap();

    submitted_by_the_one(&fleet, a_diff_call()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Advanced { .. })));

    // The second step inherits nothing. Submitting against it with no
    // declaration of its own fails rather than reusing the first step's.
    submitted_by_the_one(&fleet, a_diff_call()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(
        matches!(
            turned.ruled(),
            Some(Ruling::Failed { ref failures, .. })
                if failures == &vec![CheckFailed::OutOfScope(OutsideScope::NothingDeclared)]
        ),
        "{:?}",
        turned.ruled()
    );
}

// ------------------------------------------------ a step that asked nothing

/// **What a step with no evidence scope still does not get: a plan.**
///
/// No declaration is asked for, nothing drifts because there is nothing to
/// drift from, and no Judge is asked about where the work went. What it gets
/// since `#431` is the floor below this one — the absolute tier, over the
/// footprint alone — and the reading that floor is answered over is the only
/// thing that changed here.
#[tokio::test]
async fn a_step_with_no_scope_is_asked_nothing_it_did_not_declare() {
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
    assert_eq!(
        work.listed().len(),
        1,
        "the changed-file list is read once and answers the floor, the Checks' \
         coverage and a scope tier this step does not declare"
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
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);

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
