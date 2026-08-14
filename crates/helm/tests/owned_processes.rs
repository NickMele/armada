//! What `char clean` does with a recorded process group — against real
//! processes, real signals and the real store.
//!
//! **Nothing is killed that char cannot prove is its own.** That rule is a pure
//! function in the core and is unit-tested there; what is only testable here is
//! that the verb actually consults it before sending a signal, and that the row
//! goes either way. Three cases, and two of them are the dangerous ones:
//!
//! | Recorded row | What must happen |
//! |---|---|
//! | a live group this boot stamped | signalled, confirmed gone, counted |
//! | a group stamped by another boot | **never signalled** — that pid is recycled |
//! | `0`, or anything unparseable | **never signalled** — `killpg(0, …)` is char's own group |
//!
//! The second and third are why this suite starts a real detached group rather
//! than asserting on a summary: a regression in either sends a real SIGKILL to
//! something, and the only honest way to test "and it was still there
//! afterwards" is for there to have been something there.

mod support;

use armada_core::ctx::RunRequest;
use armada_core::id::WorkspaceId;
use armada_core::registry::{OwnedKind, OwnedRow};
use armada_manifest::db::Db;
use armada_manifest::machine;
use armada_manifest::process::{ProcessGroup, RealRun};
use armada_manifest::{fs, posix};
use serde_json::Value;
use std::path::{Path, PathBuf};
use support::Machine;

/// A detached `sleep`, in a session of its own so its pgid is reachable.
fn detached_sleeper(cwd: &Path) -> ProcessGroup {
    let request = RunRequest::new(
        vec!["sh".to_string(), "-c".to_string(), "sleep 60".to_string()],
        cwd.to_path_buf(),
    );
    let group = ProcessGroup::spawn(&request).expect("sh runs");
    assert!(group.pgid() > 0, "the wrapper must detach it");
    group
}

fn record(machine: &Machine, workspace: &WorkspaceId, row: OwnedRow) {
    let mut db = Db::open(&machine::armada_home(machine.home.path())).unwrap();
    assert_eq!(&row.workspace, workspace);
    db.record_owned(&row).unwrap();
}

/// The boot and start-time stamps char itself would write for a live group.
fn ours(workspace: &WorkspaceId, pgid: i32, cwd: &Path) -> OwnedRow {
    OwnedRow {
        workspace: workspace.clone(),
        kind: OwnedKind::Pgid,
        reference: pgid.to_string(),
        boot_id: machine::boot_id(&RealRun, cwd),
        pid_started_at: machine::process_start_at(&RealRun, cwd, pgid),
    }
}

fn init(machine: &Machine, repo: &Path) -> WorkspaceId {
    let payload: Value =
        serde_json::from_slice(&machine.run(repo, &["manifest", "init", "--json"]).stdout).unwrap();
    WorkspaceId::from_stored(
        payload["workspace"]
            .as_str()
            .expect("init resolved")
            .to_string(),
    )
}

fn clean(machine: &Machine, repo: &Path) -> Value {
    let output = machine.run(repo, &["manifest", "clean", "--json"]);
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
}

#[test]
fn clean_stops_a_recorded_group_confirms_it_is_gone_and_counts_it() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    let workspace = init(&machine, &repo);

    let mut group = detached_sleeper(&repo);
    let pgid = group.pgid();
    record(&machine, &workspace, ours(&workspace, pgid, &repo));

    // **The reap has to happen while `char clean` is running, not after it.**
    // This test is the group's parent, char is not — and a signalled child that
    // nobody has waited on is a zombie that is still a member of its group. On
    // Linux `killpg(pgid, 0)` succeeds against exactly that group and only
    // reports `ESRCH` after the `waitpid` (measured; `posix::group_alive`), so
    // reaping after `clean` returns means char watches the whole grace, sends
    // its SIGKILL and still sees the group alive: `FAILED` for a group that in
    // fact died on the first SIGTERM. darwin answers `EPERM` to that same
    // probe, so char reads the group as gone on its first grace poll and
    // reports `CLEAN` without ever escalating — the right answer for the wrong
    // reason, and why reaping late would pass here and fail in CI.
    // Waiting concurrently is what a real orphaned service gets for free — its
    // parent is gone, so init reaps it the moment it dies.
    let reaper = std::thread::spawn(move || {
        group.wait(None, &mut || {});
    });

    let payload = clean(&machine, &repo);
    reaper.join().expect("the reaper thread outlives `clean`");
    assert_eq!(payload["status"], "CLEAN", "{payload}");
    assert_eq!(
        payload["data"]["results"][0]["released"]["processes"],
        Value::from(1),
        "{payload}"
    );

    assert!(!posix::group_alive(pgid), "the group survived `char clean`");
}

/// **A pid from a previous boot is a recycled pid, not an orphaned service.**
/// The row is dropped and no signal is sent — so the process this test starts
/// under that stamp is still running afterwards.
#[test]
fn a_group_stamped_by_another_boot_is_dropped_without_a_signal() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    let workspace = init(&machine, &repo);

    let mut group = detached_sleeper(&repo);
    let pgid = group.pgid();
    let mut row = ours(&workspace, pgid, &repo);
    row.boot_id = Some("a-boot-that-is-not-this-one".to_string());
    record(&machine, &workspace, row);

    let payload = clean(&machine, &repo);
    assert_eq!(
        payload["data"]["results"][0]["released"]["processes"],
        Value::from(0),
        "nothing may be counted as stopped: {payload}"
    );
    assert!(
        posix::group_alive(pgid),
        "char signalled a group it could not prove was its own"
    );

    // The row still goes: char cannot act on it, so remembering it forever
    // would leak a row per reboot.
    assert!(owned_rows(machine.home.path()).is_empty());

    group.stop();
    group.wait(None, &mut || {});
}

/// **A pgid of zero is not a pgid.** `killpg(0, …)` signals the caller's own
/// group, so a `0` in the table would have `char clean` SIGKILL char itself.
#[test]
fn a_zero_or_unparseable_pgid_is_dropped_rather_than_signalled() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    let workspace = init(&machine, &repo);

    for reference in ["0", "-1", "not-a-pgid"] {
        record(
            &machine,
            &workspace,
            OwnedRow {
                workspace: workspace.clone(),
                kind: OwnedKind::Pgid,
                reference: reference.to_string(),
                boot_id: machine::boot_id(&RealRun, &repo),
                pid_started_at: Some("whenever".to_string()),
            },
        );
    }

    // That this process is still alive to run the assertions is half of it.
    let payload = clean(&machine, &repo);
    assert_eq!(
        payload["data"]["results"][0]["released"]["processes"],
        Value::from(0),
        "{payload}"
    );
    assert!(owned_rows(machine.home.path()).is_empty());
    assert!(
        fs::stat(&repo) == armada_core::reap::PathStat::Present,
        "the workspace itself is untouched"
    );
}

fn owned_rows(home: &Path) -> Vec<String> {
    let path: PathBuf = machine::armada_home(home).join("manifest.db");
    let conn = rusqlite::Connection::open(path).unwrap();
    let mut statement = conn.prepare("SELECT \"ref\" FROM owned").unwrap();
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows
}

const CONFIG: &str = "\
manifest:
  version: 1
  components:
    app:
      run:
        driver: command
        cmd: ./serve
        ports: { web: 3000 }
";
