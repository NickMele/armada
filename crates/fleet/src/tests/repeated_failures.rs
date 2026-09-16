//! Fleet noticing, with no Drone asking, that a step's Check failed on the
//! same test twice in a row — and probing main the way `draft_fix` does when
//! asked. #828, #999, #1001.
//!
//! The whole-suite Check prints real nextest summary shape rather than a bare
//! failing exit code, because that shape is what `spotted_repeats` reads —
//! `crate::fixing::repeated`'s own doc says how strict the match is.
//!
//! **The script is a file in the worktree, not an inline `-c` string.** The
//! Manifest's `run:` is YAML, wrapped in double quotes with no escaping at
//! all — a nextest summary line's own `(n/m)` needs shell-quoting to survive,
//! and there is no quote character both YAML and a single inline command
//! line leave free. A file has neither constraint.

use std::sync::Arc;

use adapter_traits::WorktreeSpec;
use config::ResolvedWorkflow;
use core_model::{Job, JobId, StepId};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, OneTest, Sketch};

use crate::daemon::Fleet;
use crate::fixing::{Drafted, NotFixed, Repeat};
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_over, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const COMMIT: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const RUN_THE_SCRIPT: &str = "sh check.sh";

/// A whole-suite run naming one failing test, in nextest's own summary shape.
fn names_one(test: &str) -> String {
    format!(
        "echo 'Summary [   0.010s] 1 tests run: 0 passed, 1 failed, 0 skipped'\n\
         echo 'FAIL [   0.010s] (1/1) nt {test}'\n\
         exit 1\n"
    )
}

/// The same shape, naming two — the ambiguous case nothing should fire on.
const NAMES_TWO: &str = "echo 'Summary [   0.010s] 2 tests run: 0 passed, 2 failed, 0 skipped'\n\
     echo 'FAIL [   0.010s] (1/2) nt tests::fails_one'\n\
     echo 'FAIL [   0.010s] (2/2) nt tests::fails_two'\n\
     exit 1\n";

/// One test the first time this worktree's script runs, and a different one
/// every time after — a marker file left behind is what tells the two apart.
const NAMES_A_DIFFERENT_TEST_EACH_ATTEMPT: &str = "\
    if [ -f .attempted ]; then NAME=tests::fails_two; else touch .attempted; NAME=tests::fails_one; fi\n\
    echo 'Summary [   0.010s] 1 tests run: 0 passed, 1 failed, 0 skipped'\n\
    echo \"FAIL [   0.010s] (1/1) nt $NAME\"\n\
    exit 1\n";

/// A step retried twice, gated on `suite`, which also runs one test by name.
fn gated_on(one_test: &str) -> ResolvedWorkflow {
    testkit::retried_and_testing_one(
        &[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::Check {
                name: "suite",
                run: RUN_THE_SCRIPT,
                expect_exit_code: 0,
                when: &[],
            }],
            judged_on: &[],
            scope: None,
            gaming: None,
        }],
        2,
        &[OneTest {
            check: "suite",
            run: one_test,
        }],
    )
}

fn a_fleet(home: &TempDir, workflow: ResolvedWorkflow) -> Arc<Fixture> {
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "sleep 30"]),
        FakeVcs::new().with_ref_at("main", COMMIT),
    );
    fittings.starting().workflows = one(workflow);
    std::fs::create_dir_all(home.path().join(".armada").join("bases").join(COMMIT))
        .expect("the checkout of main is on disk");
    Arc::new(Fleet::assembled(fittings))
}

/// The worktree `check.sh` runs in — the same derivation Fleet uses, per
/// `worktree_directory`'s own doc on why a case cannot take a shortcut here.
fn worktree_of(home: &TempDir, job: &Job) -> String {
    WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle())
        .expect("a legal spec")
        .worktree_path()
}

/// Start the Job and write the script its step's Check runs.
async fn started(fleet: &Arc<Fixture>, home: &TempDir, script: &str) -> JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("a proposed Job");
    worktree_directory(home, &job);
    let script_path = std::path::Path::new(&worktree_of(home, &job)).join("check.sh");
    std::fs::write(script_path, script).expect("the script written");
    dispatched(&fleet, job.id()).await.expect("an approved Job");
    job.id().clone()
}

/// One submission and the turn that rules on it — the whole [`Turned`], since
/// `Ruling` carries no `Clone` and a caller reads both the ruling and the
/// repeats it spotted off the one turn.
async fn attempt(fleet: &Arc<Fixture>) -> crate::turning::Turned {
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("taken");
    fleet.turn().await.expect("a turn")
}

fn implement() -> StepId {
    StepId::new("implement")
}

fn suite_names(test: &str) -> Repeat {
    Repeat {
        step: implement(),
        check: "suite".to_string(),
        test: test.to_string(),
    }
}

/// Every repeat one turn's ruling spotted. `repeats` is a crate-visible field
/// on [`crate::turning::Worked`] rather than a public method, so a fixture
/// reads it the way every other `Worked` field is read in this suite.
fn repeats(turned: &crate::turning::Turned) -> Vec<Repeat> {
    turned
        .each
        .iter()
        .flat_map(|worked| worked.repeats.clone())
        .collect()
}

/// **The claim of the issue.** The same Check names the same failing test on
/// two consecutive attempts, with no Drone asking, and the repeat is spotted
/// on the second one — before anything about main is run.
#[tokio::test]
async fn a_second_attempt_naming_the_same_test_spots_a_repeat() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"));
    started(&fleet, &home, &names_one("tests::fails_one")).await;

    let first = attempt(&fleet).await;
    assert!(matches!(first.ruled(), Some(Ruling::HandedBack { .. })));
    assert!(repeats(&first).is_empty(), "nothing to compare against yet");

    let second = attempt(&fleet).await;
    assert!(matches!(second.ruled(), Some(Ruling::HandedBack { .. })));
    assert_eq!(repeats(&second), vec![suite_names("tests::fails_one")]);
}

/// A different test on the second attempt is not the same repeat.
#[tokio::test]
async fn a_different_test_on_the_second_attempt_spots_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"));
    started(&fleet, &home, NAMES_A_DIFFERENT_TEST_EACH_ATTEMPT).await;
    attempt(&fleet).await;

    let second = attempt(&fleet).await;
    assert!(matches!(second.ruled(), Some(Ruling::HandedBack { .. })));
    assert!(repeats(&second).is_empty());
}

/// Two failing tests on both attempts cannot say which of the other's two is
/// the same one — the ambiguous case this module's own doc names.
#[tokio::test]
async fn several_failing_tests_on_both_attempts_spots_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"));
    started(&fleet, &home, NAMES_TWO).await;
    attempt(&fleet).await;

    let second = attempt(&fleet).await;
    assert!(matches!(second.ruled(), Some(Ruling::HandedBack { .. })));
    assert!(repeats(&second).is_empty());
}

/// **Failed on main drafts and claims it**, exactly `draft_fix`'s own answer.
#[tokio::test]
async fn a_repeat_failing_on_main_too_drafts_a_fix_and_claims_the_test() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"));
    let reporter = started(&fleet, &home, &names_one("tests::fails_one")).await;
    attempt(&fleet).await;
    let second = attempt(&fleet).await;
    assert!(matches!(second.ruled(), Some(Ruling::HandedBack { .. })));

    let Drafted(fix) = fleet
        .probed_repeat(&reporter, &suite_names("tests::fails_one"))
        .await
        .expect("something to say")
        .expect("drafted");

    let record = fleet.load(&fix).await.expect("the fix Job");
    assert_eq!(record.status(), core_model::JobStatus::AwaitingApproval);
    assert_eq!(
        record.workflow_id(),
        fleet
            .load(&reporter)
            .await
            .expect("the reporter")
            .workflow_id(),
        "no Drone named a workflow, so Fleet reuses the retrying Job's own"
    );
    let job = fleet.load(&reporter).await.expect("the reporter");
    let claims = fleet.breakages_of(&job).await.expect("read");
    assert_eq!(claims.len(), 1, "the retrying Job's own claim is recorded");
    assert_eq!(claims[0].test, "tests::fails_one");
}

/// **A pass on main is the retrying Job's own change**, and nothing drafts.
#[tokio::test]
async fn a_repeat_passing_on_main_drafts_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/true {}"));
    let reporter = started(&fleet, &home, &names_one("tests::fails_one")).await;
    attempt(&fleet).await;
    attempt(&fleet).await;

    let came_to = fleet
        .probed_repeat(&reporter, &suite_names("tests::fails_one"))
        .await
        .expect("something to say");
    assert!(
        matches!(came_to, Err(NotFixed::PassesOnMain { .. })),
        "{came_to:?}"
    );
    let job = fleet.load(&reporter).await.expect("the reporter");
    assert!(fleet.breakages_of(&job).await.expect("read").is_empty());
}

/// A name that matches nothing on main is neither a pass nor a failure, so it
/// still drafts nothing. #1204's own answer, reached with no Drone asking.
#[tokio::test]
async fn a_repeat_matching_nothing_on_main_drafts_nothing() {
    let home = TempDir::new();
    let no_match = "/bin/sh -c 'echo error: no tests to run 1>&2; exit 0' {}";
    let fleet = a_fleet(&home, gated_on(no_match));
    let reporter = started(&fleet, &home, &names_one("tests::fails_one")).await;
    attempt(&fleet).await;
    attempt(&fleet).await;

    let came_to = fleet
        .probed_repeat(&reporter, &suite_names("tests::fails_one"))
        .await
        .expect("something to say");
    assert!(
        matches!(came_to, Err(NotFixed::NoMatch { .. })),
        "{came_to:?}"
    );
}

/// A test already claimed does not draft a second time — the same test named
/// by a later `draft_fix` call is pointed at the existing fix instead.
#[tokio::test]
async fn an_already_claimed_test_probes_nothing_a_second_time() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, gated_on("/usr/bin/false {}"));
    let reporter = started(&fleet, &home, &names_one("tests::fails_one")).await;
    attempt(&fleet).await;
    attempt(&fleet).await;

    let repeat = suite_names("tests::fails_one");
    fleet
        .probed_repeat(&reporter, &repeat)
        .await
        .expect("something to say")
        .expect("drafted");

    assert!(
        fleet.probed_repeat(&reporter, &repeat).await.is_none(),
        "the claim already answers it; nothing new runs"
    );
}
