//! A finished Job's work reaches its branch, and nothing else's does.
//!
//! The Job driven here is the two-step workflow `daemon` uses, against the same
//! fakes. What is scripted is version control: whether a commit is made,
//! answers nothing, or is refused. The real thing git does with an index is
//! asserted in `adapters`, against a repository.

use core_model::{JobStatus, Timestamp};
use testkit::{FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::gate::Ruling;
use crate::tests::daemon::{
    a_fleet_committing_through, a_proposal, diff_evidence, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;

/// The window the fixture clock answers within. A commit stamped outside it
/// came from the machine rather than from the injected clock, which is the
/// thing worth asserting — the exact tick is an implementation detail.
const NINE: &str = "2026-08-26T09:00:00.000Z";
const TEN_MINUTES: i64 = 600;

/// A Job that ran every step and passed every Check has its work committed.
#[tokio::test]
async fn the_work_is_committed_when_the_last_step_advances() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new(),
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submitted_by_the_one(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    assert!(
        fleet.vcs().committed().is_empty(),
        "a step that is not the last commits nothing"
    );

    fleet.submitted_by_the_one(note_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Finished { .. })));

    let made = fleet.vcs().committed();
    assert_eq!(made.len(), 1, "one Job is one commit");
    let commit = &made[0];
    assert_eq!(
        commit.branch,
        format!("armada/{}", job.id().as_str()),
        "the Job's own branch, not the repository's"
    );

    let mut lines = commit.message.lines();
    assert_eq!(
        lines.next(),
        Some("fix the off-by-one in the log reader"),
        "the subject is the Job's title, which is the one line a person wrote"
    );
    assert_eq!(lines.next(), Some(""), "a blank line before the body");
    assert!(
        commit.message.contains(job.id().as_str()),
        "the commit joins back to the record"
    );
    assert!(
        !commit.message.contains("The reader stops one line later"),
        "nothing the Drone claimed is pasted — its words are what the gate ruled on"
    );

    let nine = Timestamp::from_rfc3339(NINE).epoch_millis().unwrap() / 1_000;
    assert!(
        (nine..nine + TEN_MINUTES).contains(&commit.at.seconds()),
        "the commit is stamped from the injected clock, not from the machine"
    );
}

/// A Job that legitimately wrote no file gets no commit, and no empty one.
#[tokio::test]
async fn a_job_that_changed_nothing_is_answered_rather_than_committed() {
    let home = TempDir::new();
    // The diff is non-empty for the gate — the step's own check has to pass for
    // the Job to reach its last step at all — and version control finds the
    // branch already holding everything, which is the `facts_note` shape.
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().with_nothing_to_commit(),
    );

    let job = fleet
        .propose(a_proposal("write down the cause"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submitted_by_the_one(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submitted_by_the_one(note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    assert!(
        fleet.vcs().committed().is_empty(),
        "an empty commit records nothing and would still land on the branch"
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "changing no file is not a failure"
    );
}

/// A Job that fails a Check leaves no commit behind.
#[tokio::test]
async fn a_job_that_fails_mid_workflow_gets_no_commit() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(&home, FakeWorkProduct::untouched(), FakeVcs::new());

    let job = fleet.propose(a_proposal("change nothing")).await.unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submitted_by_the_one(diff_evidence()).await.unwrap();
    let turned = fleet.turn().await.unwrap();
    assert!(matches!(turned.ruled(), Some(Ruling::Failed { .. })));

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedFailed
    );
    assert!(
        fleet.vcs().committed().is_empty(),
        "uncommitted is what makes a failed Job's branch unmistakably not mergeable"
    );
}

/// A commit that git refuses does not lose the work, does not wedge the slot,
/// and does not pass unsaid.
#[tokio::test]
async fn a_refused_commit_still_completes_the_job_and_says_so() {
    let home = TempDir::new();
    let fleet = a_fleet_committing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeVcs::new().refusing_to_commit("a read-only object database"),
    );

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();

    fleet.submitted_by_the_one(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    fleet.submitted_by_the_one(note_evidence()).await.unwrap();

    let adrift = fleet.turn().await.expect_err("the commit was refused");
    let Adrift::NotCommitted { job: named, .. } = &adrift else {
        panic!("the failure names the commit rather than something nearer: {adrift:?}");
    };
    assert_eq!(named, job.id());

    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "the Checks passed, which is a fact about the work and not about git"
    );
    assert_eq!(
        fleet.working_on().await,
        Vec::new(),
        "the slot came free — wedging Fleet would cost every later Job"
    );
}
