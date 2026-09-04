//! Noticing an edit to `armada.yml`, and what reaches a Job because of it.
//!
//! Three subjects, and they are separable on purpose:
//!
//! | | What it proves | What it needs |
//! |---|---|---|
//! | [`settling`] | one save is one read, however many events it made | no filesystem, no clock anybody waits on |
//! | the watch | a rename over the target is noticed at all | a real file and a real poller |
//! | the resolution | the new number is what the next step is given | a Job, and `fleet::Liveness::at` |
//!
//! **The third is the one `#430` is about.** A reload that reached a `Manifest`
//! nothing consulted would be this defect with more code in it.

use std::time::Duration;

use config::{Adopted, LoadError, Manifest, Moved};
use core_model::StepId;
use fleet::Liveness;
use tokio::sync::mpsc;

use crate::tests::TempDir;
use crate::watching::{settling, watch_every};

const PATIENT: &str = r#"
version: 1
id: armada
base: main
checks:
  build:
    run: cargo build --workspace
drone:
  quiet_after_seconds: 300
  poke_limit: 2
"#;

/// Short enough that a test finishes, long enough that a poller sees a rename
/// as two readings rather than one. See `watching::watch_every`.
const QUICK_POLL: Duration = Duration::from_millis(20);
const QUICK_SETTLE: Duration = Duration::from_millis(120);

/// The debounce a paused-time test drives. Whole seconds, because what is
/// being asserted is how many reads a stream of events produced rather than
/// when — and the arithmetic reads at a glance.
const QUIET: Duration = Duration::from_secs(1);
const LATEST: Duration = Duration::from_secs(10);

/// How long a test waits for a save to be noticed before calling it a failure.
/// Two orders of magnitude over the settle window, because what would make this
/// flake is a loaded machine rather than a wrong number.
const PATIENCE: Duration = Duration::from_secs(10);

#[tokio::test(start_paused = true)]
async fn a_save_that_made_three_events_is_read_once() {
    let (events, arrived) = mpsc::unbounded_channel();
    let (read, mut reads) = mpsc::unbounded_channel();
    tokio::spawn(settling(arrived, QUIET, LATEST, move || {
        let _ = read.send(());
    }));

    // What one save looks like: a truncate, a write, and the editor touching
    // the mtime again on its way out.
    for _ in 0..3 {
        events.send(()).expect("the settling task is listening");
    }
    // Paused time, so this advances the clock rather than waiting: the settle
    // window passes with the channel idle and the read happens.
    tokio::time::sleep(Duration::from_secs(5)).await;

    assert_eq!(reads.try_recv(), Ok(()));
    assert!(
        reads.try_recv().is_err(),
        "the file was read more than once"
    );
}

#[tokio::test(start_paused = true)]
async fn two_saves_a_minute_apart_are_two_reads() {
    let (events, arrived) = mpsc::unbounded_channel();
    let (read, mut reads) = mpsc::unbounded_channel();
    tokio::spawn(settling(arrived, QUIET, LATEST, move || {
        let _ = read.send(());
    }));

    events.send(()).expect("the settling task is listening");
    tokio::time::sleep(Duration::from_secs(60)).await;
    events.send(()).expect("the settling task is listening");
    tokio::time::sleep(Duration::from_secs(60)).await;

    assert_eq!(reads.try_recv(), Ok(()));
    assert_eq!(reads.try_recv(), Ok(()));
    assert!(reads.try_recv().is_err(), "a third read nobody asked for");
}

#[tokio::test(start_paused = true)]
async fn a_file_that_never_stops_changing_is_still_read() {
    let (events, arrived) = mpsc::unbounded_channel();
    let (read, mut reads) = mpsc::unbounded_channel();
    tokio::spawn(settling(arrived, QUIET, LATEST, move || {
        let _ = read.send(());
    }));

    // Something writing the file faster than the debounce window closes. A
    // debounce with no ceiling would never read it, which is `#430`'s own
    // defect reached by a longer road.
    for _ in 0..40 {
        events.send(()).expect("the settling task is listening");
        tokio::time::sleep(QUIET / 2).await;
    }

    assert_eq!(reads.try_recv(), Ok(()), "the ceiling never fired");
}

#[tokio::test(start_paused = true)]
async fn nothing_is_read_when_the_watch_is_dropped_mid_save() {
    let (events, arrived) = mpsc::unbounded_channel();
    let (read, mut reads) = mpsc::unbounded_channel();
    let task = tokio::spawn(settling(arrived, QUIET, LATEST, move || {
        let _ = read.send(());
    }));

    events.send(()).expect("the settling task is listening");
    // Fleet stopping. Adopting a number nothing will go on to use is work done
    // on the way out.
    drop(events);
    task.await.expect("it ends rather than hanging");

    assert!(reads.try_recv().is_err(), "it read on the way out");
}

/// A rename over the target — **the save that defeats a watch on the inode**,
/// and the common one on macOS.
#[tokio::test(flavor = "multi_thread")]
async fn a_rename_over_the_manifest_is_noticed_and_read_once() {
    let repository = TempDir::new();
    repository.write("armada.yml", PATIENT);
    let file = repository.path().join("armada.yml");
    let (manifest, reloads) = Manifest::reloadable(&file).expect("it loads");
    let (told, mut readings) = mpsc::unbounded_channel();

    let _watching = watch_every(reloads, QUICK_POLL, QUICK_SETTLE, move |read| {
        let _ = told.send(summarised(read));
    })
    .expect("a watch on a file that is there");

    repository.write("armada.yml.new", &PATIENT.replace("300", "45"));
    std::fs::rename(
        repository.path().join("armada.yml.new"),
        repository.path().join("armada.yml"),
    )
    .expect("the rename over it");

    let read = next(&mut readings).await;
    assert_eq!(read.expect("it parsed").len(), 1);
    assert_eq!(manifest.quiet_after_seconds(), Some(45));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_that_will_not_parse_is_said_and_leaves_the_fleet_alone() {
    let repository = TempDir::new();
    repository.write("armada.yml", PATIENT);
    let file = repository.path().join("armada.yml");
    let (manifest, reloads) = Manifest::reloadable(&file).expect("it loads");
    let (told, mut readings) = mpsc::unbounded_channel();

    let _watching = watch_every(reloads, QUICK_POLL, QUICK_SETTLE, move |read| {
        let _ = told.send(summarised(read));
    })
    .expect("a watch on a file that is there");

    repository.write(
        "armada.yml",
        "version: 1\nid: armada\nchecks: [ this is a list ]\n",
    );

    let read = next(&mut readings).await;
    let why = read.expect_err("a file that does not parse");
    assert!(why.contains("armada.yml"), "{why}");
    // The fleet did not stop and did not change its terms. That is the whole
    // claim: a mistyped number costs the edit, never the running Jobs.
    assert_eq!(manifest.quiet_after_seconds(), Some(300));
    assert_eq!(manifest.poke_limit(), Some(2));
}

/// The one that matters: the number a Job's next step is given moves.
#[tokio::test(flavor = "multi_thread")]
async fn an_edit_reaches_the_next_step_of_a_running_job() {
    let repository = TempDir::new();
    repository.write("armada.yml", PATIENT);
    let file = repository.path().join("armada.yml");
    let (manifest, reloads) = Manifest::reloadable(&file).expect("it loads");
    let (told, mut readings) = mpsc::unbounded_channel();

    // A Job whose step declares no patience of its own, which is what falls
    // back to the Manifest tier — the tier `#430` says is live and was not.
    let job = testkit::asking(
        "make the suite pass",
        "the reader drops a line",
        &["it passes"],
    );
    let step = StepId::new("the-step");
    // What the composition root is running with, and what a step boundary
    // resolves against it. `fleet::Liveness::at` is the only place the order of
    // the three tiers is written.
    let constant = Liveness::of(Duration::from_secs(120), 2);
    assert_eq!(
        constant.at(&manifest, &job, &step).quiet_after(),
        Duration::from_secs(300)
    );

    let _watching = watch_every(reloads, QUICK_POLL, QUICK_SETTLE, move |read| {
        let _ = told.send(summarised(read));
    })
    .expect("a watch on a file that is there");

    repository.write(
        "armada.yml",
        &PATIENT.replace("quiet_after_seconds: 300", "quiet_after_seconds: 45"),
    );
    next(&mut readings).await.expect("it parsed");

    // Resolved again, as the next step boundary would. No restart, no second
    // `Setup`, and the Job is the one that was already there.
    assert_eq!(
        constant.at(&manifest, &job, &step).quiet_after(),
        Duration::from_secs(45)
    );
    assert_eq!(constant.at(&manifest, &job, &step).pokes(), 2);
}

/// `LoadError` is not `Clone` and a test does not need it to be: what a reading
/// is asserted on is which keys moved, or what the refusal said.
fn summarised(read: Result<Adopted, LoadError>) -> Result<Vec<Moved>, String> {
    read.map(|adopted| adopted.moved().to_vec())
        .map_err(|why| why.to_string())
}

/// The next reading, or a failure naming the wait rather than hanging the
/// suite. A watch that never fires is the defect this whole module is about.
async fn next(
    readings: &mut mpsc::UnboundedReceiver<Result<Vec<Moved>, String>>,
) -> Result<Vec<Moved>, String> {
    tokio::time::timeout(PATIENCE, readings.recv())
        .await
        .expect("the save was noticed")
        .expect("the watch is still running")
}
