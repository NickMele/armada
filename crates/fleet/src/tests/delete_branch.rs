//! A person deleting a branch Fleet's own reclaim keeps. **Real git**, for
//! `crate::tests::reclaim`'s reason, whose fixtures these are.

use axum::http::StatusCode;
use core_model::JobStatus;
use ipc::RunId;
use testkit::FakeWorkProduct;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::reclaiming::Undeletable;
use crate::tests::daemon::{a_fleet, a_proposal};
use crate::tests::http::call;
use crate::tests::reclaim::{a_finished_job, a_repository, a_worktree_for, branches, commit, git};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// A commit on the Job's branch that `main` cannot reach, and the tip after it.
fn a_commit_on(home: &TempDir, handle: &str, file: &str) -> String {
    let at = home.path().join(".armada/worktrees").join(handle);
    std::fs::write(at.join(file), "what the drone wrote\n").expect("a change");
    git(&at, &["add", file]);
    commit(&at, "work nobody has taken");
    git(&at, &["rev-parse", "HEAD"]).trim().to_string()
}

/// The bug's Job: terminal, checkout reclaimed, branch kept for being unmerged.
async fn a_kept_branch(home: &TempDir, fleet: &Fixture) -> (core_model::JobId, String, String) {
    a_repository(home);
    let job_id = a_finished_job(fleet, "kept branch").await;
    let handle = fleet.load(&job_id).await.expect("the Job").handle();
    a_worktree_for(home, &handle);
    let tip = a_commit_on(home, &handle, "work.txt");
    Fleet::reclaim_worktree(fleet, &job_id)
        .await
        .expect("the checkout goes and the branch is kept");
    (job_id, handle, tip)
}

fn why(refused: Adrift) -> Undeletable {
    match refused {
        Adrift::BranchNotDeletable { why, .. } => why,
        other => panic!("a 409 refusal, not: {other}"),
    }
}

#[tokio::test]
async fn a_kept_branch_is_deleted_at_the_tip_the_person_was_shown() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let (job_id, handle, tip) = a_kept_branch(&home, &fleet).await;

    let deleted = Fleet::delete_branch(&fleet, &job_id, &tip)
        .await
        .expect("a person may delete unmerged work");

    assert_eq!(deleted.tip, tip, "the tip it is recoverable from");
    assert!(!branches(home.path()).contains(&format!("armada/{handle}")));
    assert!(
        fleet.worktrees_held().await.expect("the list").is_empty(),
        "and the Job leaves Waiting on you"
    );
}

/// **The guard.** A commit landed after the person looked, so what they agreed
/// to lose is not what is there, and nothing is deleted.
#[tokio::test]
async fn a_branch_whose_tip_moved_is_refused_and_left_standing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "moved under me").await;
    let handle = fleet.load(&job_id).await.expect("the Job").handle();
    a_worktree_for(&home, &handle);
    let shown = a_commit_on(&home, &handle, "first.txt");
    let found = a_commit_on(&home, &handle, "second.txt");
    Fleet::reclaim_worktree(&fleet, &job_id)
        .await
        .expect("reclaimed");

    let refused = Fleet::delete_branch(&fleet, &job_id, &shown)
        .await
        .expect_err("the tip moved");

    let said = refused.to_string();
    assert_eq!(
        why(refused),
        Undeletable::TipMoved {
            branch: format!("armada/{handle}"),
            asked: shown,
            found: found.clone(),
        }
    );
    assert!(
        said.contains(&found),
        "the sentence names the new tip: {said}"
    );
    assert!(branches(home.path()).contains(&format!("armada/{handle}")));
}

#[tokio::test]
async fn a_job_still_moving_keeps_its_branch() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting"))
        .await
        .expect("a Job at the gate");

    let refused = Fleet::delete_branch(&fleet, job.id(), "0000")
        .await
        .expect_err("awaiting_approval is not terminal");

    assert_eq!(
        why(refused),
        Undeletable::NotTerminal {
            status: JobStatus::AwaitingApproval
        }
    );
}

#[tokio::test]
async fn a_checkout_still_on_disk_is_reclaimed_first() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "still checked out").await;
    let handle = fleet.load(&job_id).await.expect("the Job").handle();
    a_worktree_for(&home, &handle);
    let tip = a_commit_on(&home, &handle, "work.txt");

    let refused = Fleet::delete_branch(&fleet, &job_id, &tip)
        .await
        .expect_err("the checkout is there");

    assert!(refused.to_string().contains("Reclaim the worktree first"));
    assert!(matches!(why(refused), Undeletable::CheckoutOnDisk { .. }));
    assert!(branches(home.path()).contains(&format!("armada/{handle}")));
}

#[tokio::test]
async fn a_branch_already_gone_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "never dispatched").await;

    let refused = Fleet::delete_branch(&fleet, &job_id, "0000")
        .await
        .expect_err("no branch");

    assert!(matches!(why(refused), Undeletable::Absent { .. }));
}

/// Over the router that ships: 200 with the tip, then the same call again is a
/// 409 under its own code, because the branch is gone.
#[tokio::test]
async fn the_seam_deletes_then_refuses_with_a_conflict() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let (job_id, handle, tip) = a_kept_branch(&home, &fleet).await;
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let uri = format!("/jobs/{}/delete_branch", job_id.as_str());
    let body = format!(r#"{{"tip":"{tip}"}}"#);

    let (status, answered) = call(&app, "POST", &uri, &body).await;
    assert_eq!(status, StatusCode::OK);
    let deleted: ipc::BranchDeleted = ipc::decode("the delete", &answered).expect("decodes");
    assert_eq!(deleted.job_id.as_str(), job_id.as_str());
    assert_eq!(deleted.branch, format!("armada/{handle}"));
    assert_eq!(deleted.tip, tip);

    let (status, refused) = call(&app, "POST", &uri, &body).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let error: ipc::WireError = ipc::decode("the refusal", &refused).expect("a WireError");
    assert_eq!(error.code, "fleet.branch_not_deletable");
}
