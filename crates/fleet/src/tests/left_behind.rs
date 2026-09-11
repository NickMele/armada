//! What a crashed Fleet left running, ended when the next one starts — and a
//! pid the system has since given another process, left alone.
//!
//! **Real processes**, for `crate::tests::servers`' reason: whether a group
//! is gone and a port free is not something a fake can stand in for. Each
//! group is waited on by a thread of the test's own, so an ended one is
//! collected at once rather than lingering as the test's zombie.

use std::cell::Cell;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use crate::ports::{BindConnectProbe, PortProbe};
use crate::process::{holder_of, Holder, StartedAt};
use crate::servers::left::recorded;
use crate::tests::servers::a_fleet_serving;
use crate::tests::tmp::TempDir;

/// A process leading a group of its own, as a server's `serve` does.
struct Left {
    pid: u32,
    ended: Receiver<ExitStatus>,
    seen_end: Cell<bool>,
}

impl Left {
    fn a_group(home: &TempDir, program: &str, args: &[&str]) -> Left {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(home.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("spawned");
        let pid = child.id();
        let (told, ended) = channel();
        std::thread::spawn(move || {
            if let Ok(status) = child.wait() {
                let _ = told.send(status);
            }
        });
        Left {
            pid,
            ended,
            seen_end: Cell::new(false),
        }
    }

    fn started(&self) -> StartedAt {
        match holder_of(self.pid).expect("ps answers") {
            Holder::Held(at) => at,
            Holder::Vacant => panic!("{} is not running", self.pid),
        }
    }

    fn ended_within(&self, wait: Duration) -> bool {
        let ended = self.ended.recv_timeout(wait).is_ok();
        self.seen_end.set(self.seen_end.get() || ended);
        ended
    }
}

impl Drop for Left {
    /// A group the test left running is ended — only while it is still the
    /// test's own, which the thread waiting on it has not yet collected.
    fn drop(&mut self) {
        if !self.seen_end.get() && self.ended.try_recv().is_err() {
            let _ = Command::new("kill")
                .args(["-9", &self.pid.to_string()])
                .status();
        }
    }
}

/// A main-checkout server's directory, as `crate::servers` makes one.
fn a_server_dir(home: &TempDir, id: &str) -> PathBuf {
    let dir = home
        .path()
        .join(".armada")
        .join("servers")
        .join("main")
        .join(id);
    std::fs::create_dir_all(&dir).expect("the directory");
    dir
}

/// **A record for a group still running is ended at startup**, and the record
/// goes with it.
#[tokio::test]
async fn a_group_a_crashed_fleet_left_running_is_ended_at_startup() {
    let home = TempDir::new();
    let fleet = a_fleet_serving(&home, &api::Broadcaster::new());
    let left = Left::a_group(&home, "/bin/sleep", &["60"]);
    let dir = a_server_dir(&home, "01LEFTBEHIND");
    recorded(
        &dir,
        "01LEFTBEHIND",
        "main-checkout",
        left.pid,
        &left.started(),
    )
    .expect("recorded");

    let reaped = fleet.reaped_left_servers().await;
    assert_eq!(reaped.ended, 1, "{reaped:?}");
    assert!(
        left.ended_within(Duration::from_secs(5)),
        "the group was ended"
    );
    assert!(!dir.join("held").exists(), "and its record removed");
}

/// **A pid that now belongs to a different process is left alone** — the start
/// time does not match — and the record is removed all the same.
#[tokio::test]
async fn a_record_whose_pid_is_now_another_process_is_left_alone() {
    let home = TempDir::new();
    let fleet = a_fleet_serving(&home, &api::Broadcaster::new());
    let other = Left::a_group(&home, "/bin/sleep", &["60"]);
    let dir = a_server_dir(&home, "01REUSEDPID");
    let long_ago = StartedAt::carried("Thu Jan  1 00:00:00 1970");
    recorded(&dir, "01REUSEDPID", "main-checkout", other.pid, &long_ago).expect("recorded");

    let reaped = fleet.reaped_left_servers().await;
    assert_eq!((reaped.ended, reaped.left_alone), (0, 1), "{reaped:?}");
    assert!(
        !other.ended_within(Duration::from_millis(300)),
        "a reused pid is never killed"
    );
    assert!(!dir.join("held").exists(), "the record is removed");
}

/// **The main checkout's span is free after the cleanup**, because startup
/// ends the leftover server before it re-probes the span — so the span is
/// kept rather than given up as held by something outside every tree.
#[tokio::test]
async fn the_main_checkouts_span_is_free_after_the_cleanup() {
    let home = TempDir::new();
    let fleet = a_fleet_serving(&home, &api::Broadcaster::new());
    let port = *fleet
        .main_checkout_ports()
        .await
        .get("storybook")
        .expect("the main checkout's span");
    let port_text = port.to_string();
    let left = Left::a_group(
        &home,
        "python3",
        &["-m", "http.server", &port_text, "--bind", "127.0.0.1"],
    );
    let bound = tokio::time::timeout(Duration::from_secs(10), async {
        while BindConnectProbe.free(port) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(bound.is_ok(), "the leftover server is holding its port");
    let dir = a_server_dir(&home, "01HOLDSAPORT");
    recorded(
        &dir,
        "01HOLDSAPORT",
        "main-checkout",
        left.pid,
        &left.started(),
    )
    .expect("recorded");

    fleet.reconcile().await.expect("startup reconciles");

    assert!(
        left.ended_within(Duration::from_secs(5)),
        "ended at startup"
    );
    assert!(BindConnectProbe.free(port), "its port is free");
    assert_eq!(
        fleet.main_checkout_ports().await.get("storybook"),
        Some(&port),
        "the span was kept, not given up as held"
    );
}
