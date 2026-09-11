//! A running Check's log, read as it grows.
//!
//! **The half `api` cannot prove.** The socket's behaviour is proved over there
//! against a fake reader; what is proved here is the reader against a real
//! file — that a line still being written is held back until its newline, that
//! the last pass takes the rest, and that a file not made yet is nothing yet.

use std::io::Write;

use crate::following::read_from;
use crate::tests::tmp::TempDir;

#[test]
fn a_line_still_being_written_waits_for_its_newline() {
    let dir = TempDir::new();
    let file = dir.path().join("implement.1.live.0.log");
    std::fs::write(&file, "compiling\nrunning 12 te").expect("a live log");

    let first = read_from(&file, 0, false);
    assert_eq!(first.lines, vec!["compiling".to_string()]);
    assert!(!first.unreadable);

    let mut more = std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .expect("appendable");
    more.write_all(b"sts\ntest result: ok").expect("appended");

    let second = read_from(&file, first.from, false);
    assert_eq!(
        second.lines,
        vec!["running 12 tests".to_string()],
        "the half line arrives whole, once its newline has"
    );

    let last = read_from(&file, second.from, true);
    assert_eq!(
        last.lines,
        vec!["test result: ok".to_string()],
        "the last pass takes what the Check left without a newline"
    );
}

#[test]
fn the_first_read_is_the_tail_and_says_what_it_left_out() {
    let dir = TempDir::new();
    let file = dir.path().join("implement.1.live.0.log");
    let written: String = (1..=2_500).map(|n| format!("line {n}\n")).collect();
    std::fs::write(&file, written).expect("a long live log");

    let first = read_from(&file, 0, false);
    assert_eq!(first.skipped, 500);
    assert_eq!(first.lines.len(), 2_000);
    assert_eq!(
        first.lines.first().map(String::as_str),
        Some("line 501"),
        "the tail, because a test runner prints its failures last"
    );
}

/// **The live set is the allowlist.** A name resolves to a file only while a
/// running gate of this Job wrote it; any other name reaches nothing, and the
/// log stops reading as written to the moment its Check finishes.
#[test]
fn only_a_log_a_running_check_is_writing_can_be_followed() {
    use std::sync::Arc;
    use std::time::Duration;

    use core_model::{Attempt, ResolvedCheck, StepId};
    use verification::{Exit, Observed};

    use crate::tests::gate::Stopped;
    use crate::underway::{Announcing, Underway};

    let repo = TempDir::new();
    let underway = Underway::default();
    let announcing = Announcing::on(
        ipc::JobId::carried("01JOB"),
        StepId::new("implement"),
        Attempt::FIRST,
        underway.clone(),
        api::Broadcaster::new(),
        Arc::new(Stopped),
        &repo.path().display().to_string(),
        "01JOB",
    );
    let check = ResolvedCheck::ManifestCheck {
        name: "test".to_string(),
        run: "/usr/bin/true".to_string(),
        expect_exit_code: 0,
        when: None,
        requires: Vec::new(),
        narrow: None,
    };
    announcing.began(std::slice::from_ref(&check), &[None]);
    let log = announcing.log_for(0).expect("a live log for the Check");
    announcing.started(0, Some(&log));

    let job = ipc::JobId::carried("01JOB");
    let kept = log
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a file name")
        .to_string();
    let found = underway.log(&job, &kept).expect("the running Check's log");
    assert_eq!(found.file, log);
    assert_eq!(found.name, "test");
    assert!(underway.writing(&job, &kept));
    assert_eq!(
        underway.log(&job, "implement.1.0.log"),
        None,
        "the recorded name is not a live one"
    );
    assert_eq!(underway.log(&job, "../../../etc/passwd"), None);
    assert_eq!(underway.log(&ipc::JobId::carried("01OTHER"), &kept), None);

    announcing.finished(0, &check, &Observed::Command(Exit::Code(0)), Duration::ZERO);
    assert!(
        !underway.writing(&job, &kept),
        "a finished Check writes nothing more, so its reader ends"
    );
}

#[test]
fn a_log_not_made_yet_is_nothing_yet_rather_than_unreadable() {
    let dir = TempDir::new();
    let read = read_from(&dir.path().join("not-yet.log"), 0, false);
    assert!(read.lines.is_empty());
    assert!(
        !read.unreadable,
        "a Check that has just started may not have opened its log"
    );
}
