//! That the Drone which finished its step is **gone**, and that what it said on
//! the way out survived it.
//!
//! # These start a real child, and it is the only way to ask
//!
//! Every other test of a step boundary drives it by calling what
//! `crate::dispatch` calls, and every one of them would pass over a Drone that
//! was still running: the slot was replaced, the record says the exit landed,
//! and a new process arrived. **That is bookkeeping.** A Drone is started
//! `setsid`-detached, so nothing about a slot being dropped reaches the process
//! — it keeps its worktree open and keeps spending, and the only thing that can
//! say otherwise is the operating system.
//!
//! So these ask it, through [`holder_of`] — the same `ps` probe the runtime
//! file's identity check uses. The pair is `crate::tests::detach`'s shape: one
//! case asserts the property, and the one beside it asserts that the property
//! is not free, because an assertion that passes over the broken code as well
//! proves nothing.
//!
//! **`holder_of` and not `DroneSession::exited`.** That one asks a handle this
//! test would have to keep, and standing a Drone down consumes the handle along
//! with everything else — which is the point of it. `ps` asks about a pid and
//! reports a zombie as held; `terminate` waits on the child, so a pid still
//! held afterwards is a process still there.
//!
//! # The drain is asserted on `Watching` and not through the slot
//!
//! [`Watching::drained`] is what stops a Drone's last lines being thrown away,
//! and the state it exists for — the reader task behind a pipe that has already
//! closed — is one a test has to construct rather than hope for. It is
//! constructed here by never letting the reader run: `#[tokio::test]` is a
//! current-thread runtime, so a *blocking* sleep in the test body is a window
//! in which the child writes everything it will ever write and nothing reads
//! any of it.
//!
//! Asked of the slot instead, the same case proves less than it looks like it
//! does. Ending a Drone waits on the child, and a fast fake catches up inside
//! that wait whether or not anything asked it to — so a slot-level assertion
//! passes over a missing drain and reports that the drain is tested.

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
use crate::watch::Watching;
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
fn reading_what_it_says() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", SAYS_FOUR_THINGS_AND_GOES])
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
    let harness = Arc::new(FakeHarness::running("/bin/sh", &["-c", program]));
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

    let stood_down = tokio::time::timeout(Duration::from_secs(5), working.stood_down())
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
    let harness = Arc::new(reading_what_it_says());
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

    watching.drained().await;

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
