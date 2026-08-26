//! The seven WorkflowDefs checked into this repository, run through this
//! parser.
//!
//! **A validator that cannot reject the samples in its own repository is not a
//! validator.** These files are the full schema, verbatim as the design work
//! wrote them, and every one of them is refused here — which is the correct
//! outcome and worth stating rather than discovering: M1 implements two of the
//! `armada.yml` sections and five of the nine step fields, so a file carrying
//! all nine cannot load and must not be made to.
//!
//! **Nothing under `workflow-samples/` is edited to make this pass.** The
//! registry's own README says the source disagrees with itself and that
//! repairing it here would destroy the finding. These tests are that finding,
//! written as assertions.
//!
//! # The audit the milestone step asked to be confirmed
//!
//! Four of the seven — `bug`, `feature`, `refactor`, `revert` — declare
//! `structure: "linear"` and carry `verdict_routing` on their review step,
//! which the `structure` field's own rule rejects at config load. That is
//! confirmed here: those four and only those four produce a
//! [`Fault::ContradictsStructure`]. `code_review` and `prototype` are linear
//! and carry none; `design_plan` is the one `loop`, where the same
//! `verdict_routing` is legal and the structure value is what M1 refuses.
//!
//! JSON is read by the YAML parser because YAML is a superset of it. That is
//! not a workaround: the samples are JSON because that is how the design work
//! recorded them, and an `armada.yml` is YAML because that is what a repo
//! authors. One reader covers both.

use std::path::{Path, PathBuf};

use crate::error::{Fault, LoadError, Refusal};
use crate::workflow::WorkflowDef;

/// Every sample, and whether the audit's finding holds against it.
const SAMPLES: &[(&str, Verdict)] = &[
    ("bug.json", Verdict::LinearWithRouting),
    ("code-review.json", Verdict::LinearAndConsistent),
    ("design-plan.json", Verdict::Loop),
    ("feature.json", Verdict::LinearWithRouting),
    ("prototype.json", Verdict::LinearAndConsistent),
    ("refactor.json", Verdict::LinearWithRouting),
    ("revert.json", Verdict::LinearWithRouting),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// `structure: linear` and a `verdict_routing` somewhere in its steps. The
    /// contradiction the audit named.
    LinearWithRouting,
    /// `structure: linear` and no routing edge. Refused for the M1 slice only.
    LinearAndConsistent,
    /// `structure: loop`, which M1 does not carry. Its routing edge is legal.
    Loop,
}

fn sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core-model/domain/workflow-samples")
        .join(name)
}

fn load(name: &str) -> Vec<Refusal> {
    match WorkflowDef::load(&sample(name)) {
        Ok(def) => panic!("{name} loaded as an M1 WorkflowDef: {def:?}"),
        Err(LoadError::Refused { refusals, .. }) => refusals,
        Err(other) => panic!("{name}: {other}"),
    }
}

fn routing_refusals(refusals: &[Refusal]) -> Vec<&str> {
    refusals
        .iter()
        .filter(|r| {
            r.fault
                == Fault::ContradictsStructure {
                    structure: "linear",
                }
        })
        .map(|r| r.key.as_str())
        .collect()
}

#[test]
fn every_sample_is_refused_by_the_m1_parser() {
    for (name, _) in SAMPLES {
        let refusals = load(name);
        assert!(!refusals.is_empty(), "{name} produced no refusals");
    }
}

#[test]
fn the_four_samples_the_audit_named_carry_a_routing_edge_on_a_linear_workflow() {
    for (name, verdict) in SAMPLES {
        let refusals = load(name);
        let found = routing_refusals(&refusals);
        match verdict {
            Verdict::LinearWithRouting => assert_eq!(
                found.len(),
                1,
                "{name} should carry exactly one routing edge, found {found:?}"
            ),
            Verdict::LinearAndConsistent | Verdict::Loop => {
                assert!(
                    found.is_empty(),
                    "{name} should carry none, found {found:?}"
                );
            }
        }
    }
}

#[test]
fn the_loop_sample_is_refused_for_its_structure_and_not_for_its_routing() {
    // `design_plan` is the control for the test above: the same
    // `verdict_routing` that contradicts `linear` is what declares the loop
    // here, and refusing it as a contradiction would be wrong.
    let refusals = load("design-plan.json");
    assert_eq!(
        refusals
            .iter()
            .find(|r| r.key == "structure")
            .map(|r| &r.fault),
        Some(&Fault::OutsideM1 {
            value: "loop".to_string(),
            carried: &["linear"],
        })
    );
}

#[test]
fn every_sample_step_is_missing_only_its_label() {
    // `id` was settled in the registry's favour on spelling and against it on
    // existence: the field row was renamed from `step_id` to `id` and a `label`
    // row was added, because `id` is what everything counts against and a name
    // that can be reworded without renaming what the system routes on is the
    // reason there are two fields rather than one.
    //
    // No sample carries a label yet, and that is the only thing left for a
    // sample's steps to be short of. This test is what will fail when somebody
    // writes them.
    let refusals = load("bug.json");
    assert!(
        !refusals
            .iter()
            .any(|r| r.key.ends_with(".step_id") || r.key.ends_with(".id")),
        "id is spelled the same in the registry, the samples and the parser now: {refusals:?}"
    );
    assert!(
        refusals
            .iter()
            .any(|r| r.key == "steps[0].label" && r.fault == Fault::Missing),
        "{refusals:?}"
    );
}

#[test]
fn the_samples_top_level_keys_that_m1_defers_are_named_one_by_one() {
    // Not "this file is wrong" — every deferred section by name, so the diff
    // between the full schema and the M1 slice is readable off the refusal.
    let refusals = load("bug.json");
    for key in ["workflow_id", "default_gate_policy"] {
        assert!(
            refusals
                .iter()
                .any(|r| r.key == key && matches!(r.fault, Fault::Unknown { .. })),
            "{key} should be refused by name: {refusals:?}"
        );
    }
}

#[test]
fn the_code_review_sample_names_an_evidence_type_the_schema_does_not_have() {
    // `review_findings`, which the registry records as an unanswered question
    // against itself. The parser surfaces it rather than widening the set.
    let refusals = load("code-review.json");
    assert!(
        refusals.iter().any(|r| r.key == "steps[1].evidence_type"
            && matches!(r.fault, Fault::NotInTheSchema { .. })),
        "{refusals:?}"
    );
}
