//! A Check declared `runs_at: handoff`, at the gate and across retries. #849.
//!
//! Each end-to-end Check here is `/usr/bin/touch <marker>`, so whether it ran is
//! a file on disk rather than a row that could have been written without it.

use std::path::Path;

use adapter_traits::Footprint;
use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::CheckOutcome;
use testkit::FakeWorkProduct;
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::policy::Policies;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_holding, a_proposal, diff_evidence, worktree_directory};
use crate::tests::gate::{budget, judging, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// `steps` as workflow YAML, against a Manifest declaring `suite` as `run` and
/// `e2e` as a handoff-only Check that leaves `marker` behind.
fn declared(steps: &str, suite: &str, marker: &Path) -> ResolvedWorkflow {
    let workflow = format!(
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\nsteps:\n{steps}"
    );
    let manifest = format!(
        "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  suite:\n    run: \"{suite}\"\n  \
         e2e:\n    run: \"/usr/bin/touch {}\"\n    runs_at: handoff\n",
        marker.display()
    );
    let def = WorkflowDef::parse(
        Path::new("shaped.yml"),
        &workflow,
        &Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the workflow did not parse: {refused}"));
    let manifest = Manifest::parse(Path::new("armada.yml"), &manifest)
        .unwrap_or_else(|refused| panic!("the manifest did not parse: {refused}"));
    ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared")
}

/// A step gating on every Check, then the step that delivers.
const BEFORE_HANDOFF: &str =
    "  - id: tests\n    label: Tests\n    evidence: {submitted: {type: diff}}\n    \
     delivers: false\n    advance_gate: auto\n    retry_limit: 1\n    \
     mechanical_checks: [{ type: every_manifest_check }]\n  \
     - id: handoff\n    label: Handoff\n    delivers: true\n    advance_gate: auto\n";

async fn ruled(workflow: &ResolvedWorkflow) -> Ruling {
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &crate::tests::gate::diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &FakeWorkProduct::changed(&["src/lib.rs"]),
        budget(),
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &judging(),
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await
}

fn row<'a>(ruling: &'a Ruling, name: &str) -> &'a core_model::StepCheck {
    ruling
        .checks()
        .iter()
        .find(|check| check.name == name)
        .unwrap_or_else(|| panic!("a row for `{name}`: {ruling:?}"))
}

/// **Once before handoff**, where every other Check on the gate passed.
#[tokio::test]
async fn it_runs_on_the_step_before_handoff_once_the_rest_pass() {
    let home = TempDir::new();
    let marker = home.path().join("e2e-ran");
    let workflow = declared(BEFORE_HANDOFF, "/usr/bin/true", &marker);

    let ruling = ruled(&workflow).await;

    assert!(ruling.advanced(), "{ruling:?}");
    assert!(marker.exists(), "the end-to-end Check ran");
    assert_eq!(row(&ruling, "e2e").outcome, CheckOutcome::Passed);
}

/// Where another Check failed, it is not started, and its row says so rather
/// than passing.
#[tokio::test]
async fn it_is_held_back_where_another_check_did_not_pass() {
    let home = TempDir::new();
    let marker = home.path().join("e2e-ran");
    let workflow = declared(BEFORE_HANDOFF, "/usr/bin/false", &marker);

    let ruling = ruled(&workflow).await;

    assert!(!ruling.advanced(), "{ruling:?}");
    assert!(!marker.exists(), "the end-to-end Check never started");
    let e2e = row(&ruling, "e2e");
    assert_eq!(e2e.outcome, CheckOutcome::Skipped);
    assert!(
        e2e.produced
            .as_deref()
            .is_some_and(|said| said.contains("before handoff")),
        "{e2e:?}"
    );
}

/// **The first line of the definition of done, through a real Fleet.** A
/// coding step that is not the last before handoff retries twice and never
/// starts the end-to-end Check.
#[tokio::test]
async fn a_coding_step_retries_twice_without_running_it() {
    let home = TempDir::new();
    let marker = home.path().join("e2e-ran");
    let steps = format!(
        "  - id: implement\n    label: Implement\n    evidence: {{submitted: {{type: diff}}}}\n    \
         delivers: false\n    advance_gate: auto\n    retry_limit: 2\n    \
         mechanical_checks: [{{ type: every_manifest_check }}]\n{BEFORE_HANDOFF}"
    );
    let workflow = declared(&steps, "/usr/bin/false", &marker);
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/lib.rs"]),
        workflow,
        1,
    );
    let job = fleet.propose(a_proposal("add the thing")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    for attempt in 1..=3 {
        submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
        let turned = fleet.turn().await.unwrap();
        let ruled = turned.ruled().expect("the gate ruled");
        assert_eq!(
            matches!(ruled, Ruling::HandedBack { .. }),
            attempt < 3,
            "attempt {attempt}: {ruled:?}"
        );
        assert!(
            ruled.checks().iter().all(|check| check.name != "e2e"),
            "attempt {attempt} gated on no end-to-end Check: {ruled:?}"
        );
    }
    assert!(!marker.exists(), "three attempts, and it never ran");
}

/// A handoff Check that fails goes back to the coding step under its budget,
/// rather than ending the Job.
#[tokio::test]
async fn a_handoff_check_that_fails_sends_the_work_back() {
    let home = TempDir::new();
    let marker = home.path().join("missing-dir").join("e2e-ran");
    let workflow = declared(BEFORE_HANDOFF, "/usr/bin/true", &marker);
    let fleet = a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/lib.rs"]),
        workflow,
        1,
    );
    let job = fleet.propose(a_proposal("add the thing")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    let ruled = turned.ruled().expect("the gate ruled");
    assert!(matches!(ruled, Ruling::HandedBack { .. }), "{ruled:?}");
    assert_eq!(row(ruled, "e2e").outcome, CheckOutcome::Failed);
}
