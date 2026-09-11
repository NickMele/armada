//! What the stream Bridge draws the Board from carries.
//!
//! **The kinds are the dotted names `crates/ipc/operations.toml` keys them
//! under**, so a rule can compare the two without a mapping in between. And
//! nothing generates the DTO types from this crate yet, so these cases are
//! what pin the spellings a TypeScript union mirrors by hand — a variant
//! renamed here fails in Rust rather than as a branch that never runs.

use core_model::{Actor, CriteriaOwed, CriterionId, JobStatus, Target, Ulid};

use crate::tests::{at, job};
use crate::{decode, encode, JobSummary, StreamMessage};

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
    let step = core_model::StepId::new("repro");
    let arrived = job
        .drone_spawned(
            &step,
            core_model::DroneId::carried(Ulid::carried("01DRONE")),
            Actor::Fleet,
            at("2026-08-26T09:05:00.000Z"),
        )
        .expect("nothing is on that step yet");

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
    assert!(
        json.contains("\"step_id\":\"repro\""),
        "a Drone arrives on a step, and the event says which: {json}"
    );
    assert!(json.contains("\"branch\":\"armada/01JOB\""), "{json}");
    assert_eq!(
        decode::<StreamMessage>("stream message", json.as_bytes()).expect("it round-trips"),
        spawned
    );

    let left = arrived
        .job
        .drone_exited(&step, Actor::Fleet, at("2026-08-26T09:06:00.000Z"))
        .expect("one is on that step");
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

/// A held command travels under the name the inventory declares, and the
/// message that ends the wait carries nothing — that absence is the message,
/// as it is on `job.asking`.
#[test]
fn a_held_command_goes_out_whole_and_comes_back_empty() {
    let out = crate::Event::JobCommandWaiting(crate::JobCommandWaiting {
        job_id: crate::JobId::carried("01JOB"),
        step_id: crate::StepId::carried("repro"),
        waiting: Some(crate::CommandInFlight {
            call: "toolu_01".to_string(),
            step_id: crate::StepId::carried("repro"),
            asked_at: crate::Instant::carried("2026-09-11T09:00:00.000Z"),
            tool: "Bash".to_string(),
            detail: "touch x".to_string(),
            truncated: false,
            length: Some(7),
            offers: vec![
                crate::CommandAnswer::AllowForJob,
                crate::CommandAnswer::Reject,
            ],
        }),
        actor: Actor::Drone.into(),
        at: (&at("2026-09-11T09:00:00.000Z")).into(),
    });
    let json = encode(&out).expect("plain data");
    assert!(json.contains("\"kind\":\"job.command_waiting\""), "{json}");
    assert!(
        json.contains("\"waiting\":{\"call\":\"toolu_01\""),
        "{json}"
    );
    assert_eq!(
        decode::<crate::Event>("an event", json.as_bytes()).expect("it round-trips"),
        out
    );

    let back = crate::Event::JobCommandWaiting(crate::JobCommandWaiting {
        job_id: crate::JobId::carried("01JOB"),
        step_id: crate::StepId::carried("repro"),
        waiting: None,
        actor: Actor::Human.into(),
        at: (&at("2026-09-11T09:02:00.000Z")).into(),
    });
    let json = encode(&back).expect("plain data");
    assert!(!json.contains("\"waiting\""), "absent, never null: {json}");
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
