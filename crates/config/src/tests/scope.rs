//! `evidence_scope` on a step: what the definition may say, and what it may
//! not.
//!
//! The one that matters is `context_paths`. It is a legal field of the schema
//! and an illegal field of a definition, and the two are told apart by a
//! refusal that names the object it belongs to rather than by "unknown key" —
//! which would read as a field the parser has not reached yet.

use core_model::{ContextSource, DeclarePlanAt, EvidenceRef};

use crate::error::Fault;
use crate::tests::{fault_at, named, refusals, roster};
use crate::workflow::WorkflowDef;

fn parsed(steps: &str) -> Result<WorkflowDef, crate::error::LoadError> {
    WorkflowDef::parse(
        &named("scope.yml"),
        &format!("version: 1\nworkflow_id: scoped\nname: scoped\nstructure: linear\nsteps:{steps}"),
        &roster(),
    )
}

const WATCHED: &str = "
  - id: implement
    label: Implement
    evidence_type: diff
    advance_gate: auto
    declare_plan_at: step_start
    evidence_scope:
      context_source: drone_declared
      scope_diff_check: true
      exclude_paths:
        - secrets
";

#[test]
fn a_step_carries_the_policy_and_the_moment_it_is_declared() {
    let def = parsed(WATCHED).expect("a scoped step loads");
    let scope = def.steps()[0]
        .evidence_scope()
        .expect("the step declares one");

    assert_eq!(scope.context_source(), ContextSource::DroneDeclared);
    assert!(scope.scope_diff_check());
    assert_eq!(scope.declare_plan_at(), Some(DeclarePlanAt::StepStart));
    assert_eq!(scope.exclude_paths().len(), 1);
    assert!(
        scope.watches_live_edits(),
        "both halves are there, so the live check has a plan and something to \
         measure it against"
    );
}

/// **The definition-versus-resolved split, as a refusal.** At definition time
/// nobody knows the paths — that is what `drone_declared` means — so there is
/// no field on the policy to hold them and a file that authors one is told
/// which object it belongs to.
#[test]
fn a_definition_that_authors_the_drones_answer_is_refused_by_name() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
      context_paths:
        - src
",
    ));
    assert_eq!(
        fault_at(&refusals, "steps[0].evidence_scope.context_paths"),
        &Fault::BelongsToTheResolvedObject
    );
}

/// A key nothing reads is a promise the file makes and the system does not
/// keep. `max_context_size` has no decided number and no owner — the cap bounds
/// all of verification rather than the Judge Check alone, and it stays open.
#[test]
fn the_cap_nobody_has_decided_is_refused_as_deferred_rather_than_as_a_typo() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
      max_context_size: 20
",
    ));
    assert!(
        matches!(
            fault_at(&refusals, "steps[0].evidence_scope.max_context_size"),
            Fault::OutsideM1 { .. }
        ),
        "the cap is a schema key with no decided value, not a typo"
    );
}

/// **The yardstick, reachable at last.** A later step names what an earlier
/// step established, and it names the *evidence* rather than a file: the same
/// `<step_id>.evidence` spelling `baseline_ref` uses, through the same type.
#[test]
fn a_step_can_name_the_earlier_evidence_its_work_is_measured_against() {
    let def = parsed(
        "
  - id: scope
    label: Scope
    evidence_type: facts_note
    advance_gate: auto
  - id: implement
    label: Implement
    evidence_type: diff
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
      reference_docs:
        - scope.evidence
",
    )
    .expect("a step naming an earlier step's evidence loads");
    let scope = def.steps()[1]
        .evidence_scope()
        .expect("the step declares one");
    assert_eq!(
        scope.reference_docs(),
        &[EvidenceRef::parse("scope.evidence").expect("a reference")],
        "what the step is measured against is carried, not dropped"
    );
}

/// A path on disk is not evidence. `reference_docs` names what a step recorded,
/// so a file path — which is what the deferred key used to be written with — is
/// refused rather than resolved into something nothing can reach.
#[test]
fn a_reference_that_is_not_an_evidence_reference_is_refused() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
      reference_docs:
        - docs/brief.md
        - scope
",
    ));
    for key in [
        "steps[0].evidence_scope.reference_docs[0]",
        "steps[0].evidence_scope.reference_docs[1]",
    ] {
        assert!(
            matches!(fault_at(&refusals, key), Fault::NotInTheSchema { .. }),
            "{key} names no step's evidence, and naming a step is not naming its \
             evidence either"
        );
    }
}

/// **The common shape.** Most steps are judged on their own product and
/// nothing else, and a scope block that names no yardstick carries an empty
/// one rather than failing to load.
#[test]
fn a_scope_naming_no_yardstick_carries_none() {
    let def = parsed(WATCHED).expect("a scoped step loads");
    assert!(def.steps()[0]
        .evidence_scope()
        .expect("the step declares one")
        .reference_docs()
        .is_empty());
}

/// The two keys are one intent written in two places, and a file where only one
/// of them is present has made half a statement.
#[test]
fn declaring_when_the_plan_arrives_without_saying_what_it_is_for_is_refused() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    declare_plan_at: step_start
",
    ));
    assert_eq!(
        fault_at(&refusals, "steps[0].declare_plan_at"),
        &Fault::PlanWithoutAScope
    );
}

/// `manifest_default` is in the schema and nothing supplies one: no Manifest
/// key names a default path set. Refused as deferred, so the message says wait
/// rather than says typo.
#[test]
fn a_source_no_manifest_can_supply_is_refused_as_deferred() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    evidence_scope:
      context_source: manifest_default
",
    ));
    assert!(matches!(
        fault_at(&refusals, "steps[0].evidence_scope.context_source"),
        Fault::OutsideM1 { .. }
    ));
}

/// The audit trail for trust level is required, because paths a Manifest
/// supplied and paths a Drone chose for itself are not equally trustworthy and
/// the resolved object must not lose which it was.
#[test]
fn a_scope_that_does_not_say_where_its_paths_came_from_is_refused() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    evidence_scope:
      scope_diff_check: true
",
    ));
    assert_eq!(
        fault_at(&refusals, "steps[0].evidence_scope.context_source"),
        &Fault::Missing
    );
}

/// **The common shape.** A step written before any of this existed loads
/// exactly as it did, and carries no scope rather than an empty one.
#[test]
fn a_step_with_no_scope_block_carries_none() {
    let def = parsed(
        "
  - id: implement
    label: Implement
    evidence_type: diff
    advance_gate: auto
",
    )
    .expect("a step with no scope loads");
    assert!(def.steps()[0].evidence_scope().is_none());
}
