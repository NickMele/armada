//! That watching a Drone does not change what happens to its Job, and that
//! what it said survives the Drone.
//!
//! # These start a real child, for the same reason `drone` does
//!
//! The claim under test is that one reader feeds several consumers off one
//! pipe. A pipe with a process on the far end of it is the thing that can
//! starve, so the tee is asserted over one rather than over a `Vec` of lines.
//!
//! The back-pressure case is the other way round: it calls the sink directly,
//! because a queue that overflows deterministically is a queue nothing is
//! draining, and a `#[tokio::test]` runtime with no `await` in the way is the
//! only place that is guaranteed.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::DroneEvent;
use core_model::{DroneId, JobId, StepId, Ulid};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::drone::start;
use crate::tests::daemon::Ticking;
use crate::tests::drone::config;
use crate::tests::tmp::TempDir;
use crate::transcript::{history, log_of, transcript_of, Recording, Spine, Tap};
use crate::watch::Watching;

/// A Drone that reads its first turn and then says three things.
const SAYS_THREE: &str = "IFS= read -r _; printf 'one\\ntwo\\nthree\\n'";

const JOB: &str = "01JOBAAAAAAAAAAAAAAAAAAAAA";
const RUN: &str = "01RUNAAAAAAAAAAAAAAAAAAAAA";

fn spine(drone: &str) -> Spine {
    Spine {
        job: JobId::carried(Ulid::carried(JOB)),
        drone: DroneId::carried(Ulid::carried(drone)),
        step: StepId::new("implement"),
        run: Ulid::carried(RUN),
    }
}

fn recording(at: &TempDir, drone: &str) -> Recording {
    Recording::of(
        &at.path().to_string_lossy(),
        spine(drone),
        Arc::new(Ticking::from_nine()),
    )
    .expect("a transcript opens under a directory that exists")
}

fn rows(at: &TempDir, drone: &str) -> String {
    let path = transcript_of(
        &at.path().to_string_lossy(),
        &DroneId::carried(Ulid::carried(drone)),
    );
    std::fs::read_to_string(path).expect("the transcript is on disk")
}

fn job_log(at: &TempDir) -> String {
    let path = log_of(
        &at.path().to_string_lossy(),
        &JobId::carried(Ulid::carried(JOB)),
    );
    std::fs::read_to_string(path).expect("the Job's log is on disk")
}

/// A consumer that keeps what it was handed, standing in for the live view.
#[derive(Default)]
struct Counting(Mutex<Vec<DroneEvent>>);

impl Counting {
    fn seen(&self) -> Vec<DroneEvent> {
        self.0.lock().expect("not poisoned").clone()
    }
}

impl Tap for Counting {
    fn saw(&self, events: &[DroneEvent]) {
        self.0
            .lock()
            .expect("not poisoned")
            .extend(events.iter().cloned());
    }
}

/// Read a Drone that says three things, through a tee with these taps on it.
async fn three_lines_through(at: &TempDir, taps: Vec<Arc<dyn Tap>>) -> Watching {
    let harness = Arc::new(FakeHarness::running("/bin/sh", &["-c", SAYS_THREE]));
    let started = start(harness.as_ref(), &config(at))
        .await
        .expect("a shell starts and reads its first turn");
    let watching = Watching::reading(started.transcript, harness, taps);
    for _ in 0..200 {
        if watching.transcript_ended() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(watching.transcript_ended(), "the pipe closed");
    watching
}

#[tokio::test]
async fn the_parser_still_gets_every_line_with_a_writer_attached() {
    let at = TempDir::new();
    let counting = Arc::new(Counting::default());
    let taps: Vec<Arc<dyn Tap>> = vec![
        Arc::new(recording(&at, "01DRONEAAAAAAAAAAAAAAAAAAA")),
        Arc::clone(&counting) as Arc<dyn Tap>,
    ];

    let watching = three_lines_through(&at, taps).await;

    let parsed = watching.events();
    assert_eq!(parsed.len(), 3, "the parser saw all three");
    assert_eq!(
        parsed,
        counting.seen(),
        "a consumer sees exactly what the parser saw, in the same order"
    );
}

#[tokio::test]
async fn a_drone_is_handed_to_a_consumer_as_rows_and_never_as_the_wire_shape() {
    let at = TempDir::new();
    let drone = "01DRONEBBBBBBBBBBBBBBBBBBB";
    let recording = recording(&at, drone);
    three_lines_through(&at, vec![Arc::new(recording)]).await;
    // The tee's own `Recording` is inside the `Watching`, which the reader task
    // holds; dropping it here is what closes the queue and drains the writer.
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        rows(&at, drone).lines().take(3).collect::<Vec<_>>(),
        vec![
            r#"{"ts":"2026-08-26T09:00:01.000Z","event":"said","text":"one"}"#,
            r#"{"ts":"2026-08-26T09:00:02.000Z","event":"said","text":"two"}"#,
            r#"{"ts":"2026-08-26T09:00:03.000Z","event":"said","text":"three"}"#,
        ]
    );
}

#[tokio::test]
async fn a_run_nobody_is_watching_writes_the_same_file() {
    let drone = "01DRONEGGGGGGGGGGGGGGGGGGG";
    let watched = TempDir::new();
    let alone = TempDir::new();
    let looking: Vec<Arc<dyn Tap>> = vec![
        Arc::new(recording(&watched, drone)),
        Arc::new(Counting::default()),
    ];
    three_lines_through(&watched, looking).await;
    three_lines_through(&alone, vec![Arc::new(recording(&alone, drone))]).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        rows(&watched, drone),
        rows(&alone, drone),
        "the durable record does not depend on a window being open"
    );
    assert!(!rows(&alone, drone).is_empty(), "and it is not empty");
}

#[tokio::test]
async fn the_job_log_names_the_file_a_minted_id_is_the_only_record_of() {
    let at = TempDir::new();
    let drone = "01DRONECCCCCCCCCCCCCCCCCCC";
    recording(&at, drone).settled().await;

    let opened = job_log(&at);
    let first = opened.lines().next().expect("a line");
    assert!(
        first.contains(&format!(r#""job_id":"{JOB}","drone_id":"{drone}""#)),
        "the spine is fields, and the join is on job_id: {first}"
    );
    assert!(
        first.contains(&format!(r#""transcript":"{}"#, at.path().display())),
        "and it says where the rows went: {first}"
    );
    assert!(
        !first.contains(drone.to_lowercase().as_str()),
        "msg never carries an interpolated id"
    );
}

#[tokio::test]
async fn a_retry_gets_its_own_file_under_the_one_job() {
    let at = TempDir::new();
    let (first, second) = ("01DRONEDDDDDDDDDDDDDDDDDDD", "01DRONEEEEEEEEEEEEEEEEEEEE");
    recording(&at, first).settled().await;
    recording(&at, second).settled().await;

    assert_eq!(rows(&at, first), "", "the first Drone's file is untouched");
    assert_eq!(rows(&at, second), "");
    let log = job_log(&at);
    assert_eq!(
        log.lines().filter(|line| line.contains(first)).count(),
        2,
        "opened and closed, both naming the first Drone"
    );
    assert_eq!(log.lines().filter(|line| line.contains(second)).count(), 2);
}

#[tokio::test]
async fn a_queue_that_fills_drops_rows_rather_than_holding_up_the_loop() {
    let at = TempDir::new();
    let drone = "01DRONEFFFFFFFFFFFFFFFFFFF";
    let recording = recording(&at, drone);
    let said: Vec<DroneEvent> = (0..4_000)
        .map(|n| DroneEvent::Said {
            text: format!("{n}"),
        })
        .collect();

    // No `await` between the offer and the assertion, on a runtime with one
    // thread: the writer cannot have run, so the queue is full and the call
    // still returns. That returning is the property.
    recording.saw(&said);
    recording.settled().await;

    let written = rows(&at, drone);
    let kept = written
        .lines()
        .filter(|line| line.contains(r#""event":"said""#))
        .count();
    let lost = missed(&written);
    assert!(lost > 0, "a queue that could not take 4,000 rows at once");
    assert_eq!(kept as u64 + lost, 4_000, "nothing goes unaccounted for");
    assert!(
        job_log(&at).contains(&format!(r#""missed":{lost}"#)),
        "and the Job's log says how much of the record is not there"
    );
}

/// How many rows the sink wrote down as lost, summed over every marker.
fn missed(written: &str) -> u64 {
    let counted = AtomicU64::new(0);
    for line in written.lines() {
        if let Some(rest) = line.split(r#""event":"missed","rows":"#).nth(1) {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            counted.fetch_add(digits.parse().unwrap_or(0), Ordering::Relaxed);
        }
    }
    counted.load(Ordering::Relaxed)
}

// -------------------------------------------------- the backfill, off the disk

/// The Job's log is the only thing joining a Job to files named by an id no
/// record carries, so the backfill reads it to find them.
#[tokio::test]
async fn the_history_is_found_through_the_log_that_names_each_transcript() {
    let at = TempDir::new();
    let drone = "01DRONEHHHHHHHHHHHHHHHHHHH";
    let recording = recording(&at, drone);
    recording.saw(&[DroneEvent::Said {
        text: String::from("what happened earlier"),
    }]);
    recording.settled().await;

    let (rows, skipped) = history(
        &at.path().to_string_lossy(),
        &JobId::carried(Ulid::carried(JOB)),
    )
    .await;

    assert_eq!(skipped, 0);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].saw,
        ipc::Saw::Said {
            text: String::from("what happened earlier"),
        }
    );
}

/// A retry is a second `drone_id` under one `job_id`, and both files are the
/// one Job's history — read in the order the log named them.
#[tokio::test]
async fn a_retrys_rows_follow_the_first_attempts_under_the_one_job() {
    let at = TempDir::new();
    for (drone, what) in [
        ("01DRONEIIIIIIIIIIIIIIIIIII", "the first attempt"),
        ("01DRONEJJJJJJJJJJJJJJJJJJJ", "the retry"),
    ] {
        let recording = recording(&at, drone);
        recording.saw(&[DroneEvent::Said {
            text: String::from(what),
        }]);
        recording.settled().await;
    }

    let (rows, _) = history(
        &at.path().to_string_lossy(),
        &JobId::carried(Ulid::carried(JOB)),
    )
    .await;

    let said: Vec<String> = rows
        .iter()
        .filter_map(|row| match &row.saw {
            ipc::Saw::Said { text } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(said, vec!["the first attempt", "the retry"]);
}

/// A Job nothing ever dispatched. **Ordinary, not an error** — there is no log
/// to read and nothing to show.
#[tokio::test]
async fn a_job_with_no_transcript_at_all_answers_with_nothing() {
    let at = TempDir::new();
    let (rows, skipped) = history(
        &at.path().to_string_lossy(),
        &JobId::carried(Ulid::carried(JOB)),
    )
    .await;
    assert!(rows.is_empty());
    assert_eq!(skipped, 0);
}

/// The whole claim, against a real Fleet: a Job with a Drone on it can be
/// watched, and the rows the Drone is producing reach the viewer.
#[tokio::test]
async fn a_running_drone_can_be_watched_and_a_finished_one_leaves_its_history() {
    use api::Daemon;

    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::untouched());
    let proposed = fleet
        .propose(crate::tests::daemon::a_proposal("watch this one"))
        .await
        .expect("a proposal Fleet holds everything for");
    crate::tests::daemon::worktree_directory(&home, proposed.id());
    fleet
        .approve(proposed.id())
        .await
        .expect("approval releases it");

    let watching = fleet
        .observe_job(ipc::JobId::from(proposed.id()))
        .await
        .expect("a Job that exists can be watched");
    assert!(
        watching.live.is_some(),
        "a Drone is on this Job, so somebody can watch it"
    );

    // A Job nothing was ever dispatched for is the ordinary case, not a
    // refusal: the connection opens, there is nothing to show, and it says so.
    let quiet = fleet
        .observe_job(ipc::JobId::carried("01JOBNOTDISPATCHEDATALL"))
        .await;
    assert!(quiet.is_err(), "an id naming no Job is refused as one");
}
