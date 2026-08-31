//! Every workflow sample, read by the M1 parser.
//!
//! **These are real definitions, not fixtures.** `workflows.toml` says the
//! definitions themselves are the JSON beside it, so a sample that does not
//! load is a defect in the sample rather than a test asset.
//!
//! Four of them once declared `structure: "linear"` while routing a review
//! back to an earlier step, which is a loop. They were authored where nothing
//! validated them, and the mislabel was invisible until a parser existed.
//! `structure` was the field that was wrong — the routing is the behaviour
//! somebody designed, and deleting it to satisfy the label would have removed
//! a step rather than corrected one.
//!
//! **Every sample is still refused, and that is not the same claim.** M1 carries
//! `linear` only, so a loop is refused for its structure; the two linear ones
//! are refused for keys this milestone defers. Refused for a reason a later
//! milestone removes is different from refused for being wrong, and this file
//! exists to keep those apart.

use std::path::{Path, PathBuf};

use crate::error::{Fault, LoadError, Refusal};
use crate::workflow::WorkflowDef;

/// Every sample, and whether the audit's finding holds against it.
const SAMPLES: &[(&str, Verdict)] = &[
    ("bug.json", Verdict::Loop),
    ("code-review.json", Verdict::LinearAndConsistent),
    ("design-plan.json", Verdict::Loop),
    ("feature.json", Verdict::Loop),
    ("prototype.json", Verdict::LinearAndConsistent),
    ("refactor.json", Verdict::Loop),
    ("revert.json", Verdict::Loop),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// Held the contradiction the audit named — `structure: linear` carrying a
    /// routing edge — until the structure was corrected. No sample is this now,
    /// and the variant stays so the contradiction has a name if one returns.
    #[allow(dead_code)]
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
    match WorkflowDef::load(&sample(name), &crate::tests::roster()) {
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
fn no_sample_declares_a_structure_its_own_routing_contradicts() {
    // Four of these once said `linear` and routed a review back to an earlier
    // step, which is a loop. They were authored where nothing validated them,
    // and the mislabel was invisible until a parser existed. `structure` was
    // the field that was wrong: the routing is the workflow's real behaviour,
    // and deleting it to satisfy the label would have removed a step somebody
    // designed.
    for (name, _) in SAMPLES {
        let refusals = load(name);
        let found = routing_refusals(&refusals);
        assert!(
            found.is_empty(),
            "{name} declares a structure its own routing contradicts: {found:?}"
        );
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
    // `workflow_id` is no longer among them: M1 reads it now, because nothing
    // else joined a proposal's `workflow_id` to a workflow and a Job could be
    // proposed against one that does not exist. The sample already carried the
    // key; what changed is that the parser stopped deferring it.
    let refusals = load("bug.json");
    for key in ["default_gate_policy"] {
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
