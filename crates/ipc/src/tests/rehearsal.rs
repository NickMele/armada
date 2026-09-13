//! A person's run, as the stream and the run history spell it.

use core_model::Timestamp;

use crate::{
    decode, encode, ChangeKind, ChangedFile, CheckoutRunDiff, Cursor, Delivered, DiffAgainst,
    Event, Instant, JobId, Missed, OutputClosed, OutputEnded, OutputLines, RunDiffReading,
    RunMessage, RunOpened, RunRecord, StreamMessage, PROTOCOL_VERSION,
};

fn at(text: &str) -> Instant {
    Instant::from(&Timestamp::from_rfc3339(text))
}

fn a_record() -> RunRecord {
    RunRecord {
        id: "01RUN".to_string(),
        job_id: JobId::carried("01JOB"),
        name: "fmt".to_string(),
        command: "cargo fmt --all".to_string(),
        narrowed: false,
        worktree_version: false,
        frozen: false,
        required: Vec::new(),
        started_at: at("2026-09-11T10:00:00.000Z"),
        ended_at: at("2026-09-11T10:00:04.000Z"),
        duration_ms: 4_000,
        exit_code: Some(0),
        expect_exit_code: 0,
        ended: "passed".to_string(),
        stopped: false,
        changed: vec![ChangedFile {
            path: "src/lib.rs".to_string(),
            change: ChangeKind::Modified,
            outside_plan: false,
        }],
        changed_unreadable: None,
        shared_with_drone: false,
        snapshot: Some("refs/armada/rehearsals/01RUN".to_string()),
        undone_at: None,
        log: ".armada/runs/01JOB/01RUN/output.log".to_string(),
    }
}

fn delivered(event: Event) -> String {
    encode(&StreamMessage::Event(Delivered {
        cursor: Cursor::at(3),
        event,
    }))
    .expect("plain data")
}

/// The run socket speaks `observe_job`'s four messages, spelled the same way.
#[test]
fn a_runs_socket_speaks_the_four_messages_the_other_sockets_do() {
    let spelled = |message: RunMessage| encode(&message).expect("plain data");
    let opened = spelled(RunMessage::Opened(RunOpened {
        protocol_version: PROTOCOL_VERSION,
        job_id: JobId::carried("01JOB"),
        id: "01RUN".to_string(),
        name: "test".to_string(),
        path: ".armada/runs/1-job/01RUN/output.log".to_string(),
        live: true,
        skipped: 0,
    }));
    assert!(opened.contains("\"message\":\"opened\""), "{opened}");
    let lines = spelled(RunMessage::Lines(OutputLines {
        lines: vec!["Compiling ipc".to_string()],
    }));
    assert!(lines.contains("\"message\":\"lines\""), "{lines}");
    let missed = spelled(RunMessage::Missed(Missed { dropped: 4 }));
    assert!(missed.contains("\"dropped\":4"), "{missed}");
    let closed = spelled(RunMessage::Closed(OutputClosed {
        because: OutputEnded::Finished,
    }));
    assert!(closed.contains("\"because\":\"finished\""), "{closed}");
}

/// `/events` carries a run's end and never its output.
#[test]
fn a_runs_end_travels_under_the_name_the_inventory_declares() {
    let finished = delivered(Event::RunFinished(a_record()));
    assert!(finished.contains("\"kind\":\"run.finished\""), "{finished}");
    let back: StreamMessage = decode("a stream message", finished.as_bytes()).expect("it reads");
    let StreamMessage::Event(Delivered {
        event: Event::RunFinished(record),
        ..
    }) = back
    else {
        panic!("not the end of a run");
    };
    assert_eq!(record, a_record());
}

/// A run with nothing to undo from says so by leaving the key out, not by
/// sending a null a reader would have to know about.
#[test]
fn a_record_with_no_snapshot_carries_no_key_for_one() {
    let mut record = a_record();
    record.snapshot = None;
    record.exit_code = None;
    let json = encode(&record).expect("plain data");
    assert!(!json.contains("\"snapshot\""), "{json}");
    assert!(!json.contains("\"exit_code\""), "{json}");
    let back: RunRecord = decode("a run record", json.as_bytes()).expect("it reads");
    assert_eq!(back, record);
}

/// A checkout run's diff says what it was measured against, and a snapshot
/// that is gone is its own state rather than an empty patch.
#[test]
fn a_checkout_runs_diff_names_its_base_and_a_gone_snapshot_is_a_state() {
    let read = CheckoutRunDiff {
        id: "01RUN".to_string(),
        against: DiffAgainst::RunSnapshot,
        reading: RunDiffReading::Read {
            files: a_record().changed,
            patch: Some("+formatted\n".to_string()),
        },
    };
    let json = encode(&read).expect("plain data");
    assert!(json.contains("\"against\":\"run_snapshot\""), "{json}");
    assert!(json.contains("\"state\":\"read\""), "{json}");
    let back: CheckoutRunDiff = decode("a run's diff", json.as_bytes()).expect("it reads");
    assert_eq!(back, read);

    let gone = CheckoutRunDiff {
        reading: RunDiffReading::Gone {
            why: "no snapshot".to_string(),
        },
        ..read
    };
    let json = encode(&gone).expect("plain data");
    assert!(json.contains("\"state\":\"gone\""), "{json}");
    assert!(!json.contains("\"patch\""), "{json}");
    let back: CheckoutRunDiff = decode("a run's diff", json.as_bytes()).expect("it reads");
    assert_eq!(back, gone);
}

/// **A Verify step spells its state beside its name**, one level deep, and
/// reads back whichever state it is in.
#[test]
fn a_verify_step_carries_its_state_flat_and_reads_back() {
    use crate::{CheckoutVerify, VerifyGroup, VerifyStep, VerifyStepState};
    let verify = CheckoutVerify {
        id: "01VERIFY".to_string(),
        started_at: at("2026-09-12T10:00:00.000Z"),
        ended_at: None,
        steps: vec![
            VerifyStep {
                group: VerifyGroup::Setup,
                name: "bootstrap".to_string(),
                run: "pnpm install".to_string(),
                state: VerifyStepState::Running {
                    run_id: "01RUN".to_string(),
                },
            },
            VerifyStep {
                group: VerifyGroup::Checks,
                name: "test".to_string(),
                run: "cargo test".to_string(),
                state: VerifyStepState::Waiting,
            },
            VerifyStep {
                group: VerifyGroup::Checks,
                name: "lint".to_string(),
                run: "cargo clippy".to_string(),
                state: VerifyStepState::NotRun {
                    why: "Verify was stopped during `test`".to_string(),
                },
            },
        ],
    };
    let sent = encode(&verify).expect("encodes");
    assert!(sent.contains(r#""group":"setup","name":"bootstrap","run":"pnpm install","state":"running","run_id":"01RUN""#), "{sent}");
    assert!(sent.contains(r#""state":"waiting""#), "{sent}");
    let back: CheckoutVerify = decode("a Verify", sent.as_bytes()).expect("reads back");
    assert_eq!(back, verify);
}
