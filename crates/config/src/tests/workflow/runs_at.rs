//! `checks.<name>.runs_at`, read off a Manifest and placed by a workflow. #849.

use core_model::{ResolvedCheck, ResolvedStep, RunsAt, StepId};

use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{named, refusals, refused};
use crate::workflow::WorkflowDef;

/// A repository whose end-to-end Check runs only before handoff and whose
/// story build stays out of a Drone's run.
const DECLARED: &str = r#"
version: 1
id: shop
checks:
  build:
    run: cargo build
  storybook:
    run: pnpm build-storybook
    runs_at: gate
  e2e:
    run: pnpm e2e
    runs_at: handoff
"#;

/// The `feature` workflow Armada ships, read from the file itself.
const FEATURE: &str = include_str!("../../../../../.armada/workflows/feature.json");

fn manifest() -> Manifest {
    Manifest::parse(&named("armada.yml"), DECLARED).expect("the fixture manifest")
}

fn resolved(text: &str) -> ResolvedWorkflow {
    // `feature` asks its Judge on `haiku`, so the roster offers it.
    let def = WorkflowDef::parse(
        &named("workflows/feature.json"),
        text,
        &crate::Roster::of(["haiku"]),
    )
    .expect("the workflow parses");
    ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared")
}

fn step<'a>(workflow: &'a ResolvedWorkflow, id: &str) -> &'a ResolvedStep {
    workflow.frozen().step(&StepId::new(id)).expect("the step")
}

fn names(checks: &[ResolvedCheck]) -> Vec<&str> {
    checks.iter().filter_map(ResolvedCheck::name).collect()
}

#[test]
fn each_value_is_read_and_absent_is_everywhere() {
    let read = manifest();
    assert_eq!(read.check("build").unwrap().runs_at(), RunsAt::Everywhere);
    assert_eq!(read.check("storybook").unwrap().runs_at(), RunsAt::Gate);
    assert_eq!(read.check("e2e").unwrap().runs_at(), RunsAt::Handoff);
}

#[test]
fn a_word_outside_the_three_is_refused_where_it_is_written() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    runs_at: sometimes\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.runs_at"),
        "{refused_here:?}"
    );
}

/// **The first line of the issue's definition of done.** `implement` gates on
/// every Check and retries, and the end-to-end Check is not among them.
#[test]
fn features_implement_step_does_not_gate_on_a_handoff_check() {
    let feature = resolved(FEATURE);
    let implement = step(&feature, "implement");
    assert_eq!(names(implement.checks()), ["build", "storybook"]);
    assert!(implement.retry_limit() > 0, "the step retries");
    assert_eq!(
        feature.frozen().held_for_handoff(implement.id()),
        ["e2e"],
        "and the step says where it went"
    );
}

/// **The second line.** The last step gating on every Check before the one
/// that delivers takes it, once.
#[test]
fn the_step_before_handoff_is_the_one_that_runs_it() {
    let feature = resolved(FEATURE);
    assert_eq!(
        names(step(&feature, "tests").checks()),
        ["build", "storybook", "e2e"]
    );
    assert!(feature
        .frozen()
        .held_for_handoff(&StepId::new("tests"))
        .is_empty());
    let taking: usize = feature
        .steps()
        .iter()
        .flat_map(ResolvedStep::checks)
        .filter(|check| check.name() == Some("e2e"))
        .count();
    assert_eq!(taking, 1, "one step, never more");
}

#[test]
fn a_drones_run_leaves_out_gate_and_handoff_checks() {
    let feature = resolved(FEATURE);
    assert_eq!(names(&step(&feature, "tests").mid_step_checks()), ["build"]);
}

/// A workflow that delivers nothing still runs a handoff Check somewhere: on
/// its last step gating on every Check.
#[test]
fn a_workflow_that_delivers_nothing_runs_it_on_its_last_sweeping_step() {
    let text = "version: 1\nworkflow_id: read\nname: read\nstructure: linear\nsteps:\n\
                - id: first\n  label: First\n  delivers: false\n  advance_gate: auto\n  \
                mechanical_checks: [{ type: every_manifest_check }]\n\
                - id: second\n  label: Second\n  delivers: false\n  advance_gate: auto\n  \
                mechanical_checks: [{ type: every_manifest_check }]\n";
    let workflow = resolved(text);
    assert_eq!(
        names(step(&workflow, "first").checks()),
        ["build", "storybook"]
    );
    assert_eq!(
        names(step(&workflow, "second").checks()),
        ["build", "storybook", "e2e"]
    );
}
