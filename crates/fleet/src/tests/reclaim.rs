//! Giving one terminal Job's worktree and branch back while Fleet is running.
//!
//! **Real git, because what is under test is git's own opinion** — whether a
//! checkout is gone, whether the base already reaches every commit on a branch.
//! `crates/armada/src/tests/clean.rs` says the same thing about the layer
//! underneath; a fake here would be asserting this crate's guess about the one
//! decision that must never be guessed at.
//!
//! The refusals do not need one. Every other fixture in this crate has a bare
//! `TempDir` for a `repo_root`, which is a directory git will not open — so the
//! ordinary fixture *is* the unreadable-repository case, and it is used as one
//! below rather than being worked around.

use std::path::Path;
use std::process::Command;

use adapters::{BranchGone, WorktreeGone};
use axum::http::StatusCode;
use core_model::JobStatus;
use ipc::RunId;
use testkit::FakeWorkProduct;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet, a_proposal};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

fn git(at: &Path, args: &[&str]) -> String {
    let run = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git on PATH — a test nothing can run is a test that does not exist");
    assert!(
        run.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// Make the fixture's root a real repository with one commit on `main`.
///
/// `main` by name rather than by whoever's `init.defaultBranch` is set: what
/// counts as merged is read from the repository, so the fixture states it.
///
/// Only `README` is committed. The store's database is in this directory too
/// and committing it would put a moving binary file under every assertion here.
fn a_repository(home: &TempDir) {
    std::fs::write(home.path().join("README"), "the fixture\n").expect("a file to commit");
    git(
        home.path(),
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
    );
    git(home.path(), &["add", "README"]);
    commit(home.path(), "the first commit");
}

fn commit(at: &Path, message: &str) {
    git(
        at,
        &[
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
            "commit",
            "--quiet",
            "-m",
            message,
        ],
    );
}

fn branches(at: &Path) -> Vec<String> {
    git(at, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(str::to_string)
        .collect()
}

/// The worktree and branch a dispatch would have made for this Job. Made with
/// git rather than through `Vcs`, because the fixture's version control is a
/// fake and this file is about what git holds.
fn a_worktree_for(home: &TempDir, job_id: &str) {
    let path = format!(".armada/worktrees/{job_id}");
    let branch = format!("armada/{job_id}");
    git(
        home.path(),
        &["worktree", "add", "--quiet", "-b", &branch, &path],
    );
}

/// A terminal Job, killed before anything spawned — which is legal from
/// `awaiting_approval` and needs no Drone.
async fn a_finished_job(
    fleet: &Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>,
    title: &str,
) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal(title))
        .await
        .expect("a Job at the gate");
    let killed = Fleet::kill_job(fleet, job.id())
        .await
        .expect("killable with no Drone");
    assert!(killed.status().is_terminal());
    job.id().clone()
}

/// The whole act, on a branch `main` already reaches: the checkout goes and the
/// branch goes with it. **The record survives** — this takes disk, and
/// `forget_job` is what takes the row.
#[tokio::test]
async fn a_merged_branch_and_its_worktree_are_both_given_back() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "reclaim me").await;
    a_worktree_for(&home, job_id.as_str());
    let path = home.path().join(".armada/worktrees").join(job_id.as_str());
    assert!(path.is_dir(), "the fixture built a checkout to reclaim");

    let gave_back = Fleet::reclaim_worktree(&fleet, &job_id)
        .await
        .expect("a terminal Job's disk comes back");

    assert!(
        matches!(gave_back.worktree, WorktreeGone::Removed { .. }),
        "the checkout is gone: {:?}",
        gave_back.worktree
    );
    assert!(
        matches!(gave_back.branch, BranchGone::Deleted { .. }),
        "nothing was on it that main does not have: {:?}",
        gave_back.branch
    );
    assert!(!path.exists(), "the directory is really gone");
    assert!(
        !branches(home.path()).contains(&format!("armada/{}", job_id.as_str())),
        "the branch is really gone"
    );
    assert!(
        fleet.load(&job_id).await.is_ok(),
        "the record survives a reclaim — forget_job is what takes the row"
    );
}

/// **The safety argument, run.** A branch holding a commit the base cannot
/// reach is kept, the disk still comes back, and the answer names the base and
/// the count rather than reporting one number for both halves.
///
/// There is no `--force` on this seam: a live Fleet must never be the thing
/// that deletes work nobody has taken, so this is the only outcome available
/// here and not a setting a caller chose.
#[tokio::test]
async fn a_branch_holding_unmerged_work_survives_its_own_worktree() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "keep my commits").await;
    a_worktree_for(&home, job_id.as_str());
    let worktree = home.path().join(".armada/worktrees").join(job_id.as_str());
    std::fs::write(worktree.join("work.txt"), "what the drone wrote\n").expect("a change");
    git(&worktree, &["add", "work.txt"]);
    commit(&worktree, "work nobody has taken");

    let gave_back = Fleet::reclaim_worktree(&fleet, &job_id)
        .await
        .expect("the repository opens");

    assert!(
        matches!(gave_back.worktree, WorktreeGone::Removed { .. }),
        "the disk still comes back: {:?}",
        gave_back.worktree
    );
    let BranchGone::Kept { base, commits, .. } = gave_back.branch else {
        panic!("a branch main cannot reach is kept: {:?}", gave_back.branch);
    };
    assert_eq!(base, "main");
    assert_eq!(commits, 1);
    assert!(
        branches(home.path()).contains(&format!("armada/{}", job_id.as_str())),
        "the branch is still there for a person to merge by hand"
    );
}

/// A Job still at the gate has no disk to give back, only a status to move.
/// The refusal names where the Job is and says which act ends one in flight.
#[tokio::test]
async fn a_job_that_is_not_yet_terminal_is_refused_by_name() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting"))
        .await
        .expect("a Job at the gate");

    let refused = Fleet::reclaim_worktree(&fleet, job.id())
        .await
        .expect_err("awaiting_approval is not terminal");
    assert!(matches!(
        refused,
        Adrift::NotReclaimable { status, .. } if status == JobStatus::AwaitingApproval
    ));
    assert!(
        refused.to_string().contains("awaiting_approval"),
        "the refusal names where the Job actually is: {refused}"
    );
}

/// A root git will not open comes back as a refusal **naming the repository**,
/// never a panic and never a silent success. The ordinary fixture's `repo_root`
/// is a bare directory, which is exactly that case.
///
/// The distinction this protects is the one a person acts on: "there was
/// nothing to reclaim" and "nothing was looked at" read the same on a Board and
/// mean opposite things about the disk.
#[tokio::test]
async fn a_repository_that_will_not_open_is_refused_and_names_itself() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = a_finished_job(&fleet, "nowhere to reclaim from").await;

    let refused = Fleet::reclaim_worktree(&fleet, &job_id)
        .await
        .expect_err("a bare directory is not a repository");
    assert!(matches!(refused, Adrift::NotReclaimed { .. }));
    let said = refused.to_string();
    assert!(
        said.contains(&home.path().to_string_lossy().to_string()),
        "the refusal names the repository that would not open: {said}"
    );
}

/// Over the router that ships: the two halves cross as two fields, and a kept
/// branch answers 200 rather than failing. `unmerged_commits` is what tells a
/// deliberate keep from a branch that would not delete, and it is present here.
#[tokio::test]
async fn the_seam_answers_with_both_halves_over_http() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    a_repository(&home);
    let job_id = a_finished_job(&fleet, "over the wire").await;
    a_worktree_for(&home, job_id.as_str());
    let worktree = home.path().join(".armada/worktrees").join(job_id.as_str());
    std::fs::write(worktree.join("work.txt"), "what the drone wrote\n").expect("a change");
    git(&worktree, &["add", "work.txt"]);
    commit(&worktree, "work nobody has taken");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/reclaim_worktree", job_id.as_str()),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "a kept branch is not a failure");

    let answer: ipc::WorktreeReclaimed =
        ipc::decode("what the reclaim gave back", &body).expect("the answer decodes");
    assert_eq!(answer.job_id.as_str(), job_id.as_str());
    assert!(answer.worktree.removed, "the disk came back");
    assert_eq!(answer.worktree.why, None);
    assert!(!answer.branch.deleted, "the branch was kept");
    assert_eq!(answer.branch.base.as_deref(), Some("main"));
    assert_eq!(answer.branch.unmerged_commits, Some(1));
}

/// The status refusal over HTTP: a 409, and the same 409 `forget_job` gives
/// for the same predicate — but under a code of its own, because a client
/// telling a person which act to try next has to be able to tell them apart.
#[tokio::test]
async fn a_job_still_in_flight_is_refused_with_a_conflict_over_http() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("still waiting, over HTTP"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));

    let (status, body) = call(
        &app,
        "POST",
        &format!("/jobs/{}/reclaim_worktree", job_id.as_str()),
        "",
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "there is no disk to give back while a Drone might still write to it"
    );
    let error: ipc::WireError = ipc::decode("the refusal", &body).expect("a WireError");
    assert_eq!(error.code, "fleet.not_reclaimable");
}
