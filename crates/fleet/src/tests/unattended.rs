//! A Job with no Drone yet, and nothing in Fleet holding it.
//!
//! **The cancellation is reproduced rather than simulated.** `#435`'s Job was
//! wedged because Bridge stopped waiting on `POST /jobs/:id/approve_dispatch`
//! after five seconds and axum dropped the handler's future — so the fixture
//! drops the same future, with `tokio::time::timeout` standing in for the
//! abort. A test that planted a `running` Job with no slot by hand would assert
//! that the vigil reads a state, and would not say whether anything can produce
//! it.
//!
//! **A real process again, for `tests::preparing`'s reason.** `/bin/sleep` is
//! on every machine Armada runs on and needs no shell; what is being tested is
//! a dispatch that is still inside a command when the request goes away, and a
//! fake runner has no inside to be in.

use std::path::Path;
use std::time::Duration;

use config::Manifest;
use core_model::{EscalationTrigger, JobStatus, StepState, TransitionReason};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
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

/// **(a)** A dispatch whose caller stopped waiting leaves a Job `running` with
/// no Drone and nothing preparing it — and the next turn escalates it.
///
/// The first half is the defect and it is asserted before the fix runs: the Job
/// reads as healthy, its steps have not started, its spend is nothing and the
/// roster has forgotten it. That is what six minutes of `#435` looked like from
/// every surface Armada has.
#[tokio::test]
async fn a_dispatch_its_caller_stopped_waiting_for_is_escalated_on_the_next_turn() {
    let home = TempDir::new();
    let fleet = a_fleet_requiring(&home, "/bin/sleep 30");

    let job = fleet
        .propose(a_proposal("a Job whose approval times out"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    // Exactly what Bridge's `COMMAND_MS` does to the request, one order of
    // magnitude faster: the wait is spent, the future is dropped, and
    // `kill_on_drop` takes the command with it.
    let gave_up = tokio::time::timeout(Duration::from_millis(300), fleet.approve(job.id())).await;
    assert!(
        gave_up.is_err(),
        "the approval was still inside the preparation command when the wait was spent"
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
    assert_eq!(
        fleet.last_reason(job.id()).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::NotPrepared)),
        "not `interrupted`, which would send a reader after a Drone that never started"
    );
}

/// **(b)** A Job whose preparation is still running is not escalated, however
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

    let (approved, turned) = tokio::join!(fleet.approve(job.id()), fleet.turn());
    assert_eq!(
        approved.expect("dispatch runs").status(),
        JobStatus::Running
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

/// **(c)** A turn over a Job whose Drone is working touches nothing.
///
/// The other half of (b): once a step is entered the Job is out of the span
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
    fleet.approve(job.id()).await.expect("dispatch runs");

    let turned = fleet.turn().await.expect("a turn");
    assert!(turned.unattended.is_empty());
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Running
    );
}
