//! How full the fleet is, and which one thing is holding the next Drone back.
//!
//! # The count is the roster's, and this is the case that proves it matters
//!
//! `an_escalated_job_that_kept_its_drone_still_holds_its_place` is the load
//! bearing one. A refused step leaves the Drone alive and idle so a redirect
//! costs no respawn, and the Job's status is `escalated` while it does — so a
//! count taken from `running` rows reads zero while admission starts nothing.
//! Every other case here would pass against that wrong count.
//!
//! # And the payload never disagrees with the Board
//!
//! Both come from `room_for_another`, so each case that asserts a hold asserts
//! the `queued` row's label beside it. A payload saying the disk is full while
//! the Board says the Job is fine is the two-answers defect one predicate
//! exists to prevent, one level up from where `queued_reason` prevents it.

use std::sync::Arc;

use api::Queries;
use config::ResolvedWorkflow;
use core_model::{Actor, EscalationTrigger, JobId, StepId, StepLevelTrigger, StepTarget, Target};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::Fleet;
use crate::headroom::{Bytes, Headroom, InUse, Machine, Polling, Reading, Spare};
use crate::slots::Concurrency;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// The threshold that ships, so no case here passes against a number invented
/// for it.
const SHIPPED: Headroom = Headroom::of(Spare::percent(15), Bytes::gibibytes(10));

/// A machine that answers the same thing every time, so a case about the bound
/// is never also a case about the machine.
struct Fixed(Reading);

impl Machine for Fixed {
    fn read(&self) -> Option<Reading> {
        Some(self.0)
    }
}

fn plenty() -> Arc<dyn Machine> {
    Arc::new(Fixed(Reading::of(
        InUse::percent(10),
        InUse::percent(20),
        Bytes::gibibytes(500),
    )))
}

/// Enough of everything but disk, which is under the shipped floor.
fn nearly_full() -> Arc<dyn Machine> {
    Arc::new(Fixed(Reading::of(
        InUse::percent(10),
        InUse::percent(20),
        Bytes::gibibytes(2),
    )))
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

/// A Drone that stays up, so a Job that reaches `running` keeps its place in
/// the roster for as long as the case needs it to.
fn a_drone_that_stays() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"])
}

fn a_fleet_of(bound: usize, home: &TempDir, machine: Arc<dyn Machine>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        a_drone_that_stays(),
    );
    fittings.workflows = one(one_step());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    fittings.concurrency = Concurrency::of(bound);
    fittings.machine = machine;
    fittings.headroom = SHIPPED;
    fittings.polling = Polling::every(std::time::Duration::ZERO);
    Fleet::assembled(fittings)
}

async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("a person approves it");
    job.id().clone()
}

/// The payload as the wire spells it: the two numbers and the reason.
async fn capacity(fleet: &Fixture) -> (u32, u32, Option<String>) {
    let read = fleet
        .get_capacity()
        .await
        .expect("Fleet reads its own roster");
    (
        read.bound,
        read.occupied,
        read.held_by.map(|hold| hold.as_wire().to_string()),
    )
}

/// What the Board says about one Job, so a case can assert the two agree.
async fn queued_reason(fleet: &Fixture, job: &JobId) -> (String, Option<String>) {
    let summary = fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .job;
    (
        summary.status.as_wire().to_string(),
        summary.queued_reason.map(|why| why.as_wire().to_string()),
    )
}

/// **Absent is "nothing is holding it".** A Fleet with room says so by carrying
/// no reason at all rather than by carrying one that means none.
#[tokio::test]
async fn an_idle_fleet_names_its_bound_and_holds_nothing_back() {
    let home = TempDir::new();
    let fleet = a_fleet_of(2, &home, plenty());

    assert_eq!(capacity(&fleet).await, (2, 0, None));
}

/// "2 of 2 drones", which is the sentence the design contract's status bar has
/// promised since before anything could count.
#[tokio::test]
async fn a_full_fleet_reads_as_its_bound_and_names_the_bound() {
    let home = TempDir::new();
    let fleet = a_fleet_of(1, &home, plenty());
    let first = approved(&fleet, &home, "the first").await;
    let waiting = approved(&fleet, &home, "the second").await;

    assert_eq!(fleet.working_on().await, vec![first]);
    assert_eq!(
        capacity(&fleet).await,
        (1, 1, Some("concurrency_bound".to_string()))
    );
    assert_eq!(
        queued_reason(&fleet, &waiting).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string())
        ),
        "and the Board's one label is the coarse form of the same answer"
    );
}

/// **The case a status-derived count gets wrong.** A refused step keeps its
/// Drone alive and idle, so a redirect costs no respawn — and the Job's status
/// is `escalated` the whole time. Nothing is `running`, and the place is still
/// spent.
#[tokio::test]
async fn an_escalated_job_that_kept_its_drone_still_holds_its_place() {
    let home = TempDir::new();
    let fleet = a_fleet_of(1, &home, plenty());
    let held = approved(&fleet, &home, "a step the gate refuses").await;
    let record = fleet.load(&held).await.expect("the Job reads");
    let record = fleet
        .move_step(
            &record,
            &StepId::new(IMPLEMENT),
            StepTarget::Stopped(
                StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("a step-level trigger"),
            ),
        )
        .await
        .expect("the step stops");
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::GateFailure),
            Actor::Fleet,
        )
        .await
        .expect("the Job escalates over it");
    let waiting = approved(&fleet, &home, "the one that cannot start").await;

    assert_eq!(
        queued_reason(&fleet, &held).await.0,
        "escalated",
        "nothing is running, which is the whole trap"
    );
    assert_eq!(
        capacity(&fleet).await,
        (1, 1, Some("concurrency_bound".to_string())),
        "a count of running rows would say 0 of 1 while admission starts nothing"
    );
    assert_eq!(
        fleet.working_on().await,
        vec![held.clone()],
        "and the roster still names it, which is what the count reads"
    );
    assert_eq!(
        queued_reason(&fleet, &waiting).await.1,
        Some("waiting_on_resources".to_string())
    );
}

/// The reason a person could not reach before this: the bound has room and the
/// volume does not.
#[tokio::test]
async fn a_short_disk_is_named_where_the_bound_has_room() {
    let home = TempDir::new();
    let fleet = a_fleet_of(2, &home, nearly_full());
    let waiting = approved(&fleet, &home, "a change that needs a worktree").await;

    assert_eq!(capacity(&fleet).await, (2, 0, Some("disk".to_string())));
    assert_eq!(
        queued_reason(&fleet, &waiting).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string())
        ),
        "one label on the row, and the payload says which of the four it was"
    );
}

/// **The order is the bound and then the machine, and it is not relaxed.** A
/// Fleet that is both at its cap and out of disk reports the cap, because that
/// is what is stopping admission now — and reports the disk the moment the cap
/// frees, which is the moment it starts mattering.
#[tokio::test]
async fn a_fleet_that_is_both_full_and_short_names_the_bound_first() {
    // A Fleet with its one place free and the volume short answers with the
    // volume, because that is the only thing in the way.
    let free = TempDir::new();
    let fleet = a_fleet_of(1, &free, nearly_full());
    approved(&fleet, &free, "the one that cannot be given a worktree").await;
    assert_eq!(capacity(&fleet).await, (1, 0, Some("disk".to_string())));

    // A Fleet whose one place is spent answers with the bound, and does not
    // read the machine to find that out. Both are true of one Fleet at once,
    // and only one of the two can cross.
    let spent = TempDir::new();
    let fleet = a_fleet_of(1, &spent, plenty());
    let running = approved(&fleet, &spent, "the one that takes the place").await;
    assert_eq!(fleet.working_on().await, vec![running]);
    assert_eq!(
        capacity(&fleet).await.2,
        Some("concurrency_bound".to_string()),
        "the order is admission's, and it is not relaxed to report both"
    );
}

/// **An unreadable machine admits**, so it holds nothing back and says so.
/// Absent here is not "Fleet could not look" — the numbers beside it are still
/// true and a reader must not draw a warning from a missing reason.
#[tokio::test]
async fn a_machine_that_will_not_answer_holds_nothing_back() {
    struct Silent;
    impl Machine for Silent {
        fn read(&self) -> Option<Reading> {
            None
        }
    }

    let home = TempDir::new();
    let fleet = a_fleet_of(2, &home, Arc::new(Silent));

    assert_eq!(capacity(&fleet).await, (2, 0, None));
}
