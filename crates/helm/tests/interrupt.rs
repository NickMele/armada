//! Ctrl-C ends the run and takes the children with it.
//!
//! **The bug this covers is invisible from inside the process.** Children are
//! `setsid`'d into their own sessions ([`posix::spawn`]), so a signal delivered
//! to Armada never reaches them. Before the handler existed, Ctrl-C during
//! `check` returned the shell and left `cargo test` running — measured, and
//! recorded in `PHASES.md` §9.3.
//!
//! It can only be tested by signalling a real process and then looking for the
//! grandchild, which is why this is an e2e test rather than a unit one.

mod support;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::{armada_binary, Machine};

/// A check whose child outlives any reasonable test, so the only way it ends is
/// because something killed it.
const SLOW: &str = r#"
manifest:
  version: 1
  components:
    slow:
      checks:
        sleeper:
          cmd: sleep MARKER
          timeout: 600
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

#[test]
fn ctrl_c_ends_the_run_and_leaves_no_children_behind() {
    let machine = Machine::new();
    // **The duration is the marker.** A distinctive number is unique enough to
    // identify this test's child by command line, and needs no shell to carry
    // it — which matters, because a `shell: true` entry would change what is
    // being tested from "the child" to "the shell wrapping the child".
    let seconds = 24_000 + (std::process::id() % 900);
    let marker = format!("sleep {seconds}");
    let config = SLOW.replace("sleep MARKER", &marker);
    let repo = machine.repo("interrupted", &config);

    let mut child = Command::new(armada_binary())
        .args(["manifest", "check", "--all-files"])
        .current_dir(&repo)
        .env("HOME", machine.home.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn armada");

    wait_for(
        "the check's child to start",
        Duration::from_secs(30),
        || survivors(&marker) > 0,
    );

    // SIGINT to Armada alone. **Not to the process group** — signalling the
    // group would kill the children directly and prove nothing, since the whole
    // question is whether Armada relays it to sessions the signal cannot reach.
    //
    // Through `/bin/kill` rather than `libc::kill`, because `unsafe` is denied
    // outside `manifest::posix` (`ARCHITECTURE.md` §1) and a test is not a
    // reason to make an exception.
    let killed = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("kill");
    assert!(killed.success(), "could not signal armada");

    let status = child.wait().expect("armada exits");

    // **It dies *by* the signal, and that is the point.** A signal has no error
    // class and does not get one's exit code (`ARCHITECTURE.md` §1.6): Armada
    // restores the default disposition and re-raises, so the kernel reports
    // `WIFSIGNALED` and the parent shell renders the familiar `130`. Rust
    // models that as `code() == None` and `signal() == Some(SIGINT)` — asserting
    // on `code()` would be asserting that Armada called `exit(130)` itself,
    // which is the weaker thing that leaves job control none the wiser.
    const SIGINT: i32 = 2;
    assert_eq!(
        status.signal(),
        Some(SIGINT),
        "an interrupted run must die by the signal, so the shell reports 130"
    );
    assert_eq!(
        status.code(),
        None,
        "a signal death has no exit code of its own"
    );

    wait_for("the child to be killed", Duration::from_secs(15), || {
        survivors(&marker) == 0
    });
}
