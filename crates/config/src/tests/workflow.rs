//! A WorkflowDef: the M1 slice, the four validations, and the one cross-file
//! check that is the point of this milestone step.

use core_model::{AdvanceGate, EvidenceRef, EvidenceType, GamingPattern, ResolvedCheck};

use crate::error::{BadTarget, Fault, LoadError, ResolveError};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
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

/// Both of the schema's structures load. The blocker written against `loop`
/// named a Judge and a human gate that did not exist, and both do — the gate is
/// `human_always` and `fleet::gate` holds a step at it, and a panel runs from
/// `judge_checks`. What is still missing is a step-machine edge for the return
/// and a counter to bound it, and neither is a thing the file could say.
#[test]
fn a_loop_workflow_declares_its_structure_and_loads() {
    let def = parse(
        "version: 1\nworkflow_id: fixture\nname: design_plan\nstructure: loop\nsteps:\n  - id: draft\n    label: Draft\n    advance_gate: auto\n",
    )
    .expect("a loop declaration");
    assert_eq!(def.structure(), Structure::Loop);
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
            &Fault::OutsideM1 {
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
fn the_two_unimplemented_check_types_are_refused_by_name() {
    for kind in ["test_run", "pr_merged"] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - {{ type: {kind} }}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[3].mechanical_checks[0].type"),
            &Fault::OutsideM1 {
                value: kind.to_string(),
                carried: &["manifest_check", "diff_nonempty", "artifact_exists"],
            }
        );
    }
}

/// **`artifact_exists` is carried, and it needs the path.** It was refused
/// beside `test_run` and `pr_merged` because the schema's samples name a
/// registry entry and there is no artifact registry. What there is on every
/// step is a worktree, so the target is a path in it — and a check declaring
/// none is refused rather than read as "some file, somewhere".
#[test]
fn an_artifact_check_needs_the_path_it_is_looking_for() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: artifact_exists }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[0].target"),
        &Fault::Missing
    );
}

/// **The four targets that name no single file inside the worktree.**
///
/// Each is refused where the workflow is parsed rather than met at the gate,
/// and the glob is the one that was measured: v1's `design` workflow named
/// `docs/design/*.md`, its gate probed the string as a literal path, and the
/// step was unpassable whatever the Drone wrote — the Job ran until it hit its
/// token ceiling. Refusing a pattern is also what lets Fleet quote the path to
/// the next step's Drone, since "whichever file matched" is not a path.
#[test]
fn an_artifact_target_that_cannot_name_one_file_is_refused_where_it_is_written() {
    for (target, why) in [
        ("docs/design/*.md", BadTarget::Globbed),
        ("report-?.md", BadTarget::Globbed),
        ("/etc/passwd", BadTarget::Absolute),
        ("../elsewhere/plan.md", BadTarget::Escapes),
        (".armada/artifacts/", BadTarget::ADirectory),
    ] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - {{ type: artifact_exists, target: \"{target}\" }}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[3].mechanical_checks[0].target"),
            &Fault::NotAnArtifactPath {
                value: target.to_string(),
                why,
            },
            "`{target}`"
        );
    }
}

/// **A step delivers one file.** Fleet reads the declared path into the Judge's
/// brief as *the document this step produced*, and points the next step's Drone
/// at it. Two targets would make both of those a choice nothing records, so the
/// second is refused where it is written rather than resolved by whichever a
/// reader reached first.
#[test]
fn a_step_declaring_two_artifacts_is_refused_because_it_has_one_deliverable() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: artifact_exists, target: a.md }\n      - { type: artifact_exists, target: b.md }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[1].target"),
        &Fault::TwoDeliverables {
            first: "a.md".to_string()
        }
    );
}

/// A target that names one file in the worktree loads, and the path is carried
/// through resolution rather than reduced to the fact that a check was
/// declared.
#[test]
fn an_artifact_check_carries_its_path_onto_the_step() {
    let def = bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: artifact_exists, target: .armada/artifacts/close.md }\n",
    )
    .expect("the definition loads");
    assert_eq!(
        def.steps()[3].mechanical_checks(),
        [MechanicalCheck::ArtifactExists {
            target: ".armada/artifacts/close.md".to_string()
        }]
    );
    // One place answers "what does this step deliver", because the gate, the
    // brief and the mechanical tier all ask.
    let manifest = Manifest::parse(&named("armada.yml"), MANIFEST).expect("a manifest");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("it resolves");
    assert_eq!(
        resolved.steps()[3].deliverable(),
        Some(".armada/artifacts/close.md")
    );
    assert_eq!(
        resolved.steps()[0].deliverable(),
        None,
        "a step declaring no artifact delivers no file"
    );
}

#[test]
fn a_manifest_check_needs_both_the_check_name_and_the_expected_code() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[0].check"),
        &Fault::Missing
    );
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[0].expect_exit_code"),
        &Fault::Missing
    );
}

#[test]
fn diff_nonempty_carries_nothing_else() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    advance_gate: auto\n    mechanical_checks:\n      - { type: diff_nonempty, check: build }\n",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[3].mechanical_checks[0].check"),
        Fault::Unknown { .. }
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
                when: None,
                requires: Vec::new(),
            },
            ResolvedCheck::ManifestCheck {
                name: "test".to_string(),
                run: "cargo nextest run --workspace".to_string(),
                expect_exit_code: 0,
                when: None,
                requires: Vec::new(),
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

/// The same workflow's Manifest, with `build` scoped to the Rust tree.
const SCOPED_MANIFEST: &str = r#"
version: 1
id: armada
checks:
  build:
    run: cargo build --workspace
    when: ["crates/**", "Cargo.toml"]
  test:
    run: cargo nextest run --workspace
commands:
  fmt:
    run: cargo fmt --all
"#;

#[test]
fn a_checks_when_is_lifted_off_the_manifest_and_frozen_onto_the_step() {
    // **The pattern is the Manifest's and the step does not restate it.** The
    // repository declares once what a Check covers and every workflow inherits
    // it — there is no step-level key to override it with, which is what stops
    // the same glob drifting across `bug`, `feature`, `refactor` and `revert`.
    let def = parse(BUG).expect("the worked example");
    let manifest = Manifest::parse(&named("armada.yml"), SCOPED_MANIFEST).expect("a scoped check");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");

    let checks = resolved.steps()[1].checks();
    assert_eq!(
        checks[0].when().map(core_model::Covers::written),
        Some("crates/**, Cargo.toml".to_string())
    );
    // The one that declares nothing carries nothing, and covers everything.
    assert_eq!(checks[1].when(), None);
    assert!(checks[1].covers(&["anything/at/all".to_string()]));
    // Frozen: the resolved value is a copy, so an edit to `armada.yml` after
    // this point changes the next Job and not this one.
    assert!(checks[0].covers(&["crates/store/src/read.rs".to_string()]));
    assert!(!checks[0].covers(&["packages/components/src/Badge.tsx".to_string()]));
}

#[test]
fn a_built_in_check_declares_no_paths_and_always_runs() {
    let def = parse(BUG).expect("the worked example");
    let manifest = Manifest::parse(&named("armada.yml"), SCOPED_MANIFEST).expect("a scoped check");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("every name declared");
    let diff = &resolved.steps()[1].checks()[2];
    assert_eq!(diff, &ResolvedCheck::DiffNonempty);
    assert_eq!(diff.when(), None);
    assert!(!diff.needs_changed_paths());
    assert!(diff.covers(&[]));
}
