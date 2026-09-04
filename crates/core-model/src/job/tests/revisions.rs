//! The append-only scope history, and what an entry does to the Job carrying
//! it.
//!
//! Its own file rather than more of [`record`](super::record): what is under
//! test is not what a Job is made of at creation, it is the one field that
//! grows afterwards — and the three cases that matter are the three answers a
//! `bool` does not have, which is `write_targets` being null, empty, or moved.

use super::*;

fn revision(outcome: ScopeRevisionOutcome, added: &[&str]) -> ScopeRevision {
    ScopeRevision {
        at_step: Some(StepId::new("fix")),
        paths_added: added.iter().map(|path| RepoPath::new(*path)).collect(),
        paths_removed: Vec::new(),
        atomic_before: false,
        atomic_after: false,
        rationale: "the column the fix needs is declared there".into(),
        outcome,
        approved_by: Actor::Fleet,
        at: at("2026-09-03T10:00:00.000Z"),
    }
}

fn scoped(targets: Option<WriteTargets>) -> Job {
    let mut draft = draft();
    draft.write_targets = targets;
    Job::create_top_level(
        draft,
        TopLevelOrigin::Manual,
        at("2026-08-26T09:00:00.000Z"),
    )
}

#[test]
fn a_revision_that_took_is_in_the_scope_as_well_as_in_the_history() {
    let job = scoped(Some(WriteTargets::of(vec![RepoPath::new("crates/fleet")])));
    let revised = job.scope_revised(revision(
        ScopeRevisionOutcome::took(),
        &["crates/store/src/schema.rs"],
    ));
    assert_eq!(revised.scope_revisions().len(), 1);
    assert_eq!(
        revised.write_targets().map(WriteTargets::paths),
        Some(
            [
                RepoPath::new("crates/fleet"),
                RepoPath::new("crates/store/src/schema.rs"),
            ]
            .as_slice()
        )
    );
}

/// *Was this ever asked* is a question people ask later, so the entry is
/// written whatever it answered — and only the one that took moves the scope.
#[test]
fn a_revision_that_did_not_take_is_recorded_and_moves_nothing() {
    let job = scoped(Some(WriteTargets::of(vec![RepoPath::new("crates/fleet")])));
    let revised = job.scope_revised(revision(
        ScopeRevisionOutcome::not_taken(),
        &["crates/store/src/schema.rs"],
    ));
    assert_eq!(revised.scope_revisions().len(), 1);
    assert_eq!(
        revised.write_targets().map(WriteTargets::paths),
        Some([RepoPath::new("crates/fleet")].as_slice())
    );
}

/// Null is not empty. Nothing is outside a scope nobody has stated, so there is
/// nothing for an addition to be an addition to — and a revision that invented
/// one would make a Job whose scope was undetermined read as one whose scope is
/// exactly what a Drone asked for.
#[test]
fn a_revision_determines_no_scope_on_a_job_that_has_none() {
    let job = scoped(None);
    let revised = job.scope_revised(revision(
        ScopeRevisionOutcome::took(),
        &["crates/store/src/schema.rs"],
    ));
    assert!(revised.write_targets().is_none());
    assert_eq!(revised.scope_revisions().len(), 1);
}

#[test]
fn a_path_already_in_scope_is_not_added_twice() {
    let job = scoped(Some(WriteTargets::of(vec![RepoPath::new("crates/fleet")])));
    let revised = job.scope_revised(revision(ScopeRevisionOutcome::took(), &["crates/fleet"]));
    assert_eq!(
        revised.write_targets().map(WriteTargets::paths),
        Some([RepoPath::new("crates/fleet")].as_slice())
    );
}
