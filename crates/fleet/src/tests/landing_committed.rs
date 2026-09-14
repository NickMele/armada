//! A branch that already holds its work goes out, whoever committed it.
//!
//! Beside `landing`, whose Jobs leave their work uncommitted for Fleet. What is
//! scripted here is the other shape: a clean worktree over a branch with commits
//! its base has not got, which is what a Drone running `git commit` leaves.

use api::Queries;
use core_model::Job;
use testkit::{Delivered, Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet_committing_through, a_proposal, diff_evidence, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Scripted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// Carry a Job through both steps, into and past the delivering one.
async fn finished_through(home: &TempDir, vcs: FakeVcs) -> (Scripted, Job) {
    let fleet = a_fleet_committing_through(home, FakeWorkProduct::changed(&["src/log.rs"]), vcs);
    let job = fleet
        .propose(a_proposal("rename the log reader"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    (fleet, job)
}

/// Whether the branch was pushed, and whether a pull request was opened.
fn went_out(vcs: &FakeVcs) -> (bool, bool) {
    let did = vcs.delivered();
    (
        did.iter().any(|it| matches!(it, Delivered::Pushed { .. })),
        did.iter()
            .any(|it| matches!(it, Delivered::OpenedForReview { .. })),
    )
}

fn log_of(home: &TempDir, job: &Job) -> Vec<String> {
    let root = home.path().to_string_lossy().into_owned();
    crate::journal::read_from(&root, &job.handle(), 0)
        .notes
        .into_iter()
        .map(|note| note.msg)
        .collect()
}

/// **The measured defect.** A `refactor` Job finished `completed_success` with
/// two commits on its branch and nothing pushed, because Fleet's own commit
/// found the tree clean and that was read as nothing to send.
#[tokio::test]
async fn a_branch_the_drone_committed_is_pushed_and_opened_for_review() {
    let home = TempDir::new();
    let (fleet, job) = finished_through(
        &home,
        FakeVcs::new()
            .with_nothing_to_commit()
            .with_commits_on_the_branch(2),
    )
    .await;

    assert!(
        fleet.vcs().committed().is_empty(),
        "Fleet made no commit: the Drone's are the work"
    );
    assert_eq!(
        went_out(fleet.vcs()),
        (true, true),
        "the branch holds work its base has not got, so it is pushed and opened: {:?}",
        fleet.vcs().delivered()
    );
    let said = log_of(&home, &job);
    assert!(
        !said.iter().any(|msg| msg.contains("held nothing new")),
        "and the log does not say nothing went out: {said:?}"
    );

    let delivery = fleet
        .get_job(ipc::JobId::from(job.id()))
        .await
        .expect("the Job is served")
        .delivery
        .expect("a delivery was recorded");
    assert_eq!(delivery.pushed.as_deref(), Some("origin/armada/a-job"));
    assert_eq!(
        delivery.pull_request.as_deref(),
        Some("https://forge.invalid/armada/pull/1")
    );
    assert_eq!(
        delivery.commit, None,
        "the field is the commit Fleet wrote, and Fleet wrote none"
    );
}

/// **A repository with no base has nothing to measure against, so the branch
/// goes out** — as it does where Fleet made the commit — and no pull request
/// opens, because there is nothing to open one against.
#[tokio::test]
async fn a_committed_branch_with_no_base_is_pushed_and_opens_nothing() {
    let home = TempDir::new();
    let (fleet, job) = finished_through(
        &home,
        FakeVcs::new()
            .with_nothing_to_commit()
            .with_commits_on_the_branch(2)
            .delivering(Delivering {
                base: None,
                ..Delivering::default()
            }),
    )
    .await;

    assert_eq!(
        went_out(fleet.vcs()),
        (true, false),
        "pushed, with no pull request: {:?}",
        fleet.vcs().delivered()
    );
    let said = log_of(&home, &job);
    assert!(
        !said.iter().any(|msg| msg.contains("held nothing new")),
        "the log does not call a branch with commits on it empty: {said:?}"
    );
}
