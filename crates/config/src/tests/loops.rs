//! `verdict_routing` and `iteration_cap`: the loop edge, its bound, and the
//! wiring check that reads one against `structure`.
//!
//! **The three keys were refused three different ways, and the difference was
//! the point.** `structure: loop` was outside M1, `verdict_routing` on a linear
//! workflow contradicted the file's own claim, and `iteration_cap` was a key
//! that had never existed. Two of those are gone and the third stays: this file
//! is where that stays visible, because collapsing them would make every
//! refusal read as a milestone that has not arrived.

use core_model::StepId;

use crate::error::{Fault, LoadError};
use crate::loops::GateVerdict;
use crate::tests::{fault_at, named, refusals, roster};
use crate::workflow::WorkflowDef;

/// A two-step design loop: `draft`, then a human gate that sends it back.
const DESIGN_PLAN: &str = "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
    iteration_cap: 5
";

fn parsed(structure: &str, steps: &str) -> Result<WorkflowDef, LoadError> {
    WorkflowDef::parse(
        &named("workflows/design-plan.yml"),
        &format!(
            "version: 1\nworkflow_id: design_plan\nname: design_plan\n\
             structure: {structure}\nsteps:{steps}"
        ),
        &roster(),
    )
}

/// The shape `workflows.toml` calls Armada's only instantiated loop, loading
/// whole for the first time: `draft -> present`, with `request_changes` routing
/// back to `draft` and a cap of five on how often.
#[test]
fn a_loop_carries_its_edge_and_its_cap_onto_the_step() {
    let def = parsed("loop", DESIGN_PLAN).expect("the design loop");
    let present = &def.steps()[1];

    assert_eq!(
        present.verdict_routing().get(&GateVerdict::RequestChanges),
        Some(&StepId::new("draft".to_string()))
    );
    assert_eq!(present.iteration_cap(), Some(5));
}

/// **The edge lives on the step, not on the workflow**, which is what lets one
/// definition carry more than one loop. The step that does not emit a verdict
/// carries nothing, and that is what makes its only exit forward.
#[test]
fn a_step_that_emits_no_verdict_routes_nowhere() {
    let def = parsed("loop", DESIGN_PLAN).expect("the design loop");
    assert!(def.steps()[0].verdict_routing().is_empty());
    assert_eq!(def.steps()[0].iteration_cap(), None);
}

/// **Absent is none, and none is not a number this parser invents.** The schema
/// defaults the cap from `default_gate_policy.iteration_cap`, that block is
/// still refused as deferred, and a ceiling invented here would be a bound on a
/// Job written where nobody reading the workflow would find it.
#[test]
fn a_loop_may_declare_an_edge_and_no_cap() {
    let def = parsed(
        "loop",
        "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
",
    )
    .expect("an uncapped loop");
    assert_eq!(def.steps()[1].iteration_cap(), None);
}

/// Zero is legal, for the reason `retry_limit: 0` is: a step saying its first
/// `request_changes` is its last is a sentence an author is entitled to write.
#[test]
fn a_cap_of_zero_is_a_sentence_an_author_may_write() {
    let def = parsed(
        "loop",
        "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
    iteration_cap: 0
",
    )
    .expect("a loop that goes round once");
    assert_eq!(def.steps()[1].iteration_cap(), Some(0));
}

/// A cap that is not a number is refused rather than read as absent. The file
/// meant to bound the loop, and reading it as unbounded would be the parser
/// deciding how long a Job may go round.
#[test]
fn a_cap_that_is_not_a_number_is_refused() {
    let refused = refusals(parsed(
        "loop",
        "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
    iteration_cap: five
",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[1].iteration_cap"),
        Fault::WrongType { .. }
    ));
}

/// **A cap with no edge bounds nothing.** The registry puts the two on one step
/// because a cap split from the count it bounds never fires, so a step carrying
/// only the cap is half a statement — the same shape as `declare_plan_at` with
/// no `evidence_scope`.
#[test]
fn a_cap_on_a_step_with_no_edge_is_refused() {
    let refused = refusals(parsed(
        "loop",
        "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
    iteration_cap: 5
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[0].iteration_cap"),
        &Fault::CapWithoutALoop
    );
}

/// **`approve` and `reject` have nowhere to be routed to.** One advances and
/// one ends the Job, so the map holds the one verdict that does neither, and a
/// file naming another is told what is read at that position rather than
/// silently routing on a word nothing emits.
#[test]
fn a_verdict_that_is_not_a_loop_return_is_refused() {
    let refused = refusals(parsed(
        "loop",
        "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing:
      request_changes: draft
      approve: draft
",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[1].verdict_routing.approve"),
        Fault::Unknown { known } if *known == ["request_changes"]
    ));
}

/// `{}` is the key written and left blank, which is a different mistake from
/// never having written it — and it is the one that would otherwise satisfy the
/// wiring check with an edge that goes nowhere.
#[test]
fn an_empty_routing_map_is_refused() {
    let refused = refusals(parsed(
        "loop",
        "
  - id: draft
    label: Draft
    evidence_type: document
    advance_gate: auto
  - id: present
    label: Present
    evidence_type: document
    advance_gate: human_always
    verdict_routing: {}
",
    ));
    assert_eq!(
        fault_at(&refused, "steps[1].verdict_routing"),
        &Fault::Empty
    );
}

/// **The refusal that stays, and is not a deferral.** On a linear workflow the
/// declared structure and the wiring disagree, which is wrong at every
/// milestone — so it keeps its own fault rather than becoming the unknown key
/// it would otherwise now be, since both keys are in `STEP_KEYS`.
#[test]
fn a_linear_workflow_carrying_an_edge_is_still_a_contradiction_and_not_an_unknown_key() {
    let refused = refusals(parsed("linear", DESIGN_PLAN));
    assert_eq!(
        fault_at(&refused, "steps[1].verdict_routing"),
        &Fault::ContradictsStructure {
            structure: "linear"
        }
    );
}
