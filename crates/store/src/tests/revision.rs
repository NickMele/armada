//! A scope history that grows after the Job was inserted.
//!
//! Its own file rather than more of `roundtrip`, for `plan`'s reason: what is
//! under test is not a round-trip of a value written once. It is that the
//! history and the scope move **together** — a stored history saying a widening
//! took, beside a `job_write_targets` that did not take it, is the disagreement
//! this whole path exists to make impossible.

use core_model::{Actor, RepoPath, ScopeRevision, ScopeRevisionOutcome, StepId, WriteTargets};

use crate::tests::{at, job_id, open, top_level, TempDir};

fn asking(outcome: ScopeRevisionOutcome) -> ScopeRevision {
    ScopeRevision {
        at_step: Some(StepId::new("fix")),
        paths_added: vec![RepoPath::new("crates/store/src/schema.rs")],
        paths_removed: Vec::new(),
        atomic_before: false,
        atomic_after: false,
        rationale: "the column the fix needs is declared there".to_string(),
        outcome,
        approved_by: Actor::Fleet,
        at: at("2026-09-03T10:00:00.000Z"),
    }
}

fn scope(store: &crate::Store, id: &str) -> Vec<String> {
    store
        .load_job(&job_id(id))
        .expect("the job loads")
        .write_targets()
        .map(|targets| {
            WriteTargets::paths(targets)
                .iter()
                .map(|path| path.as_str().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// **The capability.** A Judge cleared a widening mid-step, and what the Job
/// carries afterwards says so — in the history and in the scope, from a
/// reopened database rather than from the value that was written.
#[test]
fn a_cleared_widening_reaches_the_history_and_the_scope_together() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01WIDEN");
    store
        .insert_job(&job, &at("2026-09-03T09:00:00.000Z"))
        .expect("the job is inserted");
    let widened = job.scope_revised(asking(ScopeRevisionOutcome::took()));
    store
        .record_scope_revision(&widened)
        .expect("the revision is recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.load_job(&job_id("01WIDEN")).expect("it loads");
    let latest = read.scope_revisions().last().expect("an entry");
    assert_eq!(latest.outcome.as_str(), ScopeRevisionOutcome::TOOK);
    assert_eq!(latest.approved_by, Actor::Fleet);
    assert_eq!(
        latest.rationale,
        "the column the fix needs is declared there"
    );
    assert!(scope(&reopened, "01WIDEN").contains(&"crates/store/src/schema.rs".to_string()));
}

/// A refused request is on the record and moved nothing. *Was this ever asked*
/// is a question people ask later, and a history that dropped the refusals
/// could not answer it.
#[test]
fn a_refused_widening_is_on_the_record_and_moves_no_scope() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01REFUSED");
    store
        .insert_job(&job, &at("2026-09-03T09:00:00.000Z"))
        .expect("the job is inserted");
    let before = scope(&store, "01REFUSED");
    let asked = job.scope_revised(asking(ScopeRevisionOutcome::not_taken()));
    store
        .record_scope_revision(&asked)
        .expect("the revision is recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.load_job(&job_id("01REFUSED")).expect("it loads");
    assert_eq!(
        read.scope_revisions()
            .last()
            .map(|entry| entry.outcome.as_str()),
        Some(ScopeRevisionOutcome::NOT_TAKEN)
    );
    assert_eq!(scope(&reopened, "01REFUSED"), before);
}

/// The entry names a Job that is not there. **Refused rather than inserted**:
/// creation is not an update in either direction, and a scope history filed
/// against nothing is a history nothing will ever read.
#[test]
fn a_revision_against_a_job_that_is_not_there_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01ABSENT").scope_revised(asking(ScopeRevisionOutcome::took()));
    assert!(matches!(
        store.record_scope_revision(&job),
        Err(crate::WriteError::NoSuchJob { .. })
    ));
}
