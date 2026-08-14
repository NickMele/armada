//! **A lease survives its holder dying.**
//!
//! Take a lease, `kill -9` the holder, and confirm the next claimant reclaims
//! it once the heartbeat goes cold rather than blocking forever. This is the
//! mechanism ten-minute `armada manifest check` runs depend on, so it needs a test that
//! kills something — and it does: the holder here is a real `Armada` process,
//! ended with SIGKILL, which has no chance to release anything.
//!
//! **The clock is the seam, and that is the whole reason this is testable.**
//! Cold is sixty monotonic seconds of silence, and a suite that waits them out
//! is a suite nobody runs. The holder's death is real; only the passage of time
//! is injected, which is exactly what `ARCHITECTURE.md` §1.1 makes `now` a seam
//! for.

mod support;

use armada_core::ctx::{Clock, Ctx};
use armada_core::lease::COLD_MS;
use armada_helm::{app, verbs};
use armada_manifest::clock::SystemClock;
use armada_manifest::discovery;
use armada_manifest::net::RealFetch;
use armada_manifest::process::RealRun;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;
use support::Machine;

/// Real everything, except that time has moved on.
struct AdvancedClock {
    by_ms: u64,
}

impl Clock for AdvancedClock {
    fn wall_rfc3339(&self) -> String {
        SystemClock.wall_rfc3339()
    }
    fn wall_ms(&self) -> u64 {
        SystemClock.wall_ms()
    }
    fn mono(&self) -> u64 {
        SystemClock.mono() + self.by_ms
    }
    fn sleep_until(&self, _mono_ms: u64) {}
}

#[test]
fn a_lease_whose_holder_was_killed_is_reclaimed_once_it_goes_cold() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    // A real holder: a dispatched `commands:` entry takes the run lease for as
    // long as its child runs.
    let mut holder = machine.spawn(&repo, &["manifest", "sleeper"]);
    let db_path = machine.home.path().join(".armada/manifest.db");
    wait_for(|| lease_rows(&db_path) == 1);

    // SIGKILL: no unwinding, no `Drop`, nothing released. This is the crash
    // recovery path, not a graceful one.
    let holder_pid = holder.id() as i32;
    unsafe_kill9(holder_pid);
    let _ = holder.wait();
    assert_eq!(
        lease_rows(&db_path),
        1,
        "the dead holder's row must survive — that is what makes it reclaimable"
    );

    // Still warm: the next claimant fails fast rather than stealing it. Acting
    // on state Armada cannot prove is stale is the mistake this whole design
    // avoids.
    let blocked = machine.run(&repo, &["manifest", "init", "--json"]);
    let payload: Value = serde_json::from_slice(&blocked.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_invocation");

    // Now let it go cold. Nothing else about the world changes.
    let output = init_with_clock(
        &machine,
        &repo,
        AdvancedClock {
            by_ms: COLD_MS + 1_000,
        },
    );
    assert!(
        output.is_ok(),
        "a cold lease must be reclaimable: {:?}",
        output.err()
    );
    assert_eq!(
        lease_rows(&db_path),
        0,
        "the reclaimed lease should have been released again on the way out"
    );
}

/// Drive `armada manifest init` in-process against a clock that has moved on.
fn init_with_clock(
    machine: &Machine,
    repo: &Path,
    clock: AdvancedClock,
) -> Result<verbs::Output, armada_core::error::ArmadaError> {
    let run = RealRun;
    let workspace = discovery::resolve(&run, repo)?;
    let ctx = Ctx {
        workspace: Some(workspace),
        run,
        now: clock,
        fetch: RealFetch,
    };
    let inherited: BTreeMap<String, String> = BTreeMap::from([(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_default(),
    )]);
    let mut app = app::build(ctx, machine.home.path(), inherited)?;
    verbs::init::run(&mut app, false)
}

fn lease_rows(db: &Path) -> i64 {
    if !db.exists() {
        return 0;
    }
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row("SELECT count(*) FROM leases", [], |row| row.get(0))
        .unwrap_or(0)
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    for _ in 0..400 {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("condition never became true");
}

fn unsafe_kill9(pid: i32) {
    let status = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(status.success(), "could not SIGKILL {pid}");
}

const CONFIG: &str = "\
manifest:
  version: 1
  commands:
    sleeper:
      cmd: sleep 60
";
