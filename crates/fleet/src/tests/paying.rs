//! What a Drone's run cost, written down where the Job can read it back.
//!
//! **The sibling of `crate::tests::allowance`, which is about the ceiling.**
//! These cases are about the figure the ceiling is compared against: that a run
//! is billed at all, and that it is billed before the move a client re-reads
//! on. The fixtures they share are that module's, because the cap it plants and
//! the price these earn have to be the same numbers.
//!
//! **Every case here earns its figure through a real child.** The fake harness
//! reports a cost of zero, so a fixture that planted the row would prove the
//! store and not the fold.

use std::sync::Arc;
use std::time::Duration;

use core_model::JobStatus;
use store::Spend;
use testkit::FakeWorkProduct;

use crate::daemon::{Fittings, Fleet};
use crate::gate::Ruling;
use crate::tests::allowance::{approved, capped, ended, Fixture, SHIPPED};
use crate::tests::daemon::{diff_evidence, one};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// A Fleet with one running Job whose Drone has named its price and now waits
/// to be ended, its terminating line certainly in the pipe.
///
/// **It waits for the line rather than assuming it arrived.** The child has to
/// be scheduled before it can say anything, and ending the Drone signals it —
/// so an ending reached immediately would drain a pipe the shell had not
/// written to yet, which is a race about the test and not about the fold.
async fn priced(home: &TempDir, cost_micros: u64, turns: u32) -> (Fixture, core_model::JobId) {
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        crate::tests::daemon::fitted_with(
            home,
            FakeWorkProduct::changed(&["src/log.rs"]),
            a_drone_priced_at(cost_micros, turns),
        );
    fittings.allowance = SHIPPED;
    let fleet = Fleet::assembled(fittings);
    let job = approved(&fleet, home, "a change whose Drone reports a price").await;
    heard_from(&fleet).await;
    (fleet, job)
}

/// A Drone that names its price on its first line and then sits reading, so the
/// fold has a figure to find and nothing has ended the run.
fn a_drone_priced_at(cost_micros: u64, turns: u32) -> testkit::FakeHarness {
    testkit::FakeHarness::running(
        "/bin/sh",
        &["-c", "echo PRICED; while IFS= read -r line; do :; done"],
    )
    .reading("PRICED", vec![ended(turns, cost_micros)])
}

/// Wait until the slot holds something the Drone said.
async fn heard_from(fleet: &Fixture) {
    let slot = fleet.the_only_slot().await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if slot
                .lock()
                .await
                .as_ref()
                .is_some_and(|at_work| !at_work.heard().is_empty())
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the Drone said something before the ending");
}

/// The same Drone under a Judge that refuses, dispatched, worked and ruled on.
///
/// **A refusal keeps its Drone**, so the Job stands escalated with the priced
/// slot still full — which is the one ruling where nothing after the gate would
/// write the figure down.
async fn refused_at_a_price(
    home: &TempDir,
    cost_micros: u64,
    turns: u32,
) -> (Fixture, core_model::JobId) {
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        crate::tests::daemon::fitted_with(
            home,
            FakeWorkProduct::changed(&["src/log.rs"]),
            a_drone_priced_at(cost_micros, turns),
        );
    fittings.allowance = SHIPPED;
    fittings.workflows = one(crate::tests::overruling::judged_then_summarised());
    fittings.judge = Arc::new(crate::tests::overruling::a_judge_that_refuses());
    let fleet = Fleet::assembled(fittings);
    let job = approved(&fleet, home, "a change a Judge will refuse").await;
    heard_from(&fleet).await;

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Refused { .. })),
        "the fixture did not reach a refusal: {:?}",
        turned.ruled()
    );
    (fleet, job)
}

/// What the Job's own record says it has cost so far.
async fn recorded(fleet: &Fixture, job: &core_model::JobId) -> Spend {
    fleet
        .store()
        .lock()
        .await
        .spend_for(job)
        .expect("the spend reads")
}

/// **A Drone standing down writes what it spent, and the Job can read it back.**
/// The other cases plant a spend; this one earns it, through the function a
/// step boundary calls and against a harness whose Drone reports a real figure.
#[tokio::test]
async fn a_drone_standing_down_writes_what_it_spent() {
    let home = TempDir::new();
    let (fleet, job) = priced(&home, 146_473, 7).await;

    let slot = fleet.the_only_slot().await;
    let mut working = slot.lock().await;
    fleet
        .stood_down(&job, &mut working)
        .await
        .expect("the Drone is ended and its exit recorded");
    drop(working);

    let spent = recorded(&fleet, &job).await;
    assert_eq!(
        spent.cost_micros, 146_473,
        "the figure the Drone reported, against the Job rather than the Drone"
    );
    assert_eq!(spent.turns, 7);
    assert_eq!(
        spent.drones, 1,
        "one Drone worked it, and the record says so"
    );
}

/// **A Drone that is ended rather than stood down writes what it spent too**,
/// and until `#398` it wrote nothing at all.
///
/// `end_the_drone` is the ending `Ruling::Finished` takes and, since #397, the
/// one a Job whose gate-failure attempts are spent takes to `awaiting_repair` —
/// the road a failing Check travels every time. So the run that went unrecorded
/// was the one on the commonest unhappy path, and the Job's record was short by
/// a whole Drone. The figure is spike 5's dearest measured run, $0.146.
#[tokio::test]
async fn a_drone_ended_rather_than_stood_down_writes_what_it_spent() {
    let home = TempDir::new();
    let (fleet, job) = priced(&home, 146_473, 7).await;

    assert_eq!(
        recorded(&fleet, &job).await,
        Spend::default(),
        "nothing is recorded while the Drone is still working"
    );

    let slot = fleet.the_only_slot().await;
    let mut working = slot.lock().await;
    fleet.end_the_drone(&mut working).await;
    drop(working);

    let spent = recorded(&fleet, &job).await;
    assert_eq!(
        spent.cost_micros, 146_473,
        "what the Drone spent on its way out reaches the Job that paid for it"
    );
    assert_eq!(spent.turns, 7);
    assert_eq!(spent.drones, 1);
    assert_eq!(
        SHIPPED.exceeded_by(&spent),
        None,
        "one run of this size is inside the shipped cap, which is the point: \
         the cap is now reading a figure rather than a zero"
    );
}

/// **A gate ruling writes what its Drone has spent before the Job moves**, so a
/// client re-reading on that move is not told the Job cost less than it did.
///
/// A refusal is the ruling that can only pass one way. It keeps its Drone alive
/// and idle at `escalated`, so nothing behind the gate writes the figure and
/// the Job sits there for as long as a person takes to read it — which is how a
/// detail drew $4.12 against a Fleet whose own route answered $5.28, short by
/// the Drone the escalation had just finished with.
#[tokio::test]
async fn a_refusal_pays_its_drone_before_the_job_escalates() {
    let home = TempDir::new();
    let (fleet, job) = refused_at_a_price(&home, 1_162_728, 38).await;

    assert_eq!(
        fleet.load(&job).await.expect("the Job reads").status(),
        JobStatus::Escalated
    );
    assert!(
        fleet.the_only_slot().await.lock().await.is_some(),
        "the refusal kept its Drone, so nothing has stood it down and nothing \
         else is going to write the row"
    );

    let spent = recorded(&fleet, &job).await;
    assert_eq!(
        spent.cost_micros, 1_162_728,
        "the figure is on the Job before the escalation a client re-reads on"
    );
    assert_eq!(spent.turns, 38);
    assert_eq!(spent.drones, 1);
}

/// **A Drone that has named no price writes no row.** An adopted Drone's
/// terminating line went into a pipe with no reader, so folding it gives zero
/// of everything — and an upsert of that would replace the figure the Fleet
/// before this one left. `Working::spent` argues the undercount.
#[tokio::test]
async fn a_drone_that_has_named_no_price_is_not_written_down() {
    let home = TempDir::new();
    let fleet = capped(&home, SHIPPED);
    let job = approved(&fleet, &home, "a change whose Drone names no price").await;

    let slot = fleet.the_only_slot().await;
    let working = slot.lock().await;
    fleet
        .paid_so_far(&working)
        .await
        .expect("there is nothing to write");
    drop(working);

    assert_eq!(
        recorded(&fleet, &job).await,
        Spend::default(),
        "no row at all, so `drones` still tells a Job that has not spent from \
         one whose Drones came to nothing"
    );
}
