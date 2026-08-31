//! What a Drone has changed on disk, while it is still changing it.
//!
//! # The cases are the conditions, not the happy path
//!
//! One case proves the list reaches the stream. The other five prove the read
//! does not happen — no Drone, no watcher, too soon, nothing moved — because
//! the whole risk in this capability is a repository read on a 250ms loop that
//! nobody asked for and nobody is reading.
//!
//! # The clock is held rather than ticking
//!
//! `Ticking` answers a later second on every call and a turn calls it more than
//! once, so the interval would always have passed and the throttle would never
//! be under test. [`Held`] moves only when a case moves it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::Change;
use api::{Daemon, Next, Subscription};
use config::ResolvedWorkflow;
use core_model::Timestamp;
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Scoped, Sketch};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::declared_by_the_one;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A clock that answers the same instant until a test moves it.
pub(super) struct Held(AtomicU64);

impl Held {
    pub(super) fn at_nine() -> Arc<Held> {
        Arc::new(Held(AtomicU64::new(0)))
    }

    pub(super) fn advance(&self, seconds: u64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let second = self.0.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T09:{:02}:{:02}.000Z",
            second / 60,
            second % 60
        ))
    }
}

/// One step, gated on nothing, so nothing but the footprint moves.
fn one_step(scope: Option<Scoped<'static>>) -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope,
        gaming: None,
    }])
}

/// A Fleet over that worktree reading and that clock, with one step on it.
pub(super) fn a_fleet_reading(
    home: &TempDir,
    work: FakeWorkProduct,
    clock: Arc<Held>,
    scope: Option<Scoped<'static>>,
) -> Fixture {
    let mut fittings = fittings(home, work);
    fittings.clock = clock;
    fittings.workflows = one(one_step(scope));
    Fleet::assembled(fittings)
}

/// Approve the Job, with a worktree on disk, so a Drone holds the slot.
pub(super) async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.unwrap();
    job.id().clone()
}

/// Everything the stream has for this subscriber right now.
///
/// A timeout rather than a count, because the assertions below are about how
/// many events there are and a helper that waited for a number would decide
/// the answer.
async fn drained(watching: &mut Subscription) -> Vec<ipc::JobFilesChanged> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(30), watching.next()).await
    {
        if let ipc::Event::JobFilesChanged(files) = delivered.event {
            seen.push(files);
        }
    }
    seen
}

/// A worktree in which one file was written, one edited and one removed.
pub(super) fn three_kinds() -> FakeWorkProduct {
    FakeWorkProduct::changing(&[
        ("src/parse.rs", Change::Modified),
        ("src/tokens.rs", Change::Added),
        ("src/legacy.rs", Change::Deleted),
    ])
}

// ------------------------------------------------------ what reaches Bridge

/// **The capability.** The files a working Drone has touched arrive as an
/// event, with what happened to each, while the step is still running — no
/// evidence submitted, no gate reached, nothing terminal.
#[tokio::test]
async fn a_working_drones_files_reach_the_stream_with_their_kinds() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let mut watching = fleet.events().subscribe();
    let job = started(&fleet, &home).await;

    fleet.turn().await.expect("a turn");
    let published = drained(&mut watching).await;

    assert_eq!(published.len(), 1, "one reading, one event");
    let footprint = &published[0];
    assert_eq!(footprint.job_id, ipc::JobId::from(&job));
    assert_eq!(footprint.step_id.as_str(), "implement");
    assert_eq!(
        footprint
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.change))
            .collect::<Vec<(&str, ipc::ChangeKind)>>(),
        vec![
            ("src/parse.rs", ipc::ChangeKind::Modified),
            ("src/tokens.rs", ipc::ChangeKind::Added),
            ("src/legacy.rs", ipc::ChangeKind::Deleted),
        ],
        "names and change kind, and the deletion among them"
    );
}

/// **A step that declared no plan is not a step that stayed on one.** Every row
/// reads `outside_plan: false` here, and `plan_declared` is what stops a
/// surface reading that as perfect obedience.
#[tokio::test]
async fn a_step_with_no_scope_reports_its_files_and_claims_no_plan() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let mut watching = fleet.events().subscribe();
    started(&fleet, &home).await;

    fleet.turn().await.expect("a turn");
    let published = drained(&mut watching).await;

    assert!(!published[0].plan_declared);
    assert!(
        published[0].files.iter().all(|file| !file.outside_plan),
        "there is no plan for a file to be outside of"
    );
}

/// The mark, and it is a mark rather than a judgement: the Job is still
/// running, nothing failed, and the drift check made this comparison already.
#[tokio::test]
async fn a_path_outside_the_declared_plan_is_marked_on_its_row() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let scope = Scoped {
        diff_check: true,
        at_step_start: true,
        exclude: &[],
        references: &[],
    };
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), Some(scope));
    let mut watching = fleet.events().subscribe();
    started(&fleet, &home).await;
    declared_by_the_one(
        &fleet,
        &DeclareScope {
            context_paths: vec!["src/parse.rs".to_string()],
        },
    )
    .await
    .expect("a declaration");

    fleet.turn().await.expect("a turn");
    let published = drained(&mut watching).await;

    assert!(published[0].plan_declared);
    assert_eq!(
        published[0]
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.outside_plan))
            .collect::<Vec<(&str, bool)>>(),
        vec![
            ("src/parse.rs", false),
            ("src/tokens.rs", true),
            ("src/legacy.rs", true),
        ]
    );
}

// -------------------------------------------------- what is never read

/// **The cold case.** Nothing is working, so nothing opens a repository — the
/// property the whole condition list exists to keep.
#[tokio::test]
async fn an_idle_fleet_reads_no_worktree() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let _watching = fleet.events().subscribe();

    for _ in 0..4 {
        clock.advance(10);
        fleet.turn().await.expect("a turn");
    }

    assert!(
        fleet.work().listed().is_empty(),
        "an empty slot has no worktree to read"
    );
}

/// **A Fleet nobody has open pays nothing.** This event has exactly one
/// consumer and it is a window somebody closed; a reading taken for a stream
/// with no subscriber is spent on a message that is discarded as it is made.
#[tokio::test]
async fn a_drone_nobody_is_watching_is_not_read() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    started(&fleet, &home).await;

    for _ in 0..4 {
        clock.advance(10);
        fleet.turn().await.expect("a turn");
    }

    assert!(
        fleet.work().listed().is_empty(),
        "nobody is subscribed, so there is nothing to publish to"
    );
}

/// **The throttle.** Fleet turns every 250ms and the footprint is not read on
/// every one of them, so a turn inside the interval opens nothing.
#[tokio::test]
async fn a_second_turn_inside_the_interval_takes_no_reading() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let _watching = fleet.events().subscribe();
    started(&fleet, &home).await;

    fleet.turn().await.expect("the first turn");
    fleet.turn().await.expect("a turn inside the interval");
    assert_eq!(fleet.work().listed().len(), 1, "one reading, not two");

    clock.advance(10);
    fleet.turn().await.expect("a turn past the interval");
    assert_eq!(
        fleet.work().listed().len(),
        2,
        "and the interval releases it"
    );
}

/// **A footprint that has not moved is not sent again.** The event channel is
/// drop-oldest and fixed, so a Drone that is thinking rather than writing must
/// not be evicting the Board's state changes.
#[tokio::test]
async fn an_unchanged_footprint_publishes_once() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let mut watching = fleet.events().subscribe();
    started(&fleet, &home).await;

    for _ in 0..4 {
        clock.advance(10);
        fleet.turn().await.expect("a turn");
    }

    assert_eq!(fleet.work().listed().len(), 4, "read on every one of them");
    assert_eq!(
        drained(&mut watching).await.len(),
        1,
        "and published only the reading that said something new"
    );
}

/// **A Bridge that joins mid-Job is not left with an empty list.** A resync
/// carries the Job rows and no footprint, so the reading after a client arrives
/// republishes whatever it finds rather than waiting for the Drone to write
/// again.
#[tokio::test]
async fn a_client_that_joins_mid_job_is_sent_the_current_footprint() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let first = fleet.events().subscribe();
    started(&fleet, &home).await;
    fleet.turn().await.expect("a turn while the first watches");
    drop(first);

    clock.advance(10);
    fleet.turn().await.expect("a turn with nobody watching");

    let mut second = fleet.events().subscribe();
    clock.advance(10);
    fleet.turn().await.expect("a turn after the second joined");

    let published = drained(&mut second).await;
    assert_eq!(
        published.len(),
        1,
        "the footprint has not moved, and the newcomer is still told what it is"
    );
    assert_eq!(published[0].files.len(), 3);
}

// ------------------------------------- what survives the Drone that made it

/// **The capability, and #127's definition of done.** Nobody subscribed, so no
/// live reading was ever taken or published — and the Job still says what it
/// touched once it is over.
///
/// A Job read a week after it finished and one read while it ran now answer the
/// same thing, which is the whole of what a record buys over an event.
#[tokio::test]
async fn a_job_nobody_watched_says_what_it_touched_once_it_is_over() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let job = started(&fleet, &home).await;

    fleet.kill_job(&job).await.expect("a terminal status");

    let detail = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served");
    let footprint = detail.footprint.expect("a footprint was recorded");
    assert_eq!(
        footprint
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.change))
            .collect::<Vec<(&str, ipc::ChangeKind)>>(),
        vec![
            ("src/parse.rs", ipc::ChangeKind::Modified),
            ("src/tokens.rs", ipc::ChangeKind::Added),
            ("src/legacy.rs", ipc::ChangeKind::Deleted),
        ],
        "the reading taken when it stopped, in the order it was found"
    );
    assert_eq!(
        footprint.recorded_at.as_str(),
        "2026-08-26T09:00:00.000Z",
        "stamped when the Job stopped, not when it was asked for"
    );
}

/// **Absent, never empty, on a Job that is still going.** A working Drone's
/// footprint is the live event, and a record drawn beside it would be an answer
/// to a question nobody had asked yet.
#[tokio::test]
async fn a_running_job_carries_no_record_of_what_it_touched() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let job = started(&fleet, &home).await;

    let detail = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served");
    assert!(
        detail.footprint.is_none(),
        "nothing is recorded until the Job stops"
    );
}

/// **A Job that never had a worktree records nothing, and that is absent.** It
/// is not a worktree that opened and held no change — the two are different
/// answers, and only one of them is a Drone that wrote nothing.
#[tokio::test]
async fn a_job_killed_before_it_was_ever_dispatched_records_nothing() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), None);
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .unwrap();

    fleet.kill_job(job.id()).await.expect("a terminal status");

    let detail = fleet
        .get_job(ipc::JobId::from(job.id()))
        .await
        .expect("the Job is served");
    assert!(
        detail.footprint.is_none(),
        "no worktree was ever made, so there was nothing to read"
    );
}

/// **A worktree that opened and held nothing is a record, not an absence.** It
/// is what a `diff_nonempty` check refuses, and a surface that drew it as "not
/// served" would hide the one finding it carries.
#[tokio::test]
async fn a_drone_that_changed_nothing_is_recorded_as_having_changed_nothing() {
    let home = TempDir::new();
    let clock = Held::at_nine();
    let fleet = a_fleet_reading(
        &home,
        FakeWorkProduct::changing(&[]),
        Arc::clone(&clock),
        None,
    );
    let job = started(&fleet, &home).await;

    fleet.kill_job(&job).await.expect("a terminal status");

    let detail = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job is served");
    let footprint = detail.footprint.expect("a reading was taken and recorded");
    assert!(
        footprint.files.is_empty(),
        "present and empty, which is not the same sentence as absent"
    );
}
