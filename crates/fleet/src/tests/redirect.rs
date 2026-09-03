//! Steering a Drone that is alive on a Job a person is holding.
//!
//! # The case these exist for is the one that had no move
//!
//! `stalled` is Job-level, so the vigil escalates without stopping a step, and
//! the resume predicate asked for a stopped step on behalf of both acts — which
//! left a Drone **alive and holding its session** with kill-and-redispatch as
//! its only move.
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

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepTarget,
    Target, TransitionReason,
};
use store::Moved;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::{Redirection, Roused};
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

/// A Drone whose only answer is the harness's progress heartbeat — the
/// `tool_progress` line a long tool emits every thirty seconds, which this
/// vocabulary has no variant for. **A Drone that never stopped working, not one
/// that read anything**, which is why the vigil counts it as not-silent.
fn a_drone_that_only_ticks() -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            "echo BUSY; while IFS= read -r line; do echo HEARTBEAT; done",
        ],
    )
    .reading("BUSY", called())
    .reading(
        "HEARTBEAT",
        vec![DroneEvent::Unrecognised {
            kind: String::from("tool_progress"),
        }],
    )
}

/// A Drone that speaks once and leaves, emptying the slot under an escalation.
fn a_drone_that_leaves() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY"]).reading("BUSY", called())
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
    fleet.approve(job.id()).await.unwrap();
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

/// Turn until the slot empties: a Drone that left, reaped.
async fn until_reaped(fleet: &Fixture) {
    for _ in 0..400 {
        fleet.turn().await.expect("a turn");
        if fleet.the_only_slot().await.lock().await.is_none() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never left");
}

/// Who the Job's last status move is recorded against.
async fn last_mover(fleet: &Fixture, job: &JobId) -> Actor {
    fleet
        .store()
        .lock()
        .await
        .events_for(job)
        .expect("the Job's log")
        .iter()
        .rev()
        .find_map(|event| match event.moved() {
            Moved::Job { .. } => Some(event.actor()),
            Moved::Step { .. } | Moved::Drone { .. } => None,
        })
        .expect("a Job that has moved at least once")
}

/// How many status moves the Job has made. **The reading that catches a move
/// nobody meant to make**, which `last_mover` cannot: a redirect that moved a
/// `running` Job to `running` would be recorded against a person and read
/// exactly like one that moved nothing.
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

/// How long the slot says the step has been running. **The reading the
/// wall-clock tripwire is taken from** — `crate::converging::tripped` asks the
/// slot this question and nothing else, so asking it here is asking what the
/// tripwire would see.
async fn running_for(fleet: &Fixture) -> Duration {
    let now = fleet.now();
    let held = fleet.the_only_slot().await;
    let slot = held.lock().await;
    slot.as_ref()
        .map(|at| at.running_for(&now))
        .expect("a Drone in the slot")
}

/// **The defect, as a case.** A Drone alive on a `stalled` Job takes a
/// redirect, and took nothing before this.
#[tokio::test]
async fn a_stalled_job_with_a_live_drone_admits_a_redirect() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    let after = fleet
        .redirect(&job, &advice())
        .await
        .expect("a Drone that is there can be told something");

    // **Still escalated on the way out of the call.** Nothing about the Job has
    // moved yet, because nothing about the Drone has been shown.
    assert_eq!(after.status(), JobStatus::Escalated);
    assert_eq!(
        step_state(&fleet, &job).await,
        StepState::Running,
        "a Job-level escalation froze no step, so the redirect unfroze none"
    );
}

/// The owner's rule: *"if the job is stalled it should be unstalled and back to
/// running if the drone starts turning again"*. The evidence is the Drone's own
/// turn, and the Job waits for it.
#[tokio::test]
async fn the_job_returns_to_running_only_once_the_drone_turns() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    // Turns before anybody says anything move nothing: the watcher is cold on a
    // Job no redirect is outstanding on.
    for _ in 0..5 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
    }
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );

    fleet.redirect(&job, &advice()).await.unwrap();

    let roused = until_roused(&fleet).await;
    assert_eq!(roused.job, job);
    assert_eq!(roused.step, StepId::new(IMPLEMENT));
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "the Drone turned, so the Job is not stalled any more"
    );
    // **The person's move, recorded as theirs.** Fleet chose the instant; the
    // decision to take this Job out of `escalated` was a person's, and the
    // registry says a person is who acts on an escalated Job.
    assert_eq!(
        last_mover(&fleet, &job).await,
        Actor::Human,
        "Fleet did not un-escalate a Job of its own accord"
    );

    // The wait is over: a Drone that keeps answering does not move it again.
    for _ in 0..5 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
    }
}

/// **A redirect landing on a Drone which never wakes leaves the Job honestly
/// escalated.** That is the whole reason it is not moved on the send: a person
/// must be able to tell a Drone that took the advice from one past taking it.
#[tokio::test]
async fn a_drone_that_never_wakes_leaves_the_job_escalated() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_never_wakes());
    let job = stalled(&fleet, &home).await;

    fleet
        .redirect(&job, &advice())
        .await
        .expect("the pipe took the write");

    for _ in 0..40 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert_eq!(
        fleet.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Stalled)),
        "the escalation the vigil wrote still stands, and is still what a person reads"
    );
}

/// **The path that already worked, unchanged.** Where a step stopped, something
/// is frozen underneath the Job, so both machines move on the send and nothing
/// waits: a step left `stopped` is one no submission could advance.
#[tokio::test]
async fn a_step_that_stopped_is_handed_back_on_the_send() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let after = fleet.redirect(&job, &advice()).await.unwrap();

    assert_eq!(after.status(), JobStatus::Running);
    assert_eq!(step_state(&fleet, &job).await, StepState::Running);
    // Nothing is outstanding, so the Drone answering is an ordinary turn rather
    // than the thing the Job was waiting on.
    for _ in 0..20 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Running);
}

/// **A heartbeat is not a turn.** The narrow reading is the whole guard against
/// a Job that flickers back to `running` on a Drone still wedged inside the
/// call it was wedged in when the vigil caught it.
#[tokio::test]
async fn a_progress_heartbeat_does_not_bring_the_job_back() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_only_ticks());
    let job = stalled(&fleet, &home).await;
    let before = heard(&fleet).await;

    fleet.redirect(&job, &advice()).await.unwrap();

    for _ in 0..40 {
        assert!(fleet.turn().await.expect("a turn").roused().is_none());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // The pipe carried something back, so this is a reading rather than a Drone
    // that was never there.
    assert!(
        heard(&fleet).await > before,
        "the heartbeat never arrived, so nothing was told apart"
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
}

/// `stalled` can fire with the Drone dead too, and the answer there is the one
/// `adrift` already draws rather than a new one. **And nothing else applies
/// either**: a restart wants a stopped step and a Job-level escalation named
/// none, so what is left is a redispatch or Pilot. Unchanged by this act, and
/// asserted here so the pair is visible in one place.
#[tokio::test]
async fn a_stalled_job_whose_drone_is_gone_refuses_a_redirect() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = stalled(&fleet, &home).await;
    until_reaped(&fleet).await;

    assert!(
        matches!(
            fleet.redirect(&job, &advice()).await,
            Err(Adrift::NoDroneToRedirect { .. })
        ),
        "a redirect needs a session, and there is none"
    );
    assert!(
        matches!(
            fleet.restart_step(&job).await,
            Err(Adrift::NoStepStopped { .. })
        ),
        "and a restart needs a stopped step, which a Job-level escalation never named"
    );
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
}

/// **The wire, not just the enum.** `#126`: `NoDroneToRedirect` fell through
/// the daemon's match to the 500 catch-all — an excellent message on a status
/// code that told the caller Fleet broke, when it had correctly refused.
#[tokio::test]
async fn a_redirect_with_no_drone_answers_409_over_the_wire() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = stalled(&fleet, &home).await;
    until_reaped(&fleet).await;

    let refusal = api::Daemon::redirect_drone(
        &fleet,
        ipc::JobId::from(&job),
        ipc::Redirection {
            instruction: String::from("read tests/parse.rs first"),
        },
    )
    .await
    .expect_err("no Drone is there to redirect");

    assert!(matches!(refusal, api::Refusal::IllegalMove(_)));
    assert_eq!(refusal.status(), 409);
}

/// The same wire proof for `DroneStillThere`: no existing scenario reaches it,
/// so this is `refused` with a Drone that never leaves the slot, restarted
/// before anything reaps it.
#[tokio::test]
async fn a_restart_with_the_drone_still_there_answers_409_over_the_wire() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let refusal = api::Daemon::restart_step(&fleet, ipc::JobId::from(&job))
        .await
        .expect_err("the Drone `refused` left in the slot is still there");

    assert!(matches!(refusal, api::Refusal::IllegalMove(_)));
    assert_eq!(refusal.status(), 409);
}

/// **The refusal that had to survive the split.** `interrupted` and
/// `resource_exhausted` have no stopped step *and* no live Drone; loosening the
/// predicate for the act that needed it must not have loosened it for them.
#[tokio::test]
async fn a_job_escalated_with_its_drone_gone_still_refuses_both_acts() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_leaves());
    let job = started(&fleet, &home).await;
    until_reaped(&fleet).await;
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );

    assert!(matches!(
        fleet.redirect(&job, &advice()).await,
        Err(Adrift::NoDroneToRedirect { .. })
    ));
    assert!(matches!(
        fleet.restart_step(&job).await,
        Err(Adrift::NoStepStopped { .. })
    ));
}

/// **`#145`, as a case.** A healthy Drone working normally takes a redirect,
/// and refused one until now: `docs/concepts/drone.md` promises all of Redirect,
/// Kill and Pause on a non-escalated Drone, and the one Drone this act could not
/// reach was the one going the wrong way with nothing yet wrong.
///
/// A restart is still refused, and that pairing is the point — the two acts stop
/// sharing a predicate without starting to overlap.
#[tokio::test]
async fn a_healthy_job_takes_a_redirect_and_still_refuses_a_restart() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let moved = job_moves(&fleet, &job).await;

    let after = fleet
        .redirect(&job, &advice())
        .await
        .expect("a Drone that is working can be told something");

    assert_eq!(after.status(), JobStatus::Running);
    assert_eq!(
        step_state(&fleet, &job).await,
        StepState::Running,
        "nothing was frozen, so nothing was unfrozen"
    );
    assert_eq!(
        job_moves(&fleet, &job).await,
        moved,
        "a redirect into a healthy Drone moves the Job nowhere at all"
    );
    assert!(
        matches!(
            fleet.restart_step(&job).await,
            Err(Adrift::NotResumable {
                status: JobStatus::Running,
                ..
            })
        ),
        "a restart still wants a Job a person is holding"
    );
}

/// The wait rides on a healthy Job the same way, and **it is the only thing
/// that says anything happened** — this Job is `running` before the send and
/// `running` after, so a person with no `redirecting` on the wire cannot tell a
/// Drone that was steered from one that was not.
#[tokio::test]
async fn a_healthy_job_carries_the_wait_and_moves_nothing_when_it_is_answered() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let moved = job_moves(&fleet, &job).await;

    fleet.redirect(&job, &advice()).await.unwrap();
    assert!(detail(&fleet, &job).await.redirecting.is_some());

    let roused = until_roused(&fleet).await;
    assert_eq!(roused.job, job);

    let after = detail(&fleet, &job).await;
    assert_eq!(after.job.status.domain(), JobStatus::Running);
    assert!(
        after.redirecting.is_none(),
        "the Drone answered, so nothing is waiting on it"
    );
    assert_eq!(
        job_moves(&fleet, &job).await,
        moved,
        "the answer landed on a Job that was never held for it"
    );
}

/// **The restart this act must never quietly become.** A gate stops a step
/// before the Job escalates over it, so a `running` Job holds a `stopped` step
/// for an instant — and a redirect arriving in that instant must leave it
/// stopped. Handing it back would be a person's sentence silently re-running a
/// step the gate had ended, which is `docs/concepts/job.md`'s
/// *"a redirect that respawns is a restart that threw away the session"* one
/// rung shallower.
#[tokio::test]
async fn a_redirect_does_not_unfreeze_a_step_under_a_job_nobody_escalated() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_step(
            &record,
            &StepId::new(IMPLEMENT),
            StepTarget::Stopped(
                StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
            ),
        )
        .await
        .unwrap();

    let after = fleet.redirect(&job, &advice()).await.unwrap();

    assert_eq!(after.status(), JobStatus::Running);
    assert_eq!(
        step_state(&fleet, &job).await,
        StepState::Stopped,
        "the gate stopped this step and only the gate's own recovery may start it"
    );
}

/// The status is still a real question, and what it catches is a Drone that
/// outlives the status which had one. A Job at `awaiting_review` has been
/// answered by its machine gates; the act there is `request_changes`, whose note
/// waits for the next Drone.
#[tokio::test]
async fn a_job_past_its_step_refuses_a_redirect_even_with_a_drone_in_the_slot() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(&record, Target::AwaitingReview, Actor::Fleet)
        .await
        .unwrap();

    assert!(
        matches!(
            fleet.redirect(&job, &advice()).await,
            Err(Adrift::NotResumable {
                status: JobStatus::AwaitingReview,
                ..
            })
        ),
        "the slot still holds a Drone, and where the Job stands is what decides"
    );
}

/// **What the screen could not say before.** A redirect that is waiting and one
/// that never arrived were the same Job drawn the same way; the wait is served
/// on `get_job` because that is the read a second window and a reload both
/// make, and a fact Bridge remembered instead would die with the window.
#[tokio::test]
async fn a_waiting_redirect_is_on_the_wire_and_a_reread_finds_it() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_never_wakes());
    let job = stalled(&fleet, &home).await;

    assert!(
        detail(&fleet, &job).await.redirecting.is_none(),
        "nothing has been said to this Drone yet"
    );

    fleet.redirect(&job, &advice()).await.unwrap();

    let waiting = detail(&fleet, &job).await;
    assert!(
        waiting.redirecting.is_some(),
        "the instruction went down the pipe and the Job is still escalated"
    );
    // The Job did not become something else. The wait rides beside the status,
    // as a Judge call in flight rides beside a step's state.
    assert_eq!(waiting.job.status.domain(), JobStatus::Escalated);
    assert_eq!(
        detail(&fleet, &job).await.redirecting,
        waiting.redirecting,
        "a second read of the same Job says the same thing, and says it the same way"
    );
}

/// The two cases read differently, and **what decides is whether a step
/// stopped**: where one had, both machines moved on the send, so the Job is
/// `running` on the way out of the call and there is nothing to wait for.
#[tokio::test]
async fn a_redirect_onto_a_stopped_step_leaves_nothing_waiting() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    fleet.redirect(&job, &advice()).await.unwrap();

    let after = detail(&fleet, &job).await;
    assert_eq!(after.job.status.domain(), JobStatus::Running);
    assert!(
        after.redirecting.is_none(),
        "the step was handed back on the send, so nothing is outstanding"
    );
}

/// **The wait ends where the Job moves.** The Drone turns, the Job goes back to
/// `running`, and the fact about the last act goes with it — a sentence that
/// outlived the wait would be the same ambiguity pointed the other way.
#[tokio::test]
async fn the_wait_is_over_when_the_drone_turns() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = stalled(&fleet, &home).await;

    fleet.redirect(&job, &advice()).await.unwrap();
    assert!(detail(&fleet, &job).await.redirecting.is_some());

    until_roused(&fleet).await;

    let after = detail(&fleet, &job).await;
    assert_eq!(after.job.status.domain(), JobStatus::Running);
    assert!(
        after.redirecting.is_none(),
        "the Drone answered, so nothing is waiting on it"
    );
}

/// One Job's wait is not another's. The slot holds one Drone, and a Job opened
/// beside it must not draw somebody else's — the reading `Aloft::on` takes of a
/// Judge call, asked of the same slot.
#[tokio::test]
async fn a_job_that_was_not_redirected_carries_no_wait() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_never_wakes());
    let job = stalled(&fleet, &home).await;
    let other = fleet
        .propose(a_proposal("a second Job nobody has spoken to"))
        .await
        .expect("a proposal")
        .id()
        .clone();

    fleet.redirect(&job, &advice()).await.unwrap();

    assert!(detail(&fleet, &other).await.redirecting.is_none());
}

/// **The clock a redirect must not put back.** A healthy Drone's step has been
/// running for the whole time the Drone was healthy, and a person speaking to
/// it does not make that untrue — so the wall clock the tripwire reads carries
/// straight across the redirect.
///
/// Nothing caps how often a person may redirect, so a reading that went back to
/// zero here would be a step held under its ceiling for ever by being spoken
/// to. The reading is taken off the slot rather than driven to the ceiling
/// because the ceiling is `converging`'s subject and this is about the number
/// it reads.
#[tokio::test]
async fn a_redirect_into_a_healthy_drone_does_not_put_the_step_clock_back() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = started(&fleet, &home).await;

    let before = running_for(&fleet).await;
    assert!(
        before >= Duration::from_secs(5),
        "the step has run long enough for the reading to mean something: {before:?}"
    );
    fleet.redirect(&job, &advice()).await.unwrap();

    let after = running_for(&fleet).await;
    assert!(
        after >= before,
        "the step has run since it started, not since somebody typed: {after:?} < {before:?}"
    );
}

/// **And the stopped step still gets it back**, which is the half that was
/// always right: that Drone stood idle at the escalation, so what the wall
/// clock counts does begin again when a person hands the step over.
#[tokio::test]
async fn a_redirect_onto_a_stopped_step_starts_the_step_clock_again() {
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_answers());
    let job = refused(&fleet, &home).await;

    let before = running_for(&fleet).await;
    fleet.redirect(&job, &advice()).await.unwrap();

    let after = running_for(&fleet).await;
    assert!(
        after < before,
        "the step's work begins again, so what it spent begins again: {after:?} >= {before:?}"
    );
}

/// One Job, as `GET /jobs/:job_id` serves it. The wire answer and not the
/// record, because the wait is on the first and in the second is nowhere.
async fn detail(fleet: &Fixture, job: &JobId) -> ipc::JobDetail {
    api::Daemon::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists")
}
