//! `evidence_scope` on a step: what the definition may say, and what it may
//! not.
//!
//! The one that matters is `context_paths`. It is a legal field of the schema
//! and an illegal field of a definition, and the two are told apart by a
//! refusal that names the object it belongs to rather than by "unknown key" —
//! which would read as a field the parser has not reached yet.

use core_model::{ContextSource, DeclarePlanAt};

use crate::error::Fault;
use crate::tests::{fault_at, named, refusals};
use crate::workflow::WorkflowDef;

fn parsed(steps: &str) -> Result<WorkflowDef, crate::error::LoadError> {
    WorkflowDef::parse(
        &named("scope.yml"),
        &format!("version: 1\nworkflow_id: scoped\nname: scoped\nstructure: linear\nsteps:{steps}"),
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
/// keep. `max_context_size` has no decided value and no owner, and
/// `reference_docs` needs a Judge brief that carries a yardstick.
#[test]
fn the_two_deferred_keys_are_refused_as_deferred_rather_than_as_typos() {
    let refusals = refusals(parsed(
        "
  - id: implement
    label: Implement
    advance_gate: auto
    evidence_scope:
      context_source: drone_declared
      max_context_size: 20
      reference_docs:
        - docs/brief.md
",
    ));
    for key in [
        "steps[0].evidence_scope.max_context_size",
        "steps[0].evidence_scope.reference_docs",
    ] {
        assert!(
            matches!(fault_at(&refusals, key), Fault::OutsideM1 { .. }),
            "{key} is a schema key this milestone does not read, not a typo"
        );
    }
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
