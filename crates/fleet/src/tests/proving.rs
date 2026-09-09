//! Running the repository's Checks against the tree a merge left behind.
//!
//! **The three cases that decide the design.** A merge that fast-forwarded is
//! proved, and the answer is written down against the commit. A merge Fleet
//! declined to fast-forward is not, because there is no updated tree — the
//! checkout holds somebody's uncommitted work and a run against it would be
//! reporting on that. And a second Job merging into the same commit reads the
//! first one's answer rather than running the suite again, which is `#474`'s
//! whole reason for not being a row on a Job.
//!
//! **Everything here is opt-in**, so the fixture Manifest names its Checks
//! explicitly. A Fleet booted on this repository's shipped `armada.yml` proves
//! nothing, because that file names none.

use std::time::Duration;

use adapter_traits::{Landing, RepositoryStanding};
use config::Manifest;
use core_model::CheckOutcome;
use testkit::{FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";
/// What the base branch is on once the fast-forward has run.
const MERGED_INTO: &str = "5b4ec82700000000000000000000000000000000";

/// A Manifest that names two Checks after a merge: one that always succeeds and
/// one that never does.
///
/// **`true` and `false`, not `cargo test`.** Quoted, because YAML would read
/// them as booleans and the `run` key takes text. What is under test is the
/// wiring —
/// which tree the run happens in, where the answer is filed, and who is told —
/// and a fixture that ran a real suite would be measuring this machine.
fn proving_manifest() -> Manifest {
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n\
         checks:\n  green:\n    run: \"true\"\n  red:\n    run: \"false\"\n\
         after_merge:\n  checks: [green, red]\n",
    )
    .expect("a manifest that names Checks after a merge")
}

fn a_fleet_that_proves_a_merge(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.manifest = proving_manifest();
    Fleet::assembled(fittings)
}

/// Run the shipped two-step fixture workflow to the end, so the Job has a pull
/// request for somebody to merge.
async fn a_finished_job(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    job.id().clone()
}

/// Turn until a run comes back, or give up and say so.
///
/// **The run is spawned and the turn returns**, because a suite is minutes and
/// the turn interval is 250ms. So a test that turned once would be asserting
/// against a task that had not been polled yet, which is the flake this
/// function exists instead of.
async fn turned_until_proved(fleet: &Fixture) -> Vec<store::Proved> {
    for _ in 0..200 {
        let turned = fleet.turn().await.unwrap();
        if !turned.proved.is_empty() {
            return turned.proved;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no run came back within two seconds");
}

/// The whole of what `#474` asked for: after a merge Armada notices, the Checks
/// run once against the commit that merged, and what they said is on the record.
#[tokio::test]
async fn a_merge_is_proved_against_the_commit_the_repository_ended_up_on() {
    let home = TempDir::new();
    let fleet = a_fleet_that_proves_a_merge(&home);
    a_finished_job(&fleet, &home).await;

    fleet
        .vcs()
        .repository_standing(RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 1,
            head: String::from(MERGED_INTO),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    let proved = turned_until_proved(&fleet).await;
    assert_eq!(proved.len(), 1);
    assert_eq!(proved[0].at_commit, MERGED_INTO);
    assert_eq!(proved[0].base, "main");
    // One row per declared Check, in the order the Manifest named them — the
    // same refusal to record a short list that a step's gate makes.
    let said: Vec<(&str, CheckOutcome)> = proved[0]
        .checks
        .iter()
        .map(|check| (check.name.as_str(), check.outcome))
        .collect();
    assert_eq!(
        said,
        vec![
            ("green", CheckOutcome::Passed),
            ("red", CheckOutcome::Failed)
        ]
    );
    assert_eq!(proved[0].unhappy(), vec!["red"]);

    // Keyed by the commit and readable without knowing which Job noticed.
    let store = fleet.store().lock().await;
    assert!(store.already_proved(MERGED_INTO).unwrap());
    assert_eq!(
        store
            .proved(MERGED_INTO)
            .unwrap()
            .map(|run| run.checks.len()),
        Some(2)
    );
}

/// **A merge Fleet declined to fast-forward proves nothing.**
///
/// `caught_the_repository_up` declines on a dirty worktree and on a checkout
/// that is not on the base, so on those merges there is no updated tree to
/// prove anything about. A run that used whatever was in the checkout would be
/// reporting on somebody's uncommitted work — and the type makes it
/// unspeakable: `LeftAlone` carries no commit to hand over.
#[tokio::test]
async fn a_repository_that_was_left_alone_is_not_proved() {
    let home = TempDir::new();
    let fleet = a_fleet_that_proves_a_merge(&home);
    a_finished_job(&fleet, &home).await;

    fleet
        .vcs()
        .repository_standing(RepositoryStanding::LeftAlone {
            why: String::from("`main` is carrying 3 uncommitted change(s)"),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    for _ in 0..20 {
        let turned = fleet.turn().await.unwrap();
        assert!(turned.proved.is_empty(), "there was no tree to prove");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // And the merge is still recorded. Nothing about the Job turns on any of
    // this — the work was merged before any of it ran.
    let landed = fleet.store().lock().await.landed_by_job().unwrap();
    assert_eq!(landed.len(), 1);
}

/// **Two Jobs merging into one commit cost one run.** The record is keyed by
/// the commit, so the second Job's notice finds it already proved and starts
/// nothing — which is the whole reason this is not a row on a Job.
#[tokio::test]
async fn a_commit_already_proved_is_not_proved_again() {
    let home = TempDir::new();
    let fleet = a_fleet_that_proves_a_merge(&home);
    a_finished_job(&fleet, &home).await;

    fleet
        .vcs()
        .repository_standing(RepositoryStanding::AlreadyHadIt {
            base: String::from("main"),
            head: String::from(MERGED_INTO),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });
    turned_until_proved(&fleet).await;

    // A second Job, merging into the same commit a moment later.
    let second = a_finished_job(&fleet, &home).await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });
    for _ in 0..20 {
        let turned = fleet.turn().await.unwrap();
        assert!(
            turned.proved.is_empty(),
            "the commit was proved when {} was not the one that noticed",
            second.as_str()
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// A repository that names no `after_merge` Checks spends nothing.
///
/// **The default, and it is not a flag nobody can find**: what this costs is a
/// build and test run on the machine somebody is working on, and
/// `docs/practices/rust.md` section 8 names a hook that rebuilt on merge as the
/// cause of v1's real build cost.
#[tokio::test]
async fn a_manifest_that_names_none_runs_nothing_after_a_merge() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    let fleet = Fleet::assembled(fittings);
    a_finished_job(&fleet, &home).await;

    fleet
        .vcs()
        .repository_standing(RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 1,
            head: String::from(MERGED_INTO),
        });
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    for _ in 0..20 {
        assert!(fleet.turn().await.unwrap().proved.is_empty());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!fleet
        .store()
        .lock()
        .await
        .already_proved(MERGED_INTO)
        .unwrap());
}
