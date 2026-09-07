//! What the wire must keep true: the DTOs a Bridge reads.
//!
//! The DTOs round-trip, a spelling the domain does not have is refused rather
//! than defaulted, and **an unknown field does not break a parse** — the last
//! is the one a review cannot check by reading, is the whole basis of the
//! minor-skew row, and would break silently the day somebody added
//! `deny_unknown_fields` for tidiness. Each module below holds one subject's
//! share of that.
//!
//! What is left here is the Job every one of them starts from. The fixtures
//! are shared rather than copied per module because a case that built its own
//! Job would be asserting against a shape nothing else on this seam sees.
//!
//! The Evidence tool's transport is the other seam and is [`mcp`]'s. Nothing
//! about it is version-skewed — a Drone is spawned by the Fleet it reports to —
//! so what those cases hold is the opposite property: a field the tool does not
//! take is refused rather than ignored.

/// One Job, whole: the step rows, the gates' answers and the fields a Board
/// row leaves behind.
mod details;
/// What the stream carries, under the names `operations.toml` declares.
mod events;
mod fixtures;
mod gates;
/// A Job's timeline, in the three shapes a row can be.
mod history;
mod journal;
mod mcp;
/// The one DTO on this seam a peer *writes*, and what it refuses.
mod proposals;
mod reports;
/// What a reviewing person is handed, and the note they send back.
mod reviewing;
/// The Board row, and the redaction it exists for.
mod summaries;
mod turns;
mod version;

use core_model::{
    AdvanceGate, EvidenceType, Facts, FrozenWorkflow, Job, JobId, ManifestId, ModelName, NewJob,
    ResolvedStep, StepId, StepSeed, Timestamp, Title, TopLevelOrigin, Ulid, Urgency, WorkflowId,
};

use crate::{JobDetail, StepFacts};

fn at(instant: &str) -> Timestamp {
    Timestamp::from_rfc3339(instant)
}

/// The one-step workflow the fixture Jobs freeze.
fn workflow() -> FrozenWorkflow {
    FrozenWorkflow::frozen(
        WorkflowId::carried(Ulid::carried("01WF")),
        "bug".to_string(),
        1,
        vec![ResolvedStep::frozen(
            StepId::new("repro"),
            "Reproduce".to_string(),
            Some(EvidenceType::FailingTest),
            Vec::new(),
            AdvanceGate::Auto,
            Vec::new(),
            None,
            0,
            None,
        )],
    )
}

fn job() -> Job {
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried("01JOB")),
            title: Title::new("fix the off-by-one").expect("a title"),
            workflow: workflow(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: vec![StepSeed {
                step_id: StepId::new("repro"),
                ordinal: 0,
            }],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::new("a secret nobody outside Fleet needs"),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

/// One Job's detail, with the eight facts that are not the Job itself absent.
///
/// **A helper because the signature is eleven positional arguments**, eight of
/// which every case here passes `None` for. Two of the eight were added on one
/// night by two people who could not see each other's, and a run of eight
/// `None`s is where a ninth lands in the wrong slot silently. What a case is
/// about is the Job and its steps, and this says so.
pub(super) fn detail_of(job: &core_model::Job, steps: &[StepFacts]) -> JobDetail {
    JobDetail::of(
        job, None, None, None, steps, None, None, None, None, None, None, None,
    )
}
