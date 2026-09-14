//! A Job whose Check fails on a test another Job is fixing, pointed at that fix
//! and told how it settles. #1001.
//!
//! The fix here is a proposed Job holding a claim written straight to the
//! store, so what is under test is the pointing and not the drafting.

use std::sync::Arc;

use config::EvidenceType;
use core_model::{Breakage, BreakageClaim, JobId, ManifestId, Ulid};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, OneTest, Sketch};
use verification::{Claimed, NotClaimed, ShownBy};

use super::{fix_for, started, Fixture, COMMIT, TEST};
use crate::daemon::Fleet;
use crate::dry_run::DryRuns;
use crate::evidence::Call;
use crate::fixing::{FixAnswer, Fixes};
use crate::peers::{News, PeersChanged};
use crate::tests::daemon::{a_proposal, fitted_over, one};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

const NAMES_IT: &str = "/bin/sh -c 'echo FAIL parser::takes_it; exit 1'";
const NAMES_ANOTHER: &str = "/bin/sh -c 'echo FAIL parser::other; exit 1'";

/// One step gated on `suite`, which fails printing what `run` echoes.
fn printing(run: &'static str) -> config::ResolvedWorkflow {
    let gates = [Gate::Check {
        name: "suite",
        run,
        expect_exit_code: 0,
        when: &[],
    }];
    testkit::testing_one(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &gates,
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        &[OneTest {
            check: "suite",
            run: "/usr/bin/false {}",
        }],
    )
}

fn a_fleet_checking(home: &TempDir, run: &'static str) -> Arc<Fixture> {
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "sleep 30"]),
        FakeVcs::new().with_ref_at("main", COMMIT),
    );
    fittings.starting().workflows = one(printing(run));
    fittings.fixes = Fixes::of(1);
    fittings.dry_runs = DryRuns::of(3);
    Arc::new(Fleet::assembled(fittings))
}

/// A fix Job waiting at approval, claiming [`TEST`] in the reporter's repository.
async fn claimed(fleet: &Fixture, working: &JobId) -> JobId {
    let fix = fleet
        .propose(a_proposal("fix the parser on main"))
        .await
        .expect("a proposed fix")
        .id()
        .clone();
    let owner = fleet.names().owner_of(working).expect("an owner");
    let now = fleet.now();
    fleet
        .store()
        .lock()
        .await
        .claim_breakage(
            &BreakageClaim {
                fix: fix.clone(),
                repository: ManifestId::carried(Ulid::carried(owner)),
                breakage: Breakage {
                    check: String::from("suite"),
                    test: TEST.to_string(),
                    failure: String::from("exited 1"),
                },
                reported_by: fix.clone(),
            },
            &now,
        )
        .expect("claimed");
    fix
}

async fn dry_run(fleet: &Arc<Fixture>, job: &JobId) {
    let _ = fleet
        .run_checks(job, false)
        .await
        .expect("started")
        .finished()
        .await;
}

/// Pointed at the fix through a `draft_fix` the claim answers.
async fn asked(fleet: &Arc<Fixture>, job: &JobId) {
    let answer = fleet.draft_fix(job, fix_for(TEST)).await.expect("answered");
    assert!(matches!(answer, FixAnswer::AlreadyClaimed(_)), "{answer:?}");
}

/// Every peer turn written into this Job's transcript, once there is one. The
/// file is written off the turn, `tests::peers`' reason, so a read straight
/// after `tell_peers` can be early.
async fn peer_turns(fleet: &Fixture, home: &TempDir, job: &JobId) -> String {
    let record = fleet.load(job).await.expect("the Job");
    let drone = record.assigned_drone().cloned().expect("a Drone on it");
    let path =
        crate::transcript::transcript_of(&home.path().to_string_lossy(), &record.handle(), &drone);
    let read = || {
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|row| row.contains("\"occasion\":\"peers\""))
            .collect::<Vec<_>>()
            .join("\n")
    };
    for _ in 0..200 {
        let told = read();
        if !told.is_empty() {
            return told;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    read()
}

/// **The claim of the issue.** A dry run fails printing a claimed test, so the
/// Job is pointed at the fix, both Jobs' detail say so, and the Drone is told.
#[tokio::test]
async fn a_dry_run_failing_on_a_claimed_test_points_the_job_at_its_fix() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(&home, NAMES_IT);
    let working = started(&fleet, &home).await;
    let fix = claimed(&fleet, &working).await;

    dry_run(&fleet, &working).await;

    let waiting = fleet.load(&working).await.expect("the Job");
    let seen = fleet.breakages_of(&waiting).await.expect("read");
    assert_eq!(seen.len(), 1, "the waiting Job names the fix");
    assert_eq!(seen[0].fix, ipc::JobId::from(&fix));
    let fixing = fleet.load(&fix).await.expect("the fix");
    let listed = fleet.breakages_of(&fixing).await.expect("read");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0]
            .waiting
            .iter()
            .map(|one| one.job_id.clone())
            .collect::<Vec<_>>(),
        vec![ipc::JobId::from(&working)],
        "the fix lists the Job waiting on it"
    );

    fleet.tell_peers().await;
    let told = peer_turns(&fleet, &home, &working).await;
    assert!(told.contains("is fixing `parser::takes_it`"), "{told}");
}

/// A failure that does not print the claimed test is the Job's own business.
#[tokio::test]
async fn a_failure_printing_another_test_points_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(&home, NAMES_ANOTHER);
    let working = started(&fleet, &home).await;
    claimed(&fleet, &working).await;

    dry_run(&fleet, &working).await;

    let job = fleet.load(&working).await.expect("the Job");
    assert!(fleet.breakages_of(&job).await.expect("read").is_empty());
}

/// The gate points a Job exactly as a dry run does.
#[tokio::test]
async fn the_gate_failing_on_a_claimed_test_points_the_job_too() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(&home, NAMES_IT);
    let working = started(&fleet, &home).await;
    claimed(&fleet, &working).await;

    submitted_by_the_one(
        &fleet,
        Call {
            evidence_type: EvidenceType::Diff,
            claimed: Claimed("the parser takes it"),
            shown_by: ShownBy("src/parse.rs"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("submitted");
    fleet.turn().await.expect("the gate runs");

    let pointed = fleet
        .store()
        .lock()
        .await
        .fixes_waited_on_by(&working)
        .expect("read");
    assert_eq!(pointed.len(), 1, "{pointed:?}");
}

/// A Drone asking to fix a claimed test is pointed at the fix already on it.
#[tokio::test]
async fn a_drone_asking_to_fix_a_claimed_test_is_pointed_at_it() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(&home, NAMES_IT);
    let working = started(&fleet, &home).await;
    let fix = claimed(&fleet, &working).await;

    asked(&fleet, &working).await;

    let pointed = fleet
        .store()
        .lock()
        .await
        .fixes_waited_on_by(&working)
        .expect("read");
    assert_eq!(pointed.len(), 1);
    assert_eq!(pointed[0].fix, fix);
}

/// When the fix lands, every Job pointed at it hears, and the claim is given
/// back.
#[tokio::test]
async fn waiting_jobs_hear_when_the_fix_lands() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(&home, NAMES_IT);
    let working = started(&fleet, &home).await;
    let fix = claimed(&fleet, &working).await;
    asked(&fleet, &working).await;

    fleet.fix_settled(&fix, true).await;
    fleet.tell_peers().await;

    let told = peer_turns(&fleet, &home, &working).await;
    assert!(
        told.contains("landed its fix for `parser::takes_it`"),
        "{told}"
    );
    let store = fleet.store().lock().await;
    assert!(store.fixes_waited_on_by(&working).expect("read").is_empty());
    assert!(store.breakages_claimed_by(&fix).expect("read").is_empty());
}

/// A fix that ends without landing tells them nobody is fixing the test now.
#[tokio::test]
async fn waiting_jobs_hear_when_the_fix_ends_without_landing() {
    let home = TempDir::new();
    let fleet = a_fleet_checking(&home, NAMES_IT);
    let working = started(&fleet, &home).await;
    let fix = claimed(&fleet, &working).await;
    asked(&fleet, &working).await;

    fleet.kill_job(&fix).await.expect("the fix is killed");
    fleet.tell_peers().await;

    let told = peer_turns(&fleet, &home, &working).await;
    assert!(told.contains("ended without landing its fix"), "{told}");
}

/// A turn carrying only fixes says nothing about shared files.
#[test]
fn a_turn_about_a_fix_leaves_the_shared_file_sentences_out() {
    let text = PeersChanged::injected(&[News::Fixing {
        title: "fix the parser on main".to_string(),
        handle: "3-fix-the-parser-on-main".to_string(),
        test: TEST.to_string(),
    }])
    .text()
    .to_string();
    assert!(text.contains("another Job's to fix, not yours"), "{text}");
    assert!(!text.contains("Where a shared file"), "{text}");
    assert!(
        !text.contains("change files this Job changes too"),
        "{text}"
    );
}
