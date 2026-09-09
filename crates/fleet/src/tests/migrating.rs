//! Moving what an older Fleet wrote under a ULID.
//!
//! **Nothing here fakes a filesystem.** The whole subject is which name a file
//! is under, so every case writes real bytes and asserts on real paths.
//!
//! The claims worth breaking are the three the pass could get wrong: a Job's
//! log and its Drones' transcripts end up under the handle, a second boot moves
//! nothing, and a destination that is already occupied is left alone and said
//! rather than written over.

use core_model::{
    DroneId, Facts, Job, JobId, JobNumber, ManifestId, ModelName, NewJob, StepId, StepSeed,
    Timestamp, Title, TopLevelOrigin, Ulid, Urgency,
};

use crate::tests::tmp::TempDir;
use crate::transcript::{log_of, rekeyed, transcript_of, transcripts_dir};

const JOB: &str = "01M22TYSAE0023MADDP5ZQEYGW";
const DRONE: &str = "01M22V3F4T0032THVMEGXY08E4";

/// The Job the owner pasted the handle of. Its title is the one that produced
/// `1-board-s-clear-button-should-reclaim-worktr`.
fn job() -> Job {
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried(JOB)),
            title: Title::new("Board's clear button should reclaim worktrees, not delete records")
                .expect("a title"),
            workflow: crate::tests::gate::workflow("/usr/bin/true")
                .frozen()
                .clone(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01J0000000000000000000MAN0")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("the-configured-model").expect("a model name"),
            acceptance_criteria: Vec::new(),
            steps: vec![StepSeed {
                step_id: StepId::new("implement"),
                ordinal: 0,
            }],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            proposal_id: None,
            number: JobNumber::carried(1),
            facts: Facts::empty(),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        Timestamp::from_rfc3339("2026-09-09T10:23:28.349Z"),
    )
}

/// A log and a transcript written where a Fleet before `#573` wrote them.
fn as_an_older_fleet_wrote_it(at: &TempDir) {
    let root = at.path().join(".armada");
    std::fs::create_dir_all(root.join("logs")).expect("the logs directory");
    std::fs::create_dir_all(root.join("transcripts")).expect("the transcripts directory");
    // The line the rename reads: it is the only thing that says which Drone
    // belongs to which Job, and it names the path the file was opened at.
    std::fs::write(
        root.join("logs").join(format!("{JOB}.jsonl")),
        format!(
            "{{\"ts\":\"2026-09-09T10:23:33.000Z\",\"level\":\"info\",\"component\":\"fleet\",\
             \"run_id\":\"01RUN\",\"msg\":\"drone transcript opened\",\"job_id\":\"{JOB}\",\
             \"fields\":{{\"transcript\":\"{}/.armada/transcripts/{DRONE}.jsonl\"}}}}\n",
            at.path().display()
        ),
    )
    .expect("the log an older Fleet wrote");
    std::fs::write(
        root.join("transcripts").join(format!("{DRONE}.jsonl")),
        "{\"ts\":\"2026-09-09T10:23:34.000Z\",\"by\":\"drone\",\"saw\":{}}\n",
    )
    .expect("the transcript an older Fleet wrote");
}

// **The whole of what the rename is for.** Two of the six directories under
// `.armada/` disagreed with the other four, and neither could be reached from
// the one id a person can read.
#[tokio::test]
async fn a_jobs_log_and_transcripts_move_under_the_name_it_is_called_by() {
    let at = TempDir::new();
    as_an_older_fleet_wrote_it(&at);
    let job = job();
    let root = at.path().to_string_lossy().to_string();

    let moved = rekeyed(&root, std::slice::from_ref(&job)).await;

    assert_eq!(moved.logs, 1);
    assert_eq!(moved.transcripts, 1);
    assert!(moved.refused.is_empty(), "{:?}", moved.refused);
    assert_eq!(job.handle(), "1-board-s-clear-button-should-reclaim-worktr");
    assert!(log_of(&root, &job.handle()).is_file());
    assert!(transcript_of(
        &root,
        &job.handle(),
        &DroneId::carried(Ulid::carried(DRONE))
    )
    .is_file());
    assert!(
        !at.path()
            .join(".armada/transcripts")
            .join(format!("{DRONE}.jsonl"))
            .exists(),
        "moved, not copied — a long transcript must not cost twice the disk"
    );
}

// It runs at every boot and there is no marker saying it has run. What makes
// the second boot free is that the work *is* the files still under the old
// name, and there are none.
#[tokio::test]
async fn a_second_boot_moves_nothing() {
    let at = TempDir::new();
    as_an_older_fleet_wrote_it(&at);
    let job = job();
    let root = at.path().to_string_lossy().to_string();

    rekeyed(&root, std::slice::from_ref(&job)).await;
    let again = rekeyed(&root, std::slice::from_ref(&job)).await;

    assert!(again.moved_nothing(), "{again:?}");
    assert!(log_of(&root, &job.handle()).is_file());
}

// A destination that is already there is the one state a rename may not
// resolve: merging two records is not something a boot decides.
#[tokio::test]
async fn a_name_already_taken_is_left_alone_and_said() {
    let at = TempDir::new();
    as_an_older_fleet_wrote_it(&at);
    let job = job();
    let root = at.path().to_string_lossy().to_string();
    std::fs::create_dir_all(transcripts_dir(&root, &job.handle())).expect("the directory");
    std::fs::write(log_of(&root, &job.handle()), "somebody else's lines\n").expect("a log");

    let moved = rekeyed(&root, std::slice::from_ref(&job)).await;

    assert_eq!(moved.logs, 0);
    assert_eq!(moved.refused.len(), 1, "{:?}", moved.refused);
    assert_eq!(
        std::fs::read_to_string(log_of(&root, &job.handle())).expect("still there"),
        "somebody else's lines\n"
    );
    assert!(
        at.path()
            .join(".armada/logs")
            .join(format!("{JOB}.jsonl"))
            .is_file(),
        "and what would have been written over is where it was"
    );
}

// A Job with nothing on disk is the ordinary case after the first boot, and the
// commonest one on a fresh store.
#[tokio::test]
async fn a_job_with_nothing_on_disk_is_not_a_refusal() {
    let at = TempDir::new();
    let root = at.path().to_string_lossy().to_string();

    let moved = rekeyed(&root, &[job()]).await;

    assert!(moved.moved_nothing(), "{moved:?}");
}

/// The id is carried so a reader can join a moved log back to its row.
#[test]
fn the_job_the_handle_names_is_the_one_the_ulid_does() {
    assert_eq!(job().id(), &JobId::carried(Ulid::carried(JOB)));
}
