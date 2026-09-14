//! `checks.<name>.places`, read off a Manifest and frozen onto a step. #1102.

use std::num::NonZeroU32;

use core_model::{ResolvedCheck, ResolvedStep, StepId};

use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{named, refusals, refused};
use crate::workflow::WorkflowDef;

const DECLARED: &str = r#"
version: 1
id: shop
checks:
  build:
    run: cargo build
  screens_test:
    run: pnpm -C packages/screens test
    places: 3
"#;

const FEATURE: &str = include_str!("../../../../../.armada/workflows/feature.json");

fn manifest() -> Manifest {
    Manifest::parse(&named("armada.yml"), DECLARED).expect("the fixture manifest")
}

fn resolved(text: &str) -> ResolvedWorkflow {
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

#[test]
fn absent_is_one_and_a_declared_value_is_read() {
    let read = manifest();
    assert_eq!(read.check("build").unwrap().places().get(), 1);
    assert_eq!(read.check("screens_test").unwrap().places().get(), 3);
}

#[test]
fn zero_is_refused_where_it_is_written() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    places: 0\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.places"),
        "{refused_here:?}"
    );
}

#[test]
fn a_negative_number_is_refused() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    places: -1\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.places"),
        "{refused_here:?}"
    );
}

#[test]
fn a_non_integer_is_refused() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    places: 1.5\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.places"),
        "{refused_here:?}"
    );
}

#[test]
fn it_is_frozen_onto_the_resolved_step() {
    let feature = resolved(FEATURE);
    let implement = step(&feature, "implement");
    let by_name = |name: &str| {
        implement
            .checks()
            .iter()
            .find(|check| check.name() == Some(name))
            .expect("the Check is declared on this step")
    };
    assert_eq!(by_name("build").places(), NonZeroU32::MIN);
    assert_eq!(
        by_name("screens_test").places().get(),
        3,
        "a Manifest edit reaches the next Job, but this one already froze 3"
    );
    let _ = ResolvedCheck::name;
}
