//! A test broken on main, claimed once by the Job fixing it. #999.
//!
//! The last case is the one the table's shape exists for: the reporter is an id
//! and not a key, so forgetting it leaves the fix's claim standing.

use core_model::{Breakage, BreakageClaim, ManifestId, Ulid};

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::Store;

const REPORTER: &str = "01REPORTERAAAAAAAAAAAAAAAA";
const FIX: &str = "01FIXAAAAAAAAAAAAAAAAAAAAA";
const SECOND_FIX: &str = "01SECONDFIXAAAAAAAAAAAAAAA";

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

/// The claim a fix Job would take on one test.
fn claim(fix: &str, on: &ManifestId) -> BreakageClaim {
    BreakageClaim {
        fix: job_id(fix),
        repository: on.clone(),
        breakage: Breakage {
            check: "test".to_string(),
            test: "store::reads_the_last_row".to_string(),
            failure: "exited 101".to_string(),
        },
        reported_by: job_id(REPORTER),
    }
}

/// A store holding the reporter and both fixes, so every claim has a Job row.
fn with_jobs(dir: &TempDir) -> Store {
    let mut store = open(dir);
    for id in [REPORTER, FIX, SECOND_FIX] {
        store
            .insert_job(&top_level(id), &created_at())
            .expect("the job is stored");
    }
    store
}

#[test]
fn a_second_claim_on_the_same_test_takes_nothing() {
    let dir = TempDir::new();
    let mut store = with_jobs(&dir);
    let here = repository("01REPOAAAAAAAAAAAAAAAAAAAA");

    assert!(store
        .claim_breakage(&claim(FIX, &here), &created_at())
        .expect("written"));
    assert!(
        !store
            .claim_breakage(&claim(SECOND_FIX, &here), &created_at())
            .expect("written"),
        "a second report of the same test drafts nothing"
    );
    let standing = store
        .breakage_claimed(&here, "test", "store::reads_the_last_row")
        .expect("read")
        .expect("a claim stands");
    assert_eq!(standing.fix, job_id(FIX), "the first fix keeps it");
}

#[test]
fn a_claim_reads_from_the_reporter_as_well_as_the_fix() {
    let dir = TempDir::new();
    let mut store = with_jobs(&dir);
    let here = repository("01REPOAAAAAAAAAAAAAAAAAAAA");
    store
        .claim_breakage(&claim(FIX, &here), &created_at())
        .expect("written");

    let reported = store
        .breakages_reported_by(&job_id(REPORTER))
        .expect("read");
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].fix, job_id(FIX));
    assert!(store
        .breakages_reported_by(&job_id(SECOND_FIX))
        .expect("read")
        .is_empty());
}

#[test]
fn the_same_test_in_another_repository_is_its_own_breakage() {
    let dir = TempDir::new();
    let mut store = with_jobs(&dir);

    assert!(store
        .claim_breakage(
            &claim(FIX, &repository("01ONEAAAAAAAAAAAAAAAAAAAAA")),
            &created_at()
        )
        .expect("written"));
    assert!(store
        .claim_breakage(
            &claim(SECOND_FIX, &repository("01TWOAAAAAAAAAAAAAAAAAAAAA")),
            &created_at()
        )
        .expect("written"));
}

#[test]
fn a_fix_that_ends_gives_its_claim_back() {
    let dir = TempDir::new();
    let mut store = with_jobs(&dir);
    let here = repository("01REPOAAAAAAAAAAAAAAAAAAAA");
    store
        .claim_breakage(&claim(FIX, &here), &created_at())
        .expect("written");

    store.release_breakages(&job_id(FIX)).expect("released");

    assert!(store
        .breakage_claimed(&here, "test", "store::reads_the_last_row")
        .expect("read")
        .is_none());
    assert!(
        store
            .claim_breakage(&claim(SECOND_FIX, &here), &created_at())
            .expect("written"),
        "a test broken again after its fix ended can be claimed again"
    );
}

#[test]
fn forgetting_the_reporter_leaves_the_claim_and_forgetting_the_fix_removes_it() {
    let dir = TempDir::new();
    let mut store = with_jobs(&dir);
    let here = repository("01REPOAAAAAAAAAAAAAAAAAAAA");
    store
        .claim_breakage(&claim(FIX, &here), &created_at())
        .expect("written");

    store
        .forget_job(&job_id(REPORTER))
        .expect("the reporter is forgotten while its report's fix still runs");
    assert_eq!(
        store
            .breakages_claimed_by(&job_id(FIX))
            .expect("read")
            .len(),
        1,
        "the fix still holds its claim"
    );

    let removed = store
        .forget_job(&job_id(FIX))
        .expect("the fix is forgotten");
    assert_eq!(
        removed.breakage_claims, 1,
        "and the claim is counted by name"
    );
    assert!(store
        .breakages_claimed_by(&job_id(FIX))
        .expect("read")
        .is_empty());
}
