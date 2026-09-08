//! What the Checks said about a commit, and the two things about it that are
//! not true of any other per-Check record here.
//!
//! **It is keyed by the commit and by nothing else**, which is the whole of
//! `#474`: two Jobs merging within a minute are two merges into one commit, and
//! a record keyed by whoever noticed first leaves the second Job re-running the
//! suite or reading somebody else's answer as its own.
//!
//! **It survives forgetting the Job that noticed.** `forget_job` deletes from
//! every table pointing at `jobs`; this one does not point at `jobs`, and the
//! test at the foot of this file is what stops that becoming a foreign key
//! somebody adds for symmetry.

use core_model::{CheckOutcome, StepCheck};

use crate::schema::tables_pointing_at_a_job;
use crate::tests::{at, job_id, open, top_level, TempDir};
use crate::Proved;

const MERGED_INTO: &str = "9b2c1f4e5a7d3c8b0e6f2a4d9c1b7e3f5a8d0c2b";

fn passed(name: &str) -> StepCheck {
    StepCheck {
        name: name.to_string(),
        outcome: CheckOutcome::Passed,
        expected: None,
        produced: None,
        output_path: None,
    }
}

fn failed(name: &str) -> StepCheck {
    StepCheck {
        name: name.to_string(),
        outcome: CheckOutcome::Failed,
        expected: Some(String::from("exit 0")),
        produced: Some(String::from("exited 101")),
        output_path: Some(format!(".armada/checks/commits/{MERGED_INTO}/1.log")),
    }
}

fn a_run(at_commit: &str) -> Proved {
    Proved {
        at_commit: at_commit.to_string(),
        base: String::from("main"),
        at: at("2026-09-07T09:00:00.000Z"),
        checks: vec![passed("build"), failed("test")],
    }
}

#[test]
fn what_the_checks_said_is_read_back_by_the_commit() {
    // Every field, including the output path — a person reading a red wants the
    // log, and a row that lost the path sends them back to run it themselves.
    let dir = TempDir::new();
    let mut store = open(&dir);
    let run = a_run(MERGED_INTO);
    store
        .record_commit_checks(&run)
        .expect("the run is recorded");
    assert_eq!(store.proved(MERGED_INTO).expect("read back"), Some(run));
}

#[test]
fn a_commit_nobody_proved_reads_as_absent_rather_than_as_a_pass() {
    // **Absent is *nobody ran anything*.** A repository that names no
    // `after_merge` Checks proves nothing, and so does a merge Fleet declined to
    // fast-forward onto — neither is an all-clear.
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.proved(MERGED_INTO).expect("read"), None);
    assert!(!store.already_proved(MERGED_INTO).expect("asked"));
}

#[test]
fn the_second_job_merging_into_one_commit_finds_it_already_proved() {
    // The defect the key exists against, at the level the guard lives: the
    // question is about the commit, so the second Job's answer costs one read
    // rather than a second run of the whole suite.
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .record_commit_checks(&a_run(MERGED_INTO))
        .expect("the run is recorded");
    assert!(store.already_proved(MERGED_INTO).expect("asked"));
    assert!(!store
        .already_proved("0000000000000000000000000000000000000000")
        .expect("asked about another commit"));
}

#[test]
fn a_run_with_no_checks_writes_nothing_at_all() {
    // So the absence of a row stays the absence of a run. A repository whose
    // `after_merge` list is empty never gets here, and if it did, an empty
    // record would read to the next reader as a commit that had been proved.
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .record_commit_checks(&Proved {
            checks: Vec::new(),
            ..a_run(MERGED_INTO)
        })
        .expect("nothing to record");
    assert!(!store.already_proved(MERGED_INTO).expect("asked"));
}

#[test]
fn what_a_commit_was_proved_to_say_outlives_the_job_that_noticed_the_merge() {
    // **The reason there is no foreign key, asserted rather than commented.**
    // Two Jobs merge into one commit; forgetting one of them must not delete
    // the answer the other one is also pointing at.
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01PROVING0000000000000001");
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
    store
        .record_commit_checks(&a_run(MERGED_INTO))
        .expect("the run is recorded");
    store
        .forget_job(&job_id("01PROVING0000000000000001"))
        .expect("the job is forgotten");
    assert!(
        store.already_proved(MERGED_INTO).expect("asked").to_owned(),
        "the run belongs to the commit, and the commit is still there"
    );
}

#[test]
fn the_table_does_not_point_at_a_job() {
    // The same question `forget.rs` asks of the schema, asked from the other
    // side: a foreign key added here for symmetry would put this table into
    // `forget_job`'s sweep, and the test above would then be the one that
    // catches it — after somebody had already written the migration.
    let dir = TempDir::new();
    let store = open(&dir);
    let pointing = tables_pointing_at_a_job(&store.conn).expect("the schema is readable");
    assert!(
        !pointing.iter().any(|name| name == "commit_checks"),
        "commit_checks is keyed by a commit, and was found among {pointing:?}"
    );
}
