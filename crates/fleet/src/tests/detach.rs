//! That a spawned child really is detached, and that the assertion means
//! something.
//!
//! `setsid` makes the caller the leader of a new session *and* of a new process
//! group, so a detached child's process-group id is its own pid. An attached
//! child inherits Fleet's. Those two facts are what the pair of tests below
//! compare, and the second one exists because the first passes trivially if the
//! reading is wrong in the right direction.
//!
//! # What is not tested here, and why
//!
//! That a Drone survives a group-directed signal at Fleet. Sending one means
//! signalling the test runner's own process group, which kills the test. The
//! property is measured in `../../../docs/concepts/fleet.md` against launchd
//! rather than asserted here; what this file can prove is the precondition —
//! the child is not in that group.

use tokio::process::Command;

use crate::Detached;

/// The process group of the process this shell is running as.
const REPORT_OWN_PGID: &str = "ps -o pgid= -p $$";

fn pgid(output: &[u8]) -> u32 {
    String::from_utf8_lossy(output)
        .trim()
        .parse()
        .expect("ps prints a process group id")
}

#[tokio::test]
async fn a_spawned_child_leads_its_own_process_group() {
    let child = Detached::program("/bin/sh")
        .args(["-c", REPORT_OWN_PGID])
        .capturing_output()
        .spawn()
        .expect("a shell spawns");
    let pid = child.id().expect("a spawned child has a pid");
    let out = child.wait_with_output().await.expect("it runs and exits");

    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        pgid(&out.stdout),
        pid,
        "a detached child is its own process group leader, so a signal at \
         Fleet's group does not reach it"
    );
}

#[tokio::test]
async fn a_child_spawned_without_detaching_stays_in_this_process_group() {
    // The control. Without it, a test that read the wrong column and got the
    // child's pid back would pass while proving nothing.
    let out = Command::new("/bin/sh")
        .args(["-c", REPORT_OWN_PGID])
        .output()
        .await
        .expect("a shell spawns");

    let ours = std::process::Command::new("ps")
        .args(["-o", "pgid=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps runs");

    assert_eq!(
        pgid(&out.stdout),
        pgid(&ours.stdout),
        "an ordinary child inherits the spawning process's group — which is \
         exactly what a Drone must not do"
    );
}
