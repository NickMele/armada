//! What a Job touched survives the worktree it touched it in.
//!
//! Its own file rather than more of `roundtrip`, which is already one of the
//! longest in the crate — and because the distinction under test is not a
//! round-trip at all. It is **absent against empty**: a Job nothing recorded
//! and a Job recorded as having changed nothing are two answers, and the pair
//! of tables exists only to keep them apart.

use adapter_traits::{Change, Changed, ChangedFile};

use crate::tests::{created_at, job_id, open, top_level, TempDir};

fn changed(files: &[(&str, Change)]) -> Changed {
    Changed::of(
        files
            .iter()
            .map(|(path, change)| ChangedFile::new(path.to_string(), *change))
            .collect(),
    )
}

#[test]
fn the_files_a_job_touched_survive_the_process_that_read_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01FOOT"), &created_at())
        .expect("stored");
    store
        .record_footprint(
            &job_id("01FOOT"),
            &changed(&[
                ("src/read.rs", Change::Modified),
                ("src/tokens.rs", Change::Added),
                ("src/legacy.rs", Change::Deleted),
            ]),
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened
        .footprint(&job_id("01FOOT"))
        .expect("loads")
        .expect("a footprint was recorded");
    assert_eq!(
        read.files
            .iter()
            .map(|file| (file.path(), file.change()))
            .collect::<Vec<(&str, Change)>>(),
        vec![
            ("src/read.rs", Change::Modified),
            ("src/tokens.rs", Change::Added),
            ("src/legacy.rs", Change::Deleted),
        ],
        "in the order the reading found them, with what happened to each"
    );
    assert_eq!(read.recorded_at, created_at());
}

/// **The distinction the header table is for.** A Job with no record answers
/// `None`; a Job recorded as having touched nothing answers an empty list. One
/// table could only have said the first.
#[test]
fn a_job_that_touched_nothing_is_not_a_job_that_recorded_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01NONE"), &created_at())
        .expect("stored");

    assert!(
        store.footprint(&job_id("01NONE")).expect("loads").is_none(),
        "nothing was recorded"
    );

    store
        .record_footprint(&job_id("01NONE"), &changed(&[]), &created_at())
        .expect("recorded");

    let read = store
        .footprint(&job_id("01NONE"))
        .expect("loads")
        .expect("a reading was taken");
    assert!(
        read.files.is_empty(),
        "the worktree opened and held no change, which is a real answer"
    );
}

/// A second write replaces the first whole. A Job reaches a terminal status
/// once, so two writes mean the first was wrong — and rows left behind from it
/// would read as files the Job touched twice.
#[test]
fn a_second_reading_replaces_the_first_rather_than_joining_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01TWICE"), &created_at())
        .expect("stored");
    store
        .record_footprint(
            &job_id("01TWICE"),
            &changed(&[
                ("src/read.rs", Change::Modified),
                ("src/gone.rs", Change::Added),
            ]),
            &created_at(),
        )
        .expect("recorded");
    store
        .record_footprint(
            &job_id("01TWICE"),
            &changed(&[("src/read.rs", Change::Modified)]),
            &created_at(),
        )
        .expect("recorded again");

    let read = store
        .footprint(&job_id("01TWICE"))
        .expect("loads")
        .expect("a footprint");
    assert_eq!(
        read.files
            .iter()
            .map(|file| file.path())
            .collect::<Vec<&str>>(),
        vec!["src/read.rs"],
        "the second reading, whole, with nothing of the first left under it"
    );
}
