//! That a Job which was stopped by a policy says what the policy stopped.
//!
//! # The case these are built on is a real one
//!
//! Job `01M20E8JS40027M421MXTPZBP0` escalated as `blocked_by_policy`. Its log
//! line said `found: blocked_by_policy` and named no tool and no command; the
//! `ipc::Stuck` on its detail carried the trigger, the step and the acts, and
//! nothing about what had been refused. The two facts were in the transcript
//! and were on different rows:
//!
//! ```text
//! {"event":"called","tool":"Bash","call":"toolu_01B13LL","detail":"cargo nextest run --package ipc 2>&1 | tail -80"}
//! {"event":"refused","tool":"Bash","call":"toolu_01B13LL","because":""}
//! ```
//!
//! **`because` is empty and that is the ordinary case**, so a surface built on
//! it alone would be as blank as the one this replaced. The command is on the
//! other row and the call id is the only thing joining them.
//!
//! The answer is read from the two places it lives and both are here:
//! `transcript::refusals` off a stopped Job's files, and
//! `transcript::refused_in` off the events Fleet holds while it classifies.

use adapter_traits::{CallDetail, DroneEvent};
use core_model::{DroneId, JobId, StepId, Ulid};

use crate::tests::tmp::TempDir;
use crate::transcript::{refusals, refused_in, Recording, Spine, Tap, REFUSALS};

const JOB: &str = "01JOBAAAAAAAAAAAAAAAAAAAAA";
const RUN: &str = "01RUNAAAAAAAAAAAAAAAAAAAAA";
const DRONE: &str = "01DRONEKKKKKKKKKKKKKKKKKKK";

/// What the Job is called, which is what its log and its transcript directory
/// are named by.
const HANDLE: &str = "7-a-job-that-was-refused";
/// A Job nothing ever ran, for the empty answer.
const NEVER_RAN: &str = "8-a-job-that-never-ran";

/// The command the real Job was refused, as its transcript recorded it.
const REFUSED: &str = "cargo nextest run --package ipc 2>&1 | tail -80";

fn recording(at: &TempDir, drone: &str) -> Recording {
    Recording::of(
        &at.path().to_string_lossy(),
        Spine {
            job: JobId::carried(Ulid::carried(JOB)),
            handle: String::from(HANDLE),
            drone: DroneId::carried(Ulid::carried(drone)),
            step: StepId::new("implement"),
            run: Ulid::carried(RUN),
        },
        std::sync::Arc::new(crate::tests::daemon::Ticking::from_nine()),
    )
    .expect("a transcript opens under a directory that exists")
}

fn called(call: &str, command: &str) -> DroneEvent {
    DroneEvent::Called {
        tool: String::from("Bash"),
        call: String::from(call),
        detail: CallDetail::of(command),
    }
}

/// A refusal as the harness sends it: a tool, a call id, and **no reason**.
fn refused(call: &str) -> DroneEvent {
    DroneEvent::Refused {
        tool: String::from("Bash"),
        call: String::from(call),
        because: String::new(),
    }
}

/// **The defect, stated as the assertion.** A person was told a policy stopped
/// the Drone and was never told what it stopped, because the tool and the
/// command are on two rows and nothing joined them.
#[tokio::test]
async fn a_refusal_names_the_command_it_was_refused_over() {
    let at = TempDir::new();
    let recording = recording(&at, DRONE);
    recording.saw(&[called("toolu_01B13LL", REFUSED), refused("toolu_01B13LL")]);
    recording.settled().await;

    let read = refusals(&at.path().to_string_lossy(), HANDLE).await;

    assert_eq!(read.in_all(), 1);
    let [one] = read.kept() else {
        panic!("one refusal, carried: {:?}", read.kept());
    };
    assert_eq!(one.tool, "Bash");
    assert_eq!(
        one.detail, REFUSED,
        "the command, off the `called` row that shares the call id"
    );
    assert_eq!(
        one.because, "",
        "the harness gave no reason and none is invented"
    );
    assert_eq!(
        one.call, "toolu_01B13LL",
        "so the whole argument is openable"
    );
}

/// **A cut command is read back as cut.** The transcript keeps the whole and
/// the classification carries a line, so a row that did not say it was short
/// would have a person paste a truncated command into an allowlist — the
/// failure this whole reading exists to prevent, one step further on.
#[tokio::test]
async fn a_refused_heredoc_says_it_was_cut_and_how_long_it_was() {
    let at = TempDir::new();
    let recording = recording(&at, DRONE);
    let heredoc = format!("cat <<EOF > out.txt {}", "word ".repeat(400));
    recording.saw(&[called("toolu_01Haa", &heredoc), refused("toolu_01Haa")]);
    recording.settled().await;

    let read = refusals(&at.path().to_string_lossy(), HANDLE).await;

    let [one] = read.kept() else {
        panic!("one refusal, carried: {:?}", read.kept());
    };
    assert!(one.truncated, "the argument was longer than a row carries");
    assert_eq!(
        one.length,
        Some(heredoc.chars().count()),
        "how much there was, not how much is shown"
    );
    assert!(one.detail.chars().count() < heredoc.chars().count());
    assert!(
        !one.detail.is_empty(),
        "and the front of it, which is where the reason a call was worth \
         seeing usually is"
    );
}

/// A Drone that was refused nothing carries nothing, which is most of them —
/// and it is the same empty answer a Job whose transcripts are gone gives.
#[tokio::test]
async fn a_run_that_was_refused_nothing_carries_nothing() {
    let at = TempDir::new();
    let recording = recording(&at, DRONE);
    recording.saw(&[called("toolu_01Haa", "cargo build --workspace")]);
    recording.settled().await;

    let root = at.path().to_string_lossy().to_string();
    let read = refusals(&root, HANDLE).await;
    assert!(read.kept().is_empty());
    assert_eq!(read.in_all(), 0);

    let never_ran = refusals(&root, NEVER_RAN).await;
    assert!(
        never_ran.kept().is_empty() && never_ran.in_all() == 0,
        "a Job with no log is missing, not an error"
    );
}

/// **A truncated list says it was truncated.** The count is beside the list
/// because a list capped at fifty and reported as fifty reads as the whole one.
#[tokio::test]
async fn a_capped_list_states_how_many_there_were() {
    let at = TempDir::new();
    let recording = recording(&at, DRONE);
    let over = REFUSALS + 7;
    for nth in 0..over {
        let call = format!("toolu_{nth:04}");
        recording.saw(&[
            called(&call, &format!("rm -rf target/{nth}")),
            refused(&call),
        ]);
    }
    recording.settled().await;

    let read = refusals(&at.path().to_string_lossy(), HANDLE).await;

    assert_eq!(read.kept().len(), REFUSALS);
    assert_eq!(read.in_all(), over as u64, "and the rest are counted");
    assert_eq!(
        read.kept()[0].detail,
        "rm -rf target/0",
        "the earliest refusals, not the last: what stopped a Drone is what it \
         reached for before it began working around being stopped"
    );
}

/// A refusal whose `called` row is not in the file — an adopted Drone whose
/// earlier turns went into a pipe with no reader — is named by its tool and
/// nothing is invented against it.
#[tokio::test]
async fn a_refusal_with_no_call_row_names_the_tool_and_guesses_nothing() {
    let at = TempDir::new();
    let recording = recording(&at, DRONE);
    recording.saw(&[refused("toolu_01Gone")]);
    recording.settled().await;

    let read = refusals(&at.path().to_string_lossy(), HANDLE).await;

    let [one] = read.kept() else {
        panic!("the refusal is carried even with nothing to join it to");
    };
    assert_eq!(one.tool, "Bash");
    assert_eq!(
        one.detail, "",
        "empty, and never a reason built from the trigger"
    );
    assert!(
        !one.truncated && one.length.is_none(),
        "and a row that is not in the file is not a row that was cut"
    );
}

/// A retry is a second `drone_id` under one `job_id`, and a refusal in either
/// is the Job's. The join reaches across the files the log names.
#[tokio::test]
async fn both_attempts_refusals_are_the_one_jobs() {
    let at = TempDir::new();
    for (drone, command) in [
        ("01DRONEIIIIIIIIIIIIIIIIIII", "git push --force"),
        ("01DRONEJJJJJJJJJJJJJJJJJJJ", "curl https://example.invalid"),
    ] {
        let recording = recording(&at, drone);
        recording.saw(&[called(command, command), refused(command)]);
        recording.settled().await;
    }

    let read = refusals(&at.path().to_string_lossy(), HANDLE).await;

    assert_eq!(read.in_all(), 2);
    assert_eq!(
        read.kept()
            .iter()
            .map(|refusal| refusal.detail.as_str())
            .collect::<Vec<_>>(),
        ["git push --force", "curl https://example.invalid"],
        "in the order the Job's log named the transcripts"
    );
}

/// The same join, off the events Fleet is holding rather than off the file —
/// **and it is what the Job log line is built from**, at the moment the Drone's
/// process is still there and nothing has been written back yet.
#[test]
fn the_live_fold_joins_the_call_to_its_refusal_and_counts_the_rest() {
    let mut heard = vec![called("toolu_01B13LL", REFUSED), refused("toolu_01B13LL")];
    let read = refused_in(&heard);
    let [one] = read.kept() else {
        panic!("one refusal, carried: {:?}", read.kept());
    };
    assert_eq!(one.tool, "Bash");
    assert_eq!(one.detail, REFUSED);
    assert!(!one.truncated, "the command fits, and the row says so");
    assert_eq!(one.length, Some(REFUSED.chars().count()));
    assert_eq!(read.in_all(), 1);

    // Far more than the line names, so what is asserted is that the count does
    // not stop where the list does.
    for nth in 0..20 {
        heard.push(refused(&format!("toolu_never_{nth}")));
    }
    let many = refused_in(&heard);
    assert_eq!(many.in_all(), 21, "every refusal is counted");
    assert!(
        many.kept().len() < many.in_all() as usize,
        "and the line names a handful of them: {:?}",
        many.kept()
    );
    assert!(
        many.kept()[1..]
            .iter()
            .all(|refusal| refusal.detail.is_empty()),
        "a refusal with no call beside it names its tool and guesses nothing"
    );
}
