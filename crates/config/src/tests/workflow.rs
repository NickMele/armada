//! A WorkflowDef: the file as a whole, and the fixtures its parts are read
//! from.
//!
//! **What is asked here needs more than one step** — the worked example's
//! shape, two steps sharing an id, a structure the wiring contradicts, a
//! `steps: []`. [`step`] holds what one step's keys mean, [`mechanical`] the
//! deterministic tier, and [`resolving`] the cross-file check that is the point
//! of this milestone step: a definition met with a Manifest.

mod mechanical;
mod resolving;
mod step;

use core_model::{AdvanceGate, EvidenceType};

use crate::error::{Fault, LoadError};
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals, refused, roster};
use crate::workflow::{MechanicalCheck, Structure, WorkflowDef};

/// The `bug` workflow as the milestone step writes it, verbatim.
const BUG: &str = r#"
version: 1
workflow_id: bug
name: bug
structure: linear
steps:
  - id: plan
    label: Plan the change
    evidence_type: facts_note
    advance_gate: auto
  - id: implement
    label: Implement
    evidence_type: diff
    mechanical_checks:
      - { type: manifest_check, check: build, expect_exit_code: 0 }
      - { type: manifest_check, check: test, expect_exit_code: 0 }
      - { type: diff_nonempty }
    advance_gate: auto
  - id: handoff
    label: Summarise
    evidence_type: facts_note
    advance_gate: auto
"#;

const MANIFEST: &str = r#"
version: 1
id: armada
checks:
  build:
    run: cargo build --workspace
  test:
    run: cargo nextest run --workspace
commands:
  fmt:
    run: cargo fmt --all
"#;

fn parse(text: &str) -> Result<WorkflowDef, LoadError> {
    WorkflowDef::parse(&named("workflows/bug.yml"), text, &roster())
}

fn manifest() -> Manifest {
    Manifest::parse(&named("armada.yml"), MANIFEST).expect("the fixture manifest")
}

/// `BUG` with one step's body replaced, so a test changes one thing.
fn bug_with(extra: &str) -> Result<WorkflowDef, LoadError> {
    parse(&format!("{BUG}{extra}"))
}

#[test]
fn the_worked_example_loads() {
    let def = parse(BUG).expect("the worked example");
    assert_eq!(def.name(), "bug");
    assert_eq!(def.version(), 1);
    assert_eq!(def.structure(), Structure::Linear);
    assert_eq!(def.steps().len(), 3);

    let plan = &def.steps()[0];
    assert_eq!(plan.id().as_str(), "plan");
    assert_eq!(plan.label(), "Plan the change");
    assert_eq!(plan.evidence_type(), Some(EvidenceType::FactsNote));
    assert_eq!(plan.advance_gate(), AdvanceGate::Auto);
}

#[test]
fn order_is_the_semantics_and_there_is_no_field_for_it() {
    let def = parse(BUG).expect("the worked example");
    let ids: Vec<&str> = def.steps().iter().map(|s| s.id().as_str()).collect();
    assert_eq!(ids, ["plan", "implement", "handoff"]);
}

#[test]
fn a_step_with_no_mechanical_checks_is_the_common_case() {
    // Two of three steps carry none. The field is optional and its absence is
    // an empty list, never a refusal.
    let def = parse(BUG).expect("the worked example");
    let gated: Vec<usize> = def
        .steps()
        .iter()
        .map(|s| s.mechanical_checks().len())
        .collect();
    assert_eq!(gated, [0, 3, 0]);
}

#[test]
fn implement_carries_the_test_check_too_because_a_separate_verify_step_said_nothing_more() {
    // Fleet runs a Check after the Drone reports its diff, never the Drone
    // itself, so a step blocked until build and test both pass needs no
    // second step to say so again.
    let def = parse(BUG).expect("the worked example");
    assert_eq!(
        def.steps()[1].mechanical_checks(),
        [
            MechanicalCheck::ManifestCheck {
                check: "build".to_string(),
                expect_exit_code: 0,
            },
            MechanicalCheck::ManifestCheck {
                check: "test".to_string(),
                expect_exit_code: 0,
            },
            MechanicalCheck::DiffNonempty,
        ]
    );
}

#[test]
fn two_steps_with_one_id_are_refused_and_the_first_is_named() {
    let refused = refusals(bug_with(
        "  - id: plan\n    label: Plan again\n    advance_gate: auto\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].id"),
        &Fault::DuplicateStepId { first_at: 0 }
    );
}

/// A third value is still a value the schema does not have, and the message
/// says so rather than reading as a milestone that has not arrived.
#[test]
fn a_structure_the_schema_does_not_have_is_refused_as_a_typo() {
    let refused = refusals(parse(
        "version: 1\nworkflow_id: fixture\nname: fixture\nstructure: cycle\nsteps:\n  - id: draft\n    label: Draft\n    advance_gate: auto\n",
    ));
    assert_eq!(
        fault_at(&refused, "structure"),
        &Fault::NotInTheSchema {
            value: "cycle".to_string(),
            legal: &["linear", "loop"],
        }
    );
}

#[test]
fn verdict_routing_on_a_linear_workflow_is_refused_and_names_the_step() {
    // Not reported as an unknown key: on a linear workflow this is wrong at
    // every milestone, because the declared structure and the wiring disagree.
    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    verdict_routing:\n      request_changes: implement\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].verdict_routing"),
        &Fault::ContradictsStructure {
            structure: "linear"
        }
    );
}

#[test]
fn an_empty_steps_list_is_refused() {
    let refused = refusals(parse(
        "version: 1\nworkflow_id: fixture\nname: bug\nstructure: linear\nsteps: []\n",
    ));
    assert_eq!(fault_at(&refused, "steps"), &Fault::Empty);
    assert!(!refused_names_a_step(&refused));
}

fn refused_names_a_step(refusals: &[crate::error::Refusal]) -> bool {
    refused(refusals, "steps[0]")
}
