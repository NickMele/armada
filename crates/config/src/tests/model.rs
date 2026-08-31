//! `model`, on a step and on a judge check: what a definition may name, and
//! what it may not.
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

/// A step declaring one judge check whose `model` is `model`, on a step that
/// names no model of its own — so the two keys are never both set and a test
/// that reads one cannot be reading the other.
fn judged(model: &str) -> String {
    [
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
        &format!("      - model: {model}"),
        "        criteria:",
        "          - criterion_id: c1",
        "            question: Does it?",
        "",
    ]
    .join("\n")
}

#[test]
fn a_judge_check_names_the_model_that_reads_the_work() {
    let def = parse(&judged("the-reporting-model"), &roster()).expect("a legal model");
    assert_eq!(
        def.steps()[0].judge_checks()[0].model(),
        Some(&ModelName::new("the-reporting-model").expect("a model name"))
    );
}

/// **The one that matters, and the reason this key came under the roster.** A
/// typo here used to parse, freeze onto the Job and reach the Judge adapter —
/// which is a gate that cannot rule, asked for after the Drone has run and the
/// work is done. A step's typo costs the spawn; this one costs the step that
/// already happened.
#[test]
fn a_judge_model_the_machine_does_not_offer_is_refused_at_load() {
    let refusals = refusals(parse(&judged("the-repoting-model"), &roster()));
    let Fault::NoSuchModel { value, roster } =
        fault_at(&refusals, "steps[0].judge_checks[0].model")
    else {
        panic!("expected a model refusal, got {refusals:?}");
    };
    assert_eq!(
        value, "the-repoting-model",
        "the message names the value that was written, so the typo is visible \
         without opening the file"
    );
    assert_eq!(
        roster,
        &vec![
            "the-deciding-model".to_string(),
            "the-reporting-model".to_string()
        ]
    );
}

/// `judge_checks[].model: ""` stays [`Fault::Empty`], and is refused once.
///
/// The blank refusal was written out a second time inside `crate::judge` back
/// when that reader made its own `ModelName`; `yaml::text` had already refused
/// the key. Narrowing to the roster removed the copy rather than adding a
/// third — [`fault_at`] fails if two refusals name one key.
#[test]
fn a_blank_judge_model_is_empty_rather_than_unknown() {
    let refusals = refusals(parse(&judged("\"\""), &roster()));
    assert_eq!(
        fault_at(&refusals, "steps[0].judge_checks[0].model"),
        &Fault::Empty
    );
}

/// **Two dials with one spelling, and one legal set.** The step's `model` is
/// what a Drone is spawned as and the check's is what a Judge call is read by;
/// they default apart in `adapters` and neither falls back to the other. What
/// they share is the roster, because `HeadlessAgent::judge_model` is an entry
/// of that same roster rather than a constant of its own — so a Judge model
/// outside it is a name the binary would not take either.
#[test]
fn the_two_model_keys_are_separate_and_neither_fills_in_for_the_other() {
    let text = [
        "version: 1",
        "workflow_id: judged",
        "name: judged",
        "structure: linear",
        "steps:",
        "  - id: review",
        "    label: Review",
        "    evidence_type: diff",
        "    model: the-deciding-model",
        "    advance_gate: auto_if_judge_passes",
        "    judge_checks:",
        "      - model: the-reporting-model",
        "        criteria:",
        "          - criterion_id: c1",
        "            question: Does it?",
        "",
    ]
    .join("\n");
    let def = parse(&text, &roster()).expect("both models are on the roster");
    let step = &def.steps()[0];
    assert_eq!(
        step.model(),
        Some(&ModelName::new("the-deciding-model").expect("a model name"))
    );
    assert_eq!(
        step.judge_checks()[0].model(),
        Some(&ModelName::new("the-reporting-model").expect("a model name")),
        "the check keeps its own model rather than taking the step's"
    );
}

/// A step naming a model with no judge check of its own leaves the check's
/// absent. **Absent is the adapter's default and not the step's model**, and a
/// parser copying one into the other would put the fallback in a second place.
#[test]
fn a_judge_check_that_names_no_model_carries_none() {
    let text = [
        "version: 1",
        "workflow_id: judged",
        "name: judged",
        "structure: linear",
        "steps:",
        "  - id: review",
        "    label: Review",
        "    evidence_type: diff",
        "    model: the-deciding-model",
        "    advance_gate: auto_if_judge_passes",
        "    judge_checks:",
        "      - criteria:",
        "          - criterion_id: c1",
        "            question: Does it?",
        "",
    ]
    .join("\n");
    let def = parse(&text, &roster()).expect("a legal model");
    assert_eq!(def.steps()[0].judge_checks()[0].model(), None);
}
