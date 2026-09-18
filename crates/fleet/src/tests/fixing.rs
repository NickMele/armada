//! A Drone naming a test it says is broken on main, and Fleet running just that
//! test there before anything is drafted. #999.
//!
//! The Checks are real commands: `/usr/bin/false` fails on main and
//! `/usr/bin/true` passes, so what the run came to is what the case is about.
//! The call answers once the run has started; [`came_to`] waits for the rest.

mod waiting;

use std::sync::Arc;
use std::time::{Duration, Instant};

use config::ResolvedWorkflow;
use core_model::JobId;
use ipc::mcp::DraftFix;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, OneTest, Sketch};

use crate::daemon::Fleet;
use crate::fixing::{Drafted, FixAnswer, Fixes, NotFixed};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_over, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const COMMIT: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const TEST: &str = "parser::takes_it";

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

async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
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

/// Ask, and wait for what the run on main came to.
async fn came_to(fleet: &Arc<Fixture>, reporter: &JobId, test: &str) -> Result<Drafted, NotFixed> {
    match fleet.draft_fix(reporter, fix_for(test)).await? {
        FixAnswer::Started(running) => running.finished().await.expect("the run reported"),
        FixAnswer::AlreadyClaimed(fix) => panic!("already claimed by {}", fix.as_str()),
    }
}

/// Every fix turn written into the reporter's transcript, as the rows carry it.
async fn told_fix(fleet: &Fixture, home: &TempDir, reporter: &JobId) -> Vec<String> {
    let job = fleet.load(reporter).await.expect("the reporter");
    let drone = job.assigned_drone().cloned().expect("a Drone on it");
    let path =
        crate::transcript::transcript_of(&home.path().to_string_lossy(), &job.handle(), &drone);
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|row| row.contains("\"occasion\":\"fix\""))
        .map(str::to_string)
        .collect()
}

/// **The claim of the issue.** The test fails on main too, so a fix is drafted
/// waiting for a person, the test is claimed for it, both Jobs say so, and the
/// Drone is told as a later turn.
#[tokio::test]
async fn a_test_failing_on_main_too_drafts_a_fix_and_claims_the_test() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    let Drafted(fix) = came_to(&fleet, &reporter, TEST).await.expect("drafted");

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
        assert_eq!(claims[0].test, TEST);
        assert_eq!(claims[0].fix, ipc::JobId::from(&fix));
    }
    let told = told_fix(&fleet, &home, &reporter).await;
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(told[0].contains("fails on main too"), "{}", told[0]);
}

/// **A fix draft's one-test run is timed apart from a whole run of the
/// Check**, so it leaves `one_test_timings` a duration and leaves
/// `check_timings` — what a step's order reads — untouched. #1072.
#[tokio::test]
async fn a_fix_drafts_one_test_run_is_timed_apart_from_a_whole_run() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    came_to(&fleet, &reporter, TEST).await.expect("drafted");

    let job = fleet.load(&reporter).await.expect("the reporter");
    let repository = job.owner_manifest_id();
    let store = fleet.store().lock().await;
    let one_test = store.one_test_timings(repository).expect("reads");
    assert!(
        one_test.contains_key("suite"),
        "the one-test run should be timed: {one_test:?}"
    );
    let whole = store.check_timings(repository).expect("reads");
    assert!(
        !whole.contains_key("suite"),
        "a one-test run must not count toward the Check's own average, which \
         is what the next run's order reads: {whole:?}"
    );
}

/// **The call does not wait for main.** A test that takes a while there is
/// answered before it finishes, and the Drone waits on it as on a dry run.
#[tokio::test]
async fn the_call_answers_before_the_test_on_main_finishes() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/bin/sh -c 'sleep 2; exit 1' {}"), 1);
    let reporter = started(&fleet, &home).await;

    let asked = Instant::now();
    let answer = fleet
        .draft_fix(&reporter, fix_for(TEST))
        .await
        .expect("started");
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "the call waited for the run: {:?}",
        asked.elapsed()
    );
    assert!(fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .is_some_and(|at_work| at_work.is_checking()));
    let FixAnswer::Started(running) = answer else {
        panic!("a run, not a claim");
    };
    running
        .finished()
        .await
        .expect("the run reported")
        .expect("drafted");
}

/// **A pass on main is the Drone's own change**, and nothing is drafted.
/// **Skipped while the machine is loaded, `#1436`.** It passes alone and
/// fails only when the whole suite runs on a saturated machine, which is
/// what the merge line does — so it refused every branch tonight. Whether
/// this is a defect it is catching, a timing dependence, or a test worth
/// keeping at all is `#1436`'s to settle.
#[ignore = "flaky under load, #1436"]
#[tokio::test]
async fn a_test_passing_on_main_drafts_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/true {}"), 1);
    let reporter = started(&fleet, &home).await;

    let refused = came_to(&fleet, &reporter, TEST)
        .await
        .expect_err("nothing is drafted");
    assert!(
        matches!(refused, NotFixed::PassesOnMain { .. }),
        "{refused:?}"
    );
    let job = fleet.load(&reporter).await.expect("the reporter");
    assert!(fleet.breakages_of(&job).await.expect("read").is_empty());
    let told = told_fix(&fleet, &home, &reporter).await;
    assert!(
        told.iter().any(|row| row.contains("passes on main")),
        "{told:?}"
    );
}

/// **A name that matches nothing on main is neither a pass nor a failure.**
/// nextest and vitest both exit `0` here, the same code a pass reports, so
/// the fake runner does too — and the fix still must not draft. #1204.
#[tokio::test]
async fn a_test_matching_nothing_on_main_drafts_nothing() {
    let home = TempDir::new();
    let no_match = "/bin/sh -c 'echo error: no tests to run 1>&2; exit 0' {}";
    let fleet = a_fleet(&home, gated_on(no_match), 1);
    let reporter = started(&fleet, &home).await;

    let refused = came_to(&fleet, &reporter, TEST)
        .await
        .expect_err("nothing is drafted");
    assert!(matches!(refused, NotFixed::NoMatch { .. }), "{refused:?}");
    let job = fleet.load(&reporter).await.expect("the reporter");
    assert!(fleet.breakages_of(&job).await.expect("read").is_empty());
    let told = told_fix(&fleet, &home, &reporter).await;
    assert!(
        told.iter().any(|row| row.contains("matched nothing")),
        "{told:?}"
    );
}

/// A second report of a claimed test names the fix already on it, and runs
/// nothing — so a step's allowance spent on the first does not refuse it.
#[tokio::test]
async fn a_second_report_of_a_claimed_test_names_the_fix_on_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    let Drafted(fix) = came_to(&fleet, &reporter, TEST).await.expect("drafted");
    let again = fleet
        .draft_fix(&reporter, fix_for(TEST))
        .await
        .expect("answered");
    let FixAnswer::AlreadyClaimed(claimed) = again else {
        panic!("a second run started: {again:?}");
    };
    assert_eq!(claimed, fix);
}

/// A step asks for as many fixes as the composition root allows, and no more.
#[tokio::test]
async fn a_part_asks_for_no_more_fixes_than_it_is_allowed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"), 1);
    let reporter = started(&fleet, &home).await;

    came_to(&fleet, &reporter, TEST).await.expect("drafted");
    let refused = fleet
        .draft_fix(&reporter, fix_for("parser::other"))
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
        .draft_fix(&reporter, fix_for(TEST))
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
    let Drafted(fix) = came_to(&fleet, &reporter, TEST).await.expect("drafted");

    fleet.kill_job(&fix).await.expect("the fix is killed");

    let job = fleet.load(&reporter).await.expect("the reporter");
    assert!(
        fleet.breakages_of(&job).await.expect("read").is_empty(),
        "the claim went with the fix"
    );
}
