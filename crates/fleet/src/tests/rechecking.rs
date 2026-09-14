//! Running a stopped step's Checks again, from a real gate. #1105.
//!
//! **Every case starts where the act does**: a Job proposed, worked and
//! submitted, a Check that fails with no budget left, and the Job held for
//! repair with its Drone gone. What changes between cases is a file beside the
//! worktree that the Check reads, so the work itself is never touched.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::WorktreeSpec;
use core_model::{
    Attempt, EscalationTrigger, JobId, JobStatus, Recourse, Spent, StepId, StepLevelTrigger,
    StepState, StepVerdict,
};
use ipc::RunId;
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::rechecking::Unrecheckable;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_holding, a_proposal, diff_evidence, worktree_directory};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::transcript::log_of;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn implement() -> StepId {
    StepId::new("implement")
}

/// The Check. It notes each run, waits while `hold` stands until `go` does, and
/// passes only once `pass` exists — all beside the worktree, never in it.
fn the_check(home: &TempDir) -> String {
    let at = home.path().display();
    format!(
        "/bin/sh -c 'echo ran >> {at}/runs; if [ -f {at}/hold ]; then while [ ! -f {at}/go ]; \
         do sleep 0.05; done; fi; test -f {at}/pass'"
    )
}

/// Two steps and no retry budget, so the first failed Check holds the Job.
fn gated_then_summarised(run: &str) -> config::ResolvedWorkflow {
    testkit::retried(
        &[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[Gate::Check {
                    name: "suite",
                    run,
                    expect_exit_code: 0,
                    when: &[],
                }],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
            Sketch {
                id: "summarise",
                label: "Summarise",
                evidence_type: Some("facts_note"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
        ],
        0,
    )
}

/// A Job whose Check failed at a real gate and is held for repair.
async fn held_for_repair(home: &TempDir) -> (Arc<Fixture>, JobId) {
    let fleet = Arc::new(a_fleet_holding(
        home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_then_summarised(&the_check(home)),
        1,
    ));
    let job = fleet
        .propose(a_proposal("register the route"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.expect("released to run");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate ran");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Failed { .. })),
        "the fixture did not fail a Check: {:?}",
        turned.ruled()
    );
    let held = fleet.load(job.id()).await.expect("the Job reads");
    assert_eq!(held.status(), JobStatus::AwaitingRepair);
    assert!(fleet.working_on().await.is_empty(), "the Drone stood down");
    (fleet, job.id().clone())
}

/// The run the Drone worked and the retry budget it spent.
async fn counts(fleet: &Fixture, job: &JobId) -> (Attempt, Spent) {
    let store = fleet.store().lock().await;
    (
        store.step_attempt(job, &implement()).expect("runs counted"),
        store.step_spent(job, &implement()).expect("spend counted"),
    )
}

fn runs(home: &TempDir) -> usize {
    std::fs::read_to_string(home.path().join("runs"))
        .unwrap_or_default()
        .lines()
        .count()
}

fn logged(home: &TempDir, handle: &str) -> String {
    std::fs::read_to_string(log_of(&home.path().to_string_lossy(), handle)).unwrap_or_default()
}

/// Whether the classification a person reads offers the act.
async fn offered(fleet: &Fixture, job: &JobId) -> Vec<Recourse> {
    let record = fleet.load(job).await.expect("the Job reads");
    let ran = fleet
        .store()
        .lock()
        .await
        .step_checks(job)
        .expect("the runs read");
    fleet
        .why_stuck(&record, None, &ran)
        .await
        .map(|stuck| stuck.recourse().to_vec())
        .unwrap_or_default()
}

// ------------------------------------------------------------ the act itself

/// **The whole of #1105.** The Check failed on something the work did not
/// cause, the cause lifts, and a person presses: the step advances on the
/// gate's own pass, no Drone starts, and the retry budget is where it was.
#[tokio::test]
async fn a_check_that_passes_on_the_same_work_carries_the_step_on_with_no_drone() {
    let home = TempDir::new();
    let (fleet, job_id) = held_for_repair(&home).await;
    assert!(offered(&fleet, &job_id)
        .await
        .contains(&Recourse::RerunChecks));
    let before = counts(&fleet, &job_id).await;
    std::fs::write(home.path().join("pass"), "").expect("the cause lifts");

    let job = Arc::clone(&fleet)
        .rerun_checks(&job_id)
        .await
        .expect("the Checks run again");

    assert_eq!(
        job.status(),
        JobStatus::Queued,
        "the next step's Drone is admission's to start"
    );
    let row = job.step(&implement()).expect("the row");
    assert_eq!(row.state(), StepState::Advanced);
    assert_eq!(
        row.last_verdict(),
        Some(StepVerdict::Passed),
        "the gate passed it, which is what makes this not an override"
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "no Drone started on work that was already fine"
    );
    assert_eq!(
        counts(&fleet, &job_id).await,
        before,
        "no run opened and no retry spent"
    );
    assert_eq!(runs(&home), 2, "the Check ran a second time");
    assert!(logged(&home, &job.handle()).contains("\"came_to\":\"advanced\""));
}

/// **A Check still failing moves nothing**, and the second run is on the
/// record where a person reads it.
#[tokio::test]
async fn a_check_that_still_fails_leaves_the_job_held_with_the_new_run_recorded() {
    let home = TempDir::new();
    let (fleet, job_id) = held_for_repair(&home).await;
    let before = counts(&fleet, &job_id).await;

    let job = Arc::clone(&fleet)
        .rerun_checks(&job_id)
        .await
        .expect("the Checks run again and answer");

    assert_eq!(job.status(), JobStatus::AwaitingRepair);
    let row = job.step(&implement()).expect("the row");
    assert_eq!(row.state(), StepState::Stopped);
    assert_eq!(
        row.last_verdict(),
        StepLevelTrigger::of(EscalationTrigger::GateFailure).map(StepVerdict::Failed)
    );
    assert_eq!(runs(&home), 2, "the Check ran a second time");
    let failed = fleet
        .store()
        .lock()
        .await
        .step_checks(&job_id)
        .expect("the runs read")
        .into_iter()
        .filter(|(step, _)| *step == implement())
        .flat_map(|(_, checks)| checks)
        .any(|check| !check.outcome.advances());
    assert!(failed, "the new run is on the record");
    assert_eq!(counts(&fleet, &job_id).await, before);
    assert!(logged(&home, &job.handle()).contains("\"came_to\":\"failed\""));
    assert!(
        offered(&fleet, &job_id)
            .await
            .contains(&Recourse::RerunChecks),
        "and a person may press again"
    );
}

// ------------------------------------------ what it must not be able to do

/// **Refused where it is not offered**, and the offer is read off the same
/// facts: a Job that never stopped, and a worktree that is gone.
#[tokio::test]
async fn it_is_refused_exactly_where_it_is_not_offered() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_holding(
        &home,
        FakeWorkProduct::changed(&["src/routes.rs"]),
        gated_then_summarised(&the_check(&home)),
        1,
    ));
    let proposed = fleet
        .propose(a_proposal("register the route"))
        .await
        .expect("a Job at the approval gate");
    assert!(matches!(
        Arc::clone(&fleet).rerun_checks(proposed.id()).await,
        Err(Adrift::CannotRerunChecks {
            why: Unrecheckable::NotHeldForRepair { .. },
            ..
        })
    ));

    let home = TempDir::new();
    let (fleet, job_id) = held_for_repair(&home).await;
    let job = fleet.load(&job_id).await.expect("the Job reads");
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    std::fs::remove_dir_all(spec.worktree_path()).expect("the worktree is reclaimed");

    assert!(!offered(&fleet, &job_id)
        .await
        .contains(&Recourse::RerunChecks));
    assert!(matches!(
        Arc::clone(&fleet).rerun_checks(&job_id).await,
        Err(Adrift::WorktreeGone { .. })
    ));
    assert_eq!(runs(&home), 1, "nothing ran");
}

/// **While the Checks run again, nothing takes the worktree from them**: a
/// restart and a redispatch are refused, a second press is refused, and the
/// classification offers none of the three.
#[tokio::test]
async fn while_the_checks_run_again_nothing_else_is_offered_or_taken() {
    let home = TempDir::new();
    let (fleet, job_id) = held_for_repair(&home).await;
    std::fs::write(home.path().join("hold"), "").expect("the next run waits");
    let rerun = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        let job_id = job_id.clone();
        async move { fleet.rerun_checks(&job_id).await.map(|job| job.status()) }
    });
    let mut waited = 0;
    while !fleet.rechecking().holds(&job_id) {
        assert!(waited < 400, "the re-run never started");
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 1;
    }

    assert!(matches!(
        fleet.restart_step(&job_id, None).await,
        Err(Adrift::ChecksRunningAgain { .. })
    ));
    assert!(matches!(
        fleet.redispatch(&job_id).await,
        Err(Adrift::ChecksRunningAgain { .. })
    ));
    assert!(matches!(
        Arc::clone(&fleet).rerun_checks(&job_id).await,
        Err(Adrift::CannotRerunChecks {
            why: Unrecheckable::AlreadyRerunning,
            ..
        })
    ));
    assert_eq!(offered(&fleet, &job_id).await, Vec::<Recourse>::new());

    std::fs::write(home.path().join("go"), "").expect("the run goes on");
    assert_eq!(
        rerun.await.expect("the task ends").expect("it answers"),
        JobStatus::AwaitingRepair
    );
    assert!(!fleet.rechecking().holds(&job_id), "given back");
    assert!(offered(&fleet, &job_id)
        .await
        .contains(&Recourse::RestartStep));
}

/// **A Job that left `awaiting_repair` while its Checks ran is not carried
/// on.** A person killed it mid-run; the Checks then pass, and the re-run
/// refuses rather than taking a killed Job back to `running`.
#[tokio::test]
async fn a_job_that_moved_while_its_checks_ran_is_not_carried_on() {
    let home = TempDir::new();
    let (fleet, job_id) = held_for_repair(&home).await;
    std::fs::write(home.path().join("hold"), "").expect("the next run waits");
    std::fs::write(home.path().join("pass"), "").expect("and passes");
    let rerun = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        let job_id = job_id.clone();
        async move { fleet.rerun_checks(&job_id).await }
    });
    let mut waited = 0;
    while runs(&home) < 2 {
        assert!(waited < 400, "the Check never started again");
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 1;
    }

    fleet.kill_job(&job_id).await.expect("a person kills it");
    std::fs::write(home.path().join("go"), "").expect("the run goes on");

    assert!(matches!(
        rerun.await.expect("the task ends"),
        Err(Adrift::CannotRerunChecks {
            why: Unrecheckable::NotHeldForRepair { .. },
            ..
        })
    ));
    assert_eq!(
        fleet.load(&job_id).await.expect("the Job reads").status(),
        JobStatus::Killed
    );
}

// ------------------------------------------------------------ the wire

/// The route takes no body and answers the Job; a Job it no longer applies to
/// is 409.
#[tokio::test]
async fn the_route_takes_no_body_and_answers_the_job() {
    let home = TempDir::new();
    let (fleet, job_id) = held_for_repair(&home).await;
    std::fs::write(home.path().join("pass"), "").expect("the cause lifts");
    let fleet = Arc::try_unwrap(fleet)
        .ok()
        .expect("the only handle on the Fleet");

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let path = format!("/jobs/{}/rerun_checks", job_id.as_str());
    let (status, body) = call(&app, "POST", &path, "").await;

    assert_eq!(status, 200);
    let job: ipc::JobSummary = ipc::decode("a Job", &body).expect("a JobSummary");
    assert_eq!(job.status.as_wire(), "queued");

    let (status, _) = call(&app, "POST", &path, "").await;
    assert_eq!(status, 409, "a Job no longer held for repair");
}
