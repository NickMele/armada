//! A working Drone told when a Job sharing its files claims them or lands. #998.
//!
//! **The transcript file is the record of what Fleet said**, for
//! `crate::tests::plan_person_told`'s reason, so these read it back.

use std::sync::Arc;

use core_model::JobId;
use ipc::mcp::{DeclareScope, LeaveNote};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::{Fittings, Fleet};
use crate::peers::{PeersChanged, SPACING};
use crate::slots::Concurrency;
use crate::tests::concurrency::{approved, calling_from, one_scoped_step, Fixture};
use crate::tests::daemon::{fittings, one};
use crate::tests::peer::Placing;
use crate::tests::planted::Held;
use crate::tests::tmp::TempDir;
use crate::tests::transcript::reading::{Transcript, A_WRITER_HAS_LONG_ENOUGH};

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

/// This Job's Drone's transcript.
async fn transcript(fleet: &Fixture, home: &TempDir, job: &JobId) -> Transcript {
    let record = fleet.load(job).await.expect("the Job");
    let drone = record.assigned_drone().expect("a live Drone").clone();
    Transcript::of(home, &record.handle(), &drone)
}

/// The transcript once it mentions the heading more than `past` times, or as it
/// stands when the wait runs out.
async fn told_more_than(fleet: &Fixture, home: &TempDir, job: &JobId, past: usize) -> String {
    transcript(fleet, home, job)
        .await
        .until(A_WRITER_HAS_LONG_ENOUGH, |written| {
            written.matches(HEADING).count() > past
        })
        .await
        .unwrap_or_else(|stood| stood)
}

/// The transcript once it has stopped growing, for the two cases asserting a
/// Drone was **not** told. Waiting for a turn that must not arrive would wait
/// out the whole patience on every passing run, and prove nothing more.
async fn told_nothing_more(fleet: &Fixture, home: &TempDir, job: &JobId) -> String {
    transcript(fleet, home, job)
        .await
        .settled(A_WRITER_HAS_LONG_ENOUGH)
        .await
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
    let early = told_nothing_more(&fleet, &home, &reader).await;
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
        heard.contains(&format!(
            "\\\"fix the reader\\\" ({}) landed",
            record.handle()
        )),
        "names who landed, by the handle a note would be addressed to: {heard}"
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

    let heard = told_nothing_more(&fleet, &home, &writer).await;
    assert!(
        !heard.contains(HEADING),
        "nothing it writes was changed: {heard}"
    );
}

/// Two overlapping Jobs, both told of each other, with the spacing already run
/// out so the next turn delivers whatever is queued.
async fn overlapping(
    fleet: &Fixture,
    peers: &Placing,
    home: &TempDir,
    clock: &Held,
    ports: (u16, u16),
) -> (JobId, JobId) {
    let (reader, writer) = two_jobs(fleet, peers, home, ports).await;
    declares(fleet, ports.0, &["crates/store/src/migrations.rs"]).await;
    declares(fleet, ports.1, &["crates/store"]).await;
    fleet.turn().await.expect("a turn");
    told_more_than(fleet, home, &reader, 0).await;
    clock.on(SPACING.as_secs());
    (reader, writer)
}

fn a_note(to: &str, said: &str) -> LeaveNote {
    LeaveNote {
        to: to.to_string(),
        note: said.to_string(),
    }
}

/// **The definition of done.** One Drone leaves another a note, and the other
/// hears it behind the marker, named as that Drone's words.
#[tokio::test]
async fn a_note_reaches_the_other_jobs_drone_fenced_as_its_senders_words() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, writer) = overlapping(&fleet, &peers, &home, &clock, (51209, 51210)).await;
    let before = told_more_than(&fleet, &home, &reader, 0)
        .await
        .matches(HEADING)
        .count();
    let handle = fleet.load(&reader).await.expect("the reader").handle();

    fleet
        .leave_note(
            &writer,
            &a_note(&handle, "I am renumbering migrations, take V64 after me"),
        )
        .await
        .expect("a note to a Job sharing a path is taken");
    fleet.turn().await.expect("a turn");

    let heard = told_more_than(&fleet, &home, &reader, before).await;
    let last = &heard[heard.rfind(HEADING).expect("a peer turn")..];
    assert!(
        last.contains("> I am renumbering migrations, take V64 after me"),
        "the words, behind the marker: {last}"
    );
    assert!(
        last.contains("fix the writer") && last.contains("not Armada's"),
        "named as the other Drone's words: {last}"
    );
}

#[tokio::test]
async fn a_note_is_refused_where_it_has_nothing_to_warn_about_or_nobody_to_reach() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, writer) = two_jobs(&fleet, &peers, &home, (51211, 51212)).await;
    declares(&fleet, 51211, &["crates/store"]).await;
    declares(&fleet, 51212, &["apps/desktop"]).await;
    let handle = fleet.load(&reader).await.expect("the reader").handle();

    let apart = fleet
        .leave_note(&writer, &a_note(&handle, "take V64"))
        .await
        .expect_err("the two Jobs share no path");
    assert!(
        apart.because.contains("claims no path"),
        "{}",
        apart.because
    );

    let nobody = fleet
        .leave_note(&writer, &a_note("99-nobody", "take V64"))
        .await
        .expect_err("no such Job");
    assert!(
        nobody.because.contains("no unfinished Job"),
        "{}",
        nobody.because
    );

    let own = fleet.load(&writer).await.expect("the writer").handle();
    let itself = fleet
        .leave_note(&writer, &a_note(&own, "take V64"))
        .await
        .expect_err("a note to itself");
    assert!(itself.because.contains("own"), "{}", itself.because);
}

#[tokio::test]
async fn a_second_note_to_the_same_job_inside_the_spacing_is_refused() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let (fleet, peers) = two_at_once_on(&home, &clock);
    let (reader, writer) = overlapping(&fleet, &peers, &home, &clock, (51213, 51214)).await;
    let handle = fleet.load(&reader).await.expect("the reader").handle();

    fleet
        .leave_note(&writer, &a_note(&handle, "take V64"))
        .await
        .expect("the first note");
    let again = fleet
        .leave_note(&writer, &a_note(&handle, "and V65"))
        .await
        .expect_err("a second inside the spacing");
    assert!(again.because.contains("one note"), "{}", again.because);

    clock.on(SPACING.as_secs());
    fleet
        .leave_note(&writer, &a_note(&handle, "and V65"))
        .await
        .expect("taken once the spacing has passed");
}

/// A long list is cut, a note is never cut into the count, and a landing in an
/// opening brief says the rebase ran.
#[test]
fn the_turn_names_five_and_counts_the_rest() {
    use crate::peers::News;
    let mut news: Vec<News> = (0..7)
        .map(|at| News::Claimed {
            title: format!("job {at}"),
            handle: format!("{at}-job"),
            paths: vec![format!("src/{at}.rs")],
        })
        .collect();
    news.push(News::Note {
        title: "job 9".to_string(),
        handle: "9-job".to_string(),
        said: "one\nline two".to_string(),
    });
    let text = PeersChanged::injected(&news).text().to_string();
    assert!(text.contains("job 4") && !text.contains("job 5"), "{text}");
    assert!(text.contains("And 2 more."), "{text}");
    assert!(
        text.contains("> one\n> line two"),
        "every line of a note is fenced, and none is counted away: {text}"
    );

    let landed = [News::Landed {
        title: "renumber migrations".to_string(),
        handle: "14-renumber-migrations".to_string(),
        paths: vec!["crates/store/src/migrations.rs".to_string()],
    }];
    assert!(PeersChanged::opening(&landed)
        .text()
        .contains("brought into your branch as this part started"));
    assert!(PeersChanged::injected(&landed)
        .text()
        .contains("when your next part starts, not now"));
}
