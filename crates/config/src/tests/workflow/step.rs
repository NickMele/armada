//! One step's keys: what each is read as, what absent means, and the two
//! refusals that need the whole step in hand.
//!
//! The gate and the Judge are the pair worth reading together — one is a
//! statement about which tier decides and the other is that tier's
//! declaration, so every arrangement of the two is asserted here, including the
//! two where the rule does not apply.

use core_model::{AdvanceGate, EvidenceRef, GamingPattern};

use super::{bug_with, parse, BUG};
use crate::error::Fault;
use crate::tests::{fault_at, refusals};

/// A fourth step that puts one question to the Judge, at the gate and panel
/// given. Built line by line because the indentation is the syntax.
fn judged(gate: &str, panel_size: u32, enabled: bool) -> String {
    [
        "  - id: review".to_string(),
        "    label: Review".to_string(),
        // A judged step declares what it produces, because the Judge is shown
        // the work product and a step declaring none produces nothing.
        "    evidence_type: diff".to_string(),
        format!("    advance_gate: {gate}"),
        "    judge_checks:".to_string(),
        format!("      - enabled: {enabled}"),
        // The fixture roster's spelling, not a vendor's: a check's `model` is
        // refused against the same roster a step's is. `super::model` owns
        // that, and this fixture only has to be legal.
        "        model: the-reporting-model".to_string(),
        format!("        panel_size: {panel_size}"),
        "        criteria:".to_string(),
        "          - criterion_id: c1".to_string(),
        "            question: Does the fix address the cause?".to_string(),
        String::new(),
    ]
    .join("\n")
}

#[test]
fn a_step_needs_both_an_id_and_a_label() {
    // `id` is stable and everything routes on it; `label` is display only. Both
    // are required, so a workflow cannot be authored with one doing both jobs.
    let refused = refusals(parse(
        "version: 1\nworkflow_id: fixture\nname: bug\nstructure: linear\nsteps:\n  - id: plan\n    advance_gate: auto\n  - label: Implement\n    advance_gate: auto\n",
    ));
    assert_eq!(fault_at(&refused, "steps[0].label"), &Fault::Missing);
    assert_eq!(fault_at(&refused, "steps[1].id"), &Fault::Missing);
}

/// The prefix form, and it is the only gate still outside the milestone. It
/// names a key resolved against a Manifest-level policy and across a Convoy's
/// gating Manifests, and neither is built — so it is refused rather than read
/// as the value that policy would most often produce.
#[test]
fn a_gate_needing_a_manifest_policy_is_refused() {
    for key in ["review_gate", "auto_merge"] {
        let gate = format!("manifest_rule:{key}");
        let refused = refusals(bug_with(&format!(
            "  - id: review\n    label: Review\n    advance_gate: {gate}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[3].advance_gate"),
            &Fault::NotYetCarried {
                value: gate,
                carried: &["auto", "auto_if_judge_passes", "human_always"],
            }
        );
    }
}

/// The gate that makes `awaiting_review` reachable. It carries with no Judge,
/// which is what Design Plan's `present` and Prototype's `build` declare.
#[test]
fn a_human_gate_loads_and_needs_no_judge_to_do_it() {
    let def = bug_with("  - id: review\n    label: Review\n    advance_gate: human_always\n")
        .expect("a step a person answers");
    assert_eq!(def.steps()[3].advance_gate(), AdvanceGate::HumanAlways);
}

/// **The gate-and-judge agreement rule does not reach a human gate**, in either
/// direction, because the rule compares a gate that names a tier against that
/// tier's declaration and this gate names an actor. A Judge here is not spent
/// on an answer nothing reads: a refusal stops the step before the work reaches
/// a person, and a criterion it did not refuse is written down beside the
/// evidence they open. Feature's and Revert's `review` declare exactly this.
#[test]
fn a_human_gate_takes_a_judge_and_takes_none() {
    let asks = bug_with(&judged("human_always", 1, true)).expect("a human gate the Judge feeds");
    let step = &asks.steps()[3];
    assert_eq!(step.advance_gate(), AdvanceGate::HumanAlways);
    assert_eq!(step.judge_checks()[0].criteria().len(), 1);

    let unjudged = bug_with(&judged("human_always", 1, false))
        .expect("and a human gate that asks the Judge nothing");
    assert_eq!(unjudged.steps()[3].advance_gate(), AdvanceGate::HumanAlways);
    assert!(!unjudged.steps()[3].judge_checks()[0].fires());
}

#[test]
fn a_judge_gate_with_no_criterion_is_refused_and_so_is_a_criterion_with_no_judge_gate() {
    let no_criterion = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto_if_judge_passes\n",
    ));
    assert_eq!(
        fault_at(&no_criterion, "steps[3].advance_gate"),
        &Fault::GateAndJudgeDisagree {
            gate: "auto_if_judge_passes",
        }
    );

    let no_gate = refusals(bug_with(&judged("auto", 1, true)));
    assert_eq!(
        fault_at(&no_gate, "steps[3].advance_gate"),
        &Fault::GateAndJudgeDisagree { gate: "auto" }
    );
}

/// **The blind judge check, refused where it is written.** A step with no
/// `evidence_type` produces nothing, so a criterion on it is a call made
/// against an empty page and a refusal that could not have gone otherwise.
/// Four of these shipped in one commit and the first Job to reach one
/// escalated at step one — #153.
#[test]
fn a_step_that_asks_the_judge_and_produces_nothing_is_refused() {
    let blind = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto_if_judge_passes\n    \
         judge_checks:\n      - criteria:\n          - criterion_id: c1\n            \
         question: Is this right?\n",
    ));
    assert_eq!(
        fault_at(&blind, "steps[3].judge_checks"),
        &Fault::JudgedWithNothingToShow
    );
}

/// The same rule does not reach a step that asks nothing. Bug's `merge`
/// declares no evidence type and no Judge, and it is the common shape for a
/// hand-off step rather than an edge case.
#[test]
fn a_step_that_produces_nothing_and_asks_nothing_still_loads() {
    let def = bug_with("  - id: merge\n    label: Merge\n    advance_gate: auto\n")
        .expect("a step that produces nothing a Judge reads");
    assert!(def.steps()[3].evidence_type().is_none());
    assert!(def.steps()[3].judge_checks().is_empty());
}

#[test]
fn a_step_carries_the_criteria_it_declares_and_the_panel_it_asks_for() {
    let def = bug_with(&judged("auto_if_judge_passes", 3, true)).expect("a step the Judge reads");
    let judge = &def.steps()[3].judge_checks()[0];
    assert_eq!(judge.panel_size(), 3);
    assert_eq!(judge.criteria().len(), 1);
    assert_eq!(
        judge.criteria()[0].question,
        "Does the fix address the cause?"
    );
    // Three judges on one criterion, and a step that asks nothing makes none.
    assert_eq!(judge.calls(), 3);
    assert_eq!(def.steps()[0].judge_checks().len(), 0);
}

#[test]
fn a_disabled_judge_check_asks_nothing_and_reads_as_a_step_that_declares_none() {
    // `enabled: false` and an absent check are the registry's own synonyms, so
    // the gate has to stay `auto` — which is what says the two really are one.
    let def =
        bug_with(&judged("auto", 1, false)).expect("a step whose Judge check is switched off");
    assert!(!def.steps()[3].judge_checks()[0].fires());
}

/// A `gaming_check` on a step that puts no criterion to the Judge, which is
/// the shape the samples do not have and the design allows: the second look
/// gates nothing, so it does not make `advance_gate` the Judge's.
fn gamed(baseline: &str, patterns: &[&str]) -> String {
    let mut text = [
        "  - id: review",
        "    label: Review",
        "    advance_gate: auto",
        "    judge_checks:",
        "      - gaming_check:",
    ]
    .join("\n");
    text.push_str(&format!("\n          baseline_ref: \"{baseline}\"\n"));
    text.push_str("          flag_if:\n");
    for pattern in patterns {
        text.push_str(&format!("            - {pattern}\n"));
    }
    text
}

#[test]
fn a_step_carries_the_gaming_patterns_it_declares_and_the_baseline_it_names() {
    let def = bug_with(&gamed(
        "root_cause.evidence",
        &["assertion_weakened", "check_config_edited"],
    ))
    .expect("a step that asks whether its evidence was gamed");
    let judge = &def.steps()[3].judge_checks()[0];
    let gaming = judge.gaming().expect("a gaming check");
    assert_eq!(
        gaming.baseline().map(EvidenceRef::as_wire).as_deref(),
        Some("root_cause.evidence")
    );
    assert_eq!(
        gaming.flag_if(),
        [
            GamingPattern::AssertionWeakened,
            GamingPattern::CheckConfigEdited
        ]
    );
    // One call, not two: the diff answers `check_config_edited`, and a
    // mechanical pattern that cost a call would be money spent on `git diff`.
    assert_eq!(gaming.calls(), 1);
    // The gaming check does not gate, so the step stays `auto` and the Judge
    // is not what advances it.
    assert!(!judge.fires());
}

/// A pattern nothing knows is refused rather than dropped. A silently ignored
/// `flag_if` entry is a gate the author believes is watching and nothing is.
#[test]
fn a_flag_if_naming_a_pattern_nothing_knows_is_refused() {
    let refused = refusals(bug_with(&gamed("root_cause.evidence", &["looks_dodgy"])));
    assert!(matches!(
        fault_at(&refused, "steps[3].judge_checks[0].gaming_check.flag_if[0]"),
        Fault::NotInTheSchema { .. }
    ));
}

/// `baseline_ref` names a step's evidence, not a step. The two are different
/// things and the second is what an author writes by mistake.
#[test]
fn a_baseline_ref_that_is_not_a_step_s_evidence_is_refused() {
    let refused = refusals(bug_with(&gamed("root_cause", &["test_deleted"])));
    assert!(matches!(
        fault_at(
            &refused,
            "steps[3].judge_checks[0].gaming_check.baseline_ref"
        ),
        Fault::NotInTheSchema { .. }
    ));
}

#[test]
fn a_gate_the_schema_never_had_is_a_different_refusal_from_a_deferred_one() {
    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: whenever\n",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[3].advance_gate"),
        Fault::NotInTheSchema { .. }
    ));
}

#[test]
fn a_step_key_m1_does_not_read_hard_fails() {
    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    hard_prerequisite: true\n",
    ));
    assert!(
        refused
            .iter()
            .any(|r| r.key == "steps[3].hard_prerequisite"
                && matches!(r.fault, Fault::Unknown { .. })),
        "hard_prerequisite should be an unknown step key: {refused:?}"
    );
}

/// `retry_limit` was one of those keys until there was a ledger to spend it
/// against. It is read now, and this is what it is read as.
#[test]
fn a_step_carries_the_retry_budget_it_declares() {
    let def =
        bug_with("  - id: review\n    label: Review\n    advance_gate: auto\n    retry_limit: 3\n")
            .expect("a step declaring a retry budget loads");
    assert_eq!(def.steps()[3].retry_limit(), 3);
}

/// **Absent is none, and none is what every step meant before the key was
/// read.** A default invented in the parser would put the budget in the one
/// place an author reading the workflow could not see it.
#[test]
fn a_step_that_declares_no_retry_budget_has_none() {
    let def = parse(BUG).expect("the worked example");
    assert!(def.steps().iter().all(|step| step.retry_limit() == 0));
}

/// Zero is a sentence, not an absence: it says the first failure is the last.
#[test]
fn a_retry_budget_of_zero_loads_where_a_version_of_zero_would_not() {
    let def =
        bug_with("  - id: review\n    label: Review\n    advance_gate: auto\n    retry_limit: 0\n")
            .expect("zero is a legal budget");
    assert_eq!(def.steps()[3].retry_limit(), 0);
}

/// A file that wrote a budget meant to buy retries. Reading it as none would
/// be the parser quietly deciding the budget.
#[test]
fn a_retry_budget_that_is_not_a_count_is_refused_rather_than_read_as_none() {
    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    retry_limit: three\n",
    ));
    assert!(
        refused
            .iter()
            .any(|r| r.key == "steps[3].retry_limit" && matches!(r.fault, Fault::WrongType { .. })),
        "{refused:?}"
    );
}

/// **Each half on its own, which is the whole reason there are two keys.** A
/// step that wants longer between pokes does not thereby want more pokes, so
/// declaring one must leave the other deferring — and `None` here is what makes
/// `fleet::Liveness::at` fall back for that half alone.
#[test]
fn a_step_carries_either_half_of_its_patience_without_the_other() {
    let waits = bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    \
         quiet_after_seconds: 900\n",
    )
    .expect("a step declaring how long its Drone may be quiet loads");
    assert_eq!(waits.steps()[3].quiet_after_seconds(), Some(900));
    assert_eq!(waits.steps()[3].poke_limit(), None);

    let nudges =
        bug_with("  - id: review\n    label: Review\n    advance_gate: auto\n    poke_limit: 5\n")
            .expect("a step declaring how many nudges its Drone gets loads");
    assert_eq!(nudges.steps()[3].poke_limit(), Some(5));
    assert_eq!(nudges.steps()[3].quiet_after_seconds(), None);
}

/// **Absent is Fleet's, and a default invented here would be a second place the
/// shipped number lives** — which is the state `#60` found: a value nobody
/// could find because it was written down as a constant.
#[test]
fn a_step_that_declares_no_patience_has_none() {
    let def = parse(BUG).expect("the worked example");
    assert!(def
        .steps()
        .iter()
        .all(|step| step.quiet_after_seconds().is_none() && step.poke_limit().is_none()));
}

/// The two keys disagree about zero, and each is right about its own.
///
/// A `poke_limit: 0` is a step saying its Drone gets no nudge at all, which is
/// a sentence somebody is entitled to write. A `quiet_after_seconds: 0` is a
/// step whose Drone is quiet the instant it is spawned — poked on the first
/// turn and escalated by the third — which is nobody's intention.
#[test]
fn no_pokes_is_a_sentence_and_no_patience_is_not() {
    let none =
        bug_with("  - id: review\n    label: Review\n    advance_gate: auto\n    poke_limit: 0\n")
            .expect("zero pokes is a legal budget");
    assert_eq!(none.steps()[3].poke_limit(), Some(0));

    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    \
         quiet_after_seconds: 0\n",
    ));
    assert!(
        refused
            .iter()
            .any(|r| r.key == "steps[3].quiet_after_seconds"
                && matches!(r.fault, Fault::WrongType { .. })),
        "{refused:?}"
    );
}

/// A file that wrote a patience meant to buy patience. Reading it as absent
/// would put the step back on Fleet's value with nothing saying so — and the
/// symptom is a Job that escalates as `stalled` in the middle of the work it
/// declared itself slow for.
#[test]
fn a_patience_that_is_not_a_count_is_refused_rather_than_read_as_absent() {
    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    \
         quiet_after_seconds: fifteen minutes\n",
    ));
    assert!(
        refused
            .iter()
            .any(|r| r.key == "steps[3].quiet_after_seconds"
                && matches!(r.fault, Fault::WrongType { .. })),
        "{refused:?}"
    );
}

#[test]
fn an_evidence_type_outside_the_schema_is_refused() {
    // `review_findings` is used by a checked-in sample and is not among the
    // legal values. The registry records that as an open question; until it is
    // answered, the parser refuses it by name rather than guessing.
    let refused = refusals(bug_with(
        "  - id: assess\n    label: Assess\n    evidence_type: review_findings\n    advance_gate: auto\n",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[3].evidence_type"),
        Fault::NotInTheSchema { .. }
    ));
}

#[test]
fn a_step_may_declare_no_evidence_type() {
    let def = bug_with("  - id: merge\n    label: Merge\n    advance_gate: auto\n")
        .expect("a step that produces nothing a Judge reads");
    assert_eq!(def.steps()[3].evidence_type(), None);
}
