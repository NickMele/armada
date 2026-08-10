//! The real CLI against scratch repos — the phase's done-whens, end to end.
//!
//! Every one of these is a claim from `PHASES.md`'s phase 2, run rather than
//! reasoned about:
//!
//! - two directories claim non-overlapping blocks **concurrently**, and
//!   `char status --project` from either reports both;
//! - **deleting one directory outright**, then running `char init` in a third,
//!   reclaims the deleted one's block — **reported, not silently** — without
//!   disturbing the live one;
//! - `char clean --orphaned` does the same on demand;
//! - a `commands:` entry's subcommands and flags reach the child untouched, its
//!   exit code comes back verbatim, and `env:` layers over the inherited
//!   environment.

mod support;

use serde_json::Value;
use support::{char_binary, Machine};

/// The five-worktrees case §2.1 calls "the case that matters": same committed
/// `char.yml`, several ids, non-overlapping blocks, independent lifecycles.
#[test]
fn two_worktrees_claim_non_overlapping_blocks_concurrently() {
    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let worktree = machine.worktree(&main, "wt1");

    // Started together rather than in sequence: the interesting failure is the
    // one where both read the free-block set before either writes.
    let first = machine.spawn(&main, &["init", "--json"]);
    let second = machine.spawn(&worktree, &["init", "--json"]);
    let first = first.wait_with_output().unwrap();
    let second = second.wait_with_output().unwrap();
    for output in [&first, &second] {
        assert!(
            output.status.success(),
            "exit {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let a: Value = serde_json::from_slice(&first.stdout).unwrap();
    let b: Value = serde_json::from_slice(&second.stdout).unwrap();
    let (a_from, a_to) = block_of(&a);
    let (b_from, b_to) = block_of(&b);
    assert!(
        a_to < b_from || b_to < a_from,
        "blocks overlap: {a_from}-{a_to} and {b_from}-{b_to}"
    );
    assert_ne!(a["workspace"], b["workspace"], "two ids, not one");

    // `--project` is the orchestrating agent's view, and it must be the same
    // answer from either sibling: they share one `--git-common-dir`.
    for cwd in [&main, &worktree] {
        let status = machine.run(cwd, &["status", "--project", "--json"]);
        let payload: Value = serde_json::from_slice(&status.stdout).unwrap();
        let ids: Vec<&str> = payload["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), 2, "--project from {cwd:?} reported {ids:?}");
    }
}

/// **Deleting one directory outright, then running `char init` in a third,
/// automatically reclaims the deleted one's block — reported, not silently —
/// without disturbing the live one.**
#[test]
fn init_in_a_third_workspace_reclaims_a_deleted_ones_block_and_says_so() {
    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let doomed = machine.worktree(&main, "doomed");
    machine.run(&main, &["init"]);
    let doomed_payload: Value =
        serde_json::from_slice(&machine.run(&doomed, &["init", "--json"]).stdout).unwrap();
    let doomed_id = doomed_payload["workspace"].as_str().unwrap().to_string();
    let live_block = block_of(
        &serde_json::from_slice::<Value>(&machine.run(&main, &["status", "--json"]).stdout)
            .unwrap()["data"]["results"][0],
    );

    // `rm -rf`, which is what actually happens to a worktree — and measured,
    // `git worktree remove` succeeds with no complaint while a process is
    // running inside it, so the deletion vector this project is built around is
    // entirely silent.
    std::fs::remove_dir_all(&doomed).unwrap();

    let third = machine.worktree(&main, "third");
    let output = machine.run(&third, &["init", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();

    // Reported, never silent. A tool that removes things without saying so is
    // worse than one that does not remove them.
    let reaped = payload["data"]["reaped"]["workspaces"]
        .as_array()
        .expect("the reap is reported under data.reaped");
    assert!(
        reaped
            .iter()
            .any(|id| id == &Value::from(doomed_id.clone())),
        "the deleted workspace was not reported as reaped: {reaped:?}"
    );

    // The live one is untouched — flat siblings, no cascade.
    let after = machine.run(&main, &["status", "--json"]);
    let after: Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(block_of(&after["data"]["results"][0]), live_block);

    // And the block is genuinely free again: `--all` no longer knows the
    // deleted workspace.
    let all: Value =
        serde_json::from_slice(&machine.run(&main, &["status", "--all", "--json"]).stdout).unwrap();
    let ids: Vec<&str> = all["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&doomed_id.as_str()), "{ids:?}");
}

/// `char clean --orphaned` does the same **on demand**, and from outside any
/// workspace — which is the state it is most needed in.
#[test]
fn clean_orphaned_reclaims_a_deleted_workspace_from_anywhere() {
    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let doomed = machine.worktree(&main, "doomed");
    machine.run(&main, &["init"]);
    let doomed_id = workspace_id(&machine.run(&doomed, &["init", "--json"]));

    std::fs::remove_dir_all(&doomed).unwrap();

    // From a directory that is not a workspace at all.
    let outside = machine.outside();
    let output = machine.run(&outside, &["clean", "--all", "--orphaned", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let all: Value =
        serde_json::from_slice(&machine.run(&main, &["status", "--all", "--json"]).stdout).unwrap();
    let ids: Vec<&str> = all["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&doomed_id.as_str()), "{ids:?}");
    assert_eq!(
        ids.len(),
        1,
        "--orphaned must never disturb a live workspace"
    );
}

/// **Subcommands and flags reach the child untouched, and the child's exit code
/// comes back verbatim and unremapped.**
///
/// Note `--dry-run` in this invocation: char defines a flag by that name, and
/// here it is the child's.
#[test]
fn a_dispatched_command_receives_its_argv_untouched_and_returns_its_own_code() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    machine.run(&repo, &["init"]);

    let output = machine.run(&repo, &["echoer", "prune", "--dry-run", "--", "-x"]);
    let seen = String::from_utf8_lossy(&output.stdout);
    assert_eq!(seen.trim(), "prune --dry-run -- -x");

    // Verbatim: 3 is char's own `bad_config`, and this is the child's 3.
    let output = machine.run(&repo, &["exiter", "3"]);
    assert_eq!(output.status.code(), Some(3));

    // `data.dispatched` is what makes that unambiguous.
    let output = machine.run(&repo, &["--json", "exiter", "3"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["data"]["dispatched"], Value::Bool(true));
    assert_eq!(payload["data"]["child_exit"], Value::from(3));
    assert_eq!(payload["error"], Value::Null, "char did not decide this");
}

/// `env:` is **additive** — the parent environment is inherited wholesale and
/// these are layered on top, so a command needing `$HOME` already has it. And
/// `CHAR_WORKSPACE` is present without being declared anywhere.
#[test]
fn a_dispatched_command_gets_a_layered_environment_and_the_workspace_id() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    let id = workspace_id(&machine.run(&repo, &["init", "--json"]));

    let output = machine.run(&repo, &["enver"]);
    let seen = String::from_utf8_lossy(&output.stdout);
    let mut lines = seen.lines();
    assert_eq!(lines.next().unwrap(), format!("declared={id}"));
    assert_eq!(lines.next().unwrap(), format!("workspace={id}"));
    assert!(
        lines.next().unwrap().starts_with("home=/"),
        "the inherited environment must survive the layering"
    );
}

/// A command char cannot start never ran, so this is char's failure to report
/// and char's code to exit with — `bad_config`, because the repo's statement is
/// what is wrong.
#[test]
fn a_command_that_cannot_start_is_chars_own_failure_and_says_so() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    machine.run(&repo, &["init"]);

    let output = machine.run(&repo, &["--json", "missing"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["data"]["dispatched"], Value::Bool(false));
    assert_eq!(payload["error"]["class"], "bad_config");
    assert_eq!(output.status.code(), Some(3));
    assert!(payload["error"]["next_action"].is_string());
}

/// A second run in the same workspace **fails fast** rather than blocking, and
/// names the holder — because waiting on the run lease means waiting for an
/// entire other run, and the caller almost always wants to know rather than to
/// wait.
#[test]
fn a_second_mutating_verb_in_one_workspace_fails_fast_naming_the_holder() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    machine.run(&repo, &["init"]);

    let mut holder = machine.spawn(&repo, &["sleeper"]);
    // Wait until the lease row actually exists rather than guessing.
    let db = machine.home.path().join(".char/char.db");
    for _ in 0..200 {
        if lease_count(&db) > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let blocked = machine.run(&repo, &["init", "--json"]);
    let payload: Value = serde_json::from_slice(&blocked.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(blocked.status.code(), Some(2));
    assert!(
        payload["error"]["next_action"]
            .as_str()
            .unwrap()
            .contains("--wait"),
        "the remedy has to be in the error"
    );

    let _ = holder.kill();
    let _ = holder.wait();
}

/// `--json` is answered on **every** failure path, including one that fails
/// before a verb exists to answer it.
///
/// A machine caller probing the six not-built-yet verbs must read the same
/// envelope it reads everywhere else — `schema_version` and `error.class` — and
/// not human text on stderr. The case below it, which fails later during
/// workspace resolution, already worked; the two are here together because the
/// gap was invisible from the inside precisely because only one was covered.
#[test]
fn a_parse_time_failure_answers_in_the_envelope_when_json_was_asked_for() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);

    let output = machine.run(&repo, &["--json", "check"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: stdout {:?}, stderr {:?}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(payload["schema_version"], Value::from(1));
    assert_eq!(payload["status"], "FAILED");
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(payload["workspace"], Value::Null, "resolution never ran");
    assert_eq!(output.status.code(), Some(2));

    // The case that already worked: parses fine, fails afterwards.
    let outside = machine.outside();
    let output = machine.run(&outside, &["--json", "bogusverb"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_config");
}

fn lease_count(db: &std::path::Path) -> i64 {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row("SELECT count(*) FROM leases", [], |row| row.get(0))
        .unwrap_or(0)
}

fn block_of(payload: &Value) -> (u64, u64) {
    let block = if payload["data"]["port_block"].is_object() {
        &payload["data"]["port_block"]
    } else {
        &payload["port_block"]
    };
    (
        block["from"].as_u64().expect("a from"),
        block["to"].as_u64().expect("a to"),
    )
}

fn workspace_id(output: &std::process::Output) -> String {
    let payload: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&output.stdout)));
    payload["workspace"].as_str().unwrap().to_string()
}

const CONFIG: &str = "\
version: 1
components:
  app:
    run:
      driver: command
      cmd: ./serve
      ports: { web: 3000 }
";

const DISPATCH_CONFIG: &str = "\
version: 1
commands:
  echoer:
    cmd: echo
  exiter:
    cmd: ./exiter.sh
  enver:
    cmd: ./enver.sh
    env:
      DECLARED: ${workspace.id}
  sleeper:
    cmd: sleep 60
  missing:
    cmd: ./definitely-not-here
";

#[test]
fn the_binary_under_test_is_the_one_this_workspace_built() {
    assert!(char_binary().exists());
}
