//! That the Drone which finished its step is **gone**, that what it said on the
//! way out survived it, and that reading it out waits on nothing — `#211`.
//!
//! # These start a real child, and it is the only way to ask
//!
//! Every other test of a step boundary drives it by calling what
//! `crate::dispatch` calls, and every one would pass over a Drone still
//! running: the slot was replaced, the record says the exit landed, a new
//! process arrived. **That is bookkeeping.** A Drone is `setsid`-detached, so
//! nothing about a dropped slot reaches the process — and the only thing that
//! can say otherwise is the operating system, asked here through [`holder_of`].
//!
//! The pair is `crate::tests::detach`'s shape: one case asserts the property
//! and the one beside it asserts the property is not free.
//!
//! # The drain is asserted on `Watching` and not through the slot
//!
//! [`Watching::drained`] exists for a reader behind a pipe that has already
//! closed, which a test has to construct: `#[tokio::test]` is a current-thread
//! runtime, so a *blocking* sleep is a window in which the child writes
//! everything it will ever write and nothing reads any of it.
//!
//! Asked of the slot instead, the same case proves less than it looks like.
//! Ending a Drone waits on the child, and a fast fake catches up inside that
//! wait whether or not anything asked — so it passes over a missing drain.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent, Worktree};
use core_model::{DroneId, JobId, StepId, Timestamp, Ulid};
use testkit::FakeHarness;

use crate::drone::{start, Ending};
use crate::process::{holder_of, Holder};
use crate::tests::daemon::Ticking;
use crate::tests::drone::config;
use crate::tests::tmp::TempDir;
use crate::transcript::{Spine, Taps};
use crate::watch::{Drained, Watching};
use crate::working::Working;

const JOB: &str = "01JOBAAAAAAAAAAAAAAAAAAAAA";
const DRONE: &str = "01DRONEAAAAAAAAAAAAAAAAAAA";
const RUN: &str = "01RUNAAAAAAAAAAAAAAAAAAAAA";

/// A Drone that reads its first turn, says four things and exits.
///
/// **It reads before it says anything**, so nothing is written until Fleet has
/// finished starting it — which is what lets the test say exactly when the
/// output appears rather than racing the spawn.
const SAYS_FOUR_THINGS_AND_GOES: &str =
    "IFS= read -r _; printf '%s\\n' reached-for-something said-something \
     said-something-else finished";

/// A Drone that reads nothing and sits there. **It must not read its stdin**:
/// `/bin/cat` would exit the moment the slot is dropped, because dropping the
/// slot closes the pipe — which would make the control below pass for a reason
/// that has nothing to do with a detached child surviving.
const SITS_THERE: &str = "exec sleep 30";

/// A Drone that reaches for a tool, and the tool outlives it holding the
/// Drone's own stdout.
///
/// **A well-behaved child will never reproduce this.** The background `sleep`
/// inherits the write end of the transcript pipe and does not close it, so the
/// shell exits in milliseconds and the pipe stays open for as long as the
/// `sleep` runs — which is what `#211` is about, and is the one shape that
/// makes reading to end-of-file a wait on something Fleet never observed.
///
/// It says `finished` before it goes, so the terminating event is in the pipe
/// and the fold below is over a run that reported rather than one that
/// vanished.
const LEAVES_A_TOOL_HOLDING_STDOUT: &str =
    "IFS= read -r _; printf '%s\\n' reached-for-something; sleep 30 & \
     printf '%s\\n' finished";

fn job() -> JobId {
    JobId::carried(Ulid::carried(JOB))
}

fn taps(at: &TempDir) -> Taps {
    Taps::opening(
        &at.path().to_string_lossy(),
        Spine {
            job: job(),
            drone: DroneId::carried(Ulid::carried(DRONE)),
            step: StepId::new("implement"),
            run: Ulid::carried(RUN),
        },
        Arc::new(Ticking::from_nine()),
        api::Turns::new().feeding(&ipc::JobId::from(&job())),
    )
    .expect("a transcript opens under a directory that exists")
}

/// What the four lines mean. The first says the Drone reached for a tool and
/// the last is the terminating event, without which the whole run folds to
/// [`Ending::Vanished`] — the answer `crate::aftermath` reads as a process that
/// died having never reported, and escalates a Job for.
fn reading_what_it_says(program: &str) -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", program])
        .reading(
            "reached-for-something",
            vec![DroneEvent::Called {
                tool: String::from("Read"),
                call: String::from("a-call"),
                detail: CallDetail::of("a file"),
            }],
        )
        .reading(
            "finished",
            vec![DroneEvent::Ended {
                turns: 4,
                cost_micros: 0,
                refusals: 2,
            }],
        )
}

/// Put a slot together over a real child, and answer with it and its pid.
async fn a_slot_over(program: &str, at: &TempDir) -> (Working, u32) {
    a_slot_with(FakeHarness::running("/bin/sh", &["-c", program]), at).await
}

/// The same, over a harness that has been told what the child's lines mean.
async fn a_slot_with(harness: FakeHarness, at: &TempDir) -> (Working, u32) {
    let harness = Arc::new(harness);
    let started = start(harness.as_ref(), &config(at))
        .await
        .expect("a shell starts and reads its first turn");
    let pid = started.session.pid();
    let working = Working::holding(
        job(),
        DroneId::carried(Ulid::carried(DRONE)),
        StepId::new("implement"),
        Worktree::at(at.path().to_string_lossy(), "armada/01AAA"),
        started,
        harness,
        taps(at),
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    );
    (working, pid)
}

/// A test that leaves a process running has to end it: `Detached` never sets
/// `kill_on_drop`, because a Drone outliving Fleet is the whole point of
/// detaching, so a test that walks away leaves a `sleep` behind.
fn ended(pid: u32) {
    let _ = std::process::Command::new("/bin/kill")
        .arg(pid.to_string())
        .status();
}

/// End everything left in the Drone's own process group.
///
/// **The group is exactly what `setsid` made**: `Detached` spawns a Drone into
/// a session of its own, so it leads a process group whose id is its pid, and a
/// tool it spawned is in that group unless the tool detached too.
///
/// Best effort, because nothing in production signals a group and this is a
/// sweep rather than a claim — the tools these tests leave behind are `sleep`s
/// that would go on their own. It runs anyway so a failing test does not leave
/// one for the next twenty seconds.
fn the_group_ended(pgid: u32) {
    // Zero is the *caller's* group to every group-directed call, so a sweep
    // that reached here with one would kill the test runner. It cannot —
    // `Child::id` never answers zero — which is exactly why the guard is one
    // line and stays.
    assert_ne!(pgid, 0, "a Drone's group is never the test runner's");
    let _ = std::process::Command::new("/usr/bin/pkill")
        .args(["-g", &pgid.to_string()])
        .status();
}

/// **The assertion the milestone is about, and nothing else observes it.**
///
/// The step is over, so the Drone is over. Not idle, not unreferenced, not
/// replaced in a slot — gone from the process table, which is the only reading
/// that means it has stopped spending.
///
/// **The timeout is part of the claim rather than a guard against a flake.**
/// The child would exit on its own in thirty seconds, and an ending that waited
/// it out would leave `holder_of` answering `Vacant` having signalled nothing —
/// a pass meaning the opposite of what it says. It also names the hazard in the
/// order: the drain waits for a pipe to close, and what closes one is the
/// process at the far end of it ending.
#[tokio::test]
async fn a_drone_that_is_stood_down_is_gone_from_the_process_table() {
    let at = TempDir::new();
    let (working, pid) = a_slot_over(SITS_THERE, &at).await;
    assert!(
        matches!(holder_of(pid), Ok(Holder::Held(_))),
        "the child is running before the boundary, or the rest proves nothing"
    );

    let stood_down = tokio::time::timeout(
        Duration::from_secs(5),
        working.stood_down(&core_model::Timestamp::from_rfc3339(
            "2026-08-31T10:00:00.000Z",
        )),
    )
    .await
    .expect("the ending signals the Drone rather than waiting it out");

    assert!(
        matches!(holder_of(pid), Ok(Holder::Vacant)),
        "the Drone that finished its step is gone"
    );
    assert!(
        stood_down.terminated.is_ok(),
        "and Fleet is what ended it: {:?}",
        stood_down.terminated
    );
    assert_eq!(
        stood_down.step,
        StepId::new("implement"),
        "the exit belongs to the step the Drone was put on"
    );
}

/// **The control, and it is the failure this whole ordering exists for.**
///
/// Dropping the slot is what `put_a_drone_on` does when it assigns over one, so
/// this is the shape a boundary had before the ending was explicit: the record
/// says the Drone left, a fresh one is on the worktree, and the old process is
/// still there working the same directory against the same branch. Nothing in
/// the bookkeeping can tell the two apart, which is why the case above asks the
/// operating system.
#[tokio::test]
async fn a_drone_whose_slot_is_merely_dropped_is_still_running() {
    let at = TempDir::new();
    let (working, pid) = a_slot_over(SITS_THERE, &at).await;

    drop(working);
    // Long enough for a process that was going to die of the drop to have died
    // of it. Nothing was signalled, so nothing will.
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        matches!(holder_of(pid), Ok(Holder::Held(_))),
        "a `setsid`-detached child survives the slot that held it, which is \
         what makes an explicit ending the only ending"
    );
    ended(pid);
}

/// **What a Drone said on its way out is read before anything folds it.** What
/// is in the pipe when a Drone dies is the whole of what it said last.
///
/// The blocking sleep is the construction. Nothing polls the reader task while
/// the test is inside it, so the child writes its four lines and exits into a
/// pipe nobody has read a byte of — which is what `Watching`'s `Drop` used to
/// abort over, and it aborted over exactly the lines a person opens a
/// transcript for.
#[tokio::test]
async fn the_last_thing_a_drone_said_survives_the_reader_being_behind() {
    let at = TempDir::new();
    let harness = Arc::new(reading_what_it_says(SAYS_FOUR_THINGS_AND_GOES));
    let started = start(harness.as_ref(), &config(&at))
        .await
        .expect("a shell starts and reads its first turn");
    let mut watching = Watching::reading(started.transcript, harness, Vec::new());

    // Not `tokio::time::sleep`. An await would hand the runtime to the reader,
    // and there would be nothing left in the pipe for the drain to be about.
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        watching.events().is_empty(),
        "the child has said everything it will say and nothing has read it"
    );

    assert_eq!(
        watching.drained().await,
        Drained::ToTheEnd,
        "a well-behaved child closes the pipe, so the bound is never reached"
    );

    assert_eq!(
        Ending::of(&watching.events()),
        Ending::Reported {
            refusals: 2,
            called_something: true
        },
        "the whole run folds, including the terminating event that was still \
         in the pipe"
    );
}

/// **The drain gives up rather than waiting on a process nobody observed.**
///
/// End-of-file is the *last* writer to a pipe closing it, and a Drone is not
/// necessarily the last: the background `sleep` inherited the same write end
/// and holds it for thirty seconds after the shell has gone. Unbounded, this
/// drain returns in thirty seconds; bounded, it returns in two and says which.
///
/// **Every case above uses a child that closes its handles**, which is why none
/// of them could ever see this — the shape has to be built on purpose.
///
/// **The outer timeout is the whole test.** Taking the bound away does not make
/// this fail on an assertion — it makes `drained` never return, and a test that
/// hangs reports nothing. So the wait is bounded here too, above the drain's
/// own bound and below the tool's lifetime, and the failure stays legible.
#[tokio::test]
async fn the_drain_gives_up_on_a_pipe_a_surviving_tool_still_holds() {
    let at = TempDir::new();
    let harness = Arc::new(reading_what_it_says(LEAVES_A_TOOL_HOLDING_STDOUT));
    let started = start(harness.as_ref(), &config(&at))
        .await
        .expect("a shell starts and reads its first turn");
    let pid = started.session.pid();
    let mut watching = Watching::reading(started.transcript, harness, Vec::new());

    let drained = tokio::time::timeout(Duration::from_secs(20), watching.drained())
        .await
        .expect(
            "the drain is bounded — unbounded it waits the tool out, which is \
             the whole of #211",
        );
    the_group_ended(pid);

    assert!(
        matches!(drained, Drained::CutShort { .. }),
        "and it says the pipe was still held rather than reporting a clean \
         end: {drained:?}"
    );
    assert_eq!(
        Ending::of(&watching.events()),
        Ending::Reported {
            refusals: 2,
            called_something: true
        },
        "everything the Drone itself wrote is still read — the bound gives up \
         a wait, not a transcript"
    );
}

/// **The claim the issue makes: one Drone's stray tool does not stop the
/// fleet.**
///
/// A step boundary runs on the turn loop's own task and the loop walks every
/// Job in series — so a boundary that never returns is not a stuck Job, it is a
/// Fleet where nothing else rules, reaps or admits, and no status says so
/// because nothing noticed.
///
/// The case above asks the same thing of [`Watching`] directly, where the
/// answer is legible. This one asks it of the boundary, which is where it bit.
#[tokio::test]
async fn a_drone_whose_tool_outlives_it_does_not_wedge_the_step_boundary() {
    let at = TempDir::new();
    let (working, pid) = a_slot_with(reading_what_it_says(LEAVES_A_TOOL_HOLDING_STDOUT), &at).await;
    // The tool has to exist before the boundary, or the kill lands first and
    // the case is the ordinary one. `start` returns as soon as the first turn
    // is written, which is before the shell has read it.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !working
            .heard()
            .iter()
            .any(|event| matches!(event, DroneEvent::Ended { .. }))
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the Drone reaches for its tool and reports before the boundary");

    let stood_down = tokio::time::timeout(
        Duration::from_secs(20),
        working.stood_down(&Timestamp::from_rfc3339("2026-08-31T10:00:00.000Z")),
    )
    .await
    .expect("the boundary returns while the tool the Drone left is still running");
    the_group_ended(pid);

    assert_eq!(
        stood_down.ending,
        Ending::Reported {
            refusals: 2,
            called_something: true
        },
        "and what the Drone said before it went was read, not traded for the \
         bound"
    );
    assert!(
        matches!(stood_down.drained, Drained::CutShort { .. }),
        "the record carries that the transcript was cut short, so an ending \
         folded off a prefix does not read as a Drone that said nothing: {:?}",
        stood_down.drained
    );
}
