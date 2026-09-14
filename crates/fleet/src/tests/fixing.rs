//! A Drone naming a test it says is broken on main, and Fleet running just that
//! test there before anything is drafted. #999.
//!
//! The Checks are real commands: `/usr/bin/false` fails on main and
//! `/usr/bin/true` passes, so what the run came to is what the case is about.

use std::sync::Arc;

use config::ResolvedWorkflow;
use ipc::mcp::DraftFix;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, OneTest, Sketch};

use crate::daemon::Fleet;
use crate::fixing::{Drafted, Fixes, NotFixed};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_over, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const COMMIT: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

/// One step gated on `suite`, which runs one test by `one_test`.
fn gated_on(one_test: &str) -> ResolvedWorkflow {
    testkit::testing_one(
        &[step()],
        &[OneTest {
            check: "suite",
            run: one_test,
        }],
    )
}

fn step() -> Sketch<'static> {
    Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "suite",
            run: "/usr/bin/false",
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }
}

/// A Fleet over a repository whose main is at [`COMMIT`], with its checkout of
/// main on disk and a Drone that stays in the slot.
fn a_fleet(home: &TempDir, workflow: ResolvedWorkflow, fixes: u32) -> Arc<Fixture> {
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "sleep 30"]),
        FakeVcs::new().with_ref_at("main", COMMIT),
    );
    fittings.starting().workflows = one(workflow);
    fittings.fixes = Fixes::of(fixes);
    std::fs::create_dir_all(home.path().join(".armada").join("bases").join(COMMIT))
        .expect("the checkout of main is on disk");
    Arc::new(Fleet::assembled(fittings))
}

async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a proposed Job");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("an approved Job");
    job.id().clone()
}

fn fix_for(test: &str) -> DraftFix {
    DraftFix {
        check: String::from("suite"),
        test: test.to_string(),
        failure: String::from("exited 1"),
        title: format!("Fix {test} on main"),
        workflow: String::from("fixture-workflow"),
        brief: String::from("It fails on main before any change."),
        acceptance_criteria: vec![format!("{test} passes on main")],
    }
}

/// **The claim of the issue.** The test fails on main too, so a fix is drafted
/// waiting for a person, the test is claimed for it, and both Jobs say so.
#[tokio::test]
async fn a_test_failing_on_main_too_drafts_a_fix_and_claims_the_test() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    let drafted = fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect("a fix is drafted");
    let Drafted::New(fix) = drafted else {
        panic!("a new fix, not {drafted:?}");
    };

    let record = fleet.load(&fix).await.expect("the fix Job");
    assert_eq!(record.status(), core_model::JobStatus::AwaitingApproval);
    assert_eq!(
        fleet.load(&reporter).await.expect("the reporter").status(),
        core_model::JobStatus::Running,
        "the reporter carries on"
    );
    for side in [&reporter, &fix] {
        let job = fleet.load(side).await.expect("the Job");
        let claims = fleet.breakages_of(&job).await.expect("read");
        assert_eq!(claims.len(), 1, "both Jobs name the claim");
        assert_eq!(claims[0].test, "parser::takes_it");
        assert_eq!(claims[0].fix, ipc::JobId::from(&fix));
    }
}

/// **A pass on main is the Drone's own change**, and nothing is drafted.
#[tokio::test]
async fn a_test_passing_on_main_drafts_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/true {}"), 1);
    let reporter = started(&fleet, &home).await;

    let refused = fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect_err("nothing is drafted");
    assert!(
        matches!(refused, NotFixed::PassesOnMain { .. }),
        "{refused:?}"
    );
    let job = fleet.load(&reporter).await.expect("the reporter");
    assert!(fleet.breakages_of(&job).await.expect("read").is_empty());
}

/// A second report of a claimed test names the fix already on it, and runs
/// nothing — so a step's allowance spent on the first does not refuse it.
#[tokio::test]
async fn a_second_report_of_a_claimed_test_names_the_fix_on_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    let first = fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect("drafted");
    let Drafted::New(fix) = first else {
        panic!("a new fix, not {first:?}");
    };
    let again = fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect("answered");
    assert_eq!(again, Drafted::AlreadyClaimed(fix));
}

/// A step asks for as many fixes as the composition root allows, and no more.
#[tokio::test]
async fn a_part_asks_for_no_more_fixes_than_it_is_allowed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect("drafted");
    let refused = fleet
        .draft_fix(&reporter, &fix_for("parser::other"))
        .await
        .expect_err("spent");
    assert!(
        matches!(refused, NotFixed::Spent { allowed: 1 }),
        "{refused:?}"
    );
}

/// A Check that cannot run one test by name is not run whole on main instead.
#[tokio::test]
async fn a_check_with_no_way_to_run_one_test_drafts_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, testkit::resolved(&[step()]), 1);
    let reporter = started(&fleet, &home).await;

    let refused = fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect_err("nothing runs");
    assert!(
        matches!(refused, NotFixed::NoWayToRunOneTest { .. }),
        "{refused:?}"
    );
}

/// A fix that ends gives the test back, so a test broken again can be claimed.
#[tokio::test]
async fn a_fix_that_ends_gives_the_test_back() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;
    let Drafted::New(fix) = fleet
        .draft_fix(&reporter, &fix_for("parser::takes_it"))
        .await
        .expect("drafted")
    else {
        panic!("a new fix");
    };

    fleet.kill_job(&fix).await.expect("the fix is killed");

    let job = fleet.load(&reporter).await.expect("the reporter");
    assert!(
        fleet.breakages_of(&job).await.expect("read").is_empty(),
        "the claim went with the fix"
    );
}
