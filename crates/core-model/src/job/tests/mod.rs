//! The machine, tested against the registry it was built from.
//!
//! **Every test reaches its state by transitioning.** [`reach`] is the only way
//! a test here gets a Job into a status other than the entry one, and it walks
//! real edges to get there. A test that constructed a Job in a state directly
//! would be asserting against a value it made up rather than against the
//! machine — which is the crack a no-setter rule leaks through, and the reason
//! [`Job`] has no constructor that takes a status for such a test to call.
//!
//! This module is the scaffolding. [`machine`] tests what the registry says is
//! legal for a Job; [`step_machine`] tests the inner half, which has no
//! registry table to be read against; [`record`] tests what a Job is made of
//! and what a transition records.

use crate::envelope::{Actor, FieldValue, Timestamp, Ulid};
use crate::job::*;

fn at(instant: &str) -> Timestamp {
    Timestamp::from_rfc3339(instant)
}

fn id(value: &str) -> Ulid {
    Ulid::carried(value)
}

/// A title a person could read. Panics on a blank one, which is a fixture bug
/// rather than a case under test — the case under test is in [`record`].
fn title(text: &str) -> Title {
    Title::new(text).expect("a fixture title is never blank")
}

/// The two steps the fixture Job follows, frozen as a real Job's would be.
fn workflow() -> FrozenWorkflow {
    FrozenWorkflow::frozen(
        WorkflowId::carried(id("01J0000000000000000000WF00")),
        "bug".into(),
        1,
        vec![
            ResolvedStep::frozen(
                StepId::new("repro"),
                "Reproduce".into(),
                Some(EvidenceType::FailingTest),
                Vec::new(),
                AdvanceGate::Auto,
                Vec::new(),
            ),
            ResolvedStep::frozen(
                StepId::new("fix"),
                "Fix".into(),
                Some(EvidenceType::Diff),
                vec![ResolvedCheck::DiffNonempty],
                AdvanceGate::Auto,
                Vec::new(),
            ),
        ],
    )
}

fn draft() -> NewJob {
    NewJob {
        id: JobId::carried(id("01J0000000000000000000JOB0")),
        title: title("fix the parser's off-by-one"),
        workflow: workflow(),
        owner_manifest_id: ManifestId::carried(id("01J0000000000000000000MAN0")),
        urgency: Urgency::Normal,
        atomic: false,
        model: ModelName::new("the-configured-model").expect("a model name"),
        acceptance_criteria: vec![AcceptanceCriterion {
            criterion_id: CriterionId::new("c1"),
            text: "the reported symptom no longer occurs".into(),
            source: CriterionSource::Check,
        }],
        steps: vec![
            StepSeed {
                step_id: StepId::new("repro"),
                ordinal: 0,
            },
            StepSeed {
                step_id: StepId::new("fix"),
                ordinal: 1,
            },
        ],
        dependencies: Vec::new(),
        gate_manifests: Vec::new(),
        write_targets: None,
        subject: None,
        redispatched_from: None,
        facts: Facts::empty(),
        scope_revisions: Vec::new(),
    }
}

fn created() -> Job {
    Job::create_top_level(
        draft(),
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

/// Apply a sequence of transitions, failing loudly rather than silently
/// stopping. Fleet is the actor: it is the only thing that drives a transition.
fn drive(job: &Job, path: &[Target]) -> Job {
    let mut current = job.clone();
    for target in path {
        let moved = current
            .transition(target.clone(), Actor::Fleet, at("2026-08-26T09:01:00.000Z"))
            .unwrap_or_else(|e| panic!("driving to {:?}: {e}", target.status()));
        current = moved.job;
    }
    current
}

/// A canonical target for a status, carrying whatever that status stores.
fn target_for(status: JobStatus, trigger: Option<EscalationTrigger>) -> Target {
    match status {
        JobStatus::AwaitingApproval => Target::AwaitingApproval,
        JobStatus::Queued => Target::Queued,
        JobStatus::Running => Target::Running,
        JobStatus::AwaitingReview => Target::AwaitingReview,
        JobStatus::Escalated => {
            Target::Escalated(trigger.unwrap_or(EscalationTrigger::GateFailure))
        }
        JobStatus::Piloted => Target::Piloted(PilotReason::TakeOver),
        JobStatus::AwaitingAttestation => {
            Target::AwaitingAttestation(CriteriaOwed::one(CriterionId::new("c1")))
        }
        JobStatus::CompletedSuccess => Target::CompletedSuccess,
        JobStatus::CompletedFailed => Target::CompletedFailed,
        JobStatus::Rejected => Target::Rejected,
        JobStatus::Superseded => Target::Superseded,
        JobStatus::Killed => Target::Killed,
    }
}

/// A Job standing in `status`, arrived at by walking edges from the entry
/// status. There is no other way to get one.
fn reach(status: JobStatus) -> Job {
    let queued = [Target::Queued];
    let running = [Target::Queued, Target::Running];
    let job = created();
    let path: Vec<Target> = match status {
        JobStatus::AwaitingApproval => Vec::new(),
        JobStatus::Queued => queued.to_vec(),
        JobStatus::Running => running.to_vec(),
        JobStatus::AwaitingReview => [&running[..], &[Target::AwaitingReview]].concat(),
        JobStatus::AwaitingAttestation => [
            &running[..],
            &[Target::AwaitingAttestation(CriteriaOwed::one(
                CriterionId::new("c1"),
            ))],
        ]
        .concat(),
        JobStatus::Escalated => [
            &running[..],
            &[Target::Escalated(EscalationTrigger::GateFailure)],
        ]
        .concat(),
        JobStatus::Piloted => [&running[..], &[Target::Piloted(PilotReason::TakeOver)]].concat(),
        JobStatus::CompletedSuccess => [&running[..], &[Target::CompletedSuccess]].concat(),
        JobStatus::CompletedFailed => [&running[..], &[Target::CompletedFailed]].concat(),
        JobStatus::Rejected => vec![Target::Rejected],
        JobStatus::Superseded => [
            &running[..],
            &[Target::Piloted(PilotReason::TakeOver), Target::Superseded],
        ]
        .concat(),
        JobStatus::Killed => vec![Target::Killed],
    };
    let reached = drive(&job, &path);
    assert_eq!(
        reached.status(),
        status,
        "reach() walked to the wrong status"
    );
    reached
}

mod machine;
mod record;
mod step_machine;
