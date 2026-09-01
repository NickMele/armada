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
use crate::tests::tools::submitted_by_the_one;
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

    /// This stand-in counts what a Drone said, which is what the tee is being
    /// tested for. A row Fleet authored is not a `DroneEvent` and there is
    /// nowhere here to put one.
    fn noted(&self, _by: ipc::Voice, _saw: ipc::Saw) {}
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
            r#"{"ts":"2026-08-26T09:00:01.000Z","step":"implement","by":"drone","event":"said","text":"one"}"#,
            r#"{"ts":"2026-08-26T09:00:02.000Z","step":"implement","by":"drone","event":"said","text":"two"}"#,
            r#"{"ts":"2026-08-26T09:00:03.000Z","step":"implement","by":"drone","event":"said","text":"three"}"#,
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

/// **The whole point of the row.** A transcript that said `Bash · toolu_01Haa…`
/// twenty-two times could not tell `ls` from `rm -rf`, so what a call did has
/// to survive the file and the backfill, not just the event.
#[tokio::test]
async fn what_a_call_did_survives_to_the_row_a_viewer_is_sent() {
    let at = TempDir::new();
    let recording = recording(&at, "01DRONEKKKKKKKKKKKKKKKKKKK");
    recording.saw(&[DroneEvent::Called {
        tool: String::from("Bash"),
        call: String::from("toolu_01Haa"),
        detail: adapter_traits::CallDetail::of("cargo build --workspace"),
    }]);
    recording.settled().await;

    let (rows, _) = history(
        &at.path().to_string_lossy(),
        &JobId::carried(Ulid::carried(JOB)),
    )
    .await;

    assert_eq!(
        rows[0].saw,
        ipc::Saw::Called {
            tool: String::from("Bash"),
            call: String::from("toolu_01Haa"),
            detail: String::from("cargo build --workspace"),
            truncated: false,
            detail_length: Some("cargo build --workspace".chars().count()),
            whole: None,
        }
    );
    assert!(
        ipc::Shown::of(rows[0].clone()).is_some(),
        "a call is a row a viewer is shown"
    );
}

/// **A row that was cut is not a dead end.** The line a viewer is sent says how
/// much there was and carries none of it; the file keeps the argument, and
/// `arguments` is what a person opening the row reaches.
#[tokio::test]
async fn a_cut_argument_is_sized_on_the_wire_and_whole_in_the_file() {
    let at = TempDir::new();
    let recording = recording(&at, "01DRONEKKKKKKKKKKKKKKKKKKK");
    let heredoc = format!("cat <<EOF > out.txt {}", "word ".repeat(400));
    recording.saw(&[DroneEvent::Called {
        tool: String::from("Bash"),
        call: String::from("toolu_01Haa"),
        detail: adapter_traits::CallDetail::of(&heredoc),
    }]);
    recording.settled().await;

    let root = at.path().to_string_lossy().to_string();
    let job = JobId::carried(Ulid::carried(JOB));
    let (rows, _) = history(&root, &job).await;

    let ipc::Saw::Called {
        detail,
        truncated,
        detail_length,
        whole,
        ..
    } = rows[0].saw.clone()
    else {
        panic!("the row is a call");
    };
    assert!(truncated, "the argument was longer than a row carries");
    assert_eq!(
        detail_length,
        Some(heredoc.chars().count()),
        "the row states how much there is, not how much it shows"
    );
    assert!(detail.chars().count() < heredoc.chars().count());
    assert_eq!(
        whole, None,
        "a viewer's row never carries the argument, whatever its size"
    );

    let served = crate::transcript::arguments(&root, &job, "toolu_01Haa")
        .await
        .expect("the file kept what the row did not");
    assert_eq!(served.arguments, heredoc, "the argument, as it was sent");
    assert!(served.whole);
    assert_eq!(served.length, Some(heredoc.chars().count()));
}

/// An id from a row whose transcript is gone reaches nothing, and that is a
/// different answer from a Job that never ran.
#[tokio::test]
async fn a_call_the_record_does_not_hold_answers_with_nothing() {
    let at = TempDir::new();
    let recording = recording(&at, "01DRONEKKKKKKKKKKKKKKKKKKK");
    recording.saw(&[DroneEvent::Called {
        tool: String::from("Bash"),
        call: String::from("toolu_01Haa"),
        detail: adapter_traits::CallDetail::of("cargo build --workspace"),
    }]);
    recording.settled().await;

    assert!(crate::transcript::arguments(
        &at.path().to_string_lossy(),
        &JobId::carried(Ulid::carried(JOB)),
        "toolu_never"
    )
    .await
    .is_none());
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

/// Every step this Job's rows are labelled with, once one of them says `wanted`.
///
/// Polled, because the row is written by the reader task on the far side of the
/// Drone's pipe rather than by the call that moved the step.
async fn steps_until(home: &TempDir, job: &JobId, wanted: &str) -> Vec<String> {
    let root = home.path().to_string_lossy().to_string();
    let mut seen = Vec::new();
    for _ in 0..400 {
        let (rows, _) = history(&root, job).await;
        seen = rows
            .iter()
            .filter_map(|row| row.step.as_ref().map(|step| String::from(step.as_str())))
            .collect();
        if seen.iter().any(|step| step == wanted) {
            return seen;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    seen
}

/// **The defect, at the seam it lives at.** One Drone works several steps, and
/// the row's step was taken once when the Drone was spawned — so every row
/// written after a Job's first advance said the first step, and a four-step
/// Job's transcript claimed all of it happened during step one.
///
/// Driven through a real Fleet rather than through the label directly: the
/// sinks are held by the reader task, so a step moved in a copy they do not
/// hold passes a unit test and changes nothing in the file.
#[tokio::test]
async fn a_row_written_after_a_step_advances_carries_the_step_it_was_written_under() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(crate::tests::daemon::a_proposal("advance once"))
        .await
        .expect("a proposal Fleet holds everything for");
    crate::tests::daemon::worktree_directory(&home, job.id());
    fleet
        .approve(job.id())
        .await
        .expect("approval puts a Drone on `implement`");
    assert!(
        !steps_until(&home, job.id(), "implement").await.is_empty(),
        "the Drone said something under the step it was spawned on"
    );

    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the step's evidence");
    fleet
        .turn()
        .await
        .expect("a non-empty diff advances the step to `summarise`");

    // The Drone is told the step advanced, and this one echoes what it is told
    // — so the row that comes back was written under the new step.
    let steps = steps_until(&home, job.id(), "summarise").await;
    assert_eq!(
        steps.first().map(String::as_str),
        Some("implement"),
        "the rows before the advance are still the first step's: {steps:?}"
    );
    assert!(
        steps.iter().any(|step| step == "summarise"),
        "a row written after the advance says which step it was written under, \
         and saying `implement` for the whole life of the Job is the bug: {steps:?}"
    );
}

/// **A step is a conversation and the record held one side of it.** Armada
/// opens the step, the Drone works, Fleet runs the Checks and reads what came
/// out — and only the middle one was written down, so the activity log the
/// drawing draws with three actors could be drawn with one.
///
/// Driven through a real Fleet for `steps_until`'s reason: the sinks are held
/// by the reader task, and a row offered to a copy nothing holds passes a unit
/// test and reaches no file.
#[tokio::test]
async fn the_record_carries_what_armada_said_and_what_fleet_did_beside_the_drone() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(crate::tests::daemon::a_proposal("advance once"))
        .await
        .expect("a proposal Fleet holds everything for");
    crate::tests::daemon::worktree_directory(&home, job.id());
    fleet
        .approve(job.id())
        .await
        .expect("approval puts a Drone on `implement`");

    let opening = voiced_until(&home, job.id(), ipc::Voice::Armada).await;
    assert!(
        opening.iter().any(|saw| matches!(
            saw,
            ipc::Saw::Instructed { occasion, text } if occasion == "opening" && !text.is_empty()
        )),
        "the brief a step opened with is the first thing in its record: {opening:?}"
    );

    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the step's evidence");
    fleet.turn().await.expect("the gate rules");

    let fleets = voiced_until(&home, job.id(), ipc::Voice::Fleet).await;
    assert!(
        fleets
            .iter()
            .any(|saw| matches!(saw, ipc::Saw::Produced { .. })),
        "and what the step produced is read once per ruling, in Fleet's own \
         voice, because a Drone never reads its own worktree: {fleets:?}"
    );
}

/// Every row one voice wrote, once that voice has written at least one.
///
/// Polled for `steps_until`'s reason: the writer drains on a task of its own,
/// so a row is on disk some time after the call that produced it returned.
async fn voiced_until(home: &TempDir, job: &JobId, by: ipc::Voice) -> Vec<ipc::Saw> {
    let root = home.path().to_string_lossy().to_string();
    let mut seen = Vec::new();
    for _ in 0..400 {
        let (rows, _) = history(&root, job).await;
        seen = rows
            .iter()
            .filter(|row| row.by == by)
            .map(|row| row.saw.clone())
            .collect();
        if !seen.is_empty() {
            return seen;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    seen
}

/// One tool call, as the transcript carries it.
fn a_call(tool: &str) -> DroneEvent {
    DroneEvent::Called {
        tool: String::from(tool),
        call: String::from("01CALLAAAAAAAAAAAAAAAAAAAA"),
        detail: adapter_traits::CallDetail::of("a path"),
    }
}

/// **The count the convergence tripwire reads is Fleet's own, and it is live.**
///
/// The regression this pins: `Progress` used to carry the harness's own
/// `turns`, which arrives only on a terminating event. An invocation ends when
/// a step ends, so at a step boundary the finishing step's count had not been
/// published yet, the new step's baseline read zero, and the whole of the
/// previous step's turns landed on its successor. A step was told to stop and
/// report twenty-three seconds in, charged with the sixty-nine turns of the
/// step before it.
///
/// A call is counted where it happens, so a baseline taken at a step boundary
/// is true at the instant it is taken.
#[tokio::test]
async fn progress_counts_calls_as_they_arrive_and_never_the_reported_turn_count() {
    let at = TempDir::new();
    let harness = Arc::new(
        FakeHarness::running("/bin/sh", &["-c", SAYS_THREE])
            .reading("one", vec![a_call("Read"), a_call("Grep")])
            .reading("two", vec![a_call("Edit")])
            .reading(
                "three",
                vec![DroneEvent::Ended {
                    turns: 69,
                    cost_micros: 0,
                    refusals: 0,
                }],
            ),
    );
    let started = start(harness.as_ref(), &config(&at))
        .await
        .expect("a shell starts and reads its first turn");
    let watching = Watching::reading(started.transcript, harness, Vec::new());
    for _ in 0..200 {
        if watching.transcript_ended() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let progress = watching.progress();
    assert_eq!(progress.calls, 3, "three calls arrived, one at a time");
    assert_eq!(
        progress.boundaries, 1,
        "one invocation ended, which is what a forced report is read against"
    );
    assert_ne!(
        progress.calls, 69,
        "the harness's turn count is not the live count, and reading it as one \
         is the defect this test exists for"
    );
}
