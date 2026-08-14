//! `char check`, end to end: the real binary, real children, real leases.
//!
//! **The tier that catches what the unit tests structurally cannot.** The
//! reducer's suite drives one transition at a time and the replay property
//! drives a *simulated* shell; neither of them spawns anything, takes a lease,
//! or writes a file. What is asserted here is the composition — that the argv
//! the core proposed is the argv a child received, that the verdict reached the
//! envelope, that the exit code followed the class, and that the record on disk
//! describes the run that actually happened.

mod support;

use armada_core::run::RunRecord;
use armada_core::schedule::replay;
use std::path::Path;
use support::Machine;

/// One component, three checks: one that passes, one that fails, one that is
/// file-scoped and therefore skipped on a clean tree.
const CONFIG: &str = "\
manifest:
  version: 1
  components:
    app:
      root: src
      checks:
        lint: { cmd: \"./exiter.sh 0\" }
        pass: { cmd: \"./exiter.sh 0\", scope: component }
        fail: { cmd: \"./exiter.sh 3\", scope: component }
";

fn envelope(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("not JSON: {e}\n{}", String::from_utf8_lossy(&output.stdout)))
}

fn row<'a>(payload: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    payload["data"]["results"]
        .as_array()
        .expect("results[]")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no row for {id} in {payload}"))
}

/// The record a run left behind, read the way `char explain` will.
fn record(repo: &Path, run_id: &str) -> RunRecord {
    let path = repo.join(".armada/run").join(run_id).join("state.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("the record reads back")
}

// ------------------------------------------------------------------ verdicts

/// **One failing check fails the run**, which is what a merge gate needs — and
/// the code follows the *class*, not the state: `FAILED` here is exit 1 because
/// the tool failed on its own terms, which is a real result to report rather
/// than char's fault.
#[test]
fn a_failing_check_fails_the_run_with_the_tools_own_class() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "check", "--json"]);
    let payload = envelope(&output);

    assert_eq!(payload["status"], "FAILED");
    assert_eq!(payload["error"]["class"], "tool_failed");
    assert_eq!(output.status.code(), Some(1));

    assert_eq!(row(&payload, "app:pass")["status"], "PASS");
    assert_eq!(row(&payload, "app:fail")["status"], "FAILED");
    assert_eq!(row(&payload, "app:fail")["error"]["class"], "tool_failed");
}

/// **`check` never reports `PARTIAL`.** "Three of five passed" is not a
/// different action from "none passed" when the action is *fix the failing
/// one*, and `results[]` itemises exactly which.
#[test]
fn a_run_with_some_passes_and_some_failures_is_failed_and_never_partial() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    assert_eq!(payload["status"], "FAILED");
    assert_ne!(payload["status"], "PARTIAL");
}

/// **A file-scoped check with no matching files is `SKIPPED`, and says why** —
/// so an agent that expected it to run can tell "no files matched" from "never
/// selected". It reports no log, because nothing was written to one.
#[test]
fn a_file_scoped_check_on_a_clean_tree_is_skipped_with_a_reason_and_no_log() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "app:lint", "--json"]));
    let lint = row(&payload, "app:lint");
    assert_eq!(lint["status"], "SKIPPED");
    assert_eq!(lint["reason"], "no matching files");
    assert!(lint["log"].is_null(), "a skipped check pointed at a log");

    // Nothing ran, so the run is SKIPPED and exits 0 — not PASS, which would
    // claim approval for work that did not happen.
    assert_eq!(payload["status"], "SKIPPED");
    assert_eq!(payload["error"], serde_json::Value::Null);
}

/// A changed file selects the checks whose `match:` covers it, with `${files}`
/// set to exactly it — the case an agent actually has.
#[test]
fn a_changed_file_makes_its_file_scoped_check_run() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(repo.join("src/a.py"), "x\n").unwrap();

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "app:lint", "--json"]));
    assert_eq!(row(&payload, "app:lint")["status"], "PASS");
}

// ------------------------------------------------------------------ the record

/// **The strongest assertion this phase can make, against a record a real run
/// wrote.** The replay property's own suite drives a simulated shell; this one
/// replays what `char check` actually persisted, which is the thing
/// `char explain` will read.
#[test]
fn a_real_runs_record_replays_to_the_state_it_persisted() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    let run_id = payload["data"]["run_id"].as_str().expect("a run id");
    let record = record(&repo, run_id);

    assert!(
        record.journal.events.len() > 5,
        "too few events to be evidence: {:?}",
        record.journal.events
    );
    assert_eq!(
        replay(record.state.restart(), &record.journal.events),
        record.state,
        "the record does not replay to the state beside it"
    );
}

/// **Written at dispatch, because most of it cannot be recovered.** The argv is
/// the one the child received, and the environment is names only.
#[test]
fn the_dispatch_record_carries_the_argv_that_ran_and_no_environment_value() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "app:fail", "--json"]));
    let run_id = payload["data"]["run_id"].as_str().expect("a run id");
    let record = record(&repo, run_id);

    let dispatch = record
        .journal
        .dispatches
        .get(&armada_core::schedule::CheckId::new("app:fail"))
        .expect("the failing check was dispatched");
    assert_eq!(dispatch.argv, vec!["./exiter.sh", "3"]);
    assert_eq!(dispatch.cwd, repo);

    // The failure signature exists for the check that failed, and only for it.
    let signature = dispatch.signature.as_ref().expect("a failing check signs");
    assert_eq!(signature.exit_code, 3);
    assert_eq!(signature.digest.len(), 64);

    // **A cpu-slot was held, and the store chose which.** Two checks asking for
    // slot `0` is the deadlock that hung the first real run.
    assert!(
        dispatch
            .leases
            .iter()
            .any(|held| held.starts_with("cpu-slot:")),
        "no slot recorded: {:?}",
        dispatch.leases
    );
}

/// **The same failure twice signs identically**, which is what makes the
/// history row worth having: "this failed the same way in the last three runs"
/// and "this passed twenty minutes ago" are opposite problems, and a stack
/// trace is identical in both.
#[test]
fn two_runs_of_one_failure_produce_the_same_signature() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let mut digests = Vec::new();
    for _ in 0..2 {
        let payload = envelope(&machine.run(&repo, &["manifest", "check", "app:fail", "--json"]));
        let run_id = payload["data"]["run_id"].as_str().unwrap().to_string();
        let record = record(&repo, &run_id);
        digests.push(
            record.journal.dispatches[&armada_core::schedule::CheckId::new("app:fail")]
                .signature
                .as_ref()
                .unwrap()
                .digest
                .clone(),
        );
    }
    assert_eq!(digests[0], digests[1], "one failure signed two ways");
}

// ------------------------------------------------------------------- needs:

/// **`needs:` gates here and starts in phase 4** (`PHASES.md` phase 3). The end
/// state is that a check needing `postgres` brings it up; `char up` does not
/// exist yet, so the honest answer names the service and says how to start it.
/// One behaviour built in two steps, not two behaviours.
#[test]
fn a_check_needing_a_service_that_is_not_running_is_refused_by_name() {
    let machine = Machine::new();
    let repo = machine.repo("main", NEEDS_A_SERVICE);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "check", "app:test", "--json"]);
    let payload = envelope(&output);

    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(
        output.status.code(),
        Some(2),
        "not 1 — nothing was examined"
    );

    let test = row(&payload, "app:test");
    assert_eq!(test["status"], "FAILED");
    assert!(
        test["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("postgres"),
        "the service was not named: {payload}"
    );
    assert!(
        test["error"]["next_action"]
            .as_str()
            .unwrap_or_default()
            .contains("char up"),
        "the way out was not named: {payload}"
    );
}

/// **`bad_invocation` outranks a test failure.** A run mixing the two reports
/// the invocation, because the caller has to fix that before any other result
/// means anything — the mixture PLAN.md §3.1 records as having had no defined
/// maximum until `bad_invocation` joined the precedence list.
#[test]
fn a_blocked_check_beside_a_failing_one_reports_the_invocation() {
    let machine = Machine::new();
    let repo = machine.repo("main", NEEDS_A_SERVICE);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "check", "--json"]);
    let payload = envelope(&output);

    assert_eq!(row(&payload, "app:fail")["status"], "FAILED");
    assert_eq!(row(&payload, "app:fail")["error"]["class"], "tool_failed");
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(output.status.code(), Some(2), "not 1");
}

/// A check that needs no service is unaffected — the gate is per check, not per
/// run, so one component's dependency does not stop the rest of the suite.
#[test]
fn a_check_that_needs_nothing_still_runs_beside_a_blocked_one() {
    let machine = Machine::new();
    let repo = machine.repo("main", NEEDS_A_SERVICE);
    machine.run(&repo, &["manifest", "init"]);

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "--json"]));
    assert_eq!(row(&payload, "app:free")["status"], "PASS");
}

const NEEDS_A_SERVICE: &str = "\
manifest:
  version: 1
  components:
    postgres:
      run:
        driver: compose
        file: [docker-compose.yml]
    app:
      checks:
        test:
          cmd: \"./exiter.sh 0\"
          scope: component
          needs: [postgres]
        fail: { cmd: \"./exiter.sh 3\", scope: component }
        free: { cmd: \"./exiter.sh 0\", scope: component }
";

// -------------------------------------------------------------- the run lease

/// **A second run in the same workspace fails fast rather than blocking**
/// (PLAN.md §3.2.1). Blocking by default would mean an agent expecting a quick
/// lint silently waiting out a fifteen-minute test suite with no output.
#[test]
fn a_second_run_in_one_workspace_fails_fast_and_names_the_way_to_queue() {
    let machine = Machine::new();
    let repo = machine.repo("main", SLOW);
    machine.run(&repo, &["manifest", "init"]);

    let mut first = machine.spawn(&repo, &["manifest", "check"]);
    // Wait for the run lease to appear rather than sleeping a guess.
    let db = machine.home.path().join(".armada/manifest.db");
    let start = std::time::Instant::now();
    while !lease_held(&db, "run") && start.elapsed() < std::time::Duration::from_secs(20) {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let second = machine.run(&repo, &["manifest", "check", "--json"]);
    let payload = envelope(&second);
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(second.status.code(), Some(2));
    assert!(
        payload["error"]["next_action"]
            .as_str()
            .unwrap_or_default()
            .contains("--wait"),
        "{payload}"
    );

    let _ = first.wait();
}

/// Five worktrees are five workspaces, and the run lease is keyed per
/// workspace — so they never contend on it. That is the case this project is
/// built around.
#[test]
fn two_worktrees_run_at_the_same_time_without_contending() {
    let machine = Machine::new();
    let repo = machine.repo("main", SLOW);
    let other = machine.worktree(&repo, "wt-1");
    machine.run(&repo, &["manifest", "init"]);
    machine.run(&other, &["manifest", "init"]);

    let mut first = machine.spawn(&repo, &["manifest", "check"]);
    let second = machine.run(&other, &["manifest", "check", "--json"]);
    let payload = envelope(&second);

    assert_ne!(
        payload["error"]["class"], "bad_invocation",
        "a sibling workspace was refused: {payload}"
    );
    let _ = first.wait();
}

const SLOW: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        slow: { cmd: \"sleep 3\", scope: component }
";

fn lease_held(db: &Path, kind: &str) -> bool {
    let Ok(conn) = rusqlite::Connection::open(db) else {
        return false;
    };
    conn.query_row(
        "SELECT count(*) FROM leases WHERE kind = ?1",
        [kind],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

// ---------------------------------------------------------------- selectors

/// An unconventional selector matching nothing is `bad_invocation` that teaches
/// the vocabulary, rather than exit 0 for a check that never ran.
#[test]
fn a_typo_in_a_selector_is_refused_and_lists_what_would_have_worked() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "check", "lnit", "--json"]);
    let payload = envelope(&output);
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(output.status.code(), Some(2));
    assert!(payload["error"]["next_action"]
        .as_str()
        .unwrap_or_default()
        .contains("app:lint"));
}

/// A **conventional** name matching nothing is `SKIPPED` and exit 0, which is
/// what lets an orchestrating agent run `char check lint` across five
/// workspaces without special-casing the three that lack it.
#[test]
fn a_conventional_selector_matching_nothing_exits_zero() {
    let machine = Machine::new();
    let repo = machine.repo(
        "main",
        "manifest:\n  version: 1\n  components:\n    app:\n      checks:\n        audit: { cmd: \"./exiter.sh 0\" }\n",
    );
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "check", "e2e", "--json"]);
    let payload = envelope(&output);
    assert_eq!(payload["status"], "SKIPPED");
    assert_eq!(output.status.code(), Some(0));
}

/// `--dry-run` changes nothing and shows the argv that would run — the same
/// value the dispatch record would carry, from the same code path.
#[test]
fn a_dry_run_shows_the_argv_and_writes_no_run() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let payload = envelope(&machine.run(&repo, &["manifest", "check", "--dry-run", "--json"]));
    let would: Vec<String> = payload["data"]["would_run"]
        .as_array()
        .expect("would_run")
        .iter()
        .map(|line| line.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        would.iter().any(|line| line.contains("./exiter.sh 3")),
        "{would:?}"
    );

    let runs = repo.join(".armada/run");
    let count = std::fs::read_dir(&runs).map(|d| d.count()).unwrap_or(0);
    assert_eq!(count, 0, "a dry run wrote a run directory");
}

/// `--detach` and `--status` are reserved by PLAN.md §3 and not built. Refused
/// **by name**, because the flag is known and the honest answer is that char
/// cannot do it yet — "unknown flag" would send an agent looking for a typo.
#[test]
fn the_two_reserved_flags_say_they_are_not_built_rather_than_unknown() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    for flag in ["--detach", "--status"] {
        let output = machine.run(&repo, &["manifest", "check", flag, "--json"]);
        let payload = envelope(&output);
        assert_eq!(payload["error"]["class"], "bad_invocation", "{flag}");
        assert!(
            payload["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("not built yet"),
            "{flag}: {payload}"
        );
    }
}
