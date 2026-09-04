//! A Job's own log, read back off a real file.
//!
//! The socket's own behaviour is proved in `api`, against a fake reader. What
//! has to be proved here is everything about the file: that the lines Fleet
//! already writes come back as notes, that a cursor resumes where it stopped,
//! and that a line arriving half-written is not consumed half-read.
//!
//! **The lines are written through `transcript::note`**, not hand-assembled, so
//! what these read is the shape Fleet actually appends rather than a second
//! guess at it.

use std::io::Write;

use api::Journal;
use core_model::{Component, Envelope, FieldValue, JobId, Level, Timestamp, Ulid};
use ipc::{NoteLevel, Voice};

use crate::journal::JobLogs;
use crate::tests::tmp::TempDir;
use crate::transcript::{log_of, note};

fn job() -> JobId {
    JobId::carried(Ulid::carried("01JOBLOG"))
}

fn envelope(msg: &str) -> Envelope {
    Envelope::new(
        Timestamp::from_rfc3339("2026-09-04T09:00:00.000Z"),
        Level::Info,
        Component::Fleet,
        Ulid::carried("01RUN"),
        msg,
    )
    .in_job(job().as_ulid().clone())
}

fn read(root: &str, from: u64) -> api::Reading {
    JobLogs::under(root).read(&ipc::JobId::from(&job()), from)
}

/// The claim: what Fleet wrote about a Job with no Drone on it comes back.
#[test]
fn what_fleet_wrote_about_a_job_comes_back_as_notes() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().into_owned();
    note(&root, &job(), &envelope("worktree cut")).expect("a line is written");
    note(&root, &job(), &envelope("preparation began")).expect("a line is written");

    let reading = read(&root, 0);

    assert_eq!(reading.notes.len(), 2);
    assert_eq!(reading.notes[0].msg, "worktree cut");
    assert_eq!(reading.notes[1].msg, "preparation began");
    assert!(!reading.unreadable);
    assert_eq!(reading.skipped, 0);
    // Every note names who, from the wire and not from the reader.
    assert!(reading.notes.iter().all(|note| note.by == Voice::Fleet));
    // And every one of these belongs to no step, which is the whole case: there
    // is no step running while a worktree is being cut.
    assert!(reading.notes.iter().all(|note| note.step.is_none()));
}

/// The envelope's `fields` are the payload a note opens to. **Every entry opens
/// to its payload** is the component's rule, and a Fleet event drawn as a line
/// of prose with nothing behind it is the failure it was written against.
#[test]
fn a_notes_fields_come_back_as_what_it_opens_to() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().into_owned();
    let envelope = envelope("the step edited outside its declared scope")
        .with_field("paths", FieldValue::Str("crates/api/src/routes.rs".into()))
        .with_field("outside", FieldValue::Int(3))
        .with_field("declared", FieldValue::Bool(false));
    note(&root, &job(), &envelope).expect("a line is written");

    let reading = read(&root, 0);
    let fields = &reading.notes[0].fields;

    assert_eq!(fields.len(), 3);
    // Names in the file's own order, values as text whatever the type was —
    // a surface draws all four scalar shapes the same way.
    let drawn: Vec<(&str, &str)> = fields
        .iter()
        .map(|field| (field.name.as_str(), field.value.as_str()))
        .collect();
    assert_eq!(
        drawn,
        vec![
            ("declared", "false"),
            ("outside", "3"),
            ("paths", "crates/api/src/routes.rs"),
        ]
    );
}

/// A level the file carries reaches the wire, so a warning draws as one.
#[test]
fn the_level_a_line_was_written_at_reaches_the_note() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().into_owned();
    let warned = Envelope::new(
        Timestamp::from_rfc3339("2026-09-04T09:00:01.000Z"),
        Level::Warn,
        Component::Fleet,
        Ulid::carried("01RUN"),
        "a preparation command failed",
    )
    .in_job(job().as_ulid().clone());
    note(&root, &job(), &warned).expect("a line is written");

    assert_eq!(read(&root, 0).notes[0].level, NoteLevel::Warn);
}

/// The second pass reads what was appended and nothing it has already sent.
/// **This is what makes the stream lossless** — the cursor is a position in the
/// record, so nothing has to be held to be dropped later.
#[test]
fn a_second_pass_reads_only_what_was_appended_since() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().into_owned();
    note(&root, &job(), &envelope("worktree cut")).expect("a line is written");

    let first = read(&root, 0);
    assert_eq!(first.notes.len(), 1);

    note(&root, &job(), &envelope("preparation began")).expect("a line is written");

    let second = read(&root, first.from);
    assert_eq!(second.notes.len(), 1);
    assert_eq!(second.notes[0].msg, "preparation began");
    assert!(second.from > first.from);

    // And a pass over a file nothing has appended to reads nothing, at the same
    // position. A viewer sitting on a quiet Job costs one read and no notes.
    let third = read(&root, second.from);
    assert!(third.notes.is_empty());
    assert_eq!(third.from, second.from);
}

/// A line still being written is left where it is, whole, for the next pass.
///
/// **The cursor never advances past a newline it has not seen.** A reader that
/// consumed half an object would resume in the middle of the other half and
/// lose both — one note drawn as a parse failure, and the event itself gone.
#[test]
fn a_half_written_line_is_read_whole_on_the_next_pass() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().into_owned();
    note(&root, &job(), &envelope("worktree cut")).expect("a line is written");

    let at = log_of(&root, &job());
    let whole = std::fs::read_to_string(&at).expect("the log reads");
    let second = format!(
        r#"{{"ts":"2026-09-04T09:00:02.000Z","level":"info","component":"fleet","run_id":"01RUN","msg":"preparation began","job_id":"01JOBLOG"}}"#
    );
    // The first half of a second line, with no newline behind it.
    let cut = &second[..second.len() / 2];
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&at)
        .expect("the log opens");
    file.write_all(cut.as_bytes()).expect("half a line lands");
    drop(file);

    let first = read(&root, 0);
    assert_eq!(first.notes.len(), 1, "the half line is not a note");
    assert_eq!(
        first.from,
        whole.len() as u64,
        "and the cursor stops at the last newline"
    );

    // The rest of it arrives.
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&at)
        .expect("the log opens");
    writeln!(file, "{}", &second[second.len() / 2..]).expect("the rest lands");
    drop(file);

    let next = read(&root, first.from);
    assert_eq!(next.notes.len(), 1);
    assert_eq!(next.notes[0].msg, "preparation began");
}

/// A Job with no log is nothing, and never a fault. A Job at the approval gate
/// has written no line, and neither has one proposed before any of this
/// existed — an empty stream is the true answer for both.
#[test]
fn a_job_with_no_log_reads_as_nothing_rather_than_as_a_fault() {
    let dir = TempDir::new();
    let reading = read(&dir.path().to_string_lossy(), 0);
    assert!(reading.notes.is_empty());
    assert!(!reading.unreadable);
}

/// A line that will not decode is counted, not dropped quietly. It was an
/// event; a gap nobody was told about reads as a Job nothing was happening to.
#[test]
fn a_line_that_will_not_decode_is_counted() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().into_owned();
    note(&root, &job(), &envelope("worktree cut")).expect("a line is written");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(log_of(&root, &job()))
        .expect("the log opens");
    writeln!(file, "{{not json").expect("a bad line lands");
    drop(file);

    let reading = read(&root, 0);
    assert_eq!(reading.notes.len(), 1);
    assert_eq!(reading.skipped, 1);
}
