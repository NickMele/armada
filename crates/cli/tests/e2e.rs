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
//!
//! The second half of the file is the rest of the shipped surface, which the
//! done-whens do not reach and which nothing else covers:
//!
//! - **the previews.** `--dry-run` is the safety mechanism for `clean
//!   --artifacts`, so a preview listing less than the real pass deletes is worse
//!   than no preview at all — it reads as a complete answer and is not one. Only
//!   running both against one repo establishes that they agree.
//! - **`init`'s two failure paths.** A `setup:` step that exits non-zero and one
//!   that cannot be started are deliberately *different classes* — `tool_failed`
//!   against `bad_config`, the tool's failure against the repo's statement being
//!   wrong — and nothing else asserts the distinction survives to the envelope.
//! - **the human renderer's two exits.** An answer goes to stdout and a failure
//!   to stderr, so `char status | grep` is never quietly fed an error report.

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

// ---------------------------------------------------------------------------
// The surface the done-whens do not reach.
// ---------------------------------------------------------------------------

/// `--version` and `--help` answer before any workspace exists, which is the
/// state someone reaching for `--help` is usually in.
#[test]
fn version_and_help_answer_from_outside_a_workspace() {
    let machine = Machine::new();
    let outside = machine.outside();

    let version = machine.run(&outside, &["--version"]);
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout).starts_with("char "),
        "{:?}",
        String::from_utf8_lossy(&version.stdout)
    );

    let help = machine.run(&outside, &["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("char init"), "{text}");
    // The limits are stated in the usage rather than discovered by running one.
    assert!(text.contains("Not built yet"), "{text}");
}

/// `init --dry-run` decides everything and changes nothing: no block is
/// claimed, no `.char/` appears, and the workspace is still unknown to the
/// store afterwards.
#[test]
fn init_dry_run_previews_the_claim_and_claims_nothing() {
    let machine = Machine::new();
    let repo = machine.repo("main", SETUP_CONFIG);

    let output = machine.run(&repo, &["init", "--dry-run", "--json"]);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let block = &payload["data"]["would_claim"];
    assert!(block["from"].is_u64(), "{payload}");
    assert_eq!(
        payload["data"]["would_run"],
        Value::from(vec!["api: true"]),
        "the setup steps are previewed in order"
    );

    assert!(!repo.join(".char").exists(), ".char/ must not be created");
    let all: Value = serde_json::from_slice(
        &machine
            .run(&machine.outside(), &["status", "--all", "--json"])
            .stdout,
    )
    .unwrap();
    assert_eq!(
        all["data"]["results"].as_array().unwrap().len(),
        0,
        "a preview must leave the store empty: {all}"
    );

    // And the human rendering says, first, that nothing happened.
    let human = machine.run(&repo, &["init", "--dry-run"]);
    assert!(String::from_utf8_lossy(&human.stdout).starts_with("dry run"));
}

/// A `setup:` step that runs and fails is the **tool's** failure: char started
/// it, so the repo's statement was right and the command was not.
#[test]
fn a_setup_step_that_exits_non_zero_fails_the_row_and_the_verb() {
    let machine = Machine::new();
    let repo = machine.repo("main", FAILING_SETUP_CONFIG);

    let output = machine.run(&repo, &["init", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["status"], "FAILED");
    assert_eq!(payload["error"]["class"], "tool_failed");
    assert_eq!(payload["error"]["where"], "api");
    assert_eq!(payload["data"]["results"][0]["status"], "FAILED");
    assert!(
        payload["data"]["results"][0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("exited 1"),
        "the child's code is named: {payload}"
    );
    assert_eq!(output.status.code(), Some(1), "tool_failed is 1");

    // The block was still claimed: the failure is the repo's command, and
    // refusing to remember the workspace would strand the block.
    let status: Value =
        serde_json::from_slice(&machine.run(&repo, &["status", "--json"]).stdout).unwrap();
    assert_eq!(status["data"]["results"].as_array().unwrap().len(), 1);
}

/// A `setup:` step that **cannot be started** is the repo's statement being
/// wrong rather than the machine's, so it is `bad_config` and names the
/// program — and the human rendering of that goes to stderr, not stdout.
#[test]
fn a_setup_step_that_cannot_start_is_bad_config_and_prints_to_stderr() {
    let machine = Machine::new();
    let repo = machine.repo("main", UNSTARTABLE_SETUP_CONFIG);

    let output = machine.run(&repo, &["init", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let error = &payload["data"]["results"][0]["error"];
    assert_eq!(error["class"], "bad_config");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("definitely-not-here"),
        "{payload}"
    );
    assert!(error["next_action"].is_string());

    // **The aggregate carries the row's class up rather than flattening it.**
    // An earlier version of this assertion read `tool_failed` "whatever the row
    // said", which pinned a bug: `init` picked the first failed row and
    // hardcoded the class, so a config the caller must edit reported exit 1 —
    // whose documented response is "that is a real result, report it" — instead
    // of exit 3. The precedence rule exists so two implementations cannot
    // disagree, and `init` was the second implementation.
    assert_eq!(payload["error"]["class"], "bad_config");
    assert_eq!(output.status.code(), Some(3), "bad_config is 3, not 1");
    assert!(
        payload["error"]["next_action"].is_string(),
        "next_action is required for bad_config: {payload}"
    );

    // Human: a failed verb's report belongs on stderr, so a pipeline reading
    // stdout is never handed an error report as if it were an answer.
    let human = machine.run(&repo, &["init"]);
    assert!(!human.status.success());
    assert!(human.stdout.is_empty(), "nothing goes to stdout on failure");
    let text = String::from_utf8_lossy(&human.stderr);
    assert!(text.contains("could not be started"), "stderr: {text:?}");
    assert!(text.contains("class: bad_config"), "stderr: {text:?}");
}

/// `clean --dry-run --artifacts` previews **exactly** what the real pass then
/// releases: the block, the declared artifact, and the external command char
/// records and will never run.
#[test]
fn clean_previews_what_it_would_release_and_releases_none_of_it() {
    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    machine.run(&repo, &["init"]);
    std::fs::write(repo.join("node_modules"), "artifact").unwrap();

    let output = machine.run(&repo, &["clean", "--dry-run", "--artifacts", "--json"]);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let data = &payload["data"];
    assert_eq!(data["would_release"].as_array().unwrap().len(), 1);
    assert_eq!(data["would_delete"], Value::from(vec!["node_modules"]));
    assert!(
        data["would_report"][0]
            .as_str()
            .unwrap()
            .starts_with("psql -c"),
        "the recorded release command is reported, never run: {payload}"
    );

    // Nothing moved: the artifact, the row and `.char/` are all still there.
    assert!(repo.join("node_modules").exists());
    assert!(repo.join(".char").exists());
    let status: Value =
        serde_json::from_slice(&machine.run(&repo, &["status", "--json"]).stdout).unwrap();
    assert_eq!(status["data"]["results"].as_array().unwrap().len(), 1);
}

/// The real pass, against the same repo: the preview's list is what goes.
#[test]
fn clean_artifacts_deletes_the_declared_files_and_reports_the_external_command() {
    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    machine.run(&repo, &["init"]);
    std::fs::write(repo.join("node_modules"), "artifact").unwrap();

    let output = machine.run(&repo, &["clean", "--artifacts", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["status"], "CLEAN");
    let row = &payload["data"]["results"][0];
    assert_eq!(row["released"]["files"], Value::from(1));
    assert_eq!(row["released"]["port_block"], Value::Bool(true));
    assert!(
        payload["data"]["unreclaimed"][0]["command"]
            .as_str()
            .unwrap()
            .starts_with("psql -c"),
        "{payload}"
    );

    assert!(!repo.join("node_modules").exists(), "the artifact went");
    assert!(!repo.join(".char").exists(), ".char/ went with it");
    let status: Value =
        serde_json::from_slice(&machine.run(&repo, &["status", "--json"]).stdout).unwrap();
    assert_eq!(status["data"]["results"].as_array().unwrap().len(), 0);
}

/// The human renderer on the ordinary success path: `status` prints its scope,
/// the workspace's block and what it holds, to stdout.
#[test]
fn status_renders_for_a_terminal_on_stdout() {
    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    machine.run(&repo, &["init"]);

    let output = machine.run(&repo, &["status"]);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("scope workspace\n"), "{text}");
    assert!(text.contains("  ports 54"), "{text}");
    assert!(
        text.contains("RESERVED") || text.contains("CONFLICT"),
        "a port's probed state is spelled as the envelope spells it: {text}"
    );
}

const SETUP_CONFIG: &str = "\
version: 1
components:
  api:
    setup: [\"true\"]
    run:
      driver: command
      cmd: ./serve
      ports: { web: 3000 }
";

const FAILING_SETUP_CONFIG: &str = "\
version: 1
components:
  api:
    setup: [\"false\"]
";

const UNSTARTABLE_SETUP_CONFIG: &str = "\
version: 1
components:
  api:
    setup: [\"./definitely-not-here\"]
";

const OWNS_CONFIG: &str = "\
version: 1
components:
  api:
    run:
      driver: command
      cmd: ./serve
      ports: { web: 3000 }
    owns:
      files: [node_modules]
      release: \"psql -c 'DROP DATABASE app_${workspace.id}'\"
";

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

/// **`char clean --orphaned --force-rebuild` is the way out of a `char.db` char
/// cannot read**, and the property under test is that *the recovery path does
/// not need the thing that is broken*: every other verb fails against this
/// database, and this one does not.
#[test]
fn force_rebuild_recovers_a_database_no_other_verb_can_open() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["init"]);

    let db = machine.home.path().join(".char/char.db");
    let namespace_before = namespace_of(&db);
    std::fs::write(&db, b"this is not a database").unwrap();

    // The premise: an ordinary verb cannot get past opening it, and says so as
    // `environment` — the machine is broken, the repo is fine.
    let broken = machine.run(&repo, &["status", "--json"]);
    let payload: Value = serde_json::from_slice(&broken.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "environment");
    assert_eq!(broken.status.code(), Some(6));

    // `--orphaned` is required, because it is what bounds the removal to
    // workspaces whose directory is gone.
    let unbounded = machine.run(&repo, &["clean", "--all", "--force-rebuild", "--json"]);
    let payload: Value = serde_json::from_slice(&unbounded.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_invocation");

    // The invocation `PLAN.md` §4.3 spells, run from inside a workspace and
    // without `--all`: the corpus documents this exact form, so this exact form
    // is what has to work.
    let rebuilt = machine.run(&repo, &["clean", "--orphaned", "--force-rebuild", "--json"]);
    assert!(
        rebuilt.status.success(),
        "{}",
        String::from_utf8_lossy(&rebuilt.stderr)
    );
    let payload: Value = serde_json::from_slice(&rebuilt.stdout).unwrap();
    assert_eq!(payload["status"], "CLEAN");

    // **Moved aside, not deleted.** A recovery that destroys the evidence of
    // what it recovered from cannot be diagnosed afterwards.
    let reported = payload["data"]["reaped"]["skipped"].to_string();
    assert!(reported.contains("moved aside"), "{payload}");

    // Accepting the workspace-scoped-looking form means the report, not the
    // command line, is what tells the caller how far the pass reached.
    assert!(
        reported.contains("machine-scoped") && reported.contains("across namespaces"),
        "the run must state its own scope: {payload}"
    );
    let kept: Vec<_> = std::fs::read_dir(machine.home.path().join(".char"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("unreadable"))
        .collect();
    assert_eq!(
        kept.len(),
        1,
        "the unreadable file should be kept: {kept:?}"
    );

    // And the store works again.
    assert!(machine.run(&repo, &["status", "--all"]).status.success());
    let namespace_after = namespace_of(&db);
    assert_ne!(namespace_after, "", "a fresh database has a namespace");
    assert_ne!(
        namespace_after, namespace_before,
        "a database overwritten with junk cannot yield its old namespace, so the \
         replacement takes a new one and says so"
    );
}

/// **A preview flag may not delete data**, and this is the operation it would
/// matter most for: `--force-rebuild` removes labelled resources across
/// namespaces and replaces the machine-global store, so `--dry-run` has to
/// leave both exactly as it found them.
#[test]
fn force_rebuild_under_dry_run_changes_nothing_on_disk() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["init"]);

    let db = machine.home.path().join(".char/char.db");
    let junk = b"this is not a database";
    std::fs::write(&db, junk).unwrap();

    let previewed = machine.run(
        &repo,
        &[
            "clean",
            "--dry-run",
            "--all",
            "--orphaned",
            "--force-rebuild",
            "--json",
        ],
    );
    assert!(
        previewed.status.success(),
        "{}",
        String::from_utf8_lossy(&previewed.stderr)
    );
    let payload: Value = serde_json::from_slice(&previewed.stdout).unwrap();
    assert_eq!(payload["status"], "CLEAN");
    let would_release = payload["data"]["would_release"].to_string();
    assert!(
        would_release.contains("move aside") && would_release.contains("char.db"),
        "the preview must name the file it would move aside: {payload}"
    );
    // The half a caller can misread: "moved aside" alone reads as "the store is
    // otherwise preserved", and the opposite is true.
    assert!(
        would_release.contains("create a fresh")
            && would_release.contains("port block")
            && would_release.contains("new namespace"),
        "the preview must state that a fresh database replaces it, and what that costs: \
         {payload}"
    );
    assert!(
        payload["data"]["results"].is_null(),
        "a dry run answers with would_*, not results[]: {payload}"
    );

    // The unreadable file is still the unreadable file: not moved aside, and
    // not replaced by a fresh database.
    assert_eq!(
        std::fs::read(&db).unwrap(),
        junk,
        "the database was touched by a preview"
    );
    let aside: Vec<_> = std::fs::read_dir(machine.home.path().join(".char"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("unreadable"))
        .collect();
    assert!(
        aside.is_empty(),
        "a preview moved the database aside: {aside:?}"
    );

    // And the recovery is still needed, which is the same statement from the
    // other side: nothing was repaired.
    let after: Value =
        serde_json::from_slice(&machine.run(&repo, &["status", "--json"]).stdout).unwrap();
    assert_eq!(after["error"]["class"], "environment");
}

/// `--artifacts` and `--force` mean nothing on a path that reads no `char.yml`
/// and takes no lease, so they are refused rather than quietly dropped —
/// a flag that is silently ignored is indistinguishable from one that worked.
///
/// `--all` is *not* among them: `PLAN.md` §4.3 spells the recovery without it,
/// so it is accepted with or without, and the run states its own machine scope
/// in its output instead.
#[test]
fn force_rebuild_refuses_every_flag_that_has_no_meaning_on_it() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["init"]);

    for args in [
        &[
            "clean",
            "--all",
            "--orphaned",
            "--force-rebuild",
            "--artifacts",
            "--json",
        ][..],
        &[
            "clean",
            "--all",
            "--orphaned",
            "--force-rebuild",
            "--force",
            "--json",
        ][..],
    ] {
        let refused = machine.run(&repo, args);
        let payload: Value = serde_json::from_slice(&refused.stdout).unwrap();
        assert_eq!(
            payload["error"]["class"],
            "bad_invocation",
            "`char {}` was accepted: {payload}",
            args.join(" ")
        );
        assert_eq!(refused.status.code(), Some(2));
    }
}

fn namespace_of(db: &std::path::Path) -> String {
    rusqlite::Connection::open(db)
        .ok()
        .and_then(|conn| {
            conn.query_row(
                "SELECT value FROM meta WHERE key = 'namespace'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
        .unwrap_or_default()
}
