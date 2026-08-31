//! The machine, tested against the registry it was built from.
//!
//! **Every test reaches its state by transitioning.** [`reach`] is the only way
//! a test here gets a Job into a status other than the entry one, and it walks
//! real edges to get there. A test that constructed a Job in a state directly
//! would be asserting against a value it made up rather than against the
//! machine — which is the crack a no-setter rule leaks through, and the reason
//! [`Job`] has no constructor that takes a status for such a test to call.
//!
//! **A guarded edge is walked with its condition satisfied**, by
//! [`advance_every_step`], and never by relaxing the guard. A test asserting
//! that a guard *refuses* uses the same [`reach`] every other test does, since
//! a freshly created Job's steps are `not_started` and satisfy nothing.
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
                None,
                0,
                None,
            ),
            ResolvedStep::frozen(
                StepId::new("fix"),
                "Fix".into(),
                Some(EvidenceType::Diff),
                vec![ResolvedCheck::DiffNonempty],
                AdvanceGate::Auto,
                Vec::new(),
                None,
                0,
                None,
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
        attachments: Vec::new(),
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

/// Every step of the Job advanced, walking the inner machine's real edges.
///
/// **Only usable beneath an advancing status**, which is the constraint the
/// guard lives inside: a Job satisfies `every_step_advanced` before it leaves
/// `running` or `awaiting_review`, never after.
fn advance_every_step(job: &Job) -> Job {
    let mut current = job.clone();
    let steps: Vec<StepId> = current
        .steps()
        .iter()
        .map(|row| row.step_id().clone())
        .collect();
    for step_id in steps {
        for target in [StepTarget::Running, StepTarget::Advanced] {
            current = current
                .transition_step(
                    &step_id,
                    target,
                    Actor::Fleet,
                    at("2026-08-26T09:02:00.000Z"),
                )
                .unwrap_or_else(|e| panic!("advancing {}: {e}", step_id.as_str()))
                .job;
        }
    }
    current
}

/// A Job standing in `status` with every step advanced — what a guarded edge
/// is admitted from.
///
/// The steps are advanced at `running` and the walk continues from there,
/// because the inner machine is frozen everywhere else. That is the ordering
/// `fleet::landing` and `fleet::reviewing` walk, asserted here.
fn reach_with_every_step_advanced(status: JobStatus) -> Job {
    let advanced = advance_every_step(&reach(JobStatus::Running));
    let path: Vec<Target> = match status {
        JobStatus::Running => Vec::new(),
        JobStatus::AwaitingReview => vec![Target::AwaitingReview],
        JobStatus::AwaitingAttestation => vec![Target::AwaitingAttestation(CriteriaOwed::one(
            CriterionId::new("c1"),
        ))],
        JobStatus::Piloted => vec![Target::Piloted(PilotReason::TakeOver)],
        other => panic!("no advanced-steps path to {}", other.as_wire()),
    };
    let reached = drive(&advanced, &path);
    assert_eq!(
        reached.status(),
        status,
        "the walk went to the wrong status"
    );
    reached
}

/// A Job standing in `status`, arrived at by walking edges from the entry
/// status. There is no other way to get one.
fn reach(status: JobStatus) -> Job {
    let queued = [Target::Queued];
    let running = [Target::Queued, Target::Running];
    let job = created();
    // `completed_success` is guarded, and the steps have to be advanced on the
    // `running` side of the edge — the inner machine is frozen on the other.
    // Every other status is reached with its steps where creation left them.
    if status == JobStatus::CompletedSuccess {
        let advanced = advance_every_step(&drive(&job, &running));
        let reached = drive(&advanced, &[Target::CompletedSuccess]);
        assert_eq!(
            reached.status(),
            status,
            "reach() walked to the wrong status"
        );
        return reached;
    }
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
        JobStatus::CompletedFailed => [&running[..], &[Target::CompletedFailed]].concat(),
        JobStatus::Rejected => vec![Target::Rejected],
        JobStatus::Superseded => [
            &running[..],
            &[Target::Piloted(PilotReason::TakeOver), Target::Superseded],
        ]
        .concat(),
        JobStatus::Killed => vec![Target::Killed],
        // Taken above: it is the one status no path of targets alone reaches.
        JobStatus::CompletedSuccess => unreachable!("handled before the match"),
    };
    let reached = drive(&job, &path);
    assert_eq!(
        reached.status(),
        status,
        "reach() walked to the wrong status"
    );
    reached
}

mod covers;
mod machine;
mod models;
mod note;
mod record;
mod step_machine;
mod stuck;
