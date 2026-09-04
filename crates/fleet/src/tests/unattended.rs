//! A Job with no Drone yet, and nothing in Fleet holding it.
//!
//! **The cancellation is reproduced rather than simulated**, and what is
//! cancelled is what moved. `#435`'s Job was wedged because Bridge stopped
//! waiting on `POST /jobs/:id/approve_dispatch` after five seconds, axum
//! dropped the handler's future, and the whole dispatch was inside it — the
//! `pnpm install` under `kill_on_drop`, and the `tokio::time::timeout` that
//! would have noticed. **`#428` took the dispatch off that future.** The
//! approval queues and answers, `crate::turning` admits, and the first case
//! below is the assertion that a client has nothing left it can take away.
//!
//! The vigil still has a subject, and the second case is it: a turn's own
//! future can be dropped, because `Turning::drop` aborts the turn in flight
//! when Fleet is stopping. That is now the only producer of a Job that is
//! `running` with nothing attending it, and it is Fleet's own doing rather
//! than somebody's browser. A test that planted the state by hand would assert
//! that the vigil reads it, and would not say whether anything can produce it.
//!
//! **A real process again, for `tests::preparing`'s reason.** `/bin/sleep` is
//! on every machine Armada runs on and needs no shell; what is being tested is
//! a dispatch that is still inside a command when the future goes away, and a
//! fake runner has no inside to be in.

use std::path::Path;
use std::time::Duration;

use config::Manifest;
use core_model::{EscalationTrigger, JobStatus, Recourse, StepState, TransitionReason};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

/// A Fleet whose `armada.yml` requires one command that outlives the wait.
///
/// Five seconds is the fixture's `CheckBudget`, so the sleep is longer than the
/// budget and the wait below is far shorter than either: what ends this
/// dispatch is the future being dropped, and never the command's own bound.
fn a_fleet_requiring(home: &TempDir, run: &str) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!(
            "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  bootstrap:\n    run: {run}\n\
             setup:\n  requires: [bootstrap]\n"
        ),
    )
    .expect("a manifest that requires one command");
    Fleet::assembled(fittings)
}

/// **(a)** A client that stops waiting cannot destroy the work. **`#428`.**
///
/// The Manifest below requires thirty seconds of preparation and the wait given
/// to the approval is three hundred milliseconds — the same shape as Bridge's
/// `COMMAND_MS` against a cold `pnpm install`, one order of magnitude faster.
/// **The approval answers inside it**, because the dispatch is not in there any
/// more; the Job is `queued`, and a turn the caller does not own is what runs
/// the thirty seconds.
///
/// The last two assertions are what the fix has to keep true and what a
/// cancellation-safe dispatch could quietly lose: the Job is left somewhere a
/// turn will find it, and the roster is not holding a place for a Drone that
/// nothing is starting.
#[tokio::test]
async fn an_approval_its_caller_stops_waiting_for_still_leaves_the_job_dispatchable() {
    let home = TempDir::new();
    let fleet = a_fleet_requiring(&home, "/bin/sleep 30");

    let job = fleet
        .propose(a_proposal("a Job whose approval used to time out"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    let answered = tokio::time::timeout(Duration::from_millis(300), fleet.approve(job.id()))
        .await
        .expect("the approval is not inside the preparation command any more")
        .expect("released to run");
    assert_eq!(
        answered.status(),
        JobStatus::Queued,
        "the approval queues; `crate::turning` dispatches"
    );
    assert!(
        answered
            .steps()
            .iter()
            .all(|step| step.state() == StepState::NotStarted),
        "and no step is entered until something dispatches it"
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "the roster holds no place for a Drone nothing has started"
    );
}

/// **(b)** A turn whose own future is dropped mid-dispatch leaves a Job
/// `running` with no Drone and nothing preparing it — and the next turn
/// escalates it.
///
/// **The vigil `#436` built still has a subject**, and this is what it is now:
/// `Turning::drop` aborts the turn in flight, which is what a Fleet stopping
/// during a long preparation does. It is Fleet's own doing rather than a
/// person's browser, and the reading is identical from every surface Armada
/// has — the Job reads as healthy, its steps have not started, and the roster
/// has forgotten it.
#[tokio::test]
async fn a_dispatch_whose_turn_was_dropped_is_escalated_on_the_next_turn() {
    let home = TempDir::new();
    let fleet = a_fleet_requiring(&home, "/bin/sleep 30");

    let job = fleet
        .propose(a_proposal("a Job whose dispatch is abandoned"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.expect("released to run");

    // What `Turning::drop` does to the turn in flight: the wait is spent, the
    // future is dropped, and `kill_on_drop` takes the command with it.
    let gave_up = tokio::time::timeout(Duration::from_millis(300), fleet.turn()).await;
    assert!(
        gave_up.is_err(),
        "the turn was still inside the preparation command when the wait was spent"
    );

    let wedged = fleet.load(job.id()).await.expect("readable");
    assert_eq!(
        wedged.status(),
        JobStatus::Running,
        "the move to `running` had already landed, which is why this reads as healthy"
    );
    assert!(
        wedged
            .steps()
            .iter()
            .all(|step| step.state() == StepState::NotStarted),
        "no step was ever entered"
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "the roster forgot the slot when the dispatch went away, which is what left \
         nothing to watch"
    );

    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(
        turned.unattended,
        vec![job.id().clone()],
        "the turn found a Job nothing was working on"
    );

    let stopped = fleet.load(job.id()).await.expect("readable");
    assert_eq!(stopped.status(), JobStatus::Escalated);
    let reason = fleet.last_reason(job.id()).await.unwrap();
    assert_eq!(
        reason,
        Some(TransitionReason::Escalation(EscalationTrigger::NotPrepared)),
        "not `interrupted`, which would send a reader after a Drone that never started"
    );

    // **The half that stops this being a dead end.** `#436` asks for a status
    // that says so, names why, and offers at least one act — and a silent hang
    // traded for a Job nobody can restart would be the worse of the two.
    let stuck = fleet
        .why_stuck(&stopped, reason.as_ref(), &[])
        .await
        .expect("an escalated Job carries an answer");
    assert!(
        stuck.admits(Recourse::Redispatch),
        "the person is offered an act: {:?}",
        stuck.recourse()
    );
}

/// **(c)** A Job whose preparation is still running is not escalated, however
/// long it takes.
///
/// **The regression `#436` names first.** A cold `playwright install` is
/// minutes of honest work, and a bound on how long preparation may take would
/// kill it on a new laptop. There is no bound: the dispatch holds the roster
/// for as long as the command runs, so the turn below cannot even read the
/// board until the command has finished — and when it does, the Job is past the
/// span this vigil watches.
#[tokio::test]
async fn a_preparation_that_is_still_running_is_left_alone() {
    let home = TempDir::new();
    // Long enough that a wall-clock rule would have to decide something about
    // it, and short enough that the test is not one.
    let fleet = a_fleet_requiring(&home, "/bin/sleep 1");

    let job = fleet
        .propose(a_proposal("a Job with a slow install"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.expect("released to run");

    // `admit_next` is what a turn ends in, called here directly so the turn
    // beside it is a *second* one and not the same one: the roster is held for
    // the whole of a dispatch, so the turn cannot read the board until the
    // command has finished.
    let (admitted, turned) = tokio::join!(fleet.admit_next(), fleet.turn());
    assert_eq!(
        admitted.expect("dispatch runs"),
        vec![job.id().clone()],
        "the admission that ran the command is the one that started it"
    );
    assert!(
        turned.expect("a turn").unattended.is_empty(),
        "a Job being dispatched is a Job something is working on"
    );
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Running,
        "the slow command finished and the first step is under way"
    );
}

/// **(d)** A turn over a Job whose Drone is working touches nothing.
///
/// The other half of (c): once a step is entered the Job is out of the span
/// whatever the roster says, so a Drone standing at a gate with its slot
/// momentarily unheld cannot be read as one that never started.
#[tokio::test]
async fn a_job_whose_first_step_has_started_is_out_of_the_span() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet
        .propose(a_proposal("an ordinary Job"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());
    dispatched(&fleet, job.id()).await.expect("dispatch runs");

    let turned = fleet.turn().await.expect("a turn");
    assert!(turned.unattended.is_empty());
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Running
    );
}
