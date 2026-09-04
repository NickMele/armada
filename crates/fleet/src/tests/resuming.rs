//! A `queued` Job that a person put back, told apart from one that arrived.
//!
//! # The reading these exist for
//!
//! Press restart while the bound is spent and nothing on screen moves. The Job
//! is `queued`, the badge says `waiting_on_resources`, and that row is
//! indistinguishable from a Job approved an hour ago and never started — so a
//! correct system reads as a dropped press. It is new: those acts spawned on
//! the spot until re-admission put them behind the bound.
//!
//! # And the answer is re-admission's own, not a second reading
//!
//! Every case drives the Job into the shape one of the four acts leaves the
//! inner machine in, and `Fleet::owed` — the function re-admission calls to
//! decide which step a Drone goes back on — is what answers. So a row saying
//! `restarted` and the Drone that eventually arrives cannot disagree, which is
//! the drift `queued_reason` and `admit_next` share a predicate to prevent, one
//! axis over. `readmitting` proves what the spawn then does with each shape;
//! nothing here repeats it.

use std::sync::Arc;

use api::Queries;
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, StepId, StepLevelTrigger, StepState, StepTarget, Target,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// Two steps, so an overruled first step has a second one to be cleared onto —
/// `owed` refuses an `advanced` last step, there being no Drone to owe.
fn two_steps() -> ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: IMPLEMENT,
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "review",
            label: "Review",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

/// A Drone that stays up, so a place stays spent and a Job stays where the case
/// put it.
fn a_drone_that_stays() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"])
}

/// **Bounded at one on purpose.** Every case here wants the Job it acted on to
/// stay `queued` long enough to be read, which is exactly the situation the
/// field exists for.
fn a_full_fleet(home: &TempDir) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        a_drone_that_stays(),
    );
    fittings.workflows = one(two_steps());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    fittings.concurrency = Concurrency::of(1);
    Fleet::assembled(fittings)
}

async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, job.id());
    dispatched(&fleet, job.id())
        .await
        .expect("a person approves it");
    job.id().clone()
}

/// The row as the wire spells it: the status, what it is waiting for, and who
/// put it there.
async fn row(fleet: &Fixture, job: &JobId) -> (String, Option<String>, Option<String>) {
    let summary = fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .job;
    (
        summary.status.as_wire().to_string(),
        summary.queued_reason.map(|why| why.as_wire().to_string()),
        summary.resumption.map(|act| act.as_wire().to_string()),
    )
}

/// Put the Job at `queued` with its current step in `state`, which is the shape
/// each of the four acts leaves behind — the same discriminator `owed` reads.
///
/// **The Job's own Drone is left standing.** It is what keeps the place spent
/// once the Job is back at `queued`, which is the reading every case here is
/// about, and it is what an escalated Job genuinely looks like: a refusal keeps
/// its Drone alive and idle so a redirect costs no respawn.
async fn queued_with_step(fleet: &Fixture, job: &JobId, step: &str, state: StepState) {
    let record = fleet.load(job).await.expect("the Job reads");
    let record = match state {
        StepState::Stopped => fleet
            .move_step(
                &record,
                &StepId::new(step),
                StepTarget::Stopped(
                    StepLevelTrigger::of(EscalationTrigger::GateFailure)
                        .expect("a step-level trigger"),
                ),
            )
            .await
            .expect("the step stops"),
        StepState::Advanced => fleet
            .move_step(&record, &StepId::new(step), StepTarget::Advanced)
            .await
            .expect("the step advances"),
        _ => record,
    };
    let record = fleet
        .move_job(&record, Target::AwaitingReview, Actor::Fleet)
        .await
        .expect("the Job reaches the gate");
    fleet
        .move_job(&record, Target::Queued, Actor::Human)
        .await
        .expect("a person sends it back to the queue");
}

/// **The ordinary way into the queue carries nothing.** A Job approved and
/// never run is at `queued` because it arrived there, and drawing "somebody is
/// waiting on this" over every approved Job is the noise that would make the
/// mark worthless on the rows that need it.
#[tokio::test]
async fn a_job_that_was_only_approved_names_no_act() {
    let home = TempDir::new();
    let fleet = a_full_fleet(&home);
    let first = approved(&fleet, &home, "the one that takes the place").await;
    let waiting = approved(&fleet, &home, "the one that only ever waited").await;

    assert_eq!(fleet.working_on().await, vec![first]);
    assert_eq!(
        row(&fleet, &waiting).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string()),
            None
        ),
        "it is waiting, and nobody put it back — those are two different facts"
    );
}

/// The same Job, before and after. **The row it was is the row the coordinator
/// described** — queued, waiting on resources, and identical to a Job nobody
/// has touched — and the field is the whole difference.
#[tokio::test]
async fn the_only_difference_a_press_makes_is_this_field() {
    let home = TempDir::new();
    let fleet = a_full_fleet(&home);
    let job = approved(&fleet, &home, "the one a person restarts").await;
    let before = row(&fleet, &job).await;

    queued_with_step(&fleet, &job, IMPLEMENT, StepState::Stopped).await;
    let after = row(&fleet, &job).await;

    assert_eq!(before.0, "running");
    assert_eq!(after.0, "queued");
    assert_eq!(
        after.1,
        Some("waiting_on_resources".to_string()),
        "its own Drone still holds the place, so the press changed nothing else"
    );
    assert_eq!(after.2, Some("restarted".to_string()));
}

/// A person answered at a human advance gate. **One value for both acts**:
/// approving and asking for changes are a person answering either way, and
/// which of the two it was is carried by the note that waits on the record.
#[tokio::test]
async fn a_review_answered_reads_as_reviewed() {
    let home = TempDir::new();
    let fleet = a_full_fleet(&home);
    let job = approved(&fleet, &home, "the one a person answered").await;
    queued_with_step(&fleet, &job, IMPLEMENT, StepState::Running).await;

    assert_eq!(
        row(&fleet, &job).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string()),
            Some("reviewed".to_string())
        ),
        "both facts on one row: what it waits for, and that a person put it here"
    );
}

/// A person restarted a step that stopped. The verb is the button's own, which
/// is the rule an action keeps its name through the flow.
#[tokio::test]
async fn a_restarted_step_reads_as_restarted() {
    let home = TempDir::new();
    let fleet = a_full_fleet(&home);
    let job = approved(&fleet, &home, "the one a person answered").await;
    queued_with_step(&fleet, &job, IMPLEMENT, StepState::Stopped).await;

    assert_eq!(
        row(&fleet, &job).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string()),
            Some("restarted".to_string())
        ),
        "both facts on one row: what it waits for, and that a person put it here"
    );
}

/// A person overruled the verdict that stopped a step. **Not "restarted"** —
/// the step it stopped on is cleared and the one after it runs, so a row saying
/// restarted would name the wrong step to anybody reading it.
#[tokio::test]
async fn an_overruled_verdict_reads_as_overruled_and_never_as_a_restart() {
    let home = TempDir::new();
    let fleet = a_full_fleet(&home);
    let job = approved(&fleet, &home, "the one a person answered").await;
    queued_with_step(&fleet, &job, IMPLEMENT, StepState::Advanced).await;

    assert_eq!(
        row(&fleet, &job).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string()),
            Some("overruled".to_string())
        ),
        "both facts on one row: what it waits for, and that a person put it here"
    );
}

/// **Nothing is claimed on a Job that is not `queued`.** The field answers a
/// question about the queue, and a running Job that was restarted an hour ago
/// is being worked rather than waiting to be.
#[tokio::test]
async fn nothing_is_claimed_once_the_job_is_working_again() {
    let home = TempDir::new();
    let fleet = a_full_fleet(&home);
    let job = approved(&fleet, &home, "the one a person restarted").await;
    queued_with_step(&fleet, &job, IMPLEMENT, StepState::Stopped).await;
    assert_eq!(row(&fleet, &job).await.2, Some("restarted".to_string()));

    // Admission takes it, which is what the person was waiting to see.
    let record = fleet.load(&job).await.expect("the Job reads");
    fleet
        .move_job(&record, Target::Running, Actor::Fleet)
        .await
        .expect("the bound made room");

    let (status, waiting, act) = row(&fleet, &job).await;
    assert_eq!(status, "running");
    assert_eq!(waiting, None);
    assert_eq!(act, None, "it is being worked, not waiting to be");
}
