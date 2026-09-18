//! How many Checks run at once on the machine, and what a short machine does to
//! the next one: it waits for a running Check to finish, and the first on the
//! machine always starts. #284, #1063.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::Footprint;
use api::{Commands, Queries};
use ipc::SaveLimits;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Sketch};
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::daemon::{Fittings, Fleet};
use crate::gate::{rule_on, CheckBudget, Ruling};
use crate::headroom::{Bytes, Headroom, InUse, Machine, Reading, Spare};
use crate::places::{may_start, Asking, ChecksAtOnce, Places, Room};
use crate::policy::Policies;
use crate::tests::daemon::fitted_with;
use crate::tests::gate::{diff_evidence, judging, worktree};
use crate::tests::headroom::Plentiful;
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;

/// A machine with 95% of its memory in use, short of every shipped headroom.
struct Full;

impl Machine for Full {
    fn read(&self) -> Option<Reading> {
        Some(Reading::of(
            InUse::percent(10),
            InUse::percent(95),
            Bytes::gibibytes(500),
        ))
    }

    fn disk_free_at(&self, _path: &std::path::Path) -> Option<Bytes> {
        Some(Bytes::gibibytes(500))
    }
}

fn slow(name: &str) -> Gate<'_> {
    Gate::Check {
        name,
        run: "/bin/sleep 0.6",
        expect_exit_code: 0,
        when: &[],
    }
}

/// A Check that adds how many of its kind were running as it started to `tally`.
fn counting<'a>(name: &'a str, run: &'a str) -> Gate<'a> {
    Gate::Check {
        name,
        run,
        expect_exit_code: 0,
        when: &[],
    }
}

fn shipped_headroom() -> Headroom {
    Headroom::of(Spare::percent(15), Bytes::gibibytes(10))
}

/// Rule on a step of three 600ms Checks under `room`, and how long it took.
async fn ruled(room: &Room) -> (Ruling, Duration) {
    ruled_by(room, &[slow("build"), slow("test"), slow("storybook")]).await
}

/// Rule on a step gated on `gates` under `room`, and how long it took.
async fn ruled_by(room: &Room, gates: &[Gate<'_>]) -> (Ruling, Duration) {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates,
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    let worktree = worktree();
    let at_step = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/lib.rs"]);
    let began = Instant::now();
    let ruling = rule_on(
        at_step,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        crate::gate::Began::At(&Footprint::nothing()),
        &[],
        &work,
        CheckBudget::of(Duration::from_secs(20)),
        room,
        &judging(),
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await;
    (ruling, began.elapsed())
}

fn fitted(home: &TempDir) -> Fittings<FakeHarness, FakeVcs, FakeWorkProduct> {
    fitted_with(
        home,
        FakeWorkProduct::changed(&[]),
        FakeHarness::running("/bin/sh", &["-c", "true"]),
    )
}

#[test]
fn another_check_starts_only_beside_a_free_slot_and_a_machine_with_room() {
    let four = ChecksAtOnce::of(4);
    assert!(
        may_start(0, 1, four, true),
        "the first on the machine always starts, however short it is"
    );
    assert!(may_start(1, 1, four, false));
    assert!(
        !may_start(1, 1, four, true),
        "a short machine holds the second"
    );
    assert!(
        !may_start(4, 1, four, false),
        "a full bound holds the fifth"
    );
    assert_eq!(ChecksAtOnce::of(0).get(), 1, "a gate always has one slot");
}

/// A heavier ask counts every place it wants against the bound. #1102.
#[test]
fn a_heavier_ask_needs_every_place_it_wants_free() {
    let four = ChecksAtOnce::of(4);
    assert!(may_start(1, 3, four, false), "one held, three more fit");
    assert!(!may_start(2, 3, four, false), "two held, three more do not");
    assert!(
        may_start(0, 6, four, false),
        "nothing held, so a wider ask still starts alone"
    );
}

/// **A short machine slows the gate and fails nothing.** The bound is four, so
/// only the machine can be what ran these one after another.
#[tokio::test]
async fn a_short_machine_runs_a_steps_checks_one_after_another() {
    let room = Room::of(
        ChecksAtOnce::of(4),
        Arc::new(Full),
        Headroom::of(Spare::percent(15), Bytes::gibibytes(10)),
    );
    let (ruling, took) = ruled(&room).await;
    assert!(ruling.advanced(), "{ruling:?}");
    assert!(
        took >= Duration::from_millis(1700),
        "three 600ms checks took {took:?}, so two ran at once on a short machine"
    );
}

#[tokio::test]
async fn a_bound_of_one_runs_a_steps_checks_one_after_another() {
    let (ruling, took) = ruled(&Room::ignoring_the_machine(ChecksAtOnce::of(1))).await;
    assert!(ruling.advanced(), "{ruling:?}");
    assert!(
        took >= Duration::from_millis(1700),
        "three 600ms checks took {took:?} under a bound of one"
    );
}

/// **A limit like the other three**: saved, in force for the next gate, and kept
/// across a restart.
#[tokio::test]
async fn a_saved_checks_at_once_is_in_force_and_survives_a_restart() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    let now = fleet
        .save_limits(SaveLimits {
            checks_at_once: Some(ipc::ChecksAtOnce::new(2).expect("in range")),
            ..SaveLimits::default()
        })
        .await
        .expect("saved");
    assert_eq!(now.values.checks_at_once, 2);
    assert_eq!(now.shipped.checks_at_once, 4);
    assert_eq!(fleet.checks_at_once().get(), 2);
    drop(fleet);

    let fleet = Fleet::assembled(fitted(&home));
    let limits = fleet.get_limits().await.expect("limits read");
    assert_eq!(limits.values.checks_at_once, 2);
    assert_eq!(fleet.checks_at_once().get(), 2);
}

#[test]
fn the_shipped_limit_is_half_the_cores_from_one_to_eight() {
    assert_eq!(
        ChecksAtOnce::for_cores(1).get(),
        1,
        "one core still runs a Check"
    );
    assert_eq!(ChecksAtOnce::for_cores(4).get(), 2);
    assert_eq!(ChecksAtOnce::for_cores(10).get(), 5);
    assert_eq!(
        ChecksAtOnce::for_cores(64).get(),
        8,
        "eight is the most a save holds"
    );
}

/// **#1063's first line of done.** Two gates in one machine's places, six Checks
/// between them, each writing how many were running as it started.
#[tokio::test]
async fn two_gates_at_once_never_run_more_checks_between_them_than_the_machine_allows() {
    let seen = TempDir::new();
    let running = seen.path().join("running");
    std::fs::create_dir(&running).expect("a directory to count in");
    let tally = seen.path().join("tally");
    let run = format!(
        "/bin/sh -c 'touch {r}/$$; ls {r} | wc -l >> {t}; sleep 0.4; rm {r}/$$'",
        r = running.display(),
        t = tally.display()
    );
    let gates = [
        counting("build", &run),
        counting("test", &run),
        counting("storybook", &run),
    ];
    let places = Places::of(ChecksAtOnce::of(2));
    let one = Room::sharing(
        &places,
        Asking::Gate,
        Arc::new(Plentiful),
        shipped_headroom(),
        checks_runner::CheckWidth::read(1),
    );
    let other = one.clone();
    let began = Instant::now();
    let ((first, _), (second, _)) = tokio::join!(ruled_by(&one, &gates), ruled_by(&other, &gates));
    let took = began.elapsed();
    assert!(first.advanced(), "{first:?}");
    assert!(second.advanced(), "{second:?}");
    let counts: Vec<usize> = std::fs::read_to_string(&tally)
        .expect("each Check counted")
        .lines()
        .map(|line| line.trim().parse().expect("a count"))
        .collect();
    assert_eq!(counts.len(), 6, "{counts:?}");
    assert!(
        counts.iter().all(|at_once| *at_once <= 2),
        "more than the machine's two ran at once: {counts:?}"
    );
    assert!(
        took >= Duration::from_millis(1150),
        "six 400ms Checks two at a time took {took:?}"
    );
}

/// Every room one Fleet hands out, to any Job, waits in the same line.
#[tokio::test]
async fn every_room_one_fleet_hands_out_shares_the_machines_limit() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted(&home));
    fleet
        .save_limits(SaveLimits {
            checks_at_once: Some(ipc::ChecksAtOnce::new(1).expect("in range")),
            ..SaveLimits::default()
        })
        .await
        .expect("saved");
    let held = fleet.room(Asking::Gate).place().await;
    let drones = fleet.room(Asking::DronesRun);
    let waiting = tokio::spawn(async move { drones.place().await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !waiting.is_finished(),
        "a Drone's run started while a gate held the machine's one place"
    );
    drop(held);
    let _started = tokio::time::timeout(Duration::from_secs(1), waiting)
        .await
        .expect("it starts once the gate's place is back")
        .expect("it did not panic");
}

/// **The first on the machine starts however short it is**, and the next, even
/// another Job's first, waits for it rather than starting beside it.
#[tokio::test]
async fn on_a_short_machine_another_jobs_first_check_waits_for_the_one_running() {
    let places = Places::of(ChecksAtOnce::of(4));
    let one = Room::sharing(
        &places,
        Asking::Gate,
        Arc::new(Full),
        shipped_headroom(),
        checks_runner::CheckWidth::read(1),
    );
    let other = Room::sharing(
        &places,
        Asking::Gate,
        Arc::new(Full),
        shipped_headroom(),
        checks_runner::CheckWidth::read(1),
    );
    let held = one.place().await;
    let waiting = tokio::spawn(async move { other.place().await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !waiting.is_finished(),
        "a second Check started beside the first on a short machine"
    );
    drop(held);
    let _started = tokio::time::timeout(Duration::from_secs(1), waiting)
        .await
        .expect("it starts once the first finishes")
        .expect("it did not panic");
}
