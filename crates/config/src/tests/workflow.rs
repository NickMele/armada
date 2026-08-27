//! A WorkflowDef: the M1 slice, the four validations, and the one cross-file
//! check that is the point of this milestone step.

use core_model::{AdvanceGate, EvidenceRef, EvidenceType, GamingPattern, ResolvedCheck};

use crate::error::{Fault, LoadError, ResolveError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{fault_at, named, refusals, refused};
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
      - { type: diff_nonempty }
    advance_gate: auto
  - id: verify
    label: Run tests
    evidence_type: test_suite_run
    mechanical_checks:
      - { type: manifest_check, check: test, expect_exit_code: 0 }
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
    WorkflowDef::parse(&named("workflows/bug.yml"), text)
}

fn manifest() -> Manifest {
    Manifest::parse(&named("armada.yml"), MANIFEST).expect("the fixture manifest")
}

/// `BUG` with one step's body replaced, so a test changes one thing.
fn bug_with(extra: &str) -> Result<WorkflowDef, LoadError> {
    parse(&format!("{BUG}{extra}"))
}

/// A fifth step that puts one question to the Judge, at the gate and panel
/// given. Built line by line because the indentation is the syntax.
fn judged(gate: &str, panel_size: u32, enabled: bool) -> String {
    [
        "  - id: review".to_string(),
        "    label: Review".to_string(),
        format!("    advance_gate: {gate}"),
        "    judge_checks:".to_string(),
        format!("      - enabled: {enabled}"),
        "        model: haiku".to_string(),
        format!("        panel_size: {panel_size}"),
        "        criteria:".to_string(),
        "          - criterion_id: c1".to_string(),
        "            question: Does the fix address the cause?".to_string(),
        String::new(),
    ]
    .join("\n")
}

#[test]
fn the_worked_example_loads() {
    let def = parse(BUG).expect("the worked example");
    assert_eq!(def.name(), "bug");
    assert_eq!(def.version(), 1);
    assert_eq!(def.structure(), Structure::Linear);
    assert_eq!(def.steps().len(), 4);

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
    assert_eq!(ids, ["plan", "implement", "verify", "handoff"]);
}

#[test]
fn a_step_with_no_mechanical_checks_is_the_common_case() {
    // Two of four steps carry none. The field is optional and its absence is an
    // empty list, never a refusal.
    let def = parse(BUG).expect("the worked example");
    let gated: Vec<usize> = def
        .steps()
        .iter()
        .map(|s| s.mechanical_checks().len())
        .collect();
    assert_eq!(gated, [0, 2, 1, 0]);
}

#[test]
fn implement_carries_two_checks_because_a_build_passes_on_an_empty_diff() {
    let def = parse(BUG).expect("the worked example");
    assert_eq!(
        def.steps()[1].mechanical_checks(),
        [
            MechanicalCheck::ManifestCheck {
                check: "build".to_string(),
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
        fault_at(&refused, "steps[4].id"),
        &Fault::DuplicateStepId { first_at: 0 }
    );
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

#[test]
fn structure_loop_is_outside_m1_and_says_so() {
    let refused = refusals(parse(
        "version: 1\nworkflow_id: fixture\nname: design_plan\nstructure: loop\nsteps:\n  - id: draft\n    label: Draft\n    advance_gate: auto\n",
    ));
    assert_eq!(
        fault_at(&refused, "structure"),
        &Fault::OutsideM1 {
            value: "loop".to_string(),
            carried: &["linear"],
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
        fault_at(&refused, "steps[4].verdict_routing"),
        &Fault::ContradictsStructure {
            structure: "linear"
        }
    );
}

#[test]
fn a_gate_needing_a_manifest_policy_is_refused() {
    // Two of the schema's four values resolve through a Manifest-level policy
    // that does not exist. The other two — `auto` and `auto_if_judge_passes` —
    // are carried, because both tiers they name are built.
    for gate in ["human_always", "manifest_rule:review_gate"] {
        let refused = refusals(bug_with(&format!(
            "  - id: review\n    label: Review\n    advance_gate: {gate}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[4].advance_gate"),
            &Fault::OutsideM1 {
                value: gate.to_string(),
                carried: &["auto", "auto_if_judge_passes"],
            }
        );
    }
}

#[test]
fn a_judge_gate_with_no_criterion_is_refused_and_so_is_a_criterion_with_no_judge_gate() {
    let no_criterion = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto_if_judge_passes\n",
    ));
    assert_eq!(
        fault_at(&no_criterion, "steps[4].advance_gate"),
        &Fault::GateAndJudgeDisagree {
            gate: "auto_if_judge_passes",
        }
    );

    let no_gate = refusals(bug_with(&judged("auto", 1, true)));
    assert_eq!(
        fault_at(&no_gate, "steps[4].advance_gate"),
        &Fault::GateAndJudgeDisagree { gate: "auto" }
    );
}

#[test]
fn a_step_carries_the_criteria_it_declares_and_the_panel_it_asks_for() {
    let def = bug_with(&judged("auto_if_judge_passes", 3, true)).expect("a step the Judge reads");
    let judge = &def.steps()[4].judge_checks()[0];
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
    assert!(!def.steps()[4].judge_checks()[0].fires());
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
    let judge = &def.steps()[4].judge_checks()[0];
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
        fault_at(&refused, "steps[4].judge_checks[0].gaming_check.flag_if[0]"),
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
            "steps[4].judge_checks[0].gaming_check.baseline_ref"
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
        fault_at(&refused, "steps[4].advance_gate"),
        Fault::NotInTheSchema { .. }
    ));
}

#[test]
fn the_three_unimplemented_check_types_are_refused_by_name() {
    for kind in ["artifact_exists", "test_run", "pr_merged"] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - {{ type: {kind} }}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[4].mechanical_checks[0].type"),
            &Fault::OutsideM1 {
                value: kind.to_string(),
                carried: &["manifest_check", "diff_nonempty"],
            }
        );
    }
}

#[test]
fn a_manifest_check_needs_both_the_check_name_and_the_expected_code() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[4].mechanical_checks[0].check"),
        &Fault::Missing
    );
    assert_eq!(
        fault_at(&refused, "steps[4].mechanical_checks[0].expect_exit_code"),
        &Fault::Missing
    );
}

#[test]
fn diff_nonempty_carries_nothing_else() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: diff_nonempty, check: build }\n",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[4].mechanical_checks[0].check"),
        Fault::Unknown { .. }
    ));
}

#[test]
fn a_step_key_m1_does_not_read_hard_fails() {
    let refused = refusals(bug_with(
        "  - id: review\n    label: Review\n    advance_gate: auto\n    retry_limit: 3\n    hard_prerequisite: true\n",
    ));
    for key in ["retry_limit", "hard_prerequisite"] {
        assert!(
            refused
                .iter()
                .any(|r| r.key == format!("steps[4].{key}")
                    && matches!(r.fault, Fault::Unknown { .. })),
            "{key} should be an unknown step key: {refused:?}"
        );
    }
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
        fault_at(&refused, "steps[4].evidence_type"),
        Fault::NotInTheSchema { .. }
    ));
}

#[test]
fn a_step_may_declare_no_evidence_type() {
    let def = bug_with("  - id: merge\n    label: Merge\n    advance_gate: auto\n")
        .expect("a step that produces nothing a Judge reads");
    assert_eq!(def.steps()[4].evidence_type(), None);
}

// ---- the cross-file check ------------------------------------------------

#[test]
fn a_resolved_workflow_carries_the_command_not_the_name() {
    // The lookup that could miss happens once, here. Nothing at step time
    // performs one at all, which is why an absent Check is unrepresentable
    // downstream rather than checked for.
    let def = parse(BUG).expect("the worked example");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest()).expect("every name declared");
    assert_eq!(
        resolved.steps()[1].checks(),
        [
            ResolvedCheck::ManifestCheck {
                name: "build".to_string(),
                run: "cargo build --workspace".to_string(),
                expect_exit_code: 0,
            },
            ResolvedCheck::DiffNonempty,
        ]
    );
    assert_eq!(resolved.name(), "bug");
}

#[test]
fn a_step_naming_a_check_the_manifest_lacks_is_refused_before_dispatch() {
    let def = bug_with(
        "  - id: lint\n    label: Lint\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: lint, expect_exit_code: 0 }\n",
    )
    .expect("the definition itself is well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("`lint` is not declared");
    let ResolveError::ChecksNotDeclared { unknown, .. } = &error;
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].step.as_str(), "lint");
    assert_eq!(unknown[0].check, "lint");
    assert!(!unknown[0].is_a_command);
    assert_eq!(unknown[0].declared, ["build", "test"]);

    let message = error.to_string();
    assert!(message.contains("armada.yml"), "{message}");
    assert!(message.contains("workflows/bug.yml"), "{message}");
    assert!(
        message.contains("Declared Checks are `build`, `test`"),
        "{message}"
    );
}

#[test]
fn naming_a_command_where_a_check_belongs_is_told_which_mistake_it_was() {
    let def = bug_with(
        "  - id: tidy\n    label: Tidy\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: fmt, expect_exit_code: 0 }\n",
    )
    .expect("well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("`fmt` is a Command");
    let ResolveError::ChecksNotDeclared { unknown, .. } = &error;
    assert!(unknown[0].is_a_command);
    assert!(
        error
            .to_string()
            .contains("declared as a Command, not a Check"),
        "{error}"
    );
}

#[test]
fn every_unresolved_name_is_reported_not_only_the_first() {
    let def = bug_with(
        "  - id: lint\n    label: Lint\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: lint, expect_exit_code: 0 }\n      - { type: manifest_check, check: typecheck, expect_exit_code: 0 }\n",
    )
    .expect("well formed");
    let error = ResolvedWorkflow::resolve(&def, &manifest()).expect_err("two names miss");
    let ResolveError::ChecksNotDeclared { unknown, .. } = &error;
    let names: Vec<&str> = unknown.iter().map(|u| u.check.as_str()).collect();
    assert_eq!(names, ["lint", "typecheck"]);
}

#[test]
fn a_step_with_no_checks_needs_nothing_from_the_manifest() {
    let def = parse(
        "version: 1\nworkflow_id: fixture\nname: prototype\nstructure: linear\nsteps:\n  - id: frame\n    label: Frame\n    evidence_type: facts_note\n    advance_gate: auto\n",
    )
    .expect("well formed");
    let bare = Manifest::parse(&named("armada.yml"), "version: 1\nid: tooling\n").expect("bare");
    let resolved = ResolvedWorkflow::resolve(&def, &bare).expect("nothing to resolve");
    assert!(resolved.steps()[0].checks().is_empty());
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
