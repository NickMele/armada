//! `checks.<name>.width`, read off a Manifest and frozen onto a step. #1444.
//!
//! **Absent is `None` here and not a number**, which is what parts this from
//! [`places`](super::places): a Check saying nothing about width gets whatever
//! the machine hands out, and nothing in this crate knows the machine. The
//! default is applied where the machine is known, and a declared value may only
//! lower it.

use std::num::NonZeroU32;

use core_model::{ResolvedStep, StepId};

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
    run: pnpm -C packages/screens test -- --maxWorkers=${width}
    width: 2
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
fn absent_is_none_and_a_declared_value_is_read() {
    let read = manifest();
    assert_eq!(
        read.check("build").unwrap().width(),
        None,
        "a Check that declares no width is not a Check that declared one"
    );
    assert_eq!(
        read.check("screens_test").unwrap().width(),
        NonZeroU32::new(2)
    );
}

#[test]
fn zero_is_refused_where_it_is_written() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    width: 0\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.width"),
        "{refused_here:?}"
    );
}

#[test]
fn a_negative_number_is_refused() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    width: -1\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.width"),
        "{refused_here:?}"
    );
}

#[test]
fn a_non_integer_is_refused() {
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    width: 1.5\n";
    let refused_here = refusals(Manifest::parse(&named("armada.yml"), text));
    assert!(
        refused(&refused_here, "checks.e2e.width"),
        "{refused_here:?}"
    );
}

#[test]
fn a_refused_width_does_not_take_the_check_with_a_default() {
    // The whole reason `width` is parsed as a `Result` rather than an
    // `Option`: a file that wrote a width Armada could not read must not load
    // as a file that wrote none, because those two mean different numbers.
    let text = "version: 1\nid: shop\nchecks:\n  e2e:\n    run: pnpm e2e\n    width: nope\n";
    assert!(
        Manifest::parse(&named("armada.yml"), text).is_err(),
        "a Manifest with an unreadable width loaded anyway"
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
    assert_eq!(by_name("build").width(), None);
    assert_eq!(
        by_name("screens_test").width(),
        NonZeroU32::new(2),
        "a Manifest edit reaches the next Job, but this one already froze 2"
    );
}
