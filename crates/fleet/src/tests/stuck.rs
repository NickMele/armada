//! Why a Job is stuck, as `GET /jobs/:job_id` answers it.
//!
//! # These assert agreement, not arithmetic
//!
//! `core_model::Stuck` is tested against the rule in `core-model`'s own suite,
//! case by case, and none of that is repeated here. What is here is the thing
//! that suite cannot reach: **the classification and the act have to agree.**
//! An act that refuses what the classification offered is worse than either
//! alone — a person is told to press a button that returns a 409 — so every
//! case below asserts the offer and then makes the call.
//!
//! # The worktree is deleted by hand, and that is the case
//!
//! On 2026-08-28 a Drone's worktree was removed from outside Armada and the Job
//! escalated as `stalled`, which is the nearest trigger and the wrong condition.
//! Bridge offered a restart, because it derives recoverability from `status`,
//! `current_step_id` and `assigned_drone` and reads no filesystem. `a_restart`
//! is refused on such a Job, and until now nothing said so before the press.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent, WorktreeSpec};
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepTarget,
    StepVerdict, Target,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::Redirection;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";
const PLAN: &str = "plan";

/// The shape `#313` was hit on: a step that passes, and the step after it that
/// the Drone stalled on.
fn two_steps() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: PLAN,
            label: "Plan",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: IMPLEMENT,
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that stays up and never answers. **The pipe still accepts a write**,
/// which is the shape a redirect has when a Job is escalated over a live
/// process.
fn a_drone_that_stays() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called())
}

/// A Drone that speaks once and leaves, emptying the slot under an escalation.
///
/// The last of the eight copies `#443` found. It said `echo BUSY` and nothing
/// else, which leaves before it is told rather than after — see
/// [`planted::a_drone_that_leaves`](crate::tests::planted::a_drone_that_leaves)
/// for why that is a race against the spawn and not a Drone that left.
fn a_drone_that_leaves() -> FakeHarness {
    crate::tests::planted::a_drone_that_leaves("BUSY").reading("BUSY", called())
}

fn a_fleet_with(home: &TempDir, harness: FakeHarness) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    Fleet::assembled(fittings)
}

async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.unwrap();
    settled(fleet).await;
    job.id().clone()
}

/// Where the liveness vigil leaves a Job: `escalated` on `stalled`, no step
/// stopped, the Drone still in the slot.
async fn stalled(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = started(fleet, home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
        )
        .await
        .unwrap();
    job
}

/// A Job whose **step** stopped: the gate refused it, then the status moved,
/// which is the only order the machines admit.
async fn refused(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = started(fleet, home).await;
    let record = fleet.load(&job).await.unwrap();
    let record = fleet
        .move_step(
            &record,
            &StepId::new(IMPLEMENT),
            StepTarget::Stopped(
                StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
            ),
        )
        .await
        .unwrap();
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::GateFailure),
            Actor::Fleet,
        )
        .await
        .unwrap();
    job
}

/// What somebody outside Armada does with `rm -rf`, and what `armada clean`
/// does on purpose.
fn delete_the_worktree(home: &TempDir, job: &JobId) {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), job.as_str()).expect("a legal spec");
    std::fs::remove_dir_all(spec.worktree_path()).expect("a worktree that was there");
}

/// Until the slot is empty. **A Drone leaving is a turn of the loop, not a
/// sleep**: the process ends on its own and the reaper is what notices, which
/// is the same path a Drone dying in the field takes.
async fn until_reaped(fleet: &Fixture) {
    for _ in 0..400 {
        fleet.turn().await.expect("a turn");
        if fleet.the_only_slot().await.lock().await.is_none() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the Drone never left");
}

async fn settled(fleet: &Fixture) {
    let mut steady = 0;
    let mut last = usize::MAX;
    for _ in 0..400 {
        let seen = fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .map_or(0, |at| at.turned());
        steady = if seen == last { steady + 1 } else { 0 };
        if steady > 20 {
            return;
        }
        last = seen;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

/// One Job as the wire serves it.
async fn detail(fleet: &Fixture, job: &JobId) -> ipc::JobDetail {
    api::Queries::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists")
}

/// A Job that is going is not stuck, and the field is absent rather than empty
/// — an empty list is a dead end, which a running Job is not.
#[tokio::test]
async fn a_running_job_carries_no_classification() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_stays());
    let job = started(&fleet, &home).await;

    let detail = detail(&fleet, &job).await;

    assert_eq!(detail.job.status.domain(), JobStatus::Running);
    assert!(detail.stuck.is_none(), "nothing is wrong with it");
}

/// The `stalled` shape, end to end: the trigger says what stopped it and the
/// classification says what moves it, and the act it names is the act that
/// works.
#[tokio::test]
async fn a_stalled_job_is_told_it_is_redirected_and_the_redirect_lands() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_stays());
    let job = stalled(&fleet, &home).await;

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert_eq!(stuck.stopped_by.as_deref(), Some("stalled"));
    assert!(stuck.step_id.is_none(), "a Job-level trigger names no step");
    assert_eq!(
        spelled(&stuck),
        ["redirect_drone", "redispatch_job"],
        "a live Drone and no stopped step"
    );

    let said = Redirection::saying("read tests/parse.rs first").expect("something to act on");
    fleet
        .redirect(&job, &said)
        .await
        .expect("the act the classification named");
    assert!(
        matches!(
            fleet.restart_step(&job, None).await,
            Err(Adrift::NoStepStopped { .. })
        ),
        "and the one it withheld is the one Fleet refuses, for the reason the          classification withheld it: a Job-level escalation names no step"
    );
}

/// **The case this issue exists for.** The Drone is gone and the worktree was
/// deleted under it, so a restart has nothing to land on — and a person is told
/// that before pressing anything, which is the part that was missing.
#[tokio::test]
async fn a_job_whose_worktree_is_gone_is_told_only_a_redispatch_moves_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = refused(&fleet, &home).await;
    until_reaped(&fleet).await;

    let survived = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert!(
        survived.worktree_on_disk,
        "the worktree is there while the test sets up"
    );

    delete_the_worktree(&home, &job);

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert!(
        !stuck.worktree_on_disk,
        "and the fact crosses, not only its consequence"
    );
    assert_eq!(
        spelled(&stuck),
        ["redispatch_job"],
        "nothing lands on a worktree that is not there"
    );
    assert!(
        matches!(
            fleet.restart_step(&job, None).await,
            Err(Adrift::WorktreeGone { .. })
        ),
        "which is exactly what the act says on the press"
    );
}

/// **The wire, not just the enum.** `#126`: `WorktreeGone` fell through the
/// daemon's match to the 500 catch-all — an excellent message on a status
/// code that told the caller Fleet broke, when it had correctly refused.
#[tokio::test]
async fn a_restart_onto_a_gone_worktree_answers_409_over_the_wire() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = refused(&fleet, &home).await;
    until_reaped(&fleet).await;
    delete_the_worktree(&home, &job);

    let refusal = api::Commands::restart_step(&fleet, ipc::JobId::from(&job), None)
        .await
        .expect_err("the worktree was just deleted");

    assert!(matches!(refusal, api::Refusal::IllegalMove(_)));
    assert_eq!(refusal.status(), 409);
}

/// A refused step keeps its Drone, so the override and the redirect are both
/// there — and the override leads, because it takes nothing away.
#[tokio::test]
async fn a_refused_step_leads_with_the_act_that_takes_nothing_away() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_stays());
    let job = refused(&fleet, &home).await;

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");

    assert_eq!(stuck.stopped_by.as_deref(), Some("gate_failure"));
    assert_eq!(
        stuck.step_id.as_ref().map(ipc::StepId::as_str),
        Some(IMPLEMENT)
    );
    assert_eq!(
        spelled(&stuck),
        ["override_verdict", "redirect_drone", "redispatch_job"]
    );
}

/// The classification reads the slot rather than the record, for the reason
/// every other read of a live Drone does: the record's pointer survives a Fleet
/// restart and the pipe does not.
#[tokio::test]
async fn a_drone_that_left_turns_the_redirect_into_a_restart() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = refused(&fleet, &home).await;
    until_reaped(&fleet).await;

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");

    assert_eq!(
        spelled(&stuck),
        ["override_verdict", "restart_step", "redispatch_job"]
    );
    assert!(
        matches!(
            fleet
                .redirect(&job, &Redirection::saying("anything").unwrap())
                .await,
            Err(Adrift::NoDroneToRedirect { .. })
        ),
        "and the act it withheld is the one Fleet refuses"
    );
}

/// A step that stopped is one the machine wrote a verdict onto, and the
/// classification names it — which is what makes restarting *that step*
/// coherent.
#[tokio::test]
async fn the_step_named_is_the_step_that_stopped() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_stays());
    let job = refused(&fleet, &home).await;

    let record = fleet.load(&job).await.unwrap();
    let (step, _) = record.stopped_on().expect("a stopped step");
    assert_eq!(
        record.step(step).map(|row| row.state()),
        Some(StepState::Stopped)
    );

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert_eq!(
        stuck.step_id.as_ref().map(ipc::StepId::as_str),
        Some(step.as_str())
    );
}

/// **The case `#313` was filed for**, on the Job that is still `running`
/// because nothing noticed the Drone had gone quiet.
///
/// Ending the Drone stops the step it was on, so the classification names a
/// step and the act it offers is the one Fleet takes. Before this, the step and
/// its attempt sat `running` beneath a Job that was `escalated` with no Drone
/// on it, and the only act left was redispatching the whole Job.
#[tokio::test]
async fn killing_a_running_jobs_drone_leaves_a_step_a_restart_lands_on() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_stays());
    let job = started(&fleet, &home).await;

    fleet.kill_drone(&job).await.expect("the Drone ends");

    let record = fleet.load(&job).await.unwrap();
    assert_eq!(record.status(), JobStatus::Escalated);
    let row = record.step(&StepId::new(IMPLEMENT)).expect("the step");
    assert_eq!(row.state(), StepState::Stopped);
    assert_eq!(
        row.last_verdict(),
        Some(StepVerdict::Failed(
            StepLevelTrigger::of(EscalationTrigger::DroneKilled).expect("a step-level trigger")
        )),
        "the row says why, so a person a week later reads a reason and not an absence"
    );

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert_eq!(stuck.stopped_by.as_deref(), Some("drone_killed"));
    assert_eq!(
        stuck.step_id.as_ref().map(ipc::StepId::as_str),
        Some(IMPLEMENT)
    );
    assert_eq!(
        spelled(&stuck),
        ["restart_step", "redispatch_job"],
        "no override: nothing weighed the work, so there is no verdict to disagree with"
    );

    fleet
        .restart_step(&job, None)
        .await
        .expect("the act the classification named");
}

/// The same dead end reached the other way, which is the one the issue's title
/// names: the vigil escalated the Job on `stalled` first, over a Drone that was
/// still there, and a person ended it afterwards.
///
/// **The Job does not move and the step does.** `Aftermath::AlreadyStopped`
/// already said this Job becomes restartable rather than redirectable; nothing
/// wrote the step move that would make it true, because `escalated` freezes the
/// inner machine. It crosses on `step_machine`'s named exception now.
#[tokio::test]
async fn killing_a_stalled_jobs_drone_turns_the_redirect_into_a_restart() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_stays());
    let job = stalled(&fleet, &home).await;

    let before = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert_eq!(spelled(&before), ["redirect_drone", "redispatch_job"]);

    fleet.kill_drone(&job).await.expect("the Drone ends");

    let record = fleet.load(&job).await.unwrap();
    assert_eq!(
        record.status(),
        JobStatus::Escalated,
        "a Job that had already stopped does not stop again"
    );
    assert_eq!(
        record.step(&StepId::new(IMPLEMENT)).map(|row| row.state()),
        Some(StepState::Stopped)
    );

    let stuck = detail(&fleet, &job).await.stuck.expect("it stopped");
    assert_eq!(
        spelled(&stuck),
        ["restart_step", "redispatch_job"],
        "the redirect goes with the Drone and the restart arrives with the stopped step"
    );
    // **What it stopped for is not lost.** `stalled` is what the vigil
    // recorded on the Job's own transition and it is still there; the step
    // carries what became of it, which is a different sentence about a
    // different machine.
    assert_eq!(
        detail(&fleet, &job)
            .await
            .job
            .reason
            .and_then(|reason| reason.named),
        Some(String::from("stalled"))
    );
    assert_eq!(stuck.stopped_by.as_deref(), Some("drone_killed"));
}

/// The half the issue is actually about: **the step that already passed
/// survives the recovery.**
///
/// A Job on its second step, killed there. Restarting takes the second step
/// again and the first keeps its state and its verdict, which is the 6m29s of
/// work redispatching would have thrown away.
#[tokio::test]
async fn a_restart_after_a_kill_keeps_the_step_that_already_advanced() {
    let home = TempDir::new();
    let mut fittings = fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        a_drone_that_stays(),
    );
    fittings.workflows = one(two_steps());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    let fleet: Fixture = Fleet::assembled(fittings);

    let job = started(&fleet, &home).await;
    // The first step passes and the second is entered — moved rather than
    // driven, for `crate::tests::restarting`'s reason: what is under test here
    // is the kill and not the gate.
    let record = fleet.load(&job).await.unwrap();
    let record = fleet
        .move_step(&record, &StepId::new(PLAN), StepTarget::Advanced)
        .await
        .unwrap();
    fleet
        .move_step(&record, &StepId::new(IMPLEMENT), StepTarget::Running)
        .await
        .unwrap();

    fleet.kill_drone(&job).await.expect("the Drone ends");
    fleet
        .restart_step(&job, None)
        .await
        .expect("a restart lands");

    let record = fleet.load(&job).await.unwrap();
    let plan = record.step(&StepId::new(PLAN)).expect("the first step");
    assert_eq!(
        plan.state(),
        StepState::Advanced,
        "the step that passed is untouched by the recovery"
    );
    assert_eq!(plan.last_verdict(), Some(StepVerdict::Passed));
}

/// The acts, as the wire spells them.
fn spelled(stuck: &ipc::Stuck) -> Vec<&str> {
    stuck
        .recourse
        .iter()
        .map(ipc::Recourse::as_wire)
        .collect::<Vec<_>>()
}
