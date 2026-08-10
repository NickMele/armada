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

use charkit_adapters::posix;
use charkit_adapters::process::ProcessGroup;
use charkit_core::ctx::{RunRequest, StdioMode};
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
    let mut group = spawn("sleep 300 & sleep 300 & wait");
    settle();

    let before = processes_in_group(group.pgid());
    assert!(
        before >= 3,
        "expected a leader and two grandchildren, found {before}"
    );

    let report = posix::stop_group(group.pgid(), Duration::from_secs(2));
    assert!(report.gone, "the group survived: {report:?}");

    // Reap before counting. A killed direct child is a zombie until char waits
    // on it, and `ps` lists a zombie — so counting first would measure char's
    // own reaping discipline rather than whether `killpg` reached the tree.
    // Measured on this machine: `kill(pid, 0)` against a zombie answers
    // `ESRCH`, so `group_alive` says gone while `ps` still shows the entry.
    group.stop();
    settle();
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
    let mut group = spawn("trap '' TERM; sleep 300 & sleep 300 & wait");
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
    assert!(report.gone, "SIGKILL did not clear the group: {report:?}");
    group.stop();
    settle();
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

    let mut group = spawn(&format!("/usr/bin/perl {} & wait", script.display()));

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
    group.stop();
    assert!(report.gone, "the tracked group should be empty");

    // The escapee is alive, in its own session, and `killpg` never reached it.
    assert!(
        charkit_adapters::net::port_is_taken(port),
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

    // Measured here rather than assumed, because two other tests depend on it:
    // a **zombie answers `ESRCH` to `kill(pid, 0)`**, so char's liveness probe
    // reports the group gone while `ps` still lists the entry. That is why
    // those tests reap before counting — otherwise they would be measuring
    // char's reaping discipline while claiming to measure `killpg`'s reach.
    assert!(
        !posix::group_alive(leaked_pid),
        "a zombie was expected to answer ESRCH to a signal-0 probe"
    );
    // Reap it so this test leaves nothing behind either.
    reap_any();

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

/// Reap whatever this process has left, so one test's evidence is not the next
/// one's noise.
fn reap_any() {
    let mut status = 0;
    // SAFETY: `waitpid` with WNOHANG on -1 reaps any already-exited child and
    // returns immediately otherwise. This is a test helper; the crate's own
    // `unsafe` budget is unaffected.
    #[allow(unsafe_code)]
    unsafe {
        while libc::waitpid(-1, &mut status, libc::WNOHANG) > 0 {}
    }
}
