//! What a Job's log notes must keep true on the wire.
//!
//! The one case worth more than a round-trip is the level: `NoteLevel` mirrors
//! a set whose authority is `core_model::Level`, and a mirror nothing checks is
//! a second vocabulary waiting to disagree.

use core_model::Level;

use crate::{
    decode, encode, JobId, JournalClosed, JournalMessage, JournalOpened, LogNote, NoteLevel,
    NotedField, Quiet, StepId, Voice, PROTOCOL_VERSION,
};

fn note() -> LogNote {
    LogNote {
        at: crate::Instant::carried("2026-09-04T10:00:00.000Z"),
        by: Voice::Fleet,
        level: NoteLevel::Info,
        msg: "worktree cut".to_string(),
        step: None,
        drone: None,
        fields: vec![NotedField {
            name: "branch".to_string(),
            value: "armada/job_01".to_string(),
        }],
    }
}

/// **The mirror is checked, not trusted.** `NoteLevel`'s `serde` spelling is
/// `core_model::Level::as_wire`'s, and this is the only thing holding them
/// together — `ipc` cannot reach into that enum and that enum knows nothing
/// about the wire.
#[test]
fn every_level_spells_what_the_envelope_spells() {
    let pairs = [
        (Level::Trace, NoteLevel::Trace),
        (Level::Debug, NoteLevel::Debug),
        (Level::Info, NoteLevel::Info),
        (Level::Warn, NoteLevel::Warn),
        (Level::Error, NoteLevel::Error),
    ];
    for (level, mirrored) in pairs {
        let written = encode(&mirrored).expect("a level encodes");
        assert_eq!(written, format!("\"{}\"", level.as_wire()));
    }
}

/// A note the envelope stamped with no step is a Job-level note, and the
/// absence has to survive the wire — present-and-null would make it a step
/// nobody can name rather than no step at all.
#[test]
fn a_note_with_no_step_carries_no_step_key() {
    let written = encode(&note()).expect("a note encodes");
    assert!(!written.contains("step"), "{written}");
    assert!(!written.contains("drone"), "{written}");
    assert!(!written.contains("null"), "{written}");
}

#[test]
fn a_note_round_trips() {
    let mut original = note();
    original.step = Some(StepId::carried("implement"));
    let written = encode(&original).expect("a note encodes");
    let read: LogNote = decode("a note", written.as_bytes()).expect("a note decodes");
    assert_eq!(read, original);
}

/// The flattening is the shape a reader parses, so it is asserted rather than
/// assumed: a note is its own fields beside the tag, not nested under one.
#[test]
fn a_note_message_is_flat() {
    let written = encode(&JournalMessage::Note(note())).expect("a message encodes");
    assert!(written.contains("\"message\":\"note\""), "{written}");
    assert!(written.contains("\"msg\":\"worktree cut\""), "{written}");
}

#[test]
fn the_three_messages_round_trip() {
    let each = [
        JournalMessage::Opened(JournalOpened {
            protocol_version: PROTOCOL_VERSION,
            job_id: JobId::carried("01JOB"),
            skipped: 3,
        }),
        JournalMessage::Note(note()),
        JournalMessage::Closed(JournalClosed {
            because: Quiet::Unreadable,
        }),
    ];
    for message in each {
        let written = encode(&message).expect("a message encodes");
        let read: JournalMessage =
            decode("a journal message", written.as_bytes()).expect("a message decodes");
        assert_eq!(read, message);
    }
}

/// The minor-skew row, on this stream. A Fleet ahead sends a field this Bridge
/// does not read, and the note still parses.
#[test]
fn an_unknown_field_does_not_break_a_note() {
    let written = r#"{"at":"2026-09-04T10:00:00.000Z","by":"fleet","level":"warn","msg":"a preparation command failed","weather":"fine"}"#;
    let read: LogNote = decode("a note", written.as_bytes()).expect("a note decodes");
    assert_eq!(read.level, NoteLevel::Warn);
    assert_eq!(read.by, Voice::Fleet);
}
