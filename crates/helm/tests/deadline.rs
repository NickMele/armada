//! A check that ignores SIGTERM still dies, because the deadline escalates.
//!
//! **This is the test the reducer's unit test could not be.** `schedule.rs` has
//! had `a_deadline_terminates_first_and_a_second_one_escalates` since the
//! scheduler was written, and it passes by handing the reducer two `Deadline`
//! events directly — so it proves the reducer and nothing about the path. The
//! shell only ever sent the first one, because it asked "is this check late?"
//! in its own words and answered "no, it is already stopping". `escalate` was
//! therefore `false` in every production run, SIGKILL never went out, and a
//! check running `trap '' TERM` outlived its own `timeout:` forever.
//!
//! Nothing short of a real child that really ignores SIGTERM catches that,
//! which is why this is an e2e test: a fake `Run` seam would have obeyed the
//! first signal, and the assertion would have passed against the broken code.

mod support;

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::{armada_binary, Machine};

/// A check whose child **ignores SIGTERM**, so the only thing that ends it is
/// the signal that cannot be caught.
///
/// `shell: true` is required rather than incidental — `trap` is a shell
/// builtin, and there is no argv that installs a signal disposition. The
/// `timeout:` is the floor the schema allows, because the run has to reach its
/// deadline before this test's own patience runs out.
const STUBBORN: &str = r#"
manifest:
  version: 1
  components:
    stubborn:
      checks:
        ignores_term:
          cmd: "trap '' TERM; sleep MARKER"
          shell: true
          timeout: 1
"#;

/// The marker is unique per run so a stray `sleep` from another test — or from
/// the developer's own shell — can never be mistaken for this one's child.
fn survivors(marker: &str) -> usize {
    let out = Command::new("pgrep")
        .args(["-f", marker])
        .output()
        .expect("pgrep");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}

fn wait_for<F: Fn() -> bool>(what: &str, limit: Duration, f: F) {
    let start = Instant::now();
    while start.elapsed() < limit {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("timed out waiting for {what}");
}

/// Kills anything this test started, however it ends.
///
/// **The failing case is the one that needs it.** When the escalation is broken
/// the child is by definition unkillable by SIGTERM and Armada is wedged
/// polling it, so a bare `panic!` would leave both behind — and the next
/// session would find a `sleep` nobody could explain. `pkill` by the unique
/// marker is safe for exactly the reason `survivors` is: nothing else on the
/// machine carries it.
///
/// **`-KILL`, and it was measured the hard way.** `pkill` defaults to SIGTERM,
/// which this test's whole subject is a process ignoring — the first run
/// against the broken code left the pair alive despite this guard. A cleanup
/// for an uncooperative child has to use the signal that cannot be caught, for
/// the same reason the scheduler does.
struct Cleanup {
    marker: String,
    armada: u32,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        // Armada first: it is the thing that would go on polling a child this
        // is about to remove out from under it. Both are silenced because the
        // *passing* path reaches here with nothing left to kill, and a
        // `No such process` on a green run reads like a fault.
        let _ = Command::new("kill")
            .args(["-KILL", &self.armada.to_string()])
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("pkill")
            .args(["-KILL", "-f", &self.marker])
            .stderr(Stdio::null())
            .status();
    }
}

#[test]
fn a_check_that_ignores_sigterm_is_killed_when_the_grace_expires() {
    let machine = Machine::new();
    // **The duration is the marker**, as in `interrupt.rs`: a distinctive
    // number identifies this test's processes by command line. A different base
    // from that test's, so two suites running at once cannot see each other's.
    let seconds = 25_000 + (std::process::id() % 900);
    let marker = format!("sleep {seconds}");
    let config = STUBBORN.replace("sleep MARKER", &marker);
    let repo = machine.repo("stubborn", &config);

    let mut child = Command::new(armada_binary())
        .args(["manifest", "check", "--all-files"])
        .current_dir(&repo)
        .env("HOME", machine.home.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn armada");
    // Declared after `child` so it is dropped *before* it: on a panic the guard
    // has to run while there is still something to clean up.
    let cleanup = Cleanup {
        marker: marker.clone(),
        armada: child.id(),
    };

    wait_for(
        "the check's child to start",
        Duration::from_secs(30),
        || survivors(&marker) > 0,
    );

    // **Bounded, and that is the assertion.** `Child::wait` would block forever
    // against the broken escalation and hang the whole suite rather than fail
    // it; the deadline here is `timeout: 1` plus `KILL_GRACE_MS` plus room for
    // a loaded machine, and overrunning it *is* the bug.
    let limit = Duration::from_secs(60);
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("armada is waitable") {
            break status;
        }
        assert!(
            start.elapsed() < limit,
            "armada never finished: a check ignoring SIGTERM was left running, \
             which is the escalation never firing"
        );
        std::thread::sleep(Duration::from_millis(100));
    };

    // **The deadline is the verdict, not the code the child died with**
    // (`ARCHITECTURE.md` §1.6): a SIGKILLed child looks like a failure, and
    // reporting `tool_failed` would send the reader hunting a broken test.
    assert_eq!(
        status.code(),
        Some(4),
        "a run whose only check timed out exits 4"
    );

    // The whole point: the group is gone, not merely signalled.
    wait_for(
        "the SIGTERM-ignoring child to be killed",
        Duration::from_secs(15),
        || survivors(&marker) == 0,
    );

    drop(cleanup);
}
