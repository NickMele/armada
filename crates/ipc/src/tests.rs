//! What the wire must keep true.
//!
//! Three properties, and the third is the one a review cannot check by reading:
//! the DTOs round-trip, a spelling the domain does not have is refused rather
//! than defaulted, and **an unknown field does not break a parse** — which is
//! the whole basis of the minor-skew row, and would break silently the day
//! somebody added `deny_unknown_fields` for tidiness.

use core_model::{
    Actor, CriteriaOwed, CriterionId, DispatchOrigin, Facts, Job, JobId, JobStatus, ManifestId,
    ModelName, NewJob, StepId, StepSeed, Target, Timestamp, Title, TopLevelOrigin, Ulid, Urgency,
    WorkflowId,
};

use crate::{decode, encode, JobSummary, ProposeJob, StreamMessage};

fn at(instant: &str) -> Timestamp {
    Timestamp::from_rfc3339(instant)
}

fn job() -> Job {
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried("01JOB")),
            title: Title::new("fix the off-by-one").expect("a title"),
            workflow_id: WorkflowId::carried(Ulid::carried("01WF")),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("a-model"),
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
        },
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

#[test]
fn a_summary_carries_what_a_board_renders_and_nothing_else() {
    let summary = JobSummary::from(&job());
    let json = encode(&summary).expect("a summary is plain data");

    assert!(json.contains("\"status\":\"awaiting_approval\""));
    assert!(json.contains("\"origin\":\"manual\""));
    assert!(
        !json.contains("secret"),
        "facts are not on the wire: {json}"
    );
    assert!(
        !json.contains("repro"),
        "the step rows are not on the wire, only current_step_id: {json}"
    );
    assert_eq!(
        decode::<JobSummary>("job summary", json.as_bytes()).expect("it round-trips"),
        summary
    );
}

#[test]
fn a_transition_becomes_an_event_with_its_reason() {
    let owed = CriteriaOwed::one(CriterionId::new("c1"));
    let moved = job()
        .transition(Target::Queued, Actor::Human, at("2026-08-26T09:01:00.000Z"))
        .expect("awaiting_approval -> queued is an edge")
        .job
        .transition(Target::Running, Actor::Fleet, at("2026-08-26T09:02:00.000Z"))
        .expect("queued -> running is an edge")
        .job
        .transition(
            Target::AwaitingAttestation(owed),
            Actor::Fleet,
            at("2026-08-26T09:03:00.000Z"),
        )
        .expect("running -> awaiting_attestation is an edge");

    let event = crate::JobStateChanged::from(&moved.event);
    assert_eq!(event.from.domain(), JobStatus::Running);
    assert_eq!(event.to.domain(), JobStatus::AwaitingAttestation);
    let reason = event.reason.clone().expect("an attestation debt is a reason");
    assert_eq!(reason.named, None, "a debt is references, not a name");
    assert_eq!(reason.criteria_owed.len(), 1);

    let message = StreamMessage::Event(crate::Delivered {
        cursor: crate::Cursor::at(7),
        event: crate::Event::JobStateChanged(event),
    });
    let json = encode(&message).expect("plain data");
    assert!(json.contains("\"message\":\"event\""));
    assert!(json.contains("\"kind\":\"job.state_changed\""));
    assert_eq!(
        decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"),
        message
    );
}

#[test]
fn a_queued_transition_carries_no_reason_because_the_log_stores_none() {
    let moved = job()
        .transition(Target::Queued, Actor::Human, at("2026-08-26T09:01:00.000Z"))
        .expect("awaiting_approval -> queued is an edge");
    let event = crate::JobStateChanged::from(&moved.event);
    assert_eq!(event.reason, None);
    let json = encode(&event).expect("plain data");
    assert!(
        !json.contains("reason"),
        "absent, never present and null: {json}"
    );
}

/// The field is required on the wire, not merely expected: a proposal that
/// omits it does not become a `ProposeJob` at all, so nothing downstream has to
/// decide what an untitled Job is called.
#[test]
fn a_proposal_with_no_title_does_not_decode() {
    let body = br#"{"workflow_id":"01WF","owner_manifest_id":"01MF","origin":"manual",
        "urgency":"normal","atomic":false,"model":"a-model"}"#;
    let refused = decode::<ProposeJob>("proposal", body).expect_err("a Job has a title");
    assert!(refused.to_string().contains("title"), "{refused}");
}

/// The list is where a title is read, so it is on the summary — and the
/// redaction the summary exists for still holds around it.
#[test]
fn a_summary_carries_the_title_a_person_reads() {
    let summary = JobSummary::from(&job());
    assert_eq!(summary.title, "fix the off-by-one");
    let json = encode(&summary).expect("plain data");
    assert!(json.contains("\"title\":\"fix the off-by-one\""), "{json}");
}

#[test]
fn a_spelling_the_registry_does_not_have_is_refused() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF","origin":"manual",
        "urgency":"whenever","atomic":false,"model":"a-model"}"#;
    let refused = decode::<ProposeJob>("proposal", body).expect_err("`whenever` is not an urgency");
    assert!(refused.to_string().contains("whenever"));
}

#[test]
fn a_proposal_cannot_claim_to_be_sub_dispatched() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF","origin":"sub_dispatched",
        "urgency":"normal","atomic":false,"model":"a-model"}"#;
    let refused = decode::<ProposeJob>("proposal", body)
        .expect_err("a peer does not create a sub-dispatched Job");
    assert!(refused.to_string().contains("sub_dispatched"));
}

#[test]
fn an_unknown_field_parses_and_is_ignored() {
    // The minor-skew row in one assertion: a newer peer adds a field, an older
    // peer reads the message anyway. `deny_unknown_fields` would fail here.
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF","origin":"manual",
        "urgency":"normal","atomic":false,"model":"a-model","dispatch_budget":12}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("unknown fields are ignored");
    assert_eq!(proposal.model, "a-model");
    assert!(proposal.acceptance_criteria.is_empty());
}

#[test]
fn the_summary_of_a_sub_dispatched_job_says_so() {
    let parent = DispatchOrigin {
        job_id: JobId::carried(Ulid::carried("01PARENT")),
        step_id: StepId::new("fix"),
    };
    let sub = Job::create_sub_dispatched(
        NewJob {
            id: JobId::carried(Ulid::carried("01SUB")),
            title: Title::new("write the regression test").expect("a title"),
            workflow_id: WorkflowId::carried(Ulid::carried("01WF")),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Incident,
            atomic: true,
            model: ModelName::new("a-model"),
            acceptance_criteria: Vec::new(),
            steps: Vec::new(),
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::empty(),
            scope_revisions: Vec::new(),
        },
        parent,
        at("2026-08-26T09:00:00.000Z"),
    );
    let summary = JobSummary::from(&sub);
    assert_eq!(summary.origin.as_wire(), "sub_dispatched");
    assert_eq!(summary.status.as_wire(), "queued");
    assert_eq!(summary.urgency.as_wire(), "incident");
}
