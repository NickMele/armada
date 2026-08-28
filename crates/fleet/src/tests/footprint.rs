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
use api::{Next, Subscription};
use config::ResolvedWorkflow;
use core_model::Timestamp;
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Scoped, Sketch};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fittings, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A clock that answers the same instant until a test moves it.
struct Held(AtomicU64);

impl Held {
    fn at_nine() -> Arc<Held> {
        Arc::new(Held(AtomicU64::new(0)))
    }

    fn advance(&self, seconds: u64) {
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
fn a_fleet_reading(
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
async fn started(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
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
fn three_kinds() -> FakeWorkProduct {
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
    };
    let fleet = a_fleet_reading(&home, three_kinds(), Arc::clone(&clock), Some(scope));
    let mut watching = fleet.events().subscribe();
    started(&fleet, &home).await;
    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["src/parse.rs".to_string()],
        })
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
        fleet.work().asked().is_empty(),
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
        fleet.work().asked().is_empty(),
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
    assert_eq!(fleet.work().asked().len(), 1, "one reading, not two");

    clock.advance(10);
    fleet.turn().await.expect("a turn past the interval");
    assert_eq!(
        fleet.work().asked().len(),
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

    assert_eq!(fleet.work().asked().len(), 4, "read on every one of them");
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
