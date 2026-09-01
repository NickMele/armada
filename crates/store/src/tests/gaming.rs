//! What the gaming check flagged survives the process that found it.
//!
//! Its own file rather than more of `roundtrip`, which is already one of the
//! longest in the crate.

use core_model::{CitedAt, GamingFlag, GamingPattern, RepoPath, StepId};

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
                    at: Some(CitedAt::in_file(RepoPath::new("tests/reader.rs"))),
                },
                GamingFlag {
                    pattern: GamingPattern::AssertionWeakened,
                    cited: "src/read.rs:88, `assert!(rows.len() > 0)`".to_string(),
                    at: Some(CitedAt::at_line(RepoPath::new("src/read.rs"), 88)),
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
    assert_eq!(
        read[0].1.iter().map(|f| f.at.clone()).collect::<Vec<_>>(),
        vec![
            Some(CitedAt::in_file(RepoPath::new("tests/reader.rs"))),
            Some(CitedAt::at_line(RepoPath::new("src/read.rs"), 88)),
        ],
        "where each flag points survives, line and all"
    );
}

/// A flag with nowhere to point round-trips as one, rather than as a file
/// named the empty string.
///
/// **The case that made the column nullable**, and the one a `NOT NULL` with a
/// default would have quietly turned into a location. `no_findings_on_\
/// substantial_diff` is a finding about an absence and can never have one.
#[test]
fn a_flag_with_nowhere_to_point_reads_back_with_nowhere_to_point() {
    let dir = TempDir::new();
    let step = StepId::new("review");
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01NOWHERE"), &created_at())
        .expect("stored");
    store
        .record_step_gaming_flags(
            &job_id("01NOWHERE"),
            &step,
            &[GamingFlag {
                pattern: GamingPattern::NoFindingsOnSubstantialDiff,
                cited: "REVIEW.md reports nothing against 94 changed lines".to_string(),
                at: None,
            }],
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let read = open(&dir)
        .step_gaming_flags(&job_id("01NOWHERE"))
        .expect("loads");
    assert_eq!(read[0].1[0].at, None);
}

/// A second pass **over the same run** replaces the first. Two passes
/// interleaved would read as one pass that found twice as much.
///
/// The step does not move between the two writes here, so both are the same
/// run. A second run of the step keeps both sets — `tests::attempt`.
#[test]
fn a_second_look_at_the_same_run_of_a_step_supersedes_the_first() {
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
                    at: Some(CitedAt::at_line(RepoPath::new("src/lib.rs"), 1)),
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
