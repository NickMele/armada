//! What the gaming check flagged survives the process that found it.
//!
//! Its own file rather than more of `roundtrip`, which is already one of the
//! longest in the crate.

use core_model::{GamingFlag, GamingPattern, StepId};

use crate::tests::{created_at, job_id, open, top_level, TempDir};

#[test]
fn the_patterns_a_step_tripped_survive_with_what_each_cited() {
    let dir = TempDir::new();
    let step = StepId::new("fix");
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01GAME"), &created_at())
        .expect("stored");
    store
        .record_step_gaming_flags(
            &job_id("01GAME"),
            &step,
            &[
                GamingFlag {
                    pattern: GamingPattern::TestDeleted,
                    cited: "tests/reader.rs, removed whole".to_string(),
                },
                GamingFlag {
                    pattern: GamingPattern::AssertionWeakened,
                    cited: "src/read.rs:88, `assert!(rows.len() > 0)`".to_string(),
                },
            ],
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened
        .step_gaming_flags(&job_id("01GAME"))
        .expect("loads");
    assert_eq!(read.len(), 1, "one step was flagged");
    assert_eq!(read[0].0, step);
    assert_eq!(
        read[0].1.iter().map(|f| f.pattern).collect::<Vec<_>>(),
        vec![GamingPattern::TestDeleted, GamingPattern::AssertionWeakened],
        "in the order the check answered them"
    );
    assert_eq!(
        read[0].1[0].cited, "tests/reader.rs, removed whole",
        "the citation is the whole value of the flag"
    );
}

/// A second pass replaces the first. Two passes interleaved would read as one
/// pass that found twice as much.
#[test]
fn a_second_look_at_the_same_step_supersedes_the_first() {
    let dir = TempDir::new();
    let step = StepId::new("fix");
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01AGAIN"), &created_at())
        .expect("stored");
    for pattern in [GamingPattern::TestSkipped, GamingPattern::TautologicalTest] {
        store
            .record_step_gaming_flags(
                &job_id("01AGAIN"),
                &step,
                &[GamingFlag {
                    pattern,
                    cited: "src/lib.rs:1".to_string(),
                }],
                &created_at(),
            )
            .expect("recorded");
    }
    drop(store);

    let reopened = open(&dir);
    let read = reopened
        .step_gaming_flags(&job_id("01AGAIN"))
        .expect("loads");
    assert_eq!(read[0].1.len(), 1);
    assert_eq!(read[0].1[0].pattern, GamingPattern::TautologicalTest);
}

/// Nothing flagged is zero rows, not a row saying nothing — the same sentence
/// an unjudged step's empty list makes.
#[test]
fn a_job_nothing_was_flagged_on_carries_no_rows_at_all() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01CLEAN"), &created_at())
        .expect("stored");
    drop(store);

    let reopened = open(&dir);
    assert!(reopened
        .step_gaming_flags(&job_id("01CLEAN"))
        .expect("loads")
        .is_empty());
}
