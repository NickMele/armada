//! What a Board row carries, and what it deliberately does not.
//!
//! [`JobSummary`] is drawn for every Job at once, so what it withholds is the
//! redaction the type exists for — the brief, the step rows — and a case here
//! is that decision rather than a rendering choice. The same row is nested
//! whole inside [`JobDetail`](crate::JobDetail), which is what stops the two
//! from ever disagreeing about a field they share.

use core_model::{
    DispatchOrigin, Facts, Job, JobId, ManifestId, ModelName, NewJob, StepId, Title, Ulid, Urgency,
};

use crate::tests::{at, job, workflow};
use crate::{decode, encode, JobSummary};

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
    assert!(
        json.contains("\"created_at\":\"2026-08-26T09:00:00.000Z\""),
        "the instant elapsed is measured from is on the row: {json}"
    );
    assert_eq!(
        decode::<JobSummary>("job summary", json.as_bytes()).expect("it round-trips"),
        summary
    );
}

/// **Absent, never present-and-null.** A Job at the approval gate has no
/// worktree, and a client that received `branch: null` could not tell that from
/// Fleet having forgotten to say.
#[test]
fn a_row_names_its_branch_only_once_a_worktree_exists() {
    let waiting = encode(&JobSummary::from(&job())).expect("plain data");
    assert!(
        !waiting.contains("branch"),
        "a Job with no worktree claims no branch: {waiting}"
    );

    let branded = job().on_branch(core_model::Branch::new("armada/01JOB").expect("a branch"));
    let working = encode(&JobSummary::from(&branded)).expect("plain data");
    assert!(
        working.contains("\"branch\":\"armada/01JOB\""),
        "the branch a person merges is on the row: {working}"
    );
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
fn the_summary_of_a_sub_dispatched_job_says_so() {
    let parent = DispatchOrigin {
        job_id: JobId::carried(Ulid::carried("01PARENT")),
        step_id: StepId::new("fix"),
    };
    let sub = Job::create_sub_dispatched(
        NewJob {
            id: JobId::carried(Ulid::carried("01SUB")),
            title: Title::new("write the regression test").expect("a title"),
            workflow: workflow(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Incident,
            atomic: true,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: Vec::new(),
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            proposal_id: None,
            number: core_model::JobNumber::carried(1),
            facts: Facts::empty(),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        parent,
        at("2026-08-26T09:00:00.000Z"),
    );
    let summary = JobSummary::from(&sub);
    assert_eq!(summary.origin.as_wire(), "sub_dispatched");
    assert_eq!(summary.status.as_wire(), "queued");
    assert_eq!(summary.urgency.as_wire(), "incident");
}
