//! Real processes, real signals — the phase's central done-when.
//!
//! **No process outlives its workspace, tested against an *uncooperative*
//! service.** A cooperative `sleep` passes the process-group test while proving
//! nothing: measured, a leader running `trap '' TERM` leaves 3 of 3 alive after
//! `killpg(SIGTERM)`, because children inherit an *ignored* disposition across
//! `fork` and `exec`. So the suite needs three cases, and each one is a
//! different claim:
//!
//! | Case | What it proves |
//! |---|---|
//! | cooperative | `killpg` against a `setsid`'d group reaches **grandchildren** |
//! | SIGTERM-ignoring | one uncooperative leader immunises its whole group, and only the SIGKILL escalation gets it |
//! | self-`setsid` | a group char cannot reach at all, which must be **detected and reported** rather than silently missed |
//!
//! These are run directly against the wrapper rather than through `char down`,
//! which is phase 4 — an earlier draft's criteria could not be run at the end
//! of this phase at all.

use armada_core::ctx::{RunRequest, StdioMode};
use armada_manifest::posix;
use armada_manifest::process::ProcessGroup;
use std::path::PathBuf;
use std::time::Duration;

/// How many processes are in this group, counted from `ps` rather than
/// inferred. `killpg` reaching grandchildren is the claim under test, so the
/// count has to come from outside char.
fn processes_in_group(pgid: i32) -> usize {
    let output = std::process::Command::new("ps")
        .args(["-A", "-o", "pgid=,pid="])
        .output()
        .expect("ps runs on any POSIX machine");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|group| group.parse::<i32>() == Ok(pgid))
        .count()
}

fn spawn(script: &str) -> ProcessGroup {
    let request = RunRequest::new(
        vec!["/bin/sh".to_string(), "-c".to_string(), script.to_string()],
        PathBuf::from("/"),
    )
    .stdio(StdioMode::Capture);
    ProcessGroup::spawn(&request).expect("/bin/sh exists")
}

/// Give the tree a moment to fork its children before counting.
fn settle() {
    std::thread::sleep(Duration::from_millis(300));
}

#[test]
fn killpg_against_a_setsid_group_reaches_grandchildren() {
    let group = spawn("sleep 300 & sleep 300 & wait");
    settle();

    let before = processes_in_group(group.pgid());
    assert!(
        before >= 3,
        "expected a leader and two grandchildren, found {before}"
    );

    let report = posix::stop_group(group.pgid(), Duration::from_secs(2));
    assert!(report.existed, "the group was live before the stop");

    // Reap before judging, and judge afterwards — `report.gone` cannot be the
    // assertion here. A killed direct child is a zombie until char waits on it,
    // that zombie is still a member of its process group, and the two platforms
    // disagree about whether it counts: measured, `killpg(pgid, 0)` against a
    // group whose only member is an unreaped zombie *succeeds* on Linux and
    // fails on darwin. So before the wait, `stop_group` says gone on one
    // platform and survived on the other; after it, both say gone. `ps` lists a
    // zombie on both, so it has to be reaped before counting too.
    reap(group.pid());
    settle();
    assert!(
        !posix::group_alive(group.pgid()),
        "the group survived: {report:?}"
    );
    assert_eq!(
        processes_in_group(group.pgid()),
        0,
        "a process outlived its group"
    );
}

/// The measurement that makes the cooperative case insufficient, re-run rather
/// than re-trusted: SIGTERM alone leaves every one of them alive.
#[test]
fn a_sigterm_ignoring_leader_immunises_its_whole_group() {
    let group = spawn("trap '' TERM; sleep 300 & sleep 300 & wait");
    settle();

    let before = processes_in_group(group.pgid());
    assert!(before >= 3, "expected three processes, found {before}");

    posix::killpg(group.pgid(), libc::SIGTERM).expect("the group exists");
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(
        processes_in_group(group.pgid()),
        before,
        "SIGTERM was expected to be ignored by the whole group"
    );

    // **SIGTERM, wait a grace period, then SIGKILL — an unconditional
    // escalation, not a retry**, because a process that ignores SIGTERM ignores
    // the second one too.
    let report = posix::stop_group(group.pgid(), Duration::from_millis(300));
    assert!(report.escalated, "the escalation should have been needed");
    // Reap before judging, for the reason recorded above: until char waits on
    // the leader SIGKILL left behind, the group still has a member on Linux.
    reap(group.pid());
    settle();
    assert!(
        !posix::group_alive(group.pgid()),
        "SIGKILL did not clear the group: {report:?}"
    );
    assert_eq!(processes_in_group(group.pgid()), 0);
}

/// A service that calls `setsid` itself — ordinary daemonizing — leaves the
/// tracked group entirely, so its pgid is not the one recorded and no `killpg`
/// reaches it. **That case is detected by the port still being bound
/// afterwards, not prevented**, and this test asserts exactly that: the group
/// is gone, the escapee is not, and char can see the difference.
#[test]
fn a_self_setsid_service_escapes_the_group_and_is_detected_by_its_port() {
    if !std::path::Path::new("/usr/bin/perl").exists() {
        eprintln!("skipping: no /usr/bin/perl to build a self-setsid child with");
        return;
    }

    let scratch = tempfile::tempdir().unwrap();
    let pid_file = scratch.path().join("escapee.pid");

    // A port the kernel just handed out and released is free by construction.
    let port = {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    };

    // Written to a file rather than passed with `-e`: the script has to survive
    // `sh -c`, and quoting it twice is how a test starts asserting something
    // other than what it says.
    let script = scratch.path().join("escapee.pl");
    std::fs::write(
        &script,
        format!(
            "use POSIX;\nuse IO::Socket::INET;\n\
             POSIX::setsid();\n\
             my $s = IO::Socket::INET->new(LocalAddr => '127.0.0.1', LocalPort => {port}, Listen => 5) or die $!;\n\
             open(my $f, '>', '{pid}') or die $!;\nprint $f $$;\nclose $f;\n\
             sleep 300;\n",
            port = port,
            pid = pid_file.display()
        ),
    )
    .unwrap();

    let group = spawn(&format!("/usr/bin/perl {} & wait", script.display()));

    // Wait for the escapee to have detached and bound.
    for _ in 0..100 {
        if pid_file.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let escapee_pid: i32 = std::fs::read_to_string(&pid_file)
        .expect("the escapee wrote its pid")
        .trim()
        .parse()
        .unwrap();

    let report = posix::stop_group(group.pgid(), Duration::from_millis(300));
    // Reaped here rather than through `group.stop()`, for the reason recorded
    // above and one more: the escapee inherited the leader's pipes and is still
    // holding them open, so a `wait` that drained them would never return.
    reap(group.pid());
    assert!(
        !posix::group_alive(group.pgid()),
        "the tracked group should be empty: {report:?}"
    );

    // The escapee is alive, in its own session, and `killpg` never reached it.
    assert!(
        armada_manifest::net::port_is_taken(port),
        "the escaped service's port should still be bound — that is the detection"
    );
    assert!(
        posix::group_alive(escapee_pid),
        "the escapee is its own group leader and is still running"
    );

    let _ = posix::killpg(escapee_pid, libc::SIGKILL);
}

/// **Every spawned `Child` is waited on, or explicitly reaped.** Measured:
/// Rust's `Child` does not reap on drop, so a dropped handle leaves a
/// `<defunct>` entry until char itself exits — and a fifteen-minute detached
/// run accumulates them.
#[test]
fn a_child_dropped_without_wait_leaves_a_zombie_and_the_wrapper_never_does() {
    // First the measurement itself, so the rule is defended by evidence rather
    // than by a comment.
    let leaked_pid = {
        let child = std::process::Command::new("/bin/sh")
            .args(["-c", "exit 0"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id() as i32;
        std::thread::sleep(Duration::from_millis(200));
        drop(child);
        pid
    };
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        process_state(leaked_pid).contains('Z'),
        "expected a zombie from a dropped Child; got {:?}",
        process_state(leaked_pid)
    );

    // The rule the three tests above depend on, and the half of it that is
    // portable is asserted right here: **a zombie stays a member of its process
    // group until char reaps it**, and the reap is what clears it — on both
    // platforms, which is why those tests reap before they judge.
    //
    // The half that is not portable, and so is recorded rather than asserted:
    // `killpg(pgid, 0)` against a group whose only remaining member is an
    // unreaped zombie *succeeds* on Linux and fails on darwin. So no test may
    // ask `stop_group` whether it emptied a group char has not waited on yet —
    // the answer is the platform, not the kill.
    reap(leaked_pid);
    assert!(
        !process_state(leaked_pid).contains('Z'),
        "the wait was expected to clear the zombie; got {:?}",
        process_state(leaked_pid)
    );

    // Now the wrapper, which waits on every path.
    let request = RunRequest::new(
        vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            "exit 0".to_string(),
        ],
        PathBuf::from("/"),
    );
    let mut group = ProcessGroup::spawn(&request).unwrap();
    let pid = group.pid();
    group.wait(None, &mut || {});
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        !process_state(pid).contains('Z'),
        "the wrapper left a zombie behind"
    );
}

fn process_state(pid: i32) -> String {
    let output = std::process::Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .expect("ps runs");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Reap one child char has already signalled, so its `<defunct>` entry stops
/// answering the liveness probes the assertions above read.
///
/// **`waitpid` on that pid and never on `-1`.** These tests run as threads of
/// one binary, so a `waitpid(-1)` here reaps whichever child exited first —
/// including another test's group leader, which is one test silently supplying
/// the reaping the test under it is asserting about.
///
/// Bounded and polling rather than blocking: a leader that survived the signal
/// is the failure this suite exists to catch, and it should be caught by the
/// assertion below rather than by a test that never returns.
fn reap(pid: i32) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut status = 0;
        // SAFETY: `waitpid` with WNOHANG takes a pid and a pointer to a `c_int`
        // this frame owns, and returns immediately either way. This is a test
        // helper; the crate's own `unsafe` budget is unaffected.
        #[allow(unsafe_code)]
        let rc = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        // Reaped, or already gone (`ECHILD`). Zero means still running.
        if rc != 0 || std::time::Instant::now() >= deadline {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
