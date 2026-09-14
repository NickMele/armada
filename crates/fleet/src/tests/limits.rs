//! A limit a person saves reaches admission at the next turn, survives a
//! restart, and stops nothing already running.
//!
//! **Every fixture ships a bound of one**, so a case that starts two Drones is
//! a case about a saved value and nothing else.

use std::sync::Arc;
use std::time::Duration;

use api::{Commands, Queries};
use core_model::JobId;
use ipc::{DiskFloorGib, DronesAtOnce, LimitValues, MemorySparePercent, SaveLimits};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::daemon::{Fittings, Fleet};
use crate::headroom::{Bytes, Headroom, InUse, Machine, Polling, Reading, Spare};
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

struct Fixed(Reading);

impl Machine for Fixed {
    fn read(&self) -> Option<Reading> {
        Some(self.0)
    }

    fn disk_free_at(&self, _path: &std::path::Path) -> Option<Bytes> {
        Some(self.0.disk_free())
    }
}

/// Fittings on `home` whose machine has plenty of disk and `memory_in_use`
/// percent of its memory taken.
fn fitted(home: &TempDir, memory_in_use: u32) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]),
    );
    fittings.starting().workflows = one(testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    fittings.concurrency = Concurrency::of(1);
    fittings.headroom = Headroom::of(Spare::percent(15), Bytes::gibibytes(10));
    fittings.machine = Arc::new(Fixed(Reading::of(
        InUse::percent(10),
        InUse::percent(memory_in_use),
        Bytes::gibibytes(500),
    )));
    fittings.polling = Polling::every(Duration::ZERO);
    fittings
}

fn a_fleet_on(home: &TempDir) -> Fixture {
    Fleet::assembled(fitted(home, 20))
}

fn saving(concurrency: Option<u32>, memory: Option<u32>, disk: Option<u32>) -> SaveLimits {
    SaveLimits {
        concurrency: concurrency.map(|v| DronesAtOnce::new(v).expect("in range")),
        memory_spare_percent: memory.map(|v| MemorySparePercent::new(v).expect("in range")),
        disk_floor_gib: disk.map(|v| DiskFloorGib::new(v).expect("in range")),
        checks_at_once: None,
    }
}

async fn approved(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a proposal");
    worktree_directory(home, &job);
    dispatched(fleet, job.id())
        .await
        .expect("a person approves it");
    job.id().clone()
}

async fn capacity(fleet: &Fixture) -> (u32, u32, Option<String>) {
    let read = fleet.get_capacity().await.expect("capacity reads");
    (
        read.bound,
        read.occupied,
        read.held_by.map(|hold| hold.as_wire().to_string()),
    )
}

const SHIPPED: LimitValues = LimitValues {
    concurrency: 1,
    memory_spare_percent: 15,
    disk_floor_gib: 10,
    checks_at_once: 4,
};

#[tokio::test]
async fn a_saved_concurrency_admits_at_the_next_turn_without_a_restart() {
    let home = TempDir::new();
    let fleet = a_fleet_on(&home);
    let first = approved(&fleet, &home, "the first").await;
    let second = approved(&fleet, &home, "the second").await;
    assert_eq!(fleet.working_on().await, vec![first.clone()]);
    assert_eq!(
        capacity(&fleet).await,
        (1, 1, Some("concurrency_bound".to_string()))
    );

    let now = fleet
        .save_limits(saving(Some(2), None, None))
        .await
        .expect("saved");
    assert_eq!(now.values.concurrency, 2);
    assert_eq!(now.shipped, SHIPPED);
    assert_eq!(
        capacity(&fleet).await,
        (2, 1, None),
        "capacity reads the new bound the moment the save answers"
    );

    fleet.turn().await.expect("the loop turns");
    let working = fleet.working_on().await;
    assert!(
        working.contains(&first) && working.contains(&second),
        "{working:?}"
    );
}

#[tokio::test]
async fn lowering_the_bound_stops_nothing_already_running() {
    let home = TempDir::new();
    let fleet = a_fleet_on(&home);
    fleet
        .save_limits(saving(Some(2), None, None))
        .await
        .expect("saved");
    approved(&fleet, &home, "the first").await;
    approved(&fleet, &home, "the second").await;
    assert_eq!(fleet.working_on().await.len(), 2);

    fleet
        .save_limits(saving(Some(1), None, None))
        .await
        .expect("saved");
    fleet.turn().await.expect("the loop turns");

    assert_eq!(
        fleet.working_on().await.len(),
        2,
        "both Drones kept working"
    );
    assert_eq!(
        capacity(&fleet).await,
        (1, 2, Some("concurrency_bound".to_string()))
    );
}

#[tokio::test]
async fn a_saved_memory_share_holds_the_next_job_back() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home, 60));
    assert_eq!(capacity(&fleet).await, (1, 0, None), "40% spare clears 15%");

    fleet
        .save_limits(saving(None, Some(50), None))
        .await
        .expect("saved");
    assert_eq!(capacity(&fleet).await, (1, 0, Some("memory".to_string())));

    approved(&fleet, &home, "a change onto a full machine").await;
    assert!(fleet.working_on().await.is_empty());
}

/// **Kept across a restart, and a field nobody saved is still the shipped
/// one** — here a build that ships a different memory share reaches it.
#[tokio::test]
async fn a_saved_limit_survives_a_restart_and_an_unsaved_one_follows_what_ships() {
    let home = TempDir::new();
    let fleet = a_fleet_on(&home);
    fleet
        .save_limits(saving(Some(3), None, None))
        .await
        .expect("saved");
    fleet
        .save_limits(saving(None, None, Some(20)))
        .await
        .expect("an omitted field keeps its value");
    drop(fleet);

    let mut fittings = fitted(&home, 20);
    fittings.headroom = Headroom::of(Spare::percent(25), Bytes::gibibytes(10));
    let fleet = Fleet::assembled(fittings);

    let limits = fleet.get_limits().await.expect("limits read");
    assert_eq!(
        limits.values,
        LimitValues {
            concurrency: 3,
            memory_spare_percent: 25,
            disk_floor_gib: 20,
            checks_at_once: 4,
        }
    );
    assert_eq!(limits.shipped.memory_spare_percent, 25);
    assert_eq!(capacity(&fleet).await.0, 3);
}
