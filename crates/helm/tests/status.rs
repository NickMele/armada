//! `armada manifest status` against the two things it could not see.
//!
//! **Both defects were measured on a real machine before they were written
//! down**, and both are shaped so that the old code passes every existing test
//! while answering the verb's own first line — *"what is running, what is mine,
//! what is stale"* — with nothing at all.
//!
//! | Measured | What the old verb reported |
//! |---|---|
//! | `workspaces` 0 rows, `owned` 6 rows for one workspace, four from a previous boot | `"results": []` |
//! | a detached `check` executing, its group in `detached.json` | nothing; only `check --status <id>` could see it |
//!
//! **Every child is harmless and none of them costs a token.** `sleep` is the
//! whole of the slow check, the same stand-in `detach.rs` and
//! `owned_processes.rs` already use: it spends nothing, opens no session, and is
//! the only honest way to hold a run open long enough to ask about it.
//!
//! **Every group started here is stopped here.** A detached run outlives the
//! invocation that started it by design, which makes it the one thing in this
//! suite that can leak past the test that made it.

mod support;

use std::path::Path;
use std::time::{Duration, Instant};
use support::Machine;

/// One check that takes long enough to be caught in the act. Three seconds is
/// the same bound `detach.rs` chose against the same poll, for the same reason.
const SLOW: &str = "\
manifest:
  version: 1
  components:
    app:
      root: src
      checks:
        slow: { cmd: \"sleep 3\", scope: component }
";

/// A repository that declares no ports, which is the shape that produces the
/// measured defect: nothing here needs `armada manifest init`, so nothing writes
/// a `workspaces` row, while `check --detach` writes an `owned` one regardless.
const NO_PORTS: &str = "\
manifest:
  version: 1
  components:
    app:
      root: src
      checks:
        quick: { cmd: \"./exiter.sh 0\", scope: component }
";

fn envelope(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Stop whatever a detached run left behind. `ESRCH` on a group that has
/// already gone is a success for this purpose.
fn stop(pgid: i64) {
    if pgid > 1 {
        let _ = std::process::Command::new("kill")
            .args(["-9", &format!("-{pgid}")])
            .output();
    }
}

/// `status`, as JSON, from inside a repository.
fn status(machine: &Machine, repo: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["manifest", "status", "--json"];
    args.extend_from_slice(extra);
    envelope(&machine.run(repo, &args))
}

/// The single result row, which every case here has exactly one of.
fn only_row(payload: &serde_json::Value) -> &serde_json::Value {
    let rows = payload["data"]["results"]
        .as_array()
        .unwrap_or_else(|| panic!("results[] missing from {payload}"));
    assert_eq!(rows.len(), 1, "expected one workspace row, got {payload}");
    &rows[0]
}

fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| row.as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

// ----------------------------------------------------------------- defect 1

/// **A workspace that owns something is a workspace, whether or not it ever
/// claimed a port block.**
///
/// The measured case, reduced: a repository that declares no `ports:` and never
/// runs `armada manifest init`, so `claim_block` is never called and
/// `workspaces` stays empty — while `check --detach` records a `pgid` against
/// the derived workspace id. Before the fix `status` enumerated the registry,
/// asked each of its zero rows what it owned, and reported `"results": []` with
/// exit `0`: the one verb whose job is finding stale resources, blind to the
/// leak.
///
/// Asserted through `--all`, because that lens took `rows.clone()` of the
/// registry and so could not be rescued by the invoking workspace being added
/// back — it is the invocation that proves the enumeration changed rather than
/// the special case.
#[test]
fn a_workspace_with_owned_rows_and_no_registry_row_is_still_reported() {
    let machine = Machine::new();
    let repo = machine.repo("noports", NO_PORTS);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = started["data"]["detached"]["pgid"].as_i64().unwrap_or(0);

    let all = status(&machine, &repo, &["--all"]);
    stop(pgid);

    assert_eq!(all["status"], "OK");
    assert_eq!(
        all["error"],
        serde_json::Value::Null,
        "status reports; it does not judge"
    );
    let row = only_row(&all);
    // The `pgid` `--detach` recorded, named by the verb that is supposed to
    // report what will be reclaimed.
    assert!(
        strings(&row["owns"])
            .iter()
            .any(|id| id == &format!("pgid:{pgid}")),
        "the recorded group is missing from owns[]: {all}"
    );
}

/// **The workspace you are standing in always gets a row**, even when the store
/// holds nothing at all for it.
///
/// `owns  resources  —` says Armada looked and found nothing; no row says
/// nothing whatsoever, and a caller cannot tell those apart. Before the fix an
/// uninitialised repository answered the second, which is how a repository that
/// had never leaked anything and one that had looked identical.
#[test]
fn an_uninitialised_workspace_answers_about_itself() {
    let machine = Machine::new();
    let repo = machine.repo("fresh", NO_PORTS);

    let here = status(&machine, &repo, &[]);
    assert_eq!(here["status"], "OK");
    let row = only_row(&here);
    assert_eq!(row["status"], "OK");
    assert_eq!(
        row["path"].as_str().map(Path::new),
        Some(repo.as_path()),
        "the row must name the directory the caller is standing in: {here}"
    );
}

/// **A recorded group from another boot is `stale[]`, not `owns[]` alone.**
///
/// The four rows on the measured machine that carried a previous boot's id were
/// dead by definition and printed exactly like the two that were not. The rule
/// is `pgid_is_ours`, the same one `clean` kills on, so nothing lands in
/// `stale[]` that `clean` would decline to reclaim — which is what makes the
/// field an instruction rather than a worry.
///
/// The row is written straight into the store because a previous boot is not
/// something a test can arrange any other way, and this is the same reasoning
/// `golden.rs` records for writing `owned` rows by hand.
#[test]
fn a_group_from_a_previous_boot_is_reported_stale() {
    let machine = Machine::new();
    let repo = machine.repo("leaky", NO_PORTS);
    // A run first, so the store exists and the workspace is known through it.
    machine.run(&repo, &["manifest", "check", "--json"]);

    let workspace = status(&machine, &repo, &[])["workspace"]
        .as_str()
        .expect("a workspace id")
        .to_string();
    let db = rusqlite::Connection::open(machine.home.path().join(".armada").join("manifest.db"))
        .expect("the store");
    db.execute(
        "INSERT OR REPLACE INTO owned (workspace, kind, \"ref\", boot_id, pid_started_at)
         VALUES (?1, 'pgid', '61477', 'a-boot-that-has-ended', 'whenever')",
        [&workspace],
    )
    .expect("a leaked row");
    drop(db);

    let row = only_row(&status(&machine, &repo, &[])).clone();
    assert_eq!(
        strings(&row["stale"]),
        vec!["pgid:61477".to_string()],
        "a group from a boot that has ended is provably gone: {row}"
    );
    assert!(
        strings(&row["owns"]).contains(&"pgid:61477".to_string()),
        "stale[] is a subset of owns[], never a replacement for it: {row}"
    );
}

// ----------------------------------------------------------------- defect 2

/// **A detached run is visible to `status` while it is running.**
///
/// The design ask, in the repository owner's words: *"`arm manifest status`
/// should show running checks and anything that is 'up'."* Before this, a
/// `--detach` returned immediately and the only way to see it was `check
/// --status` with the run id in hand — a question only someone who already had
/// the answer could ask.
///
/// `RUNNING` is the envelope's own word for it, reached by the same two
/// questions `check --status` asks in the same order: the record holds no
/// verdict, and the recorded group is provably still this run.
#[test]
fn a_running_detached_check_shows_up_in_status() {
    let machine = Machine::new();
    let repo = machine.repo("slow", SLOW);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = started["data"]["detached"]["pgid"]
        .as_i64()
        .expect("a pgid");
    let run = started["data"]["run_id"]
        .as_str()
        .expect("a run id")
        .to_string();

    let seen = status(&machine, &repo, &[]);
    stop(pgid);

    let row = only_row(&seen);
    let runs = row["runs"].as_array().cloned().unwrap_or_default();
    assert_eq!(runs.len(), 1, "expected exactly the live run: {seen}");
    assert_eq!(runs[0]["run_id"], serde_json::json!(run));
    assert_eq!(runs[0]["status"], "RUNNING", "{seen}");
    assert_eq!(runs[0]["pgid"], serde_json::json!(pgid));
    assert!(
        runs[0]["log"]
            .as_str()
            .is_some_and(|log| log.starts_with(".armada/run/")),
        "the log reference must be workspace-relative: {seen}"
    );
    // The exit code describes the query and never the thing queried: a run in
    // flight is not a failure of the question.
    assert_eq!(seen["status"], "OK");
}

/// **A finished run is history and is left out.**
///
/// `runs[]` answers *what is running*, and a workspace that keeps a retention
/// count of decided runs would otherwise report the same finished run on every
/// poll forever. `check --status <id>` is the verb that reads a verdict back,
/// and it names the run the caller is asking about.
#[test]
fn a_run_that_reached_a_verdict_is_not_reported_as_status() {
    let machine = Machine::new();
    let repo = machine.repo("done", NO_PORTS);

    let started = envelope(&machine.run(&repo, &["manifest", "check", "--detach", "--json"]));
    let pgid = started["data"]["detached"]["pgid"].as_i64().unwrap_or(0);
    let run = started["data"]["run_id"].as_str().expect("a run id");

    // Wait for the run to decide, asking the verb that answers about one run.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = envelope(&machine.run(&repo, &["manifest", "check", "--status", run, "--json"]));
        if last["status"] != "RUNNING" {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    stop(pgid);
    assert_ne!(last["status"], "RUNNING", "the run never decided: {last}");

    let row = only_row(&status(&machine, &repo, &[])).clone();
    assert_eq!(
        row["runs"].as_array().map(Vec::len).unwrap_or(0),
        0,
        "a decided run is not status: {row}"
    );
}
