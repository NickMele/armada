//! Stored spellings for the values `core-model` does not spell.
//!
//! # This file should not exist, and says so
//!
//! Eleven of the enums a Job holds carry `as_wire`/`from_wire` in
//! `core-model`, beside the registry key they are transcribed from. Six do not,
//! and a column still has to hold them. **Every mapping below is a second
//! vocabulary for a value whose first vocabulary lives in another crate**,
//! which is exactly the drift `as_wire` was put next to the enum to prevent.
//!
//! They are here rather than there because this step may not change
//! `core-model`. When they move, they move as `as_wire`/`from_wire` beside
//! their variants, and this file shrinks to nothing.
//!
//! The spellings are `snake_case` of the variant, matching what the registry
//! does everywhere it did decide — so the move is a cut and paste rather than a
//! migration.

use core_model::{
    Actor, CriterionSource, DependencyDirection, GateOutcome, NotRunReason, Origin, PilotReason,
    TopLevelOrigin,
};

/// Who caused a transition. `core-model` closes the set and names no wire
/// value; `job_events.actor` needs one.
pub fn actor_wire(actor: Actor) -> &'static str {
    match actor {
        Actor::Human => "human",
        Actor::Fleet => "fleet",
        Actor::Drone => "drone",
    }
}

pub fn actor_from_wire(value: &str) -> Option<Actor> {
    match value {
        "human" => Some(Actor::Human),
        "fleet" => Some(Actor::Fleet),
        "drone" => Some(Actor::Drone),
        _ => None,
    }
}

/// Which verification source answers a criterion.
pub fn criterion_source_wire(source: CriterionSource) -> &'static str {
    match source {
        CriterionSource::Check => "check",
        CriterionSource::Judge => "judge",
        CriterionSource::Attested => "attested",
    }
}

pub fn criterion_source_from_wire(value: &str) -> Option<CriterionSource> {
    match value {
        "check" => Some(CriterionSource::Check),
        "judge" => Some(CriterionSource::Judge),
        "attested" => Some(CriterionSource::Attested),
        _ => None,
    }
}

/// Which way a DAG edge points.
pub fn direction_wire(direction: DependencyDirection) -> &'static str {
    match direction {
        DependencyDirection::DependsOn => "depends_on",
        DependencyDirection::Blocks => "blocks",
    }
}

pub fn direction_from_wire(value: &str) -> Option<DependencyDirection> {
    match value {
        "depends_on" => Some(DependencyDirection::DependsOn),
        "blocks" => Some(DependencyDirection::Blocks),
        _ => None,
    }
}

/// What a gating Manifest's Checks did. Two columns, because a did-not-run
/// carries a reason and the other two do not — the same absent-versus-null rule
/// the log envelope holds: the reason column is null exactly when there is no
/// reason, never as a stand-in for one.
pub fn gate_outcome_wire(outcome: GateOutcome) -> (&'static str, Option<&'static str>) {
    match outcome {
        GateOutcome::RanAndPassed => ("ran_and_passed", None),
        GateOutcome::RanAndFailed => ("ran_and_failed", None),
        GateOutcome::DidNotRun(reason) => ("did_not_run", Some(not_run_reason_wire(reason))),
    }
}

pub fn gate_outcome_from_wire(outcome: &str, reason: Option<&str>) -> Option<GateOutcome> {
    match (outcome, reason) {
        ("ran_and_passed", None) => Some(GateOutcome::RanAndPassed),
        ("ran_and_failed", None) => Some(GateOutcome::RanAndFailed),
        ("did_not_run", Some(reason)) => {
            not_run_reason_from_wire(reason).map(GateOutcome::DidNotRun)
        }
        _ => None,
    }
}

fn not_run_reason_wire(reason: NotRunReason) -> &'static str {
    match reason {
        NotRunReason::PathConditionUnmet => "path_condition_unmet",
        NotRunReason::Frozen => "frozen",
        NotRunReason::NotDeclared => "not_declared",
        NotRunReason::ScopeNarrowed => "scope_narrowed",
    }
}

fn not_run_reason_from_wire(value: &str) -> Option<NotRunReason> {
    match value {
        "path_condition_unmet" => Some(NotRunReason::PathConditionUnmet),
        "frozen" => Some(NotRunReason::Frozen),
        "not_declared" => Some(NotRunReason::NotDeclared),
        "scope_narrowed" => Some(NotRunReason::ScopeNarrowed),
        _ => None,
    }
}

/// Why a person is being asked to take the Job over. `core-model` gives this
/// one `as_wire` and no `from_wire`; the reverse is a scan of `ALL`.
pub fn pilot_reason_from_wire(value: &str) -> Option<PilotReason> {
    PilotReason::ALL
        .iter()
        .copied()
        .find(|reason| reason.as_wire() == value)
}

/// The four origins a Job with no `dispatched_by` may claim.
///
/// `From<TopLevelOrigin> for Origin` exists; the narrowing back does not, and
/// the rebuild needs it to call the constructor that made the Job.
/// `sub_dispatched` returns `None`, which is not a failure — it is the caller
/// being told to use the other constructor.
pub fn top_level_origin(origin: Origin) -> Option<TopLevelOrigin> {
    match origin {
        Origin::AutoDetected => Some(TopLevelOrigin::AutoDetected),
        Origin::Manual => Some(TopLevelOrigin::Manual),
        Origin::HelmDrafted => Some(TopLevelOrigin::HelmDrafted),
        Origin::WorkflowTriggered => Some(TopLevelOrigin::WorkflowTriggered),
        Origin::SubDispatched => None,
    }
}
