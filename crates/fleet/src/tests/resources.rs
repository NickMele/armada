//! What a Job holds on this machine, and what came of asking whether it works.
//!
//! **The parsers are proved against real tool output**, `crate::tests::headroom`'s
//! method: the strings below are `ps` and `du` as they actually print on this
//! machine, so a change to how a column is read fails here rather than on
//! somebody's screen.
//!
//! **The Fleet cases are all on a Job with no Drone**, which is deliberate. The
//! span this was built for is the one before any agent starts — a Job hung
//! there on 4 Sep 2026 and the four `spend` figures all read zero — and a
//! fixture that spawned `/bin/cat` would prove the easy half and not that one.

use core_model::JobStatus;
use ipc::{Asked, Finding, Held};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::examining::folded;
use crate::resources::{descended, measured};
use crate::tests::daemon::{a_proposal, fittings, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir) -> Fixture {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&["src/log.rs"])))
}

/// `ps -A -o pid=,ppid=,pcpu=,rss=,etime=,comm=` as it prints. The tree is
/// Fleet, a Drone under it, a build under that, and one process belonging to
/// nobody in this story.
const TABLE: &str = "\
  501     1   0.0   8192     01:00:11 /sbin/launchd
 4001   501   1.2  40960     10:22 armada
 4100  4001  12.4 393216     06:12 node
 4210  4100  98.7 819200     00:41 cargo
 4211  4210  12.0  16384     00:41 rustc
 9000     1   0.1   2048     04:00 Some Helper App
";

/// The claim: what counts as this Job's is the recorded process and everything
/// under it, and nothing else on the machine.
#[test]
fn the_recorded_process_and_its_descendants_are_what_the_job_holds() {
    let held = descended(TABLE, 4100);

    let pids: Vec<u32> = held.iter().map(|one| one.pid).collect();
    assert_eq!(pids.len(), 3, "the Drone, cargo, rustc and nothing else");
    assert_eq!(pids[0], 4100, "the recorded process leads");
    assert!(pids.contains(&4210) && pids.contains(&4211));
    // Fleet is the parent, not a child, and the helper belongs to nobody.
    assert!(!pids.contains(&4001) && !pids.contains(&9000));
}

/// Exactly one row says it is the one Fleet wrote down, and the figures come
/// off the columns rather than out of the order they appear in.
#[test]
fn the_recorded_row_is_marked_and_the_columns_are_read_by_position() {
    let held = descended(TABLE, 4100);

    assert_eq!(held.iter().filter(|one| one.recorded).count(), 1);
    let drone = &held[0];
    assert!(drone.recorded);
    assert_eq!(drone.command, "node");
    assert_eq!(drone.cpu_percent, 12.4);
    // `ps` answers kibibytes and the wire carries bytes, so nothing on the far
    // side has to know which unit the reading was taken in.
    assert_eq!(drone.memory_bytes, 393_216 * 1024);
    assert_eq!(drone.running_for, "06:12");
}

/// A command with a space in it survives, because `comm` is read last and takes
/// what is left of the line.
#[test]
fn a_command_containing_a_space_is_not_cut_at_it() {
    let held = descended(TABLE, 9000);

    assert_eq!(held.len(), 1);
    assert_eq!(held[0].command, "Some Helper App");
}

/// A table read mid-fork can name a parent that is its own child. The walk ends
/// rather than running forever.
#[test]
fn a_process_that_is_its_own_parent_does_not_walk_forever() {
    let held = descended("  7   7  0.0  1024  00:01 loop\n", 7);

    assert_eq!(held.len(), 1);
    assert_eq!(held[0].pid, 7);
}

/// A row too short to read is a process that exited during the walk, and it
/// costs the reading nothing.
#[test]
fn a_partial_row_is_skipped_rather_than_read_wrong() {
    let table = " 4100  4001  12.4 393216     06:12 node\n 4210  4100\n";

    let held = descended(table, 4100);

    assert_eq!(held.len(), 1, "the whole row is kept and the half is not");
}

/// `du -s -k` prints kibibytes and a path. Bytes are what crosses.
#[test]
fn the_worktree_reading_is_taken_off_du_in_kibibytes() {
    let said = "1048576\t/repo/.armada/worktrees/01JOB\n";

    assert_eq!(measured(said), Some(1024 * 1024 * 1024));
}

/// `du` prints a warning line to stderr and its total to stdout, and a
/// permission it could not descend into leaves the total on the last line.
#[test]
fn the_last_line_is_the_total_however_many_lines_du_printed() {
    let said = "512\t/repo/a\n2048\t/repo\n";

    assert_eq!(measured(said), Some(2048 * 1024));
}

#[test]
fn a_du_that_printed_nothing_measures_nothing_rather_than_zero() {
    assert_eq!(measured(""), None);
}

fn look(asked: Asked, found: Finding) -> ipc::Look {
    ipc::Look {
        asked,
        found,
        said: String::new(),
        fields: Vec::new(),
    }
}

/// **The rule the whole act turns on.** One look that could not tell keeps the
/// examination off `working` — "everything looks fine" on a plainly hung Job
/// spends a person's suspicion and returns nothing.
#[test]
fn one_look_that_cannot_tell_keeps_the_answer_off_working() {
    let looks = [
        look(Asked::Process, Finding::Working),
        look(Asked::Span, Finding::Working),
        look(Asked::Writing, Finding::CannotTell),
    ];

    assert_eq!(folded(&looks), Finding::CannotTell);
}

#[test]
fn one_look_that_found_a_fault_carries_the_whole_answer() {
    let looks = [
        look(Asked::Process, Finding::NotWorking),
        look(Asked::Span, Finding::Working),
        look(Asked::Writing, Finding::CannotTell),
    ];

    assert_eq!(folded(&looks), Finding::NotWorking);
}

#[test]
fn every_look_agreeing_is_the_only_road_to_working() {
    let looks = [
        look(Asked::Process, Finding::Working),
        look(Asked::Span, Finding::Working),
    ];

    assert_eq!(folded(&looks), Finding::Working);
}

/// The claim: a Job at its approval gate holds nothing, and that is stated
/// rather than left as an empty list.
#[tokio::test]
async fn a_job_at_the_gate_holds_no_process_and_says_so() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("nothing has started"))
        .await
        .expect("a Job at the gate");

    let held = fleet.job_resources(&job).await.expect("a reading");

    assert_eq!(held.held, Held::None);
    assert!(held.processes.is_empty());
    // No worktree has been cut, which is absent rather than a checkout that
    // measures nothing.
    assert!(held.worktree.is_none());
    assert!(
        !held.read_at.as_str().is_empty(),
        "every figure is as of this"
    );
}

/// The claim: a worktree on disk is measured, and the path and branch come back
/// with it.
#[tokio::test]
async fn a_worktree_on_disk_is_measured_and_named() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("a checkout exists"))
        .await
        .expect("a Job at the gate");
    worktree_directory(&home, job.id());
    std::fs::write(
        home.path()
            .join(".armada/worktrees")
            .join(job.id().as_str())
            .join("work.txt"),
        "what the drone wrote\n",
    )
    .expect("something to measure");

    let held = fleet.job_resources(&job).await.expect("a reading");

    let worktree = held.worktree.expect("a checkout was found");
    assert!(worktree.path.ends_with(job.id().as_str()));
    assert_eq!(worktree.branch, format!("armada/{}", job.id().as_str()));
    assert!(
        worktree.bytes.is_some(),
        "the walk finished inside its bound"
    );
}

/// The claim: no process on a Job that is not running is `working`, because
/// `working` means *as it should be* rather than *a processor is busy*. Under
/// the other reading every Job at its gate examines as broken.
#[tokio::test]
async fn a_job_holding_nothing_and_expected_to_is_not_a_fault() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("waiting for approval"))
        .await
        .expect("a Job at the gate");

    let examined = fleet.examined(&job).await.expect("an examination");

    assert_eq!(job.status(), JobStatus::AwaitingApproval);
    let process = examined
        .looks
        .iter()
        .find(|look| look.asked == Asked::Process)
        .expect("the process look");
    assert_eq!(process.found, Finding::Working);
    assert_ne!(
        examined.found,
        Finding::NotWorking,
        "a Job at its gate is not broken: {:?}",
        examined.looks
    );
}

/// The claim: the answer lands on the Job's own record, so a second person
/// reading the Job later finds that somebody looked and what they were told.
#[tokio::test]
async fn what_the_examination_found_is_written_into_the_jobs_own_log() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("somebody is worried"))
        .await
        .expect("a Job at the gate");

    let examined = fleet.examined(&job).await.expect("an examination");

    let written = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        job.id(),
    ))
    .expect("the Job's log");
    assert!(written.contains("a person asked whether this Job is working"));
    // Every look is on the line, named, so the record says what could not be
    // told as well as what could.
    for look in &examined.looks {
        assert!(
            written.contains(&look.said),
            "the log carries {:?}: {written}",
            look.asked
        );
    }
}

/// The claim: five looks, every time, whatever the Job is doing. A look that
/// went missing on some statuses would be a check nobody could tell from one
/// that passed.
#[tokio::test]
async fn every_examination_asks_all_five() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = fleet
        .propose(a_proposal("ask everything"))
        .await
        .expect("a Job at the gate");

    let examined = fleet.examined(&job).await.expect("an examination");

    let asked: Vec<Asked> = examined.looks.iter().map(|look| look.asked).collect();
    for one in [
        Asked::Process,
        Asked::Worktree,
        Asked::Writing,
        Asked::Span,
        Asked::Silence,
    ] {
        assert!(asked.contains(&one), "{one:?} was not asked: {asked:?}");
    }
}
