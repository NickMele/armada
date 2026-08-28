//! What the wire must keep true: the DTOs a Bridge reads.
//!
//! The DTOs round-trip, a spelling the domain does not have is refused rather
//! than defaulted, and **an unknown field does not break a parse** — the last
//! is the one a review cannot check by reading, is the whole basis of the
//! minor-skew row, and would break silently the day somebody added
//! `deny_unknown_fields` for tidiness.
//!
//! The Evidence tool's transport is the other seam and is [`mcp`]'s. Nothing
//! about it is version-skewed — a Drone is spawned by the Fleet it reports to —
//! so what those cases hold is the opposite property: a field the tool does not
//! take is refused rather than ignored.

mod gates;
mod mcp;
mod turns;
mod version;

use core_model::{
    Actor, AdvanceGate, CriteriaOwed, CriterionId, DispatchOrigin, EvidenceType, Facts,
    FrozenWorkflow, Job, JobId, JobStatus, ManifestId, ModelName, NewJob, ResolvedStep, StepId,
    StepSeed, Target, Timestamp, Title, TopLevelOrigin, Ulid, Urgency, WorkflowId,
};

use crate::{
    decode, encode, AttachmentRef, ChangesRequested, CheckRun, DeclaredCheck, DroneMoved,
    JobDetail, JobDiff, JobEvidence, JobHistory, JobSummary, Judged, Movement, ProposeJob,
    Recorded, StatusMoved, StepFacts, StepMoved, StreamMessage, Submitted, Work,
};

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

/// **The distinction the whole field exists for.** An ungated step says so with
/// an empty list; a step Fleet cannot answer for carries no key at all. A
/// client that saw a gap either way could not tell them apart.
#[test]
fn an_ungated_step_says_so_and_an_unanswerable_one_carries_no_key() {
    let job = job();
    let ungated = JobDetail::of(
        &job,
        None,
        None,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("Reproduce it".to_string()),
            declares: Some(Vec::new()),
            ran: Vec::new(),
            judged: Vec::new(),
            flagged: Vec::new(),
            judging: None,
        }],
        None,
    );
    let json = encode(&ungated).expect("a detail is plain data");
    assert!(json.contains("\"checks\":[]"), "declares none: {json}");

    let unanswerable = JobDetail::of(
        &job,
        None,
        None,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: None,
            declares: None,
            ran: Vec::new(),
            judged: Vec::new(),
            flagged: Vec::new(),
            judging: None,
        }],
        None,
    );
    let json = encode(&unanswerable).expect("a detail is plain data");
    assert!(
        !json.contains("\"checks\""),
        "absent, never present-and-null: {json}"
    );
    assert!(
        json.contains("\"check_runs\":[]"),
        "what ran is always a list — nothing ran: {json}"
    );
}

/// **The fallback is deliberate, and a blank is not a state.** A step Fleet
/// cannot name reads as its id, so nothing downstream has to decide what an
/// empty label draws as.
#[test]
fn a_step_with_no_label_reads_as_its_id() {
    let detail = JobDetail::of(
        &job(),
        None,
        None,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("   ".to_string()),
            declares: None,
            ran: Vec::new(),
            judged: Vec::new(),
            flagged: Vec::new(),
            judging: None,
        }],
        None,
    );
    assert_eq!(detail.steps[0].label, "repro");

    let unanswerable = JobDetail::of(&job(), None, None, &[], None);
    assert_eq!(unanswerable.steps[0].label, "repro");
}

/// A recorded run round-trips, and a pass carries neither sentence.
#[test]
fn a_check_run_crosses_with_which_of_the_five_outcomes_it_was() {
    let detail = JobDetail::of(
        &job(),
        None,
        None,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("Reproduce it".to_string()),
            declares: Some(vec![DeclaredCheck {
                kind: "manifest_check".to_string(),
                name: Some("suite".to_string()),
                run: Some("cargo nextest run --workspace".to_string()),
                expect_exit_code: Some(0),
            }]),
            ran: vec![CheckRun {
                name: "suite".to_string(),
                outcome: core_model::CheckOutcome::NeverRan.into(),
                expected: Some("`suite` can be run".to_string()),
                produced: Some("`suite` is not installed".to_string()),
                output_path: Some(".armada/checks/01JOB/repro.0.log".to_string()),
            }],
            judged: Vec::new(),
            flagged: Vec::new(),
            judging: None,
        }],
        None,
    );
    let json = encode(&detail).expect("a detail is plain data");

    assert!(json.contains("\"outcome\":\"never_ran\""), "{json}");
    assert!(
        json.contains("\"run\":\"cargo nextest run --workspace\""),
        "the command the workflow froze crosses whole: {json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        detail
    );
}

/// **A refusal's citation crosses, and a no-objection carries none.**
///
/// This is what makes escalating a refusal worth more than ending the Job: the
/// escalation trigger says the gate stopped, and only these three lines say
/// what was wrong with the work. A person reading the Job is the audience.
#[test]
fn a_judge_refusal_crosses_with_the_three_lines_it_cited() {
    let detail = JobDetail::of(
        &job(),
        None,
        None,
        &[StepFacts {
            step_id: crate::StepId::carried("repro"),
            label: Some("Reproduce it".to_string()),
            declares: Some(Vec::new()),
            ran: Vec::new(),
            judged: vec![
                Judged {
                    criterion_id: crate::CriterionId::carried("c1"),
                    verdict: core_model::JudgeVerdict::NotMet.into(),
                    expected: Some("the caller's bound narrowed".to_string()),
                    produced: Some("the reader's bound widened".to_string()),
                    consequence: Some("every other caller reads one row too many".to_string()),
                },
                Judged {
                    criterion_id: crate::CriterionId::carried("c2"),
                    verdict: core_model::JudgeVerdict::Met.into(),
                    expected: None,
                    produced: None,
                    consequence: None,
                },
            ],
            flagged: Vec::new(),
            judging: None,
        }],
        None,
    );
    let json = encode(&detail).expect("a detail is plain data");

    assert!(json.contains("\"verdict\":\"not_met\""), "{json}");
    assert!(
        json.contains("every other caller reads one row too many"),
        "the line a person triages on crosses: {json}"
    );
    assert!(
        !json.contains("\"expected\":null"),
        "a no-objection cites nothing, and absent is not null: {json}"
    );
    assert_eq!(
        decode::<JobDetail>("a Job in full", json.as_bytes()).expect("it round-trips"),
        detail
    );
}

/// A step nothing asked the Judge about says so with an empty list, the way an
/// ungated step says so about its Checks.
#[test]
fn a_step_the_judge_was_never_asked_about_carries_an_empty_list() {
    let detail = JobDetail::of(&job(), None, None, &[], None);
    let json = encode(&detail).expect("a detail is plain data");
    assert!(json.contains("\"judged\":[]"), "{json}");
}

#[test]
fn a_transition_becomes_an_event_with_its_reason() {
    let owed = CriteriaOwed::one(CriterionId::new("c1"));
    let moved = job()
        .transition(Target::Queued, Actor::Human, at("2026-08-26T09:01:00.000Z"))
        .expect("awaiting_approval -> queued is an edge")
        .job
        .transition(
            Target::Running,
            Actor::Fleet,
            at("2026-08-26T09:02:00.000Z"),
        )
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
    let reason = event
        .reason
        .clone()
        .expect("an attestation debt is a reason");
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

/// The kinds are the dotted names `operations.toml` keys them under, so a rule
/// can compare the two without a mapping in between — and `branch` is absent
/// rather than null on an exit, which is the rule the whole file holds.
#[test]
fn the_drone_lifecycle_pair_travels_under_the_names_the_inventory_declares() {
    let job = job();
    let arrived = job
        .drone_spawned(
            core_model::DroneId::carried(Ulid::carried("01DRONE")),
            Actor::Fleet,
            at("2026-08-26T09:05:00.000Z"),
        )
        .expect("nothing is on it yet");

    let spawned = StreamMessage::Event(crate::Delivered {
        cursor: crate::Cursor::at(9),
        event: crate::Event::DroneSpawned(crate::DroneSpawned::of(
            &arrived.event,
            JobSummary::from(&arrived.job),
            Some("armada/01JOB".to_string()),
        )),
    });
    let json = encode(&spawned).expect("plain data");
    assert!(json.contains("\"kind\":\"drone.spawned\""), "{json}");
    assert!(json.contains("\"drone_id\":\"01DRONE\""), "{json}");
    assert!(json.contains("\"branch\":\"armada/01JOB\""), "{json}");
    assert_eq!(
        decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"),
        spawned
    );

    let left = arrived
        .job
        .drone_exited(Actor::Fleet, at("2026-08-26T09:06:00.000Z"))
        .expect("one is on it");
    let exited = crate::Event::DroneExited(crate::DroneExited::of(
        &left.event,
        JobSummary::from(&left.job),
    ));
    let json = encode(&exited).expect("plain data");
    assert!(json.contains("\"kind\":\"drone.exited\""), "{json}");
    assert!(
        !json.contains("assigned_drone"),
        "the row it carries no longer names a Drone, and absent is not null: {json}"
    );
}

/// The footprint's kind is the dotted name too, and the change kinds are the
/// spellings a TypeScript union has to mirror by hand — nothing generates the
/// DTO types from this crate yet, so this test is what pins them.
#[test]
fn a_footprint_travels_with_names_and_kinds_and_never_bytes() {
    let message = StreamMessage::Event(crate::Delivered {
        cursor: crate::Cursor::at(11),
        event: crate::Event::JobFilesChanged(crate::JobFilesChanged {
            job_id: crate::JobId::carried("01JOB"),
            step_id: crate::StepId::carried("repro"),
            drone_id: crate::DroneId::carried("01DRONE"),
            plan_declared: true,
            files: vec![
                crate::ChangedFile {
                    path: "src/parse.rs".to_string(),
                    change: crate::ChangeKind::Modified,
                    outside_plan: false,
                },
                crate::ChangedFile {
                    path: "src/legacy.rs".to_string(),
                    change: crate::ChangeKind::Deleted,
                    outside_plan: true,
                },
                crate::ChangedFile {
                    path: "docs/notes.md".to_string(),
                    change: crate::ChangeKind::TypeChanged,
                    outside_plan: true,
                },
            ],
            actor: Actor::Fleet.into(),
            at: (&at("2026-08-26T09:07:00.000Z")).into(),
        }),
    });
    let json = encode(&message).expect("plain data");

    assert!(json.contains("\"kind\":\"job.files_changed\""), "{json}");
    assert!(json.contains("\"change\":\"modified\""), "{json}");
    assert!(
        json.contains("\"change\":\"type_changed\""),
        "snake_case, as every other closed set on this wire is: {json}"
    );
    assert!(
        !json.contains("+++") && !json.contains("@@"),
        "names and kinds, never the patch: {json}"
    );
    assert_eq!(
        decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"),
        message
    );
}

/// **The mark defaults to unmarked.** A row from a peer that predates the field
/// reads as inside the plan rather than failing the parse — the same additive
/// rule the whole minor-skew row rests on.
#[test]
fn a_changed_file_with_no_mark_reads_as_inside_the_plan() {
    let file = decode::<crate::ChangedFile>(
        "a changed file",
        br#"{"path":"src/parse.rs","change":"added"}"#,
    )
    .expect("the mark defaults");
    assert!(!file.outside_plan);
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
    assert_eq!(proposal.model.as_deref(), Some("a-model"));
    assert!(proposal.acceptance_criteria.is_empty());
}

/// **Absent is the ordinary case.** A caller with no opinion about the model
/// sends nothing, and Fleet fills the value in from configuration — which is
/// why the field is optional rather than required-and-emptyable. The empty
/// string still decodes, because a DTO is deserialised rather than constructed;
/// it is refused where text becomes a Job.
#[test]
fn a_proposal_may_name_no_model_at_all() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF",
        "origin":"manual","urgency":"normal","atomic":false}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("a model is optional");
    assert_eq!(proposal.model, None);
}

/// **Additive, like `model`.** A proposal that predates this field carries no
/// `attachments` key at all, and `#[serde(default)]` is what lets it still
/// decode — the minor bump this field cost rests on exactly this.
#[test]
fn a_proposal_with_no_attachments_key_still_decodes() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF",
        "origin":"manual","urgency":"normal","atomic":false}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("attachments default to none");
    assert!(proposal.attachments.is_empty());
}

/// A staged file crosses as a path, never as bytes — the same same-machine
/// assumption `write_targets` already rests on.
#[test]
fn a_proposal_carries_the_staged_files_a_person_attached() {
    let body = br#"{"title":"fix the parser","workflow_id":"01WF","owner_manifest_id":"01MF",
        "origin":"manual","urgency":"normal","atomic":false,
        "attachments":[{"staged_path":"/tmp/armada-attachments/01/before.png",
        "filename":"before.png","mime_type":"image/png"}]}"#;
    let proposal = decode::<ProposeJob>("proposal", body).expect("attachments decode");
    assert_eq!(
        proposal.attachments,
        vec![AttachmentRef {
            staged_path: "/tmp/armada-attachments/01/before.png".to_string(),
            filename: "before.png".to_string(),
            mime_type: "image/png".to_string(),
        }]
    );
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

/// One history, of all three shapes, round-trips — and **the tag is what tells
/// them apart.** A client reading a timeline matches on `kind`, so a row that
/// serialised without one would be a row nothing could draw.
#[test]
fn a_history_carries_all_three_shapes_and_names_each_one() {
    let history = JobHistory {
        job_id: crate::JobId::carried("01JOB"),
        moves: vec![
            Recorded {
                seq: 1,
                status: crate::JobStatus::from_wire("awaiting_approval").expect("a status"),
                moved: Movement::Status(StatusMoved {
                    to: crate::JobStatus::from_wire("queued").expect("a status"),
                    reason: None,
                }),
                actor: crate::Actor::from_wire("human").expect("an actor"),
                at: crate::Instant::carried("2026-08-26T09:01:00.000Z"),
            },
            Recorded {
                seq: 2,
                status: crate::JobStatus::from_wire("running").expect("a status"),
                moved: Movement::Drone(DroneMoved {
                    drone_id: crate::DroneId::carried("01DRONE"),
                    presence: crate::DronePresence::from_wire("drone_spawned").expect("a presence"),
                }),
                actor: crate::Actor::from_wire("fleet").expect("an actor"),
                at: crate::Instant::carried("2026-08-26T09:02:00.000Z"),
            },
            Recorded {
                seq: 3,
                status: crate::JobStatus::from_wire("running").expect("a status"),
                moved: Movement::Step(StepMoved {
                    step_id: crate::StepId::carried("repro"),
                    from: crate::StepState::from_wire("running").expect("a step state"),
                    to: crate::StepState::from_wire("stopped").expect("a step state"),
                    why: Some("gate_failure".to_string()),
                }),
                actor: crate::Actor::from_wire("fleet").expect("an actor"),
                at: crate::Instant::carried("2026-08-26T09:03:00.000Z"),
            },
        ],
    };
    let json = encode(&history).expect("plain data");
    assert!(json.contains(r#""kind":"status""#), "{json}");
    assert!(json.contains(r#""kind":"drone""#), "{json}");
    assert!(json.contains(r#""kind":"step""#), "{json}");
    assert!(
        json.contains(r#""presence":"drone_spawned""#),
        "the registry's own spelling, not a second one: {json}"
    );
    assert!(
        !json.contains(r#""reason":null"#),
        "absent, never present and null: {json}"
    );
    assert_eq!(
        decode::<JobHistory>("a Job's history", json.as_bytes()).expect("it round-trips"),
        history
    );
}

/// **A history is a list, and an empty one is an answer.** A Job created and
/// not yet moved has no rows, which a client must be able to draw as "nothing
/// has happened" rather than as a failure.
#[test]
fn a_history_with_no_moves_decodes() {
    let history = decode::<JobHistory>("a Job's history", br#"{"job_id":"01JOB","moves":[]}"#)
        .expect("empty is a shape");
    assert!(history.moves.is_empty());
}

/// A step move that stopped nothing carries no trigger, and a row from a peer
/// that omits it still parses — the additive rule the minor-skew row rests on,
/// applied to the newest DTO.
#[test]
fn a_step_move_that_stopped_nothing_carries_no_trigger() {
    let history = decode::<JobHistory>(
        "a Job's history",
        br#"{"job_id":"01JOB","moves":[{"seq":4,"status":"running",
            "moved":{"kind":"step","step_id":"repro","from":"not_started","to":"running"},
            "actor":"fleet","at":"2026-08-26T09:04:00.000Z"}]}"#,
    )
    .expect("a step move without a trigger");
    let Movement::Step(step) = &history.moves[0].moved else {
        panic!("a step move");
    };
    assert_eq!(step.why, None);
}

/// **The reviewing reads keep the absent-never-empty rule, in both places it
/// bites.** A Job with no worktree carries no reading at all, and a submission
/// that drew no boundary carries no `not_claimed` — an empty object and an
/// empty string would each read as a value somebody lost.
#[test]
fn the_reviewing_reads_are_absent_rather_than_empty() {
    let nothing_to_read = JobDiff {
        job_id: crate::JobId::carried("01JOB"),
        work: None,
    };
    let json = encode(&nothing_to_read).expect("a diff is plain data");
    assert!(
        !json.contains("\"work\""),
        "there was no worktree, and absent is not an empty reading: {json}"
    );

    let read_and_empty = JobDiff {
        job_id: crate::JobId::carried("01JOB"),
        work: Some(Work {
            files: Vec::new(),
            plan_declared: false,
            patch: None,
        }),
    };
    let json = encode(&read_and_empty).expect("a diff is plain data");
    assert!(
        json.contains("\"files\":[]"),
        "a worktree that opened and holds no change is a real answer: {json}"
    );
    assert!(
        !json.contains("\"patch\""),
        "and nothing in it is absent rather than blank: {json}"
    );

    let boundless = JobEvidence {
        job_id: crate::JobId::carried("01JOB"),
        steps: vec![Submitted {
            step_id: crate::StepId::carried("repro"),
            evidence_type: EvidenceType::FailingTest.into(),
            claimed: "the reader stops one line early".to_string(),
            shown_by: "a failing test".to_string(),
            not_claimed: None,
        }],
    };
    let json = encode(&boundless).expect("evidence is plain data");
    assert!(
        !json.contains("not_claimed"),
        "legitimately empty on the record is absent on the wire: {json}"
    );
    assert!(
        json.contains("\"evidence_type\":\"failing_test\""),
        "the registry's own spelling, through the domain pair: {json}"
    );
}

/// A note is refused on its spelling, not on its emptiness. **Blank is Fleet's
/// refusal** — a decoded request is well-formed, and a value that cannot work
/// is a 422 rather than a 400.
#[test]
fn a_blank_review_note_decodes_and_is_refused_further_in() {
    let note = decode::<ChangesRequested>("a review note", br#"{"note":"   "}"#)
        .expect("the bytes became a request");
    assert_eq!(note.note, "   ");
}
