//! A working Drone told when a Job sharing its files claims them or lands. #998.
//!
//! **The transcript file is the record of what Fleet said**, for
//! `crate::tests::plan_person_told`'s reason, so these read it back.

use std::sync::Arc;
use std::time::Duration;

use core_model::JobId;
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::{Fittings, Fleet};
use crate::peers::{PeersChanged, SPACING};
use crate::slots::Concurrency;
use crate::tests::concurrency::{approved, calling_from, one_scoped_step, Fixture};
use crate::tests::daemon::{fittings, one};
use crate::tests::peer::Placing;
use crate::tests::planted::Held;
use crate::tests::tmp::TempDir;
use crate::transcript::transcript_of;

const HEADING: &str = "OTHER JOBS WRITING WHERE YOU ARE";

/// `concurrency::two_at_once`, on a clock the case can move.
fn two_at_once_on(home: &TempDir, clock: &Arc<Held>) -> (Fixture, Arc<Placing>) {
    let peers = Placing::nothing();
    let mut fittings: Fittings<FakeHarness, FakeVcs, FakeWorkProduct> =
        fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(one_scoped_step());
    fittings.concurrency = Concurrency::of(2);
    fittings.peers = Arc::clone(&peers) as Arc<dyn crate::peer::PeerOf>;
    fittings.clock = Arc::clone(clock) as Arc<dyn crate::clock::Clock>;
    (Fleet::assembled(fittings), peers)
}

/// Two approved Jobs, each Drone placed on its own port.
async fn two_jobs(
    fleet: &Fixture,
    peers: &Placing,
    home: &TempDir,
    ports: (u16, u16),
) -> (JobId, JobId) {
    let reader = approved(fleet, home, "fix the reader").await;
    let writer = approved(fleet, home, "fix the writer").await;
    let drones = fleet.drones_at_work();
    calling_from(peers, &drones, &reader, ports.0);
    calling_from(peers, &drones, &writer, ports.1);
    (reader, writer)
}

/// Declare as the Drone calling from `port`.
async fn declares(fleet: &Fixture, port: u16, paths: &[&str]) {
    let caller = api::Caller::at(
        format!("127.0.0.1:{port}")
            .parse()
            .expect("an address the plant was given"),
    );
    let job = fleet.caller_of(&caller).expect("the Drone is placed");
    fleet
        .declare_scope(
            &job,
            &DeclareScope {
                context_paths: paths.iter().map(|path| path.to_string()).collect(),
            },
        )
        .await
        .expect("the Drone declares");
}

/// What Fleet has written into this Job's Drone's transcript so far.
async fn transcript(fleet: &Fixture, home: &TempDir, job: &JobId) -> String {
    let record = fleet.load(job).await.expect("the Job");
    let drone = record.assigned_drone().expect("a live Drone").clone();
    let path = transcript_of(&home.path().to_string_lossy(), &record.handle(), &drone);
    std::fs::read_to_string(path).unwrap_or_default()
}

/// The transcript once it mentions the heading more than `past` times, or as it
/// stands when the wait runs out. The file is written off the turn, so a read
/// straight after one can be early.
async fn told_more_than(fleet: &Fixture, home: &TempDir, job: &JobId, past: usize) -> String {
    for _ in 0..200 {
        let written = transcript(fleet, home, job).await;
        if written.matches(HEADING).count() > past {
            return written;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    transcript(fleet, home, job).await
}

#[tokio::test]
async fn both_drones_are_told_which_job_claims_the_same_file() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, writer) = two_jobs(&fleet, &peers, &home, (51201, 51202)).await;

    declares(&fleet, 51201, &["crates/store/src/migrations.rs"]).await;
    declares(&fleet, 51202, &["crates/store"]).await;
    fleet.turn().await.expect("a turn");

    let heard = told_more_than(&fleet, &home, &reader, 0).await;
    assert!(
        heard.contains(HEADING),
        "the reader's Drone was told: {heard}"
    );
    assert!(
        heard.contains("fix the writer"),
        "it names the other Job: {heard}"
    );
    assert!(
        heard.contains("crates/store/src/migrations.rs"),
        "and the file: {heard}"
    );
    let heard = told_more_than(&fleet, &home, &writer, 0).await;
    assert!(
        heard.contains("fix the reader"),
        "the writer's Drone too: {heard}"
    );
}

/// **Said once, and spaced.** A plan declared again says nothing new, and a new
/// shared path waits out [`SPACING`] rather than interrupting straight away.
#[tokio::test]
async fn a_new_shared_path_waits_out_the_spacing_and_a_repeated_one_is_not_said() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, _) = two_jobs(&fleet, &peers, &home, (51203, 51204)).await;
    declares(
        &fleet,
        51203,
        &["crates/store/src/migrations.rs", "crates/ipc/src/lib.rs"],
    )
    .await;
    declares(&fleet, 51204, &["crates/store"]).await;
    fleet.turn().await.expect("a turn");
    let once = told_more_than(&fleet, &home, &reader, 0)
        .await
        .matches(HEADING)
        .count();
    assert!(once > 0, "the first overlap is told");

    declares(&fleet, 51204, &["crates/store", "crates/ipc"]).await;
    fleet.turn().await.expect("a turn inside the spacing");
    let early = told_more_than(&fleet, &home, &reader, once).await;
    assert_eq!(
        early.matches(HEADING).count(),
        once,
        "nothing inside the spacing: {early}"
    );

    clock.on(SPACING.as_secs());
    fleet.turn().await.expect("a turn past the spacing");
    let later = told_more_than(&fleet, &home, &reader, once).await;
    assert!(
        later.matches(HEADING).count() > once,
        "told once the spacing passed"
    );
    let last = &later[later.rfind(HEADING).expect("the heading")..];
    assert!(
        last.contains("crates/ipc/src/lib.rs"),
        "the new path: {last}"
    );
    assert!(
        !last.contains("migrations.rs"),
        "and not the one already said: {last}"
    );
}

#[tokio::test]
async fn a_drone_is_told_which_job_landed_and_what_it_changed_in_its_files() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, writer) = two_jobs(&fleet, &peers, &home, (51205, 51206)).await;
    declares(&fleet, 51206, &["src"]).await;

    // The reader changed `src/log.rs`, which is what the fixture's work product reads.
    let record = fleet.load(&reader).await.expect("the reader");
    fleet.kept_footprint(&record).await;
    fleet.landing_announced(&reader).await;
    fleet.turn().await.expect("a turn");

    let heard = told_more_than(&fleet, &home, &writer, 0).await;
    assert!(
        heard.contains("\\\"fix the reader\\\" landed"),
        "names who landed: {heard}"
    );
    assert!(
        heard.contains("src/log.rs"),
        "and what it changed there: {heard}"
    );
}

#[tokio::test]
async fn a_job_writing_elsewhere_hears_nothing_of_a_landing() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, writer) = two_jobs(&fleet, &peers, &home, (51207, 51208)).await;
    declares(&fleet, 51208, &["apps/desktop"]).await;

    let record = fleet.load(&reader).await.expect("the reader");
    fleet.kept_footprint(&record).await;
    fleet.landing_announced(&reader).await;
    fleet.turn().await.expect("a turn");

    let heard = told_more_than(&fleet, &home, &writer, 0).await;
    assert!(
        !heard.contains(HEADING),
        "nothing it writes was changed: {heard}"
    );
}

/// A long list is cut, and a landing in an opening brief says the rebase ran.
#[test]
fn the_turn_names_five_and_counts_the_rest() {
    use crate::peers::News;
    let news: Vec<News> = (0..7)
        .map(|at| News::Claimed {
            title: format!("job {at}"),
            paths: vec![format!("src/{at}.rs")],
        })
        .collect();
    let text = PeersChanged::injected(&news).text().to_string();
    assert!(text.contains("job 4") && !text.contains("job 5"), "{text}");
    assert!(text.contains("And 2 more."), "{text}");

    let landed = [News::Landed {
        title: "renumber migrations".to_string(),
        paths: vec!["crates/store/src/migrations.rs".to_string()],
    }];
    assert!(PeersChanged::opening(&landed)
        .text()
        .contains("brought into your branch as this part started"));
    assert!(PeersChanged::injected(&landed)
        .text()
        .contains("when your next part starts, not now"));
}
