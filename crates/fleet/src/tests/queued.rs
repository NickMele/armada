//! Why an approved Job has not started, as the Board reads it.
//!
//! The reason is computed rather than stored, so these drive the real read
//! rather than planting a value: a Job is put into each state and its summary
//! is asked for.
//!
//! **The last case is the one that matters.** The label and admission answer
//! the same question, and a Board saying a Job is blocked while Fleet is
//! starting it would be worse than a Board saying nothing at all.

use api::Queries;
use testkit::{FakeJudge, FakeWorkProduct};

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_fleet, a_fleet_proposing_through, a_proposal, diff_evidence, worktree_directory,
};
use crate::tests::planning::A_PLAN;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// A Job held behind one that has not landed says so.
#[tokio::test]
async fn a_job_waiting_on_a_peer_reads_blocked_by_dependency() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes", None)
        .await
        .expect("a plan");
    worktree_directory(&home, made[1].id());
    dispatched(&fleet, made[1].id())
        .await
        .expect("approval lands");

    let summary = fleet
        .get_job(ipc::JobId::from(made[1].id()))
        .await
        .expect("the Job reads")
        .job;

    assert_eq!(summary.status.as_wire(), "queued");
    assert_eq!(
        summary.queued_reason.map(|why| why.as_wire()),
        Some("blocked_by_dependency"),
        "the slot is free, so nothing but the edge is holding it"
    );
}

/// A Job held only by the slot says that instead.
#[tokio::test]
async fn a_job_waiting_for_the_slot_reads_waiting_on_resources() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let first = fleet.propose(a_proposal("the first")).await.unwrap();
    worktree_directory(&home, first.id());
    dispatched(&fleet, first.id()).await.unwrap();
    let second = fleet.propose(a_proposal("the second")).await.unwrap();
    worktree_directory(&home, second.id());
    dispatched(&fleet, second.id()).await.unwrap();

    let summary = fleet
        .get_job(ipc::JobId::from(second.id()))
        .await
        .expect("the Job reads")
        .job;

    assert_eq!(summary.status.as_wire(), "queued");
    assert_eq!(
        summary.queued_reason.map(|why| why.as_wire()),
        Some("waiting_on_resources"),
        "it depends on nothing — the Job in the slot is the whole of the wait"
    );
}

/// Every other status carries none, because none of them can have one.
#[tokio::test]
async fn a_job_that_is_not_queued_carries_no_queued_reason() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet.propose(a_proposal("at the gate")).await.unwrap();

    let summary = fleet
        .get_job(ipc::JobId::from(job.id()))
        .await
        .expect("the Job reads")
        .job;

    assert_eq!(summary.status.as_wire(), "awaiting_approval");
    assert!(
        summary.queued_reason.is_none(),
        "a Job nobody has approved is not waiting on anything"
    );
}

/// The label and admission are one answer.
///
/// A Board reading `blocked` about a Job that Fleet would start, or reading
/// nothing about one it refuses to start, is the drift that sharing
/// `clear_to_run` exists to prevent. Both halves are asserted in one test on
/// purpose: apart, each can pass while the two disagree.
#[tokio::test]
async fn what_the_board_says_is_what_admission_did() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes", None)
        .await
        .expect("a plan");
    worktree_directory(&home, made[0].id());
    worktree_directory(&home, made[1].id());
    dispatched(&fleet, made[1].id())
        .await
        .expect("the dependent");

    // Says blocked, and is not running.
    let blocked = fleet
        .get_job(ipc::JobId::from(made[1].id()))
        .await
        .expect("the Job reads")
        .job;
    assert_eq!(
        blocked.queued_reason.map(|why| why.as_wire()),
        Some("blocked_by_dependency")
    );
    assert!(fleet.working_on().await.is_empty(), "and it did not start");

    // The upstream runs and lands. The same Job is admitted, and stops saying
    // it is held — the two moved together or one of these fails.
    dispatched(&fleet, made[0].id())
        .await
        .expect("the upstream");
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.expect("the loop turns");

    let started = fleet
        .get_job(ipc::JobId::from(made[1].id()))
        .await
        .expect("the Job reads")
        .job;
    assert_eq!(started.status.as_wire(), "running");
    assert!(
        started.queued_reason.is_none(),
        "the label cleared exactly when admission did"
    );
}
