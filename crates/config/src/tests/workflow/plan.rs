//! `plan`, `plan_recorded` and `follows_plan`: the plan step's shape, and each
//! way the loader refuses one.
//!
//! **Its own fixture, not [`super::BUG`].** That constant is the worked
//! example as it stands before #895 switches it — its `plan` step's product
//! is `facts_note`, and switching it here would be doing the switch #894
//! carries, in the wrong file.

use std::num::NonZeroU32;

use core_model::EvidenceType;

use crate::error::Fault;
use crate::tests::{fault_at, named, refusals, roster};
use crate::workflow::{MechanicalCheck, WorkflowDef};

/// A plan step and a step that follows it, minimal enough that one line
/// changes one thing.
const PLAN_AND_IMPLEMENT: &str = r#"
version: 1
workflow_id: fixture-plan
name: fixture
structure: linear
steps:
  - id: plan
    label: Plan the change
    evidence: {submitted: {type: plan}}
    mechanical_checks:
      - { type: plan_recorded }
    delivers: false
    advance_gate: auto
  - id: implement
    label: Implement
    follows_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    delivers: true
    advance_gate: auto
"#;

fn parse(text: &str) -> Result<WorkflowDef, crate::error::LoadError> {
    WorkflowDef::parse(&named("workflows/fixture-plan.yml"), text, &roster())
}

#[test]
fn a_plan_step_and_a_following_step_load() {
    let def = parse(PLAN_AND_IMPLEMENT).expect("plan and follows_plan both parse");
    let plan = &def.steps()[0];
    assert_eq!(plan.evidence_type(), Some(EvidenceType::Plan));
    assert_eq!(
        plan.mechanical_checks(),
        [MechanicalCheck::PlanRecorded {
            min_tasks: NonZeroU32::MIN
        }]
    );
    assert!(!plan.follows_plan());

    let implement = &def.steps()[1];
    assert!(implement.follows_plan());
    assert_eq!(implement.evidence_type(), Some(EvidenceType::Diff));
}

/// **Absent is one.** A step that says nothing about `min_tasks` is held to
/// the plan existing at all, not to a count nobody wrote.
#[test]
fn min_tasks_defaults_to_one_and_may_be_stated() {
    let def = parse(PLAN_AND_IMPLEMENT).expect("plan and follows_plan both parse");
    assert_eq!(
        def.steps()[0].mechanical_checks(),
        [MechanicalCheck::PlanRecorded {
            min_tasks: NonZeroU32::new(1).expect("one")
        }]
    );

    let stated = parse(&PLAN_AND_IMPLEMENT.replace(
        "{ type: plan_recorded }",
        "{ type: plan_recorded, min_tasks: 3 }",
    ))
    .expect("a stated count is legal");
    assert_eq!(
        stated.steps()[0].mechanical_checks(),
        [MechanicalCheck::PlanRecorded {
            min_tasks: NonZeroU32::new(3).expect("three")
        }]
    );
}

/// **`min_tasks` below 1 is refused rather than carried.** A check nothing
/// could ever fail is not the shape a default takes, and zero is that check.
#[test]
fn a_min_tasks_below_one_is_refused() {
    let refused = refusals(parse(&PLAN_AND_IMPLEMENT.replace(
        "{ type: plan_recorded }",
        "{ type: plan_recorded, min_tasks: 0 }",
    )));
    assert!(matches!(
        fault_at(&refused, "steps[0].mechanical_checks[0].min_tasks"),
        Fault::WrongType { .. }
    ));
}

/// **Two steps whose product is `plan` are refused, and the second names the
/// first.** Fleet holds one plan per Job, so a second recording is a second
/// record nothing after either could tell from the first.
#[test]
fn a_second_plan_step_is_refused_and_names_the_first() {
    let refused = refusals(parse(&format!(
        "{PLAN_AND_IMPLEMENT}  - id: replan\n    label: Plan again\n    \
         evidence: {{submitted: {{type: plan}}}}\n    mechanical_checks:\n      \
         - {{ type: plan_recorded }}\n    delivers: false\n    advance_gate: auto\n"
    )));
    assert_eq!(
        fault_at(&refused, "steps[2].evidence.submitted.type"),
        &Fault::TwoPlanSteps { first_at: 0 }
    );
}

/// **A step whose product is `plan` and declares no `plan_recorded` is
/// refused.** Nothing else in `mechanical_checks` reads Fleet's record of the
/// plan, so a step naming the product without the check is not gated on it at
/// all.
#[test]
fn a_plan_step_without_plan_recorded_is_refused() {
    let refused = refusals(parse(&PLAN_AND_IMPLEMENT.replace(
        "    mechanical_checks:\n      - { type: plan_recorded }\n",
        "",
    )));
    assert_eq!(
        fault_at(&refused, "steps[0].mechanical_checks"),
        &Fault::PlanStepWithoutPlanRecorded
    );
}

/// **`plan_recorded` on a step whose product is not `plan` is refused.** The
/// check reads Fleet's record of the plan, which is a fact about the step
/// that records one and about no other.
#[test]
fn plan_recorded_on_a_step_whose_product_is_not_plan_is_refused() {
    let refused = refusals(parse(&PLAN_AND_IMPLEMENT.replace(
        "    mechanical_checks:\n      - { type: diff_nonempty }\n",
        "    mechanical_checks:\n      - { type: diff_nonempty }\n      - { type: plan_recorded }\n",
    )));
    assert_eq!(
        fault_at(&refused, "steps[1].mechanical_checks[1].type"),
        &Fault::PlanRecordedNotOnAPlanStep
    );
}

/// **`follows_plan` on the plan step itself is refused.** Nothing before it
/// in the file has `plan` as its product, and that includes itself.
#[test]
fn follows_plan_on_the_plan_step_itself_is_refused() {
    let refused = refusals(parse(&PLAN_AND_IMPLEMENT.replace(
        "  - id: plan\n    label: Plan the change\n    evidence: {submitted: {type: plan}}",
        "  - id: plan\n    label: Plan the change\n    follows_plan: true\n    evidence: {submitted: {type: plan}}",
    )));
    assert_eq!(
        fault_at(&refused, "steps[0].follows_plan"),
        &Fault::FollowsPlanWithNoPlanStep
    );
}

/// **`follows_plan` on a step before the plan step is refused.** Reordering
/// the two fixture steps puts `implement` first, with nothing before it that
/// records a plan yet.
#[test]
fn follows_plan_on_a_step_before_the_plan_step_is_refused() {
    let reordered = r#"
version: 1
workflow_id: fixture-plan
name: fixture
structure: linear
steps:
  - id: implement
    label: Implement
    follows_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    delivers: true
    advance_gate: auto
  - id: plan
    label: Plan the change
    evidence: {submitted: {type: plan}}
    mechanical_checks:
      - { type: plan_recorded }
    delivers: false
    advance_gate: auto
"#;
    let refused = refusals(parse(reordered));
    assert_eq!(
        fault_at(&refused, "steps[0].follows_plan"),
        &Fault::FollowsPlanWithNoPlanStep
    );
}

/// **`follows_plan` in a workflow with no plan step at all is refused.**
#[test]
fn follows_plan_with_no_plan_step_in_the_workflow_is_refused() {
    let no_plan_step = r#"
version: 1
workflow_id: fixture-plan
name: fixture
structure: linear
steps:
  - id: implement
    label: Implement
    follows_plan: true
    evidence: {submitted: {type: diff}}
    mechanical_checks:
      - { type: diff_nonempty }
    delivers: true
    advance_gate: auto
"#;
    let refused = refusals(parse(no_plan_step));
    assert_eq!(
        fault_at(&refused, "steps[0].follows_plan"),
        &Fault::FollowsPlanWithNoPlanStep
    );
}
