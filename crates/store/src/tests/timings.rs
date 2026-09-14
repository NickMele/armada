//! How long a repository's Checks take, kept across a reopen and per repository.

use std::time::Duration;

use core_model::{ManifestId, Timestamp, Ulid};

use crate::tests::{open, TempDir};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-14T00:00:{second:02}Z"))
}

#[test]
fn a_repository_nothing_timed_has_no_timings() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert!(store
        .check_timings(&repository("01REPO"))
        .expect("reads")
        .is_empty());
}

/// The recent average, per Check, surviving a reopen and kept apart per
/// repository.
#[test]
fn the_latest_runs_are_averaged_per_check_and_per_repository() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mine = repository("01REPO");
    let theirs = repository("01OTHER");
    for (second, ms) in [(1, 1_000), (2, 3_000)] {
        store
            .record_check_took(&mine, "build", Duration::from_millis(ms), &at(second))
            .expect("kept");
    }
    store
        .record_check_took(&mine, "format", Duration::from_millis(200), &at(3))
        .expect("kept");
    store
        .record_check_took(&theirs, "build", Duration::from_secs(90), &at(4))
        .expect("kept");
    drop(store);

    let store = open(&dir);
    let timed = store.check_timings(&mine).expect("reads");
    assert_eq!(timed.get("build"), Some(&Duration::from_millis(2_000)));
    assert_eq!(timed.get("format"), Some(&Duration::from_millis(200)));
    assert_eq!(
        timed.len(),
        2,
        "another repository's Check leaked in: {timed:?}"
    );
}

/// Only the latest few count, so a Check that got faster reorders soon.
#[test]
fn an_old_run_stops_counting_once_enough_newer_ones_are_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mine = repository("01REPO");
    store
        .record_check_took(&mine, "test", Duration::from_secs(600), &at(0))
        .expect("kept");
    for second in 1..=5 {
        store
            .record_check_took(&mine, "test", Duration::from_secs(10), &at(second))
            .expect("kept");
    }
    assert_eq!(
        store.check_timings(&mine).expect("reads").get("test"),
        Some(&Duration::from_secs(10))
    );
}
