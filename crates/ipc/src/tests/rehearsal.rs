//! A person's run, as the stream and the run history spell it.

use core_model::Timestamp;

use crate::{
    decode, encode, ChangeKind, ChangedFile, Cursor, Delivered, Event, Instant, JobId, Missed,
    OutputClosed, OutputEnded, OutputLines, RunMessage, RunOpened, RunRecord, StreamMessage,
    PROTOCOL_VERSION,
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
