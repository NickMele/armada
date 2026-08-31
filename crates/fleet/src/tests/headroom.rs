//! A Job held back because the machine has too little left.
//!
//! **The pre-spawn half only.** Every case here is a Job that has not started.
//! A Job that runs out of something while running escalates as
//! `resource_exhausted` and nothing raises that yet — the last case asserts it
//! stays that way, because the cheapest way to conflate the two failures is to
//! make the poll fire on a Job it was never meant to see.
//!
//! **The board and admission are asserted together, never apart.** Each of them
//! alone can pass while the two disagree, which is the drift one predicate
//! exists to prevent.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::Daemon;
use testkit::FakeWorkProduct;

use crate::daemon::{Fittings, Fleet};
use crate::headroom::{
    disk_free, load_in_use, memory_in_use, Bytes, Headroom, InUse, Machine, Polling, Reading,
    Short, Spare, TheMachine,
};
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// The threshold that ships. Every fixture is held against the real one.
const SHIPPED: Headroom = Headroom::of(Spare::percent(15), Bytes::gibibytes(10));

/// A machine with room to spare on all three.
const PLENTY: Reading = Reading::of(
    InUse::percent(10),
    InUse::percent(20),
    Bytes::gibibytes(500),
);

/// A machine with plenty of everything, and no way to change its mind.
///
/// **What every fixture in this crate but this file's gets.** Reading the real
/// machine in `tests::daemon::fittings` would make every test here pass or fail
/// on whatever else the operator happens to be running.
pub struct Plentiful;

impl Machine for Plentiful {
    fn read(&self) -> Option<Reading> {
        Some(PLENTY)
    }
}

/// A machine a test moves, counting how often it was asked.
pub struct Plant {
    saw: Mutex<Option<Reading>>,
    reads: AtomicUsize,
}

impl Plant {
    fn showing(reading: Reading) -> Arc<Plant> {
        Arc::new(Plant {
            saw: Mutex::new(Some(reading)),
            reads: AtomicUsize::new(0),
        })
    }

    /// A machine that will not answer at all.
    fn unreadable() -> Arc<Plant> {
        Arc::new(Plant {
            saw: Mutex::new(None),
            reads: AtomicUsize::new(0),
        })
    }

    fn now_shows(&self, reading: Reading) {
        *self.saw.lock().expect("the plant is not poisoned") = Some(reading);
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl Machine for Plant {
    fn read(&self) -> Option<Reading> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        *self.saw.lock().expect("the plant is not poisoned")
    }
}

/// A Fleet reading a machine the test holds.
fn watching(home: &TempDir, plant: &Arc<Plant>, polling: Polling) -> Fixture {
    let mut fittings: Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> =
        fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.machine = Arc::clone(plant) as Arc<dyn Machine>;
    fittings.headroom = SHIPPED;
    fittings.polling = polling;
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

// ------------------------------------------------------------- the threshold

/// Enough of all three is no answer at all.
#[test]
fn a_machine_with_room_is_short_of_nothing() {
    assert_eq!(SHIPPED.short_of(&PLENTY), None);
}

/// Each resource on its own, so a passing suite cannot be one signal doing the
/// work of three.
#[test]
fn each_resource_holds_a_job_back_on_its_own() {
    let no_disk = Reading::of(InUse::percent(10), InUse::percent(20), Bytes::gibibytes(9));
    let no_cpu = Reading::of(
        InUse::percent(90),
        InUse::percent(20),
        Bytes::gibibytes(500),
    );
    let no_memory = Reading::of(
        InUse::percent(10),
        InUse::percent(95),
        Bytes::gibibytes(500),
    );

    assert_eq!(SHIPPED.short_of(&no_disk), Some(Short::Disk));
    assert_eq!(SHIPPED.short_of(&no_cpu), Some(Short::Cpu));
    assert_eq!(SHIPPED.short_of(&no_memory), Some(Short::Memory));
}

/// Exactly at the threshold is enough. The refusal is short *of* it, not at it,
/// so a machine sitting on the number is not held back for ever.
#[test]
fn a_machine_exactly_at_the_threshold_has_room() {
    let exactly = Reading::of(InUse::percent(85), InUse::percent(85), Bytes::gibibytes(10));
    assert_eq!(SHIPPED.short_of(&exactly), None);
}

/// Disk is named first when more than one is short: it is the one that has
/// actually run out here, and the only one whose exhaustion destroys work.
#[test]
fn disk_is_the_one_named_when_everything_is_short() {
    let nothing_left = Reading::of(InUse::percent(99), InUse::percent(99), Bytes::gibibytes(0));
    assert_eq!(SHIPPED.short_of(&nothing_left), Some(Short::Disk));
}

/// A load above the cores there are reads as over capacity rather than as full,
/// so the difference between busy and hopeless survives the reading.
#[test]
fn more_runnable_work_than_cores_reads_past_full() {
    assert_eq!(
        load_in_use("load averages: 25.00 9.60 6.61", 10),
        Some(InUse::percent(250))
    );
    assert_eq!(InUse::percent(250).spare(), Spare::percent(0));
}

// ---------------------------------------------------------------- the parsers

/// Both spellings of `uptime`. Darwin writes three bare numbers and Linux
/// writes three comma-separated ones, and the last three fields are the
/// averages on both.
#[test]
fn the_load_average_is_read_off_either_platforms_uptime() {
    let darwin = "15:39  up  6:07, 8 users, load averages: 4.00 9.60 6.61";
    let linux = "15:39:12 up 3 days,  2:11,  5 users,  load average: 4.00, 0.58, 0.59";

    assert_eq!(load_in_use(darwin, 8), Some(InUse::percent(50)));
    assert_eq!(load_in_use(linux, 8), Some(InUse::percent(50)));
}

/// A machine reporting nothing readable is not a machine reporting zero.
#[test]
fn an_unreadable_uptime_is_no_reading() {
    assert_eq!(load_in_use("", 8), None);
    assert_eq!(load_in_use("uptime: command not found", 8), None);
}

/// `ps` writes one share per line, and a process that exits mid-walk can leave
/// a short one. The short line is skipped rather than taking the reading down.
#[test]
fn memory_is_the_sum_of_what_ps_printed() {
    assert_eq!(
        memory_in_use(" 12.4\n  0.6\n 30.0\n"),
        Some(InUse::percent(43))
    );
    assert_eq!(memory_in_use(" 12.4\n\n  0.6\n"), Some(InUse::percent(13)));
    assert_eq!(memory_in_use("\n"), None);
}

/// The fourth field of `df -P -k`, in kibibytes, as bytes.
#[test]
fn the_free_disk_is_the_fourth_field_of_df() {
    let said = "Filesystem   1024-blocks      Used Available Capacity  Mounted on\n\
                /dev/disk3s5   482797652 227768264   1048576    50%    /System/Volumes/Data\n";
    assert_eq!(disk_free(said), Some(Bytes::gibibytes(1)));
    assert_eq!(disk_free("Filesystem 1024-blocks Used Available\n"), None);
}

// ------------------------------------------------------- against the machine

/// **The one case that asks the operating system.** Every parser above is fed a
/// string somebody typed, and a string somebody typed cannot tell you the flags
/// are right — `df` without `-P` wraps a long device name onto its own line and
/// moves every column. This reads the machine the tests are running on and
/// asserts only that all three came back, because the numbers are whatever the
/// machine happens to be doing.
#[test]
fn the_real_machine_answers_all_three() {
    let read = TheMachine::watching(".")
        .read()
        .expect("uptime, ps and df all answered");

    assert!(
        read.disk_free().count() > 0,
        "a volume with no bytes free at all would not be running this test"
    );
    assert!(read.memory().percentage() > 0, "something is using memory");
}

// ------------------------------------------------------------ against a Fleet

/// A Job approved onto a machine with no disk left stays where it was, and the
/// Board says so. **Both halves in one case**: the label and the admission are
/// one predicate, and apart each can pass while the two disagree.
#[tokio::test]
async fn a_job_is_held_back_when_the_disk_is_nearly_full() {
    let home = TempDir::new();
    let plant = Plant::showing(Reading::of(
        InUse::percent(10),
        InUse::percent(20),
        Bytes::gibibytes(2),
    ));
    let fleet = watching(&home, &plant, Polling::every(Duration::ZERO));
    let job = approved(&fleet, &home, "a change that needs a worktree").await;

    assert!(
        fleet.working_on().await.is_empty(),
        "nothing was started on a volume that cannot hold a worktree"
    );
    assert_eq!(
        board(&fleet, &job).await,
        (
            "queued".to_string(),
            Some("waiting_on_resources".to_string())
        ),
        "and the Board says the same thing admission decided"
    );
}

/// The reason is recomputed, never stored: the machine frees and the very next
/// turn starts the Job that was held.
#[tokio::test]
async fn the_job_starts_when_the_machine_frees() {
    let home = TempDir::new();
    let plant = Plant::showing(Reading::of(
        InUse::percent(10),
        InUse::percent(20),
        Bytes::gibibytes(2),
    ));
    let fleet = watching(&home, &plant, Polling::every(Duration::ZERO));
    let job = approved(&fleet, &home, "a change that waits for room").await;
    assert!(fleet.working_on().await.is_empty());

    plant.now_shows(PLENTY);
    fleet.turn().await.expect("the loop turns");

    assert_eq!(
        board(&fleet, &job).await,
        ("running".to_string(), None),
        "a stored reason would still be saying it was held"
    );
    assert_eq!(fleet.working_on().await, vec![job]);
}

/// CPU holds a Job back the same way disk does, so the poll is not one signal
/// wearing three names.
#[tokio::test]
async fn a_loaded_machine_holds_a_job_back_too() {
    let home = TempDir::new();
    let plant = Plant::showing(Reading::of(
        InUse::percent(97),
        InUse::percent(20),
        Bytes::gibibytes(500),
    ));
    let fleet = watching(&home, &plant, Polling::every(Duration::ZERO));
    let job = approved(&fleet, &home, "a change onto a busy machine").await;

    assert!(fleet.working_on().await.is_empty());
    assert_eq!(
        board(&fleet, &job).await.1,
        Some("waiting_on_resources".to_string())
    );
}

/// **A machine that cannot be read admits.** A reading that fails must not hold
/// every Job back for ever with nothing saying why — that is a Fleet that looks
/// dead. The bound is still the bound.
#[tokio::test]
async fn a_machine_that_will_not_answer_does_not_hold_anything_back() {
    let home = TempDir::new();
    let plant = Plant::unreadable();
    let fleet = watching(&home, &plant, Polling::every(Duration::ZERO));
    let job = approved(&fleet, &home, "a change on an unreadable machine").await;

    assert_eq!(fleet.working_on().await, vec![job.clone()]);
    assert_eq!(board(&fleet, &job).await, ("running".to_string(), None));
    assert!(plant.reads() > 0, "and it did ask");
}

/// The reading is taken once inside the interval, however many times it is
/// asked for. Three processes on every turn — four a second — is what the poll
/// interval exists to stop.
///
/// **Both intervals in one case.** An idle Fleet turning three times asks three
/// times with no interval and once with one, so the assertion is about the
/// interval rather than about how often anything happened to look.
#[tokio::test]
async fn a_reading_is_not_taken_again_inside_the_poll_interval() {
    let held = TempDir::new();
    let plant = Plant::showing(PLENTY);
    let fleet = watching(&held, &plant, Polling::every(Duration::from_secs(3600)));
    for _ in 0..3 {
        fleet.turn().await.expect("the loop turns");
    }
    assert_eq!(
        plant.reads(),
        1,
        "the machine was read once and the answer was reused"
    );

    let fresh = TempDir::new();
    let every_time = Plant::showing(PLENTY);
    let eager = watching(&fresh, &every_time, Polling::every(Duration::ZERO));
    for _ in 0..3 {
        eager.turn().await.expect("the loop turns");
    }
    assert_eq!(
        every_time.reads(),
        3,
        "and with no interval it is read every time it is asked"
    );
}

/// **The bound is asked first, and a Fleet already at its cap never pays for a
/// reading.** The cheap question comes before the expensive one.
#[tokio::test]
async fn a_spent_bound_reads_no_machine() {
    let home = TempDir::new();
    let plant = Plant::showing(PLENTY);
    let fleet = watching(&home, &plant, Polling::every(Duration::ZERO));
    let first = approved(&fleet, &home, "the first").await;
    let before = plant.reads();
    let second = approved(&fleet, &home, "the second").await;

    assert_eq!(fleet.working_on().await, vec![first]);
    assert_eq!(
        board(&fleet, &second).await.1,
        Some("waiting_on_resources".to_string()),
        "the bound answers with the same label the machine would have"
    );
    assert_eq!(
        plant.reads(),
        before,
        "and nothing asked the machine, because the bound had already said no"
    );
}

/// **A running Job is not touched by this.** The machine emptying under a Job
/// that has already started is `resource_exhausted`, which has nowhere to queue
/// back to and which nothing raises. A poll that reached a running Job would be
/// the two failures conflated.
#[tokio::test]
async fn a_running_job_is_left_alone_when_the_machine_fills() {
    let home = TempDir::new();
    let plant = Plant::showing(PLENTY);
    let fleet = watching(&home, &plant, Polling::every(Duration::ZERO));
    let job = approved(&fleet, &home, "a change already under way").await;
    assert_eq!(fleet.working_on().await, vec![job.clone()]);

    plant.now_shows(Reading::of(
        InUse::percent(99),
        InUse::percent(99),
        Bytes::gibibytes(0),
    ));
    fleet.turn().await.expect("the loop turns");

    assert_eq!(
        board(&fleet, &job).await,
        ("running".to_string(), None),
        "it kept running: nothing here escalates, and nothing here queues it back"
    );
    assert_eq!(fleet.working_on().await, vec![job]);
}
