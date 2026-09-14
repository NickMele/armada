//! How many of a step's Checks run at once, and what a short machine does to the
//! next one: it waits for a running Check to finish, and the first always starts.
//! #284.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::Footprint;
use api::{Commands, Queries};
use ipc::SaveLimits;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Sketch};
use verification::{Lifted, Request};

use crate::at_step::AtStep;
use crate::checking::{may_start, ChecksAtOnce, Room};
use crate::daemon::{Fittings, Fleet};
use crate::gate::{rule_on, CheckBudget, Ruling};
use crate::headroom::{Bytes, Headroom, InUse, Machine, Reading, Spare};
use crate::policy::Policies;
use crate::tests::daemon::fitted_with;
use crate::tests::gate::{diff_evidence, judging, worktree};
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

/// Rule on a step of three 600ms Checks under `room`, and how long it took.
async fn ruled(room: &Room) -> (Ruling, Duration) {
    let gates = [slow("build"), slow("test"), slow("storybook")];
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &gates,
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
        Some(&Footprint::nothing()),
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
        may_start(0, four, true),
        "the first always starts, however short the machine"
    );
    assert!(may_start(1, four, false));
    assert!(
        !may_start(1, four, true),
        "a short machine holds the second"
    );
    assert!(!may_start(4, four, false), "a full bound holds the fifth");
    assert_eq!(ChecksAtOnce::of(0).get(), 1, "a gate always has one slot");
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
