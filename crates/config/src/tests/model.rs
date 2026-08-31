//! `model` on a step: what a definition may name, and what it may not.
//!
//! Its own file rather than a section of [`super::workflow`], for the reason
//! that one is split by subject: the interesting thing about this key is not
//! that it parses but *when it is refused*, and that reads as one argument from
//! top to bottom.
//!
//! **The refusal is the half worth testing.** A model name that parses and is
//! wrong does not fail until spawn, where `fleet::spawning` turns a
//! `SpawnConfigRefused` into an interrupt and escalates the Job — by which time
//! there is a worktree, an approval and every earlier step's work behind it,
//! and the Job is reported as `Interrupted`, which names the wrong cause.

use core_model::ModelName;

use crate::error::{Fault, LoadError};
use crate::roster::Roster;
use crate::tests::{fault_at, named, refusals, roster};
use crate::workflow::WorkflowDef;

/// A two-step definition: the first names `model`, the second names none.
fn two_steps(model: &str) -> String {
    format!(
        "version: 1\nworkflow_id: modelled\nname: modelled\nstructure: linear\nsteps:\n  \
         - id: decide\n    label: Decide\n    evidence_type: diff\n    advance_gate: auto\n  \
         - id: report\n    label: Report\n    evidence_type: facts_note\n    model: \
         {model}\n    advance_gate: auto\n"
    )
}

fn parse(text: &str, roster: &Roster) -> Result<WorkflowDef, LoadError> {
    WorkflowDef::parse(&named("workflows/modelled.yml"), text, roster)
}

#[test]
fn a_step_names_the_model_its_work_needs() {
    let def = parse(&two_steps("the-reporting-model"), &roster()).expect("a legal model");
    assert_eq!(
        def.steps()[1].model(),
        Some(&ModelName::new("the-reporting-model").expect("a model name"))
    );
}

/// **Absent stays absent.** The fallback to the Job's model is spelled on the
/// record, at `Job::model_at`, and a parser that filled the default in here
/// would put a second copy of it in every workflow — which is the thing that
/// then has to be kept in sync when the default moves.
#[test]
fn a_step_that_names_none_carries_none() {
    let def = parse(&two_steps("the-reporting-model"), &roster()).expect("a legal model");
    assert_eq!(def.steps()[0].model(), None);
}

/// **The one that matters.** A name the machine does not have is a typo, and a
/// typo in a file is refused where the file is read.
#[test]
fn a_model_the_machine_does_not_offer_is_refused_at_load() {
    let refusals = refusals(parse(&two_steps("the-repoting-model"), &roster()));
    let Fault::NoSuchModel { value, roster } = fault_at(&refusals, "steps[1].model") else {
        panic!("expected a model refusal, got {refusals:?}");
    };
    assert_eq!(value, "the-repoting-model");
    assert_eq!(
        roster,
        &vec![
            "the-deciding-model".to_string(),
            "the-reporting-model".to_string()
        ],
        "the message names what this machine does offer, because the legal set \
         is the caller's rather than the schema's and a reader cannot look it up"
    );
}

/// The refusal names the machine's list rather than a constant, so the same
/// file is legal on a machine that offers the name and refused on one that does
/// not — which is the difference between this and every other closed set the
/// parser reads.
#[test]
fn the_same_definition_is_legal_against_a_roster_that_offers_the_name() {
    let text = two_steps("something-else");
    assert!(parse(&text, &roster()).is_err());
    assert!(parse(&text, &Roster::of(["something-else"])).is_ok());
}

/// A machine offering nothing refuses every name. **Not read as "accept
/// anything"**: the caller that failed to resolve a roster would otherwise be
/// the one caller with no check.
#[test]
fn a_roster_offering_nothing_refuses_a_name_that_is_otherwise_legal() {
    let refusals = refusals(parse(
        &two_steps("the-reporting-model"),
        &Roster::offering_nothing(),
    ));
    assert!(matches!(
        fault_at(&refusals, "steps[1].model"),
        Fault::NoSuchModel { .. }
    ));
}

/// `model: ""` is `Empty` at the key and not a name nothing offers. The author
/// wrote the key and left it blank, which is a different mistake with a
/// different fix — and it is the distinction [`Fault::Empty`] already exists
/// for.
#[test]
fn a_blank_model_is_empty_rather_than_unknown() {
    let refusals = refusals(parse(&two_steps("\"\""), &roster()));
    assert_eq!(fault_at(&refusals, "steps[1].model"), &Fault::Empty);
}

/// The step's `model` and a judge check's `model` are two dials with one
/// spelling. **Only the step's is checked against the roster** — the Judge's is
/// read by `crate::judge` and refused only for being blank, which is what it
/// did before this key existed and is not changed here.
#[test]
fn a_judge_checks_model_is_a_different_key_and_is_not_checked_against_the_roster() {
    let text = [
        "version: 1",
        "workflow_id: judged",
        "name: judged",
        "structure: linear",
        "steps:",
        "  - id: review",
        "    label: Review",
        "    evidence_type: diff",
        "    advance_gate: auto_if_judge_passes",
        "    judge_checks:",
        "      - model: a-model-no-roster-has",
        "        criteria:",
        "          - criterion_id: c1",
        "            question: Does it?",
        "",
    ]
    .join("\n");
    let def = parse(&text, &roster()).expect("the judge's model is not this roster's business");
    assert_eq!(def.steps()[0].model(), None);
    assert_eq!(
        def.steps()[0].judge_checks()[0]
            .model()
            .map(ModelName::as_str),
        Some("a-model-no-roster-has")
    );
}
