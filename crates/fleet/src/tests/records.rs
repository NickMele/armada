//! The seam between a repository and its own records, proved end to end.
//!
//! **Every other case in this suite plants `records_root` equal to
//! `repo_root`.** That is deliberate — `crate::tests::daemon::fittings` says
//! so — and it proves nothing about the two being read from the right field: a
//! typo that read `repo_root` everywhere `records_root` belongs would pass
//! every one of those cases too, because the two paths agree there. This file
//! is the one case that gives the two different homes and checks what a real
//! dispatch put where.

use std::time::Duration;

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, fittings, one, worktree_directory};
use crate::tests::gate::workflow;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
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

/// **`#634` and `#638` in one place.** `#634` gave a running Check's log a
/// file while the gate is still waiting on it; `#638` moved every kept record
/// off the checkout. Written in parallel, the two agreed on where the
/// *finished* record goes and disagreed on where the *live* one does — a
/// Fleet built from both still wrote the live log under `repo_root`, and
/// `#638`'s migration spent its first boot moving 21 of them back out.
///
/// **Polled while the Check is still running, not just read after.** A fix
/// that only moved the recorded file and left `Announcing` pointed at
/// `repo_root` would pass every other case in this file, because none of them
/// has a Check running long enough to observe where its log lands before it
/// is recorded over.
#[tokio::test]
async fn a_running_checks_live_log_lands_under_records_root_and_never_under_repo_root() {
    let repo = TempDir::new();
    let records = TempDir::new();
    let mut fittings = fittings(&repo, FakeWorkProduct::changed(&["src/lib.rs"]));
    fittings.host.records_root = records.path().to_string_lossy().to_string();
    fittings.workflows = one(workflow("/bin/sleep 0.4"));
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal(
            "keep a running Check's log off the checkout too",
        ))
        .await
        .expect("a proposal");
    worktree_directory(&repo, &job);
    let job = dispatched(&fleet, job.id()).await.expect("a dispatch");
    let handle = job.handle();

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the tool took it");

    let repo_checks = repo.path().join(".armada").join("checks");
    let records_checks = records.path().join(".armada").join("checks").join(&handle);

    // The gate's turn runs the Check — a real `/bin/sleep` — to completion.
    // Polled alongside it on the same runtime, so this sees the directory the
    // instant the gate creates it, well before the sleep ends.
    let watched = async {
        for _ in 0..200 {
            if records_checks.exists() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        false
    };
    let (seen_while_running, turned) = tokio::join!(watched, fleet.turn());
    turned.expect("the gate ruled");

    assert!(
        seen_while_running,
        "the live log appeared under records_root while the Check was still running"
    );
    assert!(
        !repo_checks.exists(),
        "repo_root never grew a checks/ directory, live or recorded"
    );
    assert!(
        records_checks.exists(),
        "the recorded output is under records_root once the gate ruled"
    );
    assert!(
        repo.path()
            .join(".armada")
            .join("worktrees")
            .join(&handle)
            .exists(),
        "the worktree is still under repo_root — that one does not move"
    );
}
