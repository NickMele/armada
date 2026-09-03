//! What a Drone's process group holds once the Drone is gone.
//!
//! # These start a real child, and then ask the operating system about it
//!
//! The property `#371` is about cannot be asserted on a value: whether a
//! process Fleet never started is still running. So each case starts a real
//! shell, has it background a real tool, and reads the answer back out of the
//! process table through [`holder_of`](crate::holder_of) — the same probe the
//! runtime file uses, and one that reports a zombie as held, which is why every
//! reading below is polled rather than taken once.
//!
//! **The two cases are the two ways a Drone ends**, and only one of them was
//! ever a kill. `crate::session` reaches the group from both.

use std::time::Duration;

use testkit::FakeHarness;

use crate::drone::start;
use crate::session::DroneSession;
use crate::tests::drone::config;
use crate::tests::tmp::TempDir;
use crate::{holder_of, Holder, LiveSession};

/// Ten milliseconds a turn, so five seconds of them. Generous, and spent only
/// by a case that is already failing — every wait below breaks the moment the
/// answer changes.
const A_CHILD_HAS_LONG_ENOUGH: u32 = 500;

/// **`#371`, from outside the process.** A tool a Drone backgrounded is not the
/// process Fleet signalled, and a kill at the Drone's pid alone left it running
/// — holding the Drone's stdout, spending whatever it spends, watched by
/// nobody. On 2 Sep one outlived its Fleet by an hour.
///
/// The assertion is `ps`'s answer about a real pid, because the property is the
/// operating system's: no value Fleet holds says whether a process it never
/// started is still there. **The `&` is the whole case**, and the reading
/// before the kill is what stops a tool that never started from passing this.
#[tokio::test]
async fn a_tool_the_drone_left_running_dies_with_the_drone() {
    let at = TempDir::new();
    let marker = at.path().join("tool.pid");
    let backgrounds_a_tool = format!("sleep 30 & echo $! > '{}'; sleep 30", marker.display());
    let started = start(
        &FakeHarness::running("/bin/sh", &["-c", backgrounds_a_tool.as_str()]),
        &config(&at),
    )
    .await
    .expect("a shell starts");

    let tool = pid_written_to(&marker).await;
    assert!(
        matches!(holder_of(tool), Ok(Holder::Held(_))),
        "the tool was not running before the Drone was ended, so ending it \
         proves nothing"
    );

    started.session.terminate().await.expect("it can be ended");

    assert!(
        nothing_holds(tool).await,
        "the tool the Drone backgrounded is still running after the Drone was \
         ended: pid {tool}"
    );
}

/// **The incident `#371` was filed from, and the half a kill does not reach.**
///
/// The Drone on 2 Sep was not killed: its run *ended*, it wrote its own
/// terminating row and exited, and the tool it had started was still there an
/// hour later. Nothing calls `terminate` on that path — `dispatch::reap` sees
/// the process is gone and drops the slot — so the only call that runs is
/// [`DroneSession::exited`], and ending the group is part of what it means to
/// reap one.
///
/// **Remove the group signal from `exited` and this fails**, on the poll below:
/// the shell is gone within milliseconds either way, and the `sleep` it left is
/// still in the process table five seconds later.
#[tokio::test]
async fn a_tool_outliving_a_drone_that_ended_on_its_own_dies_when_it_is_reaped() {
    let at = TempDir::new();
    let marker = at.path().join("tool.pid");
    // It reads its first turn before it does anything, so the spawn is over
    // before the shell is — a Drone that exited during `start` is a different
    // case, and it is the one above.
    let ends_leaving_a_tool = format!(
        "IFS= read -r _; sleep 30 & echo $! > '{}'",
        marker.display()
    );
    let started = start(
        &FakeHarness::running("/bin/sh", &["-c", ends_leaving_a_tool.as_str()]),
        &config(&at),
    )
    .await
    .expect("a shell starts");

    let tool = pid_written_to(&marker).await;
    assert!(
        matches!(holder_of(tool), Ok(Holder::Held(_))),
        "the tool was not running before the Drone ended, so its going proves \
         nothing"
    );

    assert!(
        reaped(&started.session).await,
        "the Drone never ended on its own, so nothing here was measured"
    );
    assert!(
        nothing_holds(tool).await,
        "the tool outlived the Drone that started it and nothing signalled it: \
         pid {tool}"
    );
}

/// Whether the Drone has ended, asked the way Fleet asks — **on a turn, and
/// never as a wait**. `exited` is the reap, so this is also what performs it.
async fn reaped(session: &DroneSession) -> bool {
    for _ in 0..A_CHILD_HAS_LONG_ENOUGH {
        if session.exited().await.expect("the child can be waited on") {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

/// The pid a shell wrote down, once it has written it.
async fn pid_written_to(marker: &std::path::Path) -> u32 {
    for _ in 0..A_CHILD_HAS_LONG_ENOUGH {
        let written = std::fs::read_to_string(marker)
            .ok()
            .and_then(|text| text.trim().parse().ok());
        if let Some(pid) = written {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the shell never wrote down the pid of the tool it started");
}

/// Whether nothing holds `pid` any more.
///
/// **Polled rather than read once**: a killed process stays a zombie until
/// whatever it was reparented to collects it, and `ps` reports a zombie as
/// held. The wait is only ever spent on a run that is actually wrong.
async fn nothing_holds(pid: u32) -> bool {
    for _ in 0..A_CHILD_HAS_LONG_ENOUGH {
        if matches!(holder_of(pid), Ok(Holder::Vacant)) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}
