//! Stage 1 of the `scripts/land` port: the state this line keeps on disk,
//! independent of any runner or subprocess. `scripts/test_land.py`'s
//! `test_an_entry_missing_its_place_does_not_stop_the_line` and the
//! place-survives-a-resubmit half of `test_a_conflict_stops_and_keeps_its_place`
//! / `test_a_red_branch_keeps_its_place` are ported here as pure unit tests;
//! everything else in that file needs the runner or real git and waits for a
//! later stage.

use std::path::Path;
use std::process::Command;

use crate::land::{
    key, merge_outcome, nonce, queued, read_outcome, read_queue_entry, read_stamp,
    write_queue_entry, write_stamp, OutcomePatch, PreflightStamp, QueueEntry, StateDir,
};
use crate::tests::TempDir;

fn a_git_repository() -> TempDir {
    let dir = TempDir::new();
    let run = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(["-c", "init.defaultBranch=main", "init", "--quiet"])
        .output()
        .expect("git on PATH — a test nothing can run is a test that does not exist");
    assert!(run.status.success(), "git init failed");
    dir
}

fn a_queue_entry(branch: &str, place: i64) -> QueueEntry {
    QueueEntry {
        branch: branch.to_string(),
        pr: 7,
        head: "a".repeat(40),
        tree: "b".repeat(40),
        place,
        worktree: "/tmp/somewhere".to_string(),
        nonce: nonce(),
    }
}

#[test]
fn a_fresh_state_dir_gets_all_six_subdirectories() {
    let repo = a_git_repository();
    let state = StateDir::resolve(repo.path()).expect("a state directory");
    for sub in [
        "queue",
        "outcomes",
        "stamps",
        "logs",
        "foundations",
        "checks",
    ] {
        assert!(state.path().join(sub).is_dir(), "missing {sub}");
    }
}

#[test]
fn a_queue_entry_round_trips_exactly() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let entry = a_queue_entry("feature/round-trip", 123);
    write_queue_entry(&state, &entry).expect("written");
    let read = read_queue_entry(&state, "feature/round-trip")
        .expect("read")
        .expect("present");
    assert_eq!(read, entry);
}

#[test]
fn a_stamp_round_trips_exactly() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let stamp = PreflightStamp {
        branch: "feature/stamped".to_string(),
        head: "c".repeat(40),
        tree: "d".repeat(40),
        base: "e".repeat(40),
        pr: 9,
        checks: vec!["build".to_string(), "typecheck".to_string()],
    };
    write_stamp(&state, &stamp).expect("written");
    let read = read_stamp(&state, "feature/stamped")
        .expect("read")
        .expect("present");
    assert_eq!(read, stamp);
}

#[test]
fn a_missing_stamp_is_none_not_an_error() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    assert_eq!(read_stamp(&state, "nothing/known").expect("read"), None);
}

#[test]
fn an_entry_missing_its_place_does_not_stop_the_line() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let good = a_queue_entry("fix/beside-a-bad-entry", 1);
    write_queue_entry(&state, &good).expect("written");
    std::fs::write(
        state.path().join("queue").join("half-written.json"),
        r#"{"branch": "fix/half"}"#,
    )
    .expect("the half-written file");

    let line = queued(&state).expect("queued");
    assert_eq!(line, vec![good]);
}

#[test]
fn queued_is_sorted_by_place_ascending() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let later = a_queue_entry("fix/later", 200);
    let earlier = a_queue_entry("fix/earlier", 100);
    write_queue_entry(&state, &later).expect("written");
    write_queue_entry(&state, &earlier).expect("written");

    let line = queued(&state).expect("queued");
    assert_eq!(line, vec![earlier, later]);
}

#[test]
fn a_resubmit_keeps_the_place_an_earlier_outcome_recorded() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let branch = "fix/two";

    merge_outcome(
        &state,
        branch,
        "conflict",
        "main does not merge in cleanly",
        "10:00:00",
        OutcomePatch {
            place: Some(123),
            conflicts: Some(vec!["shared.txt".to_string()]),
            ..OutcomePatch::default()
        },
    )
    .expect("the first outcome");

    let resubmitted = merge_outcome(
        &state,
        branch,
        "waiting",
        "in line",
        "10:05:00",
        OutcomePatch::default(),
    )
    .expect("the resubmitted outcome");

    assert_eq!(resubmitted.place, Some(123));
    assert_eq!(resubmitted.conflicts, vec!["shared.txt".to_string()]);
    assert_eq!(resubmitted.state, "waiting");
}

#[test]
fn merging_a_new_field_leaves_the_old_ones_in_place() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let branch = "fix/gating";

    merge_outcome(
        &state,
        branch,
        "gating",
        "reading the pull request",
        "09:00:00",
        OutcomePatch {
            logs: Some(vec!["foundations.log".to_string()]),
            failed: Some(vec!["build".to_string()]),
            ..OutcomePatch::default()
        },
    )
    .expect("the first merge");

    let merged = merge_outcome(
        &state,
        branch,
        "gating",
        "running typecheck (typecheck)",
        "09:01:00",
        OutcomePatch {
            already: Some(vec!["typecheck".to_string()]),
            ..OutcomePatch::default()
        },
    )
    .expect("the second merge");

    assert_eq!(merged.logs, vec!["foundations.log".to_string()]);
    assert_eq!(merged.failed, vec!["build".to_string()]);
    assert_eq!(merged.already, vec!["typecheck".to_string()]);

    let read = read_outcome(&state, branch)
        .expect("read")
        .expect("present");
    assert_eq!(read, merged);
}

#[test]
fn key_is_stable_and_filesystem_safe() {
    let branch = "feature/behind/deep";
    let first = key(branch);
    let second = key(branch);
    assert_eq!(first, second);
    assert!(!first.contains('/'));
    assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn keys_for_branches_sharing_a_prefix_differ() {
    assert_ne!(key("feature/a"), key("feature/ab"));
}

#[test]
fn a_state_dir_is_not_a_real_path_check() {
    // Guards the test helper itself: `for_testing` must not silently accept a
    // path it never created subdirectories under.
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    assert_eq!(state.path(), dir.path().join("armada-land"));
    assert!(Path::new(&state.queue_entry_path("x")).starts_with(state.path()));
}
