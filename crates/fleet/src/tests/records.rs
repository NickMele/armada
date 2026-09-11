//! The seam between a repository and its own records, proved end to end.
//!
//! **Every other case in this suite plants `records_root` equal to
//! `repo_root`.** That is deliberate — `crate::tests::daemon::fittings` says
//! so — and it proves nothing about the two being read from the right field: a
//! typo that read `repo_root` everywhere `records_root` belongs would pass
//! every one of those cases too, because the two paths agree there. This file
//! is the one case that gives the two different homes and checks what a real
//! dispatch put where.

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::transcript::log_of;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet whose repository and whose records live in two different
/// directories, so nothing about them can agree by coincidence.
fn a_fleet_with_records_elsewhere(repo: &TempDir, records: &TempDir) -> Fixture {
    let mut fittings = fittings(repo, FakeWorkProduct::changed(&[]));
    fittings.host.records_root = records.path().to_string_lossy().to_string();
    Fleet::assembled(fittings)
}

#[tokio::test]
async fn a_dispatched_jobs_log_lands_under_records_root_and_never_under_repo_root() {
    let repo = TempDir::new();
    let records = TempDir::new();
    let fleet = a_fleet_with_records_elsewhere(&repo, &records);

    let job = fleet
        .propose(a_proposal("keep this job's records off the checkout"))
        .await
        .expect("a proposal");
    worktree_directory(&repo, &job);
    let job = dispatched(&fleet, job.id()).await.expect("a dispatch");
    let handle = job.handle();

    assert!(
        log_of(&records.path().to_string_lossy(), &handle).exists(),
        "the Job's log is under records_root"
    );
    assert!(
        !repo.path().join(".armada").join("logs").exists(),
        "and repo_root never grew a `logs/` directory at all"
    );
    assert!(
        repo.path()
            .join(".armada")
            .join("worktrees")
            .join(&handle)
            .exists(),
        "the worktree is still under repo_root — that one does not move"
    );
    assert!(
        !records.path().join(".armada").join("worktrees").exists(),
        "and records_root never grows a worktree"
    );
}
