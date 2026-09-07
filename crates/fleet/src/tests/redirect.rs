//! Steering a Drone that is alive on a Job a person is holding.
//!
//! # The case these exist for is the one that had no move
//!
//! `stalled` is Job-level: the vigil escalates without stopping a step, and the
//! resume predicate wanted a stopped step for both acts — which left a Drone
//! **alive and holding its session** with kill-and-redispatch as its only move.
//!
//! Why there are four modules: `admitting` is which Job takes the act at all,
//! `rousing` is the wait between the send and the Drone's answer, `unfreezing`
//! is what the act does to the step underneath, and `waiting` is what a person
//! reads while it is outstanding. The states they start from are here, because
//! four modules arguing about four differently-escalated Jobs prove nothing.
//!
//! # The Job is escalated by hand, and that is not a shortcut
//!
//! `silence` already proves the vigil reaches `stalled` over a live process at
//! the real threshold. These are about the act that follows, so they take the
//! state the vigil produces — `escalated`, no step stopped, a Drone in the slot
//! — and start there. Driving the clock again would test the vigil twice and
//! this act nowhere.
//!
//! **Nothing here reads what a Drone said.** Whether it turned is a count of
//! the events it produced, which is why a heartbeat cannot satisfy it and why
//! `/bin/sh` can stand in for one.

mod admitting;
mod rousing;
mod unfreezing;
mod waiting;

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepTarget,
    Target,
};
use store::Moved;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::resume::{Redirection, Roused};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The step every case below is on.
const IMPLEMENT: &str = "implement";

/// What a person says. Never empty — `Redirection` is that guard.
fn advice() -> Redirection {
    Redirection::saying("the failing test is in tests/parse.rs — read it before writing anything")
        .expect("an instruction with something in it")
}

/// One step, gated on nothing, so nothing but this act can move the Job.
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

/// One tool call — the cheapest thing a Drone that is turning emits.
fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that says one thing and answers every turn put to it.
fn a_drone_that_answers() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo ANSWERED; done",
        ],
    )
    .reading("BUSY", called())
    .reading("ANSWERED", called())
}

/// A Drone that takes what is written to it and never speaks again. **The pipe
/// still accepts the write**: a live process with an unread stdin is the shape
/// a redirect landing on nobody has.
fn a_drone_that_never_wakes() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called())
}

/// A Fleet on one step with that Drone on it. **The Judge fails every call**:
/// nothing here may ask a model anything.
fn a_fleet_with(home: &TempDir, harness: FakeHarness) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.workflows = one(one_step());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a redirect"));
    Fleet::assembled(fittings)
}

/// Approve a Job with a worktree on disk, and wait until its Drone has said
/// what it starts with and answered its brief — so no event from before a
/// redirect can arrive after the baseline that redirect takes.
async fn started(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    dispatched(&fleet, job.id()).await.unwrap();
    settled(fleet).await;
    job.id().clone()
}

/// Where the liveness vigil leaves a Job: `escalated` on `stalled`, its step
/// still `running`, its Drone still in the slot. The escalation goes through
/// the same call `silence` makes, so what these start from is what it produces.
async fn stalled(fleet: &Fixture, home: &TempDir) -> JobId {
    let job = started(fleet, home).await;
    let record = fleet.load(&job).await.unwrap();
    assert_eq!(
        record.status(),
        JobStatus::Running,
        "the Drone was admitted"
    );
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

/// A Job where the **step** stopped: the gate refused it, the Job escalated on
/// a step-level trigger, the Drone alive beside it. The shape a redirect has
/// always had. The step stops before the status moves, which is the only order
/// the inner machine admits — see `dispatch`.
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

/// Wait until nothing more is arriving. **Not a fixed sleep** — what matters is
/// that the transcript has stopped moving, however long that takes.
async fn settled(fleet: &Fixture) {
    let mut steady = 0;
    let mut last = usize::MAX;
    for _ in 0..400 {
        let now = heard(fleet).await;
        steady = if now == last { steady + 1 } else { 0 };
        if steady == 10 && now > 0 {
            return;
        }
        last = now;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never stopped talking");
}

/// How much the Drone in the slot has said. A count, never the content.
async fn heard(fleet: &Fixture) -> usize {
    let held = fleet.the_only_slot().await;
    let slot = held.lock().await;
    slot.as_ref().map(|at| at.heard().len()).unwrap_or_default()
}

/// Turn until the Drone answers the redirect.
async fn until_roused(fleet: &Fixture) -> Roused {
    for _ in 0..400 {
        let turned = fleet.turn().await.expect("a turn");
        if let Some(roused) = turned.each.into_iter().find_map(|worked| worked.roused) {
            return roused;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never turned");
}

/// How many status moves the Job has made. **The reading that catches a move
/// nobody meant to make**, which a last-mover reading cannot: a redirect that
/// moved a `running` Job to `running` would be recorded against a person and
/// read exactly like one that moved nothing.
async fn job_moves(fleet: &Fixture, job: &JobId) -> usize {
    fleet
        .store()
        .lock()
        .await
        .events_for(job)
        .expect("the Job's log")
        .iter()
        .filter(|event| matches!(event.moved(), Moved::Job { .. }))
        .count()
}

/// Which state the one step is in.
async fn step_state(fleet: &Fixture, job: &JobId) -> StepState {
    fleet
        .load(job)
        .await
        .unwrap()
        .step(&StepId::new(IMPLEMENT))
        .expect("the step being worked")
        .state()
}
