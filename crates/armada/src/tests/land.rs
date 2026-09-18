//! Stages 1 and 2 of the `scripts/land` port: the state this line keeps on
//! disk, and the pure comparisons its gate makes — both independent of any
//! runner or subprocess. `scripts/test_land.py`'s
//! `test_an_entry_missing_its_place_does_not_stop_the_line` and the
//! place-survives-a-resubmit half of `test_a_conflict_stops_and_keeps_its_place`
//! / `test_a_red_branch_keeps_its_place` are Stage 1's; `test_only_a_new_
//! failing_foundations_line_is_red`, `test_a_renumbered_finding_is_not_a_
//! new_one`, `test_a_second_instance_of_a_known_finding_is_new`,
//! `test_a_foundations_run_that_names_no_rule_is_red` and the missing-tool
//! detection in `test_a_check_whose_command_is_missing_stops_rather_than_reds`
//! are Stage 2's, ported as direct unit tests on the pure functions rather
//! than as subprocess-driven scenarios — the fixture strings are theirs,
//! verbatim. Everything else in that file needs the runner or real git and
//! waits for a later stage.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::land::{
    a_report, already_red_on_base, failing_lines, foundations_delta, key, merge_outcome, nonce,
    not_installed, queued, read_outcome, read_queue_entry, read_stamp, write_queue_entry,
    write_stamp, FoundationsComparison, OutcomePatch, OutcomeState, PreflightStamp, QueueEntry,
    StateDir,
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
        OutcomeState::Conflict,
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
        OutcomeState::Waiting,
        "in line",
        "10:05:00",
        OutcomePatch::default(),
    )
    .expect("the resubmitted outcome");

    assert_eq!(resubmitted.place, Some(123));
    assert_eq!(resubmitted.conflicts, vec!["shared.txt".to_string()]);
    assert_eq!(resubmitted.state, OutcomeState::Waiting);
}

#[test]
fn merging_a_new_field_leaves_the_old_ones_in_place() {
    let dir = TempDir::new();
    let state = StateDir::for_testing(dir.path().join("armada-land"));
    let branch = "fix/gating";

    merge_outcome(
        &state,
        branch,
        OutcomeState::Gating,
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
        OutcomeState::Gating,
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

// -------------------------------------------------------- Stage 2: the gate

#[test]
fn every_outcome_state_maps_to_the_exit_code_running_locally_names() {
    let table = [
        (OutcomeState::Landed, 0),
        (OutcomeState::Waiting, 3),
        (OutcomeState::Gating, 3),
        (OutcomeState::Merging, 3),
        (OutcomeState::Red, 4),
        (OutcomeState::Conflict, 5),
        (OutcomeState::Ungated, 6),
        (OutcomeState::Stopped, 7),
    ];
    for (state, code) in table {
        assert_eq!(state.exit_code(), code, "{state:?}");
    }
}

#[test]
fn only_a_new_failing_foundations_line_is_red() {
    let known = "FAIL  a rule main already fails\n        missing: its subject\n";
    let warned = format!(
        "{known}        warn:    a new warning\n\nverify-foundations: RED — 1 failing, 1 warning\n"
    );
    let worse = format!(
        "FAIL  a new rule\n        missing: a new subject\n{known}\nverify-foundations: RED — 1 failing, 0 warning\n"
    );

    assert_eq!(
        foundations_delta(known, &warned, 1),
        FoundationsComparison::New(Vec::new()),
        "a new warning is not a new failure"
    );

    let FoundationsComparison::New(new) = foundations_delta(known, &worse, 1) else {
        panic!("a report, not a crash");
    };
    assert_eq!(
        new,
        vec![
            "FAIL  a new rule".to_string(),
            "missing: a new subject".to_string(),
        ]
    );
}

#[test]
fn a_renumbered_finding_is_not_a_new_one() {
    let known = "FAIL  a rule main already fails\n        missing: a/b.rs:10 — over 500\n";
    let shifted = known.replace(":10", ":12");
    assert_eq!(
        foundations_delta(known, &shifted, 1),
        FoundationsComparison::New(Vec::new())
    );
}

#[test]
fn a_second_instance_of_a_known_finding_is_new() {
    // The fixture's own vendor name in `scripts/test_land.py` is renamed
    // here — this file sits outside `crates/adapters`, where naming a real
    // one would trip `no vendor literal outside adapters` for real.
    let known =
        "FAIL  no vendor literal outside adapters\n        missing: crates/a.rs:10 — `acme`\n";
    let second = format!("{known}        missing: crates/a.rs:80 — `acme`\n");

    let FoundationsComparison::New(new) = foundations_delta(known, &second, 1) else {
        panic!("a report, not a crash");
    };
    assert_eq!(new, vec!["missing: crates/a.rs:80 — `acme`".to_string()]);
}

#[test]
fn a_foundations_run_that_names_no_rule_is_red() {
    let base = "FAIL  a rule main already fails\n        missing: its subject\n";
    let broken = "error[E0433]: cannot find `covers`\n";

    assert!(!a_report(broken, 101));
    assert_eq!(
        foundations_delta(base, broken, 101),
        FoundationsComparison::Crashed(vec!["error[E0433]: cannot find `covers`".to_string()])
    );
}

#[test]
fn an_exit_zero_is_a_report_even_with_no_failing_line() {
    assert!(a_report("verify-foundations: green\n", 0));
}

#[test]
fn failing_lines_keeps_only_fail_and_missing_lines() {
    let text = "  FAIL  a rule\n    missing: a subject\nwarn:    unrelated\nnote\n";
    assert_eq!(
        failing_lines(text),
        vec!["FAIL  a rule".to_string(), "missing: a subject".to_string()]
    );
}

#[test]
fn a_check_whose_command_is_missing_stops_rather_than_reds() {
    let text = "error: no such command: nextest\n";
    assert_eq!(not_installed(text), Some("nextest".to_string()));
}

#[test]
fn not_installed_reads_checks_runners_own_wording_too() {
    let text = "needs `nextest`, which is not on this machine's PATH\n";
    assert_eq!(not_installed(text), Some("nextest".to_string()));
}

#[test]
fn not_installed_is_none_for_an_ordinary_failure() {
    assert_eq!(not_installed("FAIL  a rule\n        missing: x\n"), None);
}

#[test]
fn already_red_on_base_keeps_only_the_ones_known_to_fail() {
    let names = vec![
        "build".to_string(),
        "ui".to_string(),
        "typecheck".to_string(),
    ];
    let known = BTreeMap::from([
        ("build".to_string(), false),
        ("ui".to_string(), true),
        // "typecheck" left out: not yet run on the base.
    ]);
    assert_eq!(
        already_red_on_base(&names, &known),
        vec!["build".to_string()]
    );
}
