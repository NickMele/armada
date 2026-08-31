//! A Job held back because of what it has already spent.
//!
//! **The board and admission are asserted together, never apart.** Each alone
//! can pass while the two disagree, which is the drift one shared predicate
//! exists to prevent — the same pairing `headroom`'s cases hold to.
//!
//! **What the cap cannot do is a case too.** `a_running_drone_is_not_stopped_by
//! _the_cap` is here because that limitation is the one a person will believe
//! is not there: a ceiling that reads as "spending stops here" and means
//! "nothing new starts after here" is worse than no ceiling.

use std::time::Duration;

use adapter_traits::DroneEvent;
use api::Daemon;
use store::Spend;
use testkit::FakeWorkProduct;

use crate::allowance::{spent, Allowance, Micros, Overspent};
use crate::daemon::{Fittings, Fleet};
use crate::headroom::{Bytes, InUse, Reading, Spare};
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// The allowance that ships. Every fixture here is held against the real one,
/// so a case that trips it trips the number a person would meet.
/// See `armada::serve::PROVISIONAL_ALLOWANCE`.
const SHIPPED: Allowance = Allowance::of(Micros::dollars(5), 300);

/// A Fleet whose Jobs are held against a cap the test chooses.
fn capped(home: &TempDir, allowance: Allowance) -> Fixture {
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.allowance = allowance;
    Fleet::assembled(fittings)
}

/// Approve a Job, with the worktree its dispatch would want.
async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> core_model::JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("a person approves it");
    job.id().clone()
}

/// What the Board says about a Job, as the wire spells it.
async fn board(fleet: &Fixture, job: &core_model::JobId) -> (String, Option<String>) {
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

/// Write a Drone's spend against a Job, as a finished Drone's exit would.
///
/// **Planted rather than earned**, in the cases about the ceiling. The fake
/// harness reports a `cost_micros` of zero, so a fixture that ran a Drone to
/// spend money would prove nothing about the cap. That a real exit writes the
/// row is the last case in this file, which earns it.
async fn spend(fleet: &Fixture, job: &core_model::JobId, drone: &str, cost: u64, turns: u64) {
    fleet
        .store()
        .lock()
        .await
        .record_drone_spend(
            job,
            &core_model::DroneId::carried(core_model::Ulid::carried(drone)),
            &store::DroneSpend {
                cost_micros: cost,
                turns,
                ran_ms: 900_000,
            },
        )
        .expect("the spend is recorded");
}

fn ended(turns: u32, cost_micros: u64) -> DroneEvent {
    DroneEvent::Ended {
        turns,
        cost_micros,
        refusals: 0,
    }
}

// --------------------------------------------------------------- the ceiling

/// A Job that has spent nothing is inside both ceilings.
#[test]
fn a_job_that_has_spent_nothing_is_inside_its_allowance() {
    assert_eq!(SHIPPED.exceeded_by(&Spend::default()), None);
}

/// **The shipped dollar cap swallows spike 5's whole measured spread.** Three
/// identical successful runs priced at $0.063, $0.087 and $0.146, and a cap
/// that refused the dearest of them would be refusing a Job for having started
/// with a cold cache. Fifty of the dearest still fits.
#[test]
fn the_shipped_cap_is_wide_enough_for_a_cold_cache() {
    for measured in [63_366u64, 87_344, 146_473] {
        let spend = Spend {
            cost_micros: measured,
            ..Spend::default()
        };
        assert_eq!(
            SHIPPED.exceeded_by(&spend),
            None,
            "a run measured at {measured} micros is not a runaway"
        );
    }
}

/// Each ceiling refuses on its own, so a passing case is not one signal doing
/// the work of two.
#[test]
fn each_ceiling_holds_a_job_back_on_its_own() {
    let dear = Spend {
        cost_micros: 5_000_000,
        turns: 4,
        ..Spend::default()
    };
    let long = Spend {
        cost_micros: 1_000,
        turns: 300,
        ..Spend::default()
    };
    assert_eq!(SHIPPED.exceeded_by(&dear), Some(Overspent::Cost));
    assert_eq!(SHIPPED.exceeded_by(&long), Some(Overspent::Turns));
}

/// Exactly the cap is spent, not spare. A Job that has used its whole allowance
/// has nothing left to start a Drone with.
#[test]
fn a_job_that_has_spent_exactly_its_cap_has_nothing_left() {
    let spend = Spend {
        cost_micros: 5_000_000,
        ..Spend::default()
    };
    assert_eq!(SHIPPED.exceeded_by(&spend), Some(Overspent::Cost));
}

// ------------------------------------------------------------------ the fold

/// **Cost is the last figure and turns are the sum**, which is the shape
/// `004-transcript-idle-session.ndjson` measured: one session, two terminating
/// lines, `num_turns` 3 then 2, and the second line's `total_cost_usd` holding
/// the whole session's spend. Adding the costs would bill the first invocation
/// twice.
#[test]
fn a_second_terminating_line_replaces_the_cost_and_adds_the_turns() {
    let folded = spent(
        &[ended(3, 101_189), ended(2, 121_961)],
        Duration::from_millis(11_786),
    );
    assert_eq!(folded.cost_micros, 121_961, "the session's running total");
    assert_eq!(folded.turns, 5, "3 + 2, because turns are per invocation");
    assert_eq!(folded.ran_ms, 11_786, "Fleet's clock, not the stream's");
}

/// A Drone that never reported still spent whatever it spent, and the fold says
/// zero rather than refusing to answer. A `Vanished` run is not an absent one.
#[test]
fn a_drone_that_never_reported_folds_to_nothing_rather_than_to_nothing_at_all() {
    let folded = spent(
        &[DroneEvent::Said {
            text: "working on it".to_string(),
        }],
        Duration::from_millis(400),
    );
    assert_eq!(folded.cost_micros, 0);
    assert_eq!(folded.turns, 0);
    assert_eq!(folded.ran_ms, 400, "it still held a slot for four hundred ms");
}

// ------------------------------------------------------------ against a Fleet

/// A Job whose Drones have already spent past the dollar cap stays where it
/// was, and the Board says so. **Both halves in one case**, because the label
/// and the admission are one predicate.
#[tokio::test]
async fn a_job_past_its_dollar_cap_is_not_started_and_says_why() {
    let home = TempDir::new();
    let fleet = capped(&home, SHIPPED);
    // The bound is one, so the first Job takes the slot and the second waits —
    // which is the moment the cap gets to answer. A Job approved onto a free
    // slot has spent nothing yet.
    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let waiting = approved(&fleet, &home, "a change that has already cost too much").await;
    spend(&fleet, &waiting, "01DRONE00000000000000001", 6_000_000, 40).await;

    fleet.kill_job(&running).await.expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");

    assert!(
        !fleet.working_on().await.contains(&waiting),
        "the slot is free and nothing was started on a Job that has spent its allowance"
    );
    assert_eq!(
        board(&fleet, &waiting).await,
        ("queued".to_string(), Some("over_budget".to_string())),
        "and the Board says the same thing admission decided"
    );
}

/// The turn ceiling holds a Job back the same way the dollar one does, so a
/// passing suite is not the price doing the work of both.
#[tokio::test]
async fn a_job_past_its_turn_cap_is_held_back_too() {
    let home = TempDir::new();
    let fleet = capped(&home, SHIPPED);
    let running = approved(&fleet, &home, "a change that holds the slot").await;
    let waiting = approved(&fleet, &home, "a change that went round and round").await;
    spend(&fleet, &waiting, "01DRONE00000000000000002", 40_000, 400).await;

    fleet.kill_job(&running).await.expect("the slot is given back");
    fleet.turn().await.expect("the loop turns");

    assert!(
        !fleet.working_on().await.contains(&waiting),
        "four cents is well inside the dollar cap, and the turns are not"
    );
    assert_eq!(
        board(&fleet, &waiting).await,
        ("queued".to_string(), Some("over_budget".to_string())),
        "four cents and four hundred turns is still over budget"
    );
}

/// **`over_budget` reads before `waiting_on_resources` when both hold.**
/// Headroom frees on its own and a spent budget does not, so telling a person
/// their Job is waiting for the machine would send them to watch something that
/// is already on its way while the thing actually holding it needs them.
#[tokio::test]
async fn a_job_over_budget_on_a_full_machine_reads_over_budget() {
    let home = TempDir::new();
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.allowance = SHIPPED;
    fittings.headroom = crate::headroom::Headroom::of(Spare::percent(15), Bytes::gibibytes(10));
    fittings.machine = std::sync::Arc::new(NoDisk);
    let fleet = Fleet::assembled(fittings);

    let job = approved(&fleet, &home, "a change with two reasons to wait").await;
    spend(&fleet, &job, "01DRONE00000000000000003", 9_000_000, 12).await;

    assert_eq!(
        board(&fleet, &job).await.1,
        Some("over_budget".to_string()),
        "the reason that needs a person outranks the one that clears itself"
    );
}

/// A machine with no disk left, so the budget case can stand beside a resource
/// one without the machine mattering to any other fixture here.
struct NoDisk;

impl crate::headroom::Machine for NoDisk {
    fn read(&self) -> Option<Reading> {
        Some(Reading::of(
            InUse::percent(10),
            InUse::percent(20),
            Bytes::gibibytes(1),
        ))
    }
}

/// **The cap does not stop a Drone that is spending, and this is the case that
/// says so out loud.** `cost_micros` arrives on the final result line of a
/// session and nowhere else, so a Job already running past its cap keeps
/// running: the refusal is the next dispatch, not this one. A reader who
/// believes otherwise is the failure this case exists to name.
#[tokio::test]
async fn a_running_drone_is_not_stopped_by_the_cap() {
    let home = TempDir::new();
    let fleet = capped(&home, SHIPPED);
    let job = approved(&fleet, &home, "a change already spending").await;
    assert_eq!(fleet.working_on().await, vec![job.clone()]);

    spend(&fleet, &job, "01DRONE00000000000000004", 50_000_000, 900).await;
    fleet.turn().await.expect("the loop turns");

    assert_eq!(
        fleet.working_on().await,
        vec![job.clone()],
        "ten times the cap, and the Drone that is spending it keeps going"
    );
    assert_eq!(
        board(&fleet, &job).await.0,
        "running".to_string(),
        "and the Job is not held anywhere: there is nothing to hold it back from"
    );
}

// ------------------------------------------------------------ the recording

/// **A Drone standing down writes what it spent, and the Job can read it back.**
/// The other cases plant a spend; this one earns it, through the function a
/// step boundary calls and against a harness whose Drone reports a real figure.
#[tokio::test]
async fn a_drone_standing_down_writes_what_it_spent() {
    let home = TempDir::new();
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        crate::tests::daemon::fitted_with(
            &home,
            FakeWorkProduct::changed(&["src/log.rs"]),
            // A Drone that names its price and then waits to be ended, so the
            // terminating line is certainly in the pipe when the boundary
            // drains it. The whole run is `stood_down`'s to read.
            testkit::FakeHarness::running(
                "/bin/sh",
                &["-c", "echo PRICED; while IFS= read -r line; do :; done"],
            )
            .reading(
                "PRICED",
                vec![DroneEvent::Ended {
                    turns: 7,
                    cost_micros: 146_473,
                    refusals: 0,
                }],
            ),
        );
    fittings.allowance = SHIPPED;
    let fleet = Fleet::assembled(fittings);
    let job = approved(&fleet, &home, "a change whose Drone reports a price").await;

    // **Wait for the line rather than assuming it arrived.** The child has to
    // be scheduled before it can say anything, and standing the Drone down
    // signals it — so a boundary reached immediately would drain a pipe the
    // shell had not written to yet, which is a race about the test and not
    // about the fold.
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
    .expect("the Drone said something before the boundary");

    let mut working = slot.lock().await;
    fleet
        .stood_down(&job, &mut working)
        .await
        .expect("the Drone is ended and its exit recorded");
    drop(working);

    let spent = fleet
        .store()
        .lock()
        .await
        .spend_for(&job)
        .expect("the spend reads");
    assert_eq!(
        spent.cost_micros, 146_473,
        "the figure the Drone reported, against the Job rather than the Drone"
    );
    assert_eq!(spent.turns, 7);
    assert_eq!(spent.drones, 1, "one Drone worked it, and the record says so");
}
