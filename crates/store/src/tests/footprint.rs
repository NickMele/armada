//! What a Job touched survives the worktree it touched it in.
//!
//! Its own file rather than more of `roundtrip`, which is already one of the
//! longest in the crate — and because the distinction under test is not a
//! round-trip at all. It is **absent against empty**: a Job nothing recorded
//! and a Job recorded as having changed nothing are two answers, and the pair
//! of tables exists only to keep them apart.

use adapter_traits::{Change, ChangedFile, Counted, CountedFile, LineCount};

use crate::tests::{created_at, job_id, open, top_level, TempDir};

fn changed(files: &[(&str, Change)]) -> Counted {
    counted(
        &files
            .iter()
            .map(|(path, change)| (*path, *change, None))
            .collect::<Vec<(&str, Change, Option<(u32, u32)>)>>(),
    )
}

fn counted(files: &[(&str, Change, Option<(u32, u32)>)]) -> Counted {
    Counted::of(
        files
            .iter()
            .map(|(path, change, lines)| {
                CountedFile::new(
                    ChangedFile::new(path.to_string(), *change),
                    lines.map(|(added, deleted)| LineCount::of(added, deleted)),
                )
            })
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

/// **The counts survive the process too, and absent survives as absent.** A
/// file nothing could count comes back with no numbers, and a file that gained
/// and lost nothing comes back as zero — the pair the nullable columns exist to
/// keep apart, read back rather than assumed.
#[test]
fn what_each_file_gained_and_lost_survives_and_uncounted_stays_uncounted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01LINES"), &created_at())
        .expect("stored");
    store
        .record_footprint(
            &job_id("01LINES"),
            &counted(&[
                ("src/read.rs", Change::Modified, Some((61, 4))),
                ("assets/logo.png", Change::Added, None),
                ("src/moved.rs", Change::Renamed, Some((0, 0))),
            ]),
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened
        .footprint(&job_id("01LINES"))
        .expect("loads")
        .expect("a footprint was recorded");
    assert_eq!(
        read.files
            .iter()
            .map(|file| (file.path(), file.lines()))
            .collect::<Vec<(&str, Option<LineCount>)>>(),
        vec![
            ("src/read.rs", Some(LineCount::of(61, 4))),
            ("assets/logo.png", None),
            ("src/moved.rs", Some(LineCount::of(0, 0))),
        ],
        "a binary nobody counted is absent, and a move that edited nothing is zero"
    );
}

/// **A footprint written before the counts existed is not a footprint of
/// nothing.** [`crate::footprint::V25`] backfills no numbers, so every file a
/// migrated file already held reads as uncounted — which is the sentence the
/// nullable column was chosen to say.
#[test]
fn a_footprint_recorded_before_the_counts_existed_reads_as_uncounted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01OLD"), &created_at())
        .expect("stored");
    store
        .record_footprint(
            &job_id("01OLD"),
            &changed(&[("src/read.rs", Change::Modified)]),
            &created_at(),
        )
        .expect("recorded");

    let read = store
        .footprint(&job_id("01OLD"))
        .expect("loads")
        .expect("a footprint");
    assert_eq!(read.files[0].lines(), None);
}
