//! The real CLI against scratch repos — the phase's done-whens, end to end.
//!
//! Every one of these is a claim from `PHASES.md`'s phase 2, run rather than
//! reasoned about:
//!
//! - two directories claim non-overlapping blocks **concurrently**, and
//!   `armada manifest status --project` from either reports both;
//! - **deleting one directory outright**, then running `armada manifest init` in a third,
//!   reclaims the deleted one's block — **reported, not silently** — without
//!   disturbing the live one;
//! - `armada manifest clean --orphaned` does the same on demand;
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
//!   to stderr, so `armada manifest status | grep` is never quietly fed an error report.

mod support;

use serde_json::Value;
use support::{armada_binary, Machine};

/// The five-worktrees case §2.1 calls "the case that matters": same committed
/// `armada.yml`, several ids, non-overlapping blocks, independent lifecycles.
#[test]
fn two_worktrees_claim_non_overlapping_blocks_concurrently() {
    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let worktree = machine.worktree(&main, "wt1");

    // Started together rather than in sequence: the interesting failure is the
    // one where both read the free-block set before either writes.
    let first = machine.spawn(&main, &["manifest", "init", "--json"]);
    let second = machine.spawn(&worktree, &["manifest", "init", "--json"]);
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
        let status = machine.run(cwd, &["manifest", "status", "--project", "--json"]);
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

/// **Deleting one directory outright, then running `armada manifest init` in a third,
/// automatically reclaims the deleted one's block — reported, not silently —
/// without disturbing the live one.**
#[test]
fn init_in_a_third_workspace_reclaims_a_deleted_ones_block_and_says_so() {
    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let doomed = machine.worktree(&main, "doomed");
    machine.run(&main, &["manifest", "init"]);
    let doomed_payload: Value =
        serde_json::from_slice(&machine.run(&doomed, &["manifest", "init", "--json"]).stdout)
            .unwrap();
    let doomed_id = doomed_payload["workspace"].as_str().unwrap().to_string();
    let live_block = block_of(
        &serde_json::from_slice::<Value>(
            &machine.run(&main, &["manifest", "status", "--json"]).stdout,
        )
        .unwrap()["data"]["results"][0],
    );

    // `rm -rf`, which is what actually happens to a worktree — and measured,
    // `git worktree remove` succeeds with no complaint while a process is
    // running inside it, so the deletion vector this project is built around is
    // entirely silent.
    std::fs::remove_dir_all(&doomed).unwrap();

    let third = machine.worktree(&main, "third");
    let output = machine.run(&third, &["manifest", "init", "--json"]);
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
    let after = machine.run(&main, &["manifest", "status", "--json"]);
    let after: Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(block_of(&after["data"]["results"][0]), live_block);

    // And the block is genuinely free again: `--all` no longer knows the
    // deleted workspace.
    let all: Value = serde_json::from_slice(
        &machine
            .run(&main, &["manifest", "status", "--all", "--json"])
            .stdout,
    )
    .unwrap();
    let ids: Vec<&str> = all["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&doomed_id.as_str()), "{ids:?}");
}

/// `armada manifest clean --orphaned` does the same **on demand**, and from outside any
/// workspace — which is the state it is most needed in.
#[test]
fn clean_orphaned_reclaims_a_deleted_workspace_from_anywhere() {
    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let doomed = machine.worktree(&main, "doomed");
    machine.run(&main, &["manifest", "init"]);
    let doomed_id = workspace_id(&machine.run(&doomed, &["manifest", "init", "--json"]));

    std::fs::remove_dir_all(&doomed).unwrap();

    // From a directory that is not a workspace at all.
    let outside = machine.outside();
    let output = machine.run(
        &outside,
        &["manifest", "clean", "--all", "--orphaned", "--json"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let all: Value = serde_json::from_slice(
        &machine
            .run(&main, &["manifest", "status", "--all", "--json"])
            .stdout,
    )
    .unwrap();
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
/// Note `--dry-run` in this invocation: Armada defines a flag by that name, and
/// here it is the child's.
#[test]
fn a_dispatched_command_receives_its_argv_untouched_and_returns_its_own_code() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(
        &repo,
        &["manifest", "echoer", "prune", "--dry-run", "--", "-x"],
    );
    let seen = String::from_utf8_lossy(&output.stdout);
    assert_eq!(seen.trim(), "prune --dry-run -- -x");

    // Verbatim: 3 is Armada's own `bad_config`, and this is the child's 3.
    let output = machine.run(&repo, &["manifest", "exiter", "3"]);
    assert_eq!(output.status.code(), Some(3));

    // `data.dispatched` is what makes that unambiguous.
    let output = machine.run(&repo, &["--json", "manifest", "exiter", "3"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["data"]["dispatched"], Value::Bool(true));
    assert_eq!(payload["data"]["child_exit"], Value::from(3));
    assert_eq!(payload["error"], Value::Null, "Armada did not decide this");
}

/// `env:` is **additive** — the parent environment is inherited wholesale and
/// these are layered on top, so a command needing `$HOME` already has it. And
/// `ARMADA_WORKSPACE` is present without being declared anywhere.
#[test]
fn a_dispatched_command_gets_a_layered_environment_and_the_workspace_id() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    let id = workspace_id(&machine.run(&repo, &["manifest", "init", "--json"]));

    let output = machine.run(&repo, &["manifest", "enver"]);
    let seen = String::from_utf8_lossy(&output.stdout);
    let mut lines = seen.lines();
    assert_eq!(lines.next().unwrap(), format!("declared={id}"));
    assert_eq!(lines.next().unwrap(), format!("workspace={id}"));
    assert!(
        lines.next().unwrap().starts_with("home=/"),
        "the inherited environment must survive the layering"
    );
}

/// A command Armada cannot start never ran, so this is Armada's failure to report
/// and Armada's code to exit with — `bad_config`, because the repo's statement is
/// what is wrong.
#[test]
fn a_command_that_cannot_start_is_chars_own_failure_and_says_so() {
    let machine = Machine::new();
    let repo = machine.repo("main", DISPATCH_CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["--json", "manifest", "missing"]);
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
    machine.run(&repo, &["manifest", "init"]);

    let mut holder = machine.spawn(&repo, &["manifest", "sleeper"]);
    // Wait until the lease row actually exists rather than guessing.
    let db = machine.home.path().join(".armada/manifest.db");
    for _ in 0..200 {
        if lease_count(&db) > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let blocked = machine.run(&repo, &["manifest", "init", "--json"]);
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

    let output = machine.run(&repo, &["--json", "manifest", "check"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: stdout {:?}, stderr {:?}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(payload["schema_version"], Value::from(2));
    assert_eq!(payload["status"], "FAILED");
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(payload["workspace"], Value::Null, "resolution never ran");
    assert_eq!(output.status.code(), Some(2));

    // The case that already worked: parses fine, fails afterwards.
    let outside = machine.outside();
    let output = machine.run(&outside, &["--json", "manifest", "bogusverb"]);
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
        String::from_utf8_lossy(&version.stdout).starts_with("armada "),
        "{:?}",
        String::from_utf8_lossy(&version.stdout)
    );

    let help = machine.run(&outside, &["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("MANIFEST"), "{text}");
    assert!(text.contains("a commands: entry"), "{text}");
    // The limits are stated on the page rather than discovered by running one.
    assert!(text.contains("NOT BUILT YET"), "{text}");
}

/// **A page per verb**, reachable where a reader would reach for it — and only
/// for a verb Armada owns. `armada manifest <a commands: entry> --help` is the
/// child's `--help`, exactly as its `--dry-run` is the child's (PLAN.md §4.5).
#[test]
fn help_answers_at_every_level_of_the_grammar() {
    let machine = Machine::new();
    let repo = machine.repo("main", HELP_CONFIG);

    for (args, expected) in [
        (&["manifest", "--help"][..], "armada manifest — "),
        (&["manifest"][..], "armada manifest — "),
        (
            &["manifest", "check", "--help"][..],
            "armada manifest check",
        ),
        (&["manifest", "clean", "-h"][..], "armada manifest clean"),
    ] {
        let output = machine.run(&repo, args);
        assert!(output.status.success(), "`armada {}`", args.join(" "));
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            text.starts_with(expected),
            "`armada {}` answered: {text}",
            args.join(" ")
        );
    }

    // The child's, passed through untouched — `echoer` prints its argv.
    let child = machine.run(&repo, &["manifest", "echoer", "--help"]);
    assert_eq!(
        String::from_utf8_lossy(&child.stdout).trim(),
        "--help",
        "a commands: entry's --help was swallowed"
    );
}

const HELP_CONFIG: &str = "\
manifest:
  version: 1
  components:
    api:
      root: services/api
  commands:
    echoer:
      cmd: echo
      help: Echo whatever it is given
";

/// **A workspace that declares no `ports:` gets no block, and says so.**
///
/// A block exists so parallel worktrees do not collide on a service's port
/// (PLAN.md §2.2). A repository with no service has nothing to collide over, so
/// a range reserved for it reserved nothing — and took ten ports of a finite
/// pool from a workspace that did need them. Found in real use: `armada.yml`
/// declared no ports anywhere and `init` still printed `ports 5460–5469`.
///
/// The registration is separate from the claim and still happens: that row is
/// what makes the workspace reclaimable once its directory is gone.
#[test]
fn a_workspace_with_no_declared_ports_claims_no_block() {
    let machine = Machine::new();
    let repo = machine.repo("main", PORTLESS_CONFIG);

    let inited: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "init", "--json"]).stdout)
            .unwrap();
    assert_eq!(inited["status"], "READY", "{inited}");
    // **`null`, not absent.** A consumer reading this key unconditionally finds
    // out that there is no block rather than finding nothing at all — the rule
    // the envelope's own `workspace` already follows.
    assert_eq!(inited["data"]["port_block"], Value::Null, "{inited}");
    assert!(inited["data"]["ports"].is_null(), "{inited}");

    // The human render says why the corner is empty rather than leaving a blank.
    let drawn =
        String::from_utf8_lossy(&machine.run(&repo, &["manifest", "init"]).stdout).to_string();
    assert!(drawn.contains("no ports declared"), "{drawn}");

    // Registered all the same, and `status` reports it without a range.
    let status: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "status", "--json"]).stdout)
            .unwrap();
    let rows = status["data"]["results"].as_array().expect("one row");
    assert_eq!(rows.len(), 1, "{status}");
    assert!(
        rows[0].get("port_block").is_none(),
        "a row with no block must not carry one: {status}"
    );

    // And it is reclaimable, which is the half that has nothing to do with
    // ports.
    let cleaned: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "clean", "--json"]).stdout)
            .unwrap();
    assert_eq!(cleaned["status"], "CLEAN", "{cleaned}");
}

/// The other half of the rule: **one declared port anywhere is enough.** Without
/// this, "claims no block" could be true because claiming broke.
#[test]
fn a_workspace_that_declares_one_port_still_gets_its_block() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);

    let inited: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "init", "--json"]).stdout)
            .unwrap();
    assert!(
        inited["data"]["port_block"]["from"].is_u64(),
        "a workspace with a service lost its block: {inited}"
    );
    assert!(inited["data"]["ports"]["web"].is_u64(), "{inited}");
}

/// `init --dry-run` decides everything and changes nothing: no block is
/// claimed, no `.armada/` appears, and the workspace is still unknown to the
/// store afterwards.
#[test]
fn init_dry_run_previews_the_claim_and_claims_nothing() {
    let machine = Machine::new();
    let repo = machine.repo("main", SETUP_CONFIG);

    let output = machine.run(&repo, &["manifest", "init", "--dry-run", "--json"]);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    let block = &payload["data"]["would_claim"];
    assert!(block["from"].is_u64(), "{payload}");
    assert_eq!(
        payload["data"]["would_run"],
        Value::from(vec!["api: true"]),
        "the setup steps are previewed in order"
    );

    assert!(
        !repo.join(".armada").exists(),
        ".armada/ must not be created"
    );
    let all: Value = serde_json::from_slice(
        &machine
            .run(
                &machine.outside(),
                &["manifest", "status", "--all", "--json"],
            )
            .stdout,
    )
    .unwrap();
    assert_eq!(
        all["data"]["results"].as_array().unwrap().len(),
        0,
        "a preview must leave the store empty: {all}"
    );

    // And the human rendering says that nothing happened, in the status column
    // of the same table a real run draws (docs/reference-output).
    let human = machine.run(&repo, &["manifest", "init", "--dry-run"]);
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("WOULD"), "{text}");
    assert!(text.contains("nothing was changed"), "{text}");
}

/// A `setup:` step that runs and fails is the **tool's** failure: Armada started
/// it, so the repo's statement was right and the command was not.
#[test]
fn a_setup_step_that_exits_non_zero_fails_the_row_and_the_verb() {
    let machine = Machine::new();
    let repo = machine.repo("main", FAILING_SETUP_CONFIG);

    let output = machine.run(&repo, &["manifest", "init", "--json"]);
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
        serde_json::from_slice(&machine.run(&repo, &["manifest", "status", "--json"]).stdout)
            .unwrap();
    assert_eq!(status["data"]["results"].as_array().unwrap().len(), 1);
}

/// A `setup:` step that **cannot be started** is the repo's statement being
/// wrong rather than the machine's, so it is `bad_config` and names the
/// program — and the human rendering of that goes to stderr, not stdout.
#[test]
fn a_setup_step_that_cannot_start_is_bad_config_and_prints_to_stderr() {
    let machine = Machine::new();
    let repo = machine.repo("main", UNSTARTABLE_SETUP_CONFIG);

    let output = machine.run(&repo, &["manifest", "init", "--json"]);
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
    let human = machine.run(&repo, &["manifest", "init"]);
    assert!(!human.status.success());
    assert!(human.stdout.is_empty(), "nothing goes to stdout on failure");
    let text = String::from_utf8_lossy(&human.stderr);
    assert!(text.contains("could not be started"), "stderr: {text:?}");
    assert!(text.contains("class: bad_config"), "stderr: {text:?}");
}

/// `clean --dry-run --artifacts` previews **exactly** what the real pass then
/// releases: the block, the declared artifact, and the external command Armada
/// records and will never run.
#[test]
fn clean_previews_what_it_would_release_and_releases_none_of_it() {
    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    machine.run(&repo, &["manifest", "init"]);
    std::fs::write(repo.join("node_modules"), "artifact").unwrap();

    let output = machine.run(
        &repo,
        &["manifest", "clean", "--dry-run", "--artifacts", "--json"],
    );
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

    // Nothing moved: the artifact, the row and `.armada/` are all still there.
    assert!(repo.join("node_modules").exists());
    assert!(repo.join(".armada").exists());
    let status: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "status", "--json"]).stdout)
            .unwrap();
    assert_eq!(status["data"]["results"].as_array().unwrap().len(), 1);
}

/// The real pass, against the same repo: the preview's list is what goes.
#[test]
fn clean_artifacts_deletes_the_declared_files_and_reports_the_external_command() {
    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    machine.run(&repo, &["manifest", "init"]);
    std::fs::write(repo.join("node_modules"), "artifact").unwrap();

    let output = machine.run(&repo, &["manifest", "clean", "--artifacts", "--json"]);
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
    assert!(!repo.join(".armada").exists(), ".armada/ went with it");
    let status: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "status", "--json"]).stdout)
            .unwrap();
    assert_eq!(status["data"]["results"].as_array().unwrap().len(), 0);
}

/// The human renderer on the ordinary success path: `status` prints its scope,
/// the workspace's block and what it holds, to stdout.
#[test]
fn status_renders_for_a_terminal_on_stdout() {
    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "status"]);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("armada  "), "{text}");
    assert!(text.contains("ports 54"), "{text}");
    assert!(text.contains("scope workspace"), "{text}");
    assert!(
        // `status` speaks a probed port as the state of the component behind it,
        // which is the question the reader is asking. The envelope keeps
        // RESERVED / LISTENING / CONFLICT, unchanged.
        text.contains("DOWN") || text.contains("CONFLICT") || text.contains("UP"),
        "a component's state is a word in the first column: {text}"
    );
}

const SETUP_CONFIG: &str = "\
manifest:
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
manifest:
  version: 1
  components:
    api:
      setup: [\"false\"]
";

const UNSTARTABLE_SETUP_CONFIG: &str = "\
manifest:
  version: 1
  components:
    api:
      setup: [\"./definitely-not-here\"]
";

const OWNS_CONFIG: &str = "\
manifest:
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
manifest:
  version: 1
  components:
    app:
      run:
        driver: command
        cmd: ./serve
        ports: { web: 3000 }
";

/// A repository that runs something and publishes nothing — the shape that
/// exposed the bug. Checks, a command, no `ports:` anywhere.
const PORTLESS_CONFIG: &str = "\
manifest:
  version: 1
  components:
    app:
      checks:
        lint: { cmd: true }
";

const DISPATCH_CONFIG: &str = "\
manifest:
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
    assert!(armada_binary().exists());
}

/// **`armada manifest clean --orphaned --force-rebuild` is the way out of a `manifest.db` Armada
/// cannot read**, and the property under test is that *the recovery path does
/// not need the thing that is broken*: every other verb fails against this
/// database, and this one does not.
#[test]
fn force_rebuild_recovers_a_database_no_other_verb_can_open() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let db = machine.home.path().join(".armada/manifest.db");
    let namespace_before = namespace_of(&db);
    std::fs::write(&db, b"this is not a database").unwrap();

    // The premise: an ordinary verb cannot get past opening it, and says so as
    // `environment` — the machine is broken, the repo is fine.
    let broken = machine.run(&repo, &["manifest", "status", "--json"]);
    let payload: Value = serde_json::from_slice(&broken.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "environment");
    assert_eq!(broken.status.code(), Some(6));

    // `--orphaned` is required, because it is what bounds the removal to
    // workspaces whose directory is gone.
    let unbounded = machine.run(
        &repo,
        &["manifest", "clean", "--all", "--force-rebuild", "--json"],
    );
    let payload: Value = serde_json::from_slice(&unbounded.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_invocation");

    // The invocation `PLAN.md` §4.3 spells, run from inside a workspace and
    // without `--all`: the corpus documents this exact form, so this exact form
    // is what has to work.
    let rebuilt = machine.run(
        &repo,
        &[
            "manifest",
            "clean",
            "--orphaned",
            "--force-rebuild",
            "--json",
        ],
    );
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
    let kept: Vec<_> = std::fs::read_dir(machine.home.path().join(".armada"))
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
    assert!(machine
        .run(&repo, &["manifest", "status", "--all"])
        .status
        .success());
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
    machine.run(&repo, &["manifest", "init"]);

    let db = machine.home.path().join(".armada/manifest.db");
    let junk = b"this is not a database";
    std::fs::write(&db, junk).unwrap();

    let previewed = machine.run(
        &repo,
        &[
            "manifest",
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
        would_release.contains("move aside") && would_release.contains("manifest.db"),
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
    let aside: Vec<_> = std::fs::read_dir(machine.home.path().join(".armada"))
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
        serde_json::from_slice(&machine.run(&repo, &["manifest", "status", "--json"]).stdout)
            .unwrap();
    assert_eq!(after["error"]["class"], "environment");
}

/// `--artifacts` and `--force` mean nothing on a path that reads no `armada.yml`
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
    machine.run(&repo, &["manifest", "init"]);

    for args in [
        &[
            "manifest",
            "clean",
            "--all",
            "--orphaned",
            "--force-rebuild",
            "--artifacts",
            "--json",
        ][..],
        &[
            "manifest",
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
            "`armada {}` was accepted: {payload}",
            args.join(" ")
        );
        assert_eq!(refused.status.code(), Some(2));
    }
}

/// **`config scan` in a directory that is not a workspace at all**, which is
/// the only situation it ever runs in (PLAN.md §5): a repository has no
/// `armada.yml` and this is how it gets one.
///
/// Every other verb resolves a workspace first and fails without one. Nothing
/// but an end-to-end run establishes that this one does not — the unit tests
/// call the verb directly, past the entrypoint where that decision is made.
#[test]
fn config_scan_answers_in_a_directory_with_no_workspace() {
    let machine = Machine::new();
    let loose = machine.root.path().join("not-a-workspace");
    std::fs::create_dir_all(&loose).unwrap();
    std::fs::write(
        loose.join("package.json"),
        r#"{"scripts":{"test":"vitest run","lint":"eslint ."}}"#,
    )
    .unwrap();
    std::fs::write(loose.join("package-lock.json"), "{}").unwrap();

    let output = machine.run(&loose, &["manifest", "config", "scan", "--json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "scan reports rather than judges: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["workspace"], Value::Null, "nothing was claimed");
    assert_eq!(payload["data"]["evidence"]["config_present"], false);
    let scripts = &payload["data"]["evidence"]["scripts"][0]["scripts"];
    assert_eq!(scripts[0]["name"], "test", "declaration order, not sorted");
    assert_eq!(scripts[1]["name"], "lint");

    // The human render is the one an authoring agent actually reads, and it
    // ends by handing over (PLAN.md §5, ARCHITECTURE.md §1.9 — the rule governs
    // inputs, and printing a choice is an output). This scratch machine has no
    // guild, so the handover is the command plus the reason it cannot be run
    // for you — not a menu, and not silence.
    let human = machine.run(&loose, &["manifest", "config", "scan"]);
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.starts_with("no armada.yml here"), "{text}");
    assert!(text.contains("Evidence only."), "{text}");
    assert!(text.contains("--append-system-prompt"), "{text}");
    assert!(text.contains("no onboarding skill in your guild"), "{text}");
    assert!(!text.contains('\x1b'), "a pipe gets no escapes: {text}");
}

/// **Pass 1 short-circuits, and pass 2 is not attempted.** That is the whole
/// reason layer 3 has two passes: the hallucinated script name is caught in
/// seconds, and the build nobody asked for does not run.
///
/// The scratch repo declares `cmd: ./serve`, which is not on `PATH` and is not
/// an executable file under the component root — the exact shape an agent
/// authoring a config produces, and the one this verb exists to catch.
#[test]
fn config_verify_fails_pass_one_and_does_not_attempt_pass_two() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);

    let output = machine.run(&repo, &["manifest", "config", "verify", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_config", "{payload}");
    assert_eq!(output.status.code(), Some(3), "bad_config is exit 3");
    assert!(
        payload["data"]["pass_2"].is_null(),
        "pass 2 ran anyway: {payload}"
    );

    let rows: Vec<(&str, &str)> = payload["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["id"].as_str().unwrap(), row["status"].as_str().unwrap()))
        .collect();
    assert_eq!(
        rows,
        [
            ("schema", "PASS"),
            ("references", "PASS"),
            ("argv[0]", "FAILED")
        ],
        "{payload}"
    );
    // **`bad_config` requires a next_action**, and on this verb it is the whole
    // point: a report naming a problem without the fix sends the reader to the
    // documentation.
    assert!(payload["error"]["next_action"].is_string(), "{payload}");

    let human = machine.run(&repo, &["manifest", "config", "verify"]);
    // A failure goes to stderr, so `armada manifest config verify | grep` is
    // never quietly fed an error report.
    let text = String::from_utf8_lossy(&human.stderr);
    assert!(
        text.starts_with("pass 1, static, nothing is executed"),
        "{text}"
    );
    assert!(text.contains("UNCHECKED  shell entries"), "{text}");
    assert!(
        text.contains("pass 2 not attempted, fix pass 1 first"),
        "{text}"
    );
    assert!(text.contains("\n  -> "), "no fix line: {text}");
}

/// **`config scan` must never block on stdin that will never arrive.**
///
/// That is the failure mode the whole handover design turns on: an agent
/// running this inside a Job reads stdout and has nothing to type, so a menu
/// there is a prompt that hangs until the Job's ceiling expires and reports
/// nothing. Only an end-to-end run establishes it — the decision is unit-tested
/// where it is made, and this is the half that proves the process actually
/// exits.
///
/// `Machine::run` captures stdout, so stdin is not a terminal here, which is
/// exactly the situation under test.
#[test]
fn config_scan_prints_the_next_command_instead_of_a_prompt_it_cannot_answer() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);

    // No timeout wrapper and none needed: if this blocks, the suite hangs and
    // that is the bug. It returns because nothing was ever asked.
    let output = machine.run(&repo, &["manifest", "config", "scan"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);

    assert!(
        !text.contains("1 let an agent write it with me"),
        "a menu was drawn for a reader with no stdin:\n{text}"
    );
    assert!(
        text.contains("next") && text.contains("--append-system-prompt"),
        "the next step was not printed:\n{text}"
    );

    // `--json` is the third audience: the envelope, and no handover at all.
    let piped = machine.run(&repo, &["manifest", "config", "scan", "--json"]);
    let payload: Value = serde_json::from_slice(&piped.stdout).unwrap();
    assert_eq!(payload["data"]["handover"], "silent", "{payload}");
}

/// **`config scan` on a monorepo, end to end.** The unit tests hand the parser
/// a list of files; only a real run establishes that the *search* finds them,
/// which is the half that was wrong.
#[test]
fn config_scan_finds_evidence_below_the_root_of_a_real_checkout() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    let write = |relative: &str, text: &str| {
        let path = repo.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };

    write("web/package.json", r#"{"scripts":{"test":"vitest run"}}"#);
    write("web/pnpm-lock.yaml", "");
    write("backend/pyproject.toml", "[project]\n[tool.ruff]\n");
    write("backend/uv.lock", "");
    // Ignored, so it is not evidence — and the rule that says so is git's.
    write(".gitignore", "generated/\n");
    write("generated/package.json", r#"{"scripts":{"nope":"nope"}}"#);
    // Never evidence, ignored or not.
    write("node_modules/left-pad/package.json", "{}");
    // Past the depth bound: a vendored copy, not a package.
    write("a/b/c/d/package.json", "{}");

    let output = machine.run(&repo, &["manifest", "config", "scan", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{payload}");

    let found: Vec<&str> = payload["data"]["evidence"]["lockfiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        found,
        ["backend/uv.lock", "web/pnpm-lock.yaml"],
        "{payload}"
    );

    let scripts: Vec<&str> = payload["data"]["evidence"]["scripts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        scripts,
        ["web/package.json"],
        "an ignored, a vendored or a too-deep manifest reached the report: {payload}"
    );

    // The layout, which is the fact the flat lists cannot carry.
    let packages: Vec<(&str, bool)> = payload["data"]["evidence"]["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["dir"].as_str().unwrap(),
                p["independent"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(packages, [("backend", true), ("web", true)], "{payload}");

    // And the kind that was read as a path is no longer spellable as one.
    let human = machine.run(&repo, &["manifest", "config", "scan"]);
    let text = String::from_utf8_lossy(&human.stdout);
    assert!(text.contains("package scripts"), "{text}");
    assert!(text.contains("workspaces: candidates"), "{text}");
}

/// The other side of the short-circuit: pass 1 passes, so pass 2 runs the check
/// suite **for real** — against a scratch repository and nowhere else.
#[test]
fn config_verify_runs_the_suite_for_real_when_pass_one_passes() {
    let machine = Machine::new();
    let repo = machine.repo(
        "main",
        "manifest:\n  version: 1\n  components:\n    repo:\n      match: [\"*.yml\"]\n      \
         checks:\n        ok:\n          cmd: \"true\"\n          scope: component\n",
    );

    let output = machine.run(&repo, &["manifest", "config", "verify", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{payload}");
    assert_eq!(payload["status"], "PASS");
    assert_eq!(payload["data"]["pass_2"]["results"][0]["id"], "repo:ok");
    assert_eq!(payload["data"]["pass_2"]["results"][0]["status"], "PASS");
}

/// **There is no way to run a skill, and its absence is the design**
/// (PLAN.md §4.8). Only an end-to-end run establishes that: the parser is where
/// a third subcommand would have to appear, and the unit tests call the verb
/// past it.
#[test]
fn a_skill_can_be_listed_and_resolved_and_never_run() {
    let machine = Machine::new();
    let repo = machine.repo(
        "main",
        "manifest:\n  version: 1\n  commands:\n    tickets: { cmd: ./exiter.sh 0 }\n  \
         skills:\n    add-endpoint:\n      summary: Add an API endpoint\n      \
         doc: docs/skills/add-endpoint.md\n      uses: [tickets]\n",
    );

    let listed = machine.run(&repo, &["manifest", "skills", "--json"]);
    let payload: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.status.code(), Some(0), "{payload}");
    assert_eq!(payload["data"]["skills"][0]["name"], "add-endpoint");
    // `uses:` grants nothing new, so what it resolves to is the command the
    // repository already declared.
    assert_eq!(
        payload["data"]["skills"][0]["uses"][0]["cmd"],
        "./exiter.sh 0"
    );

    let shown = machine.run(&repo, &["manifest", "skills", "show", "add-endpoint"]);
    let text = String::from_utf8_lossy(&shown.stdout);
    // Words rather than column positions: the widths are the golden fixtures'
    // to pin, and asserting them here would fail twice for one change.
    let row = |words: &[&str]| {
        text.lines()
            .any(|line| line.split_whitespace().collect::<Vec<_>>() == words)
    };
    assert!(row(&["GRANTS", "tickets", "./exiter.sh", "0"]), "{text}");
    assert!(
        row(&["READS", "doc", "docs/skills/add-endpoint.md"]),
        "{text}"
    );

    // An unknown name is the *caller's* mistake, not the config's.
    let unknown = machine.run(&repo, &["manifest", "skills", "show", "nope", "--json"]);
    let payload: Value = serde_json::from_slice(&unknown.stdout).unwrap();
    assert_eq!(payload["error"]["class"], "bad_invocation");
    assert_eq!(unknown.status.code(), Some(2));

    // And there is no runner, in any spelling.
    for args in [
        &["manifest", "skills", "run", "add-endpoint", "--json"][..],
        &["manifest", "skills", "add-endpoint", "--json"][..],
    ] {
        let refused = machine.run(&repo, args);
        let payload: Value = serde_json::from_slice(&refused.stdout).unwrap();
        assert_eq!(
            payload["error"]["class"],
            "bad_invocation",
            "`armada {}` was accepted: {payload}",
            args.join(" ")
        );
    }
}

/// **The machine that was broken, run.** `~/.armada/machine.yml` carried one
/// module's keys flat at the top level and a second module's section beside
/// them, and the strict reader rejected the whole document — so every Manifest
/// verb failed with `unknown field \`guild\`` before it did anything.
///
/// Asserted through the real binary, against the file as it was actually
/// written, because every unit test in the reader constructs the document it
/// then reads: the bug was that the two modules disagreed about the file, and a
/// test that writes it in one module's voice cannot see that.
#[test]
fn a_machine_yml_holding_two_modules_facts_does_not_fail_every_verb() {
    let machine = Machine::new();
    std::fs::create_dir_all(machine.home.path().join(".armada")).unwrap();
    let path = machine.home.path().join(".armada/machine.yml");
    // Exactly the shape found on a real machine: pre-namespace keys, plus the
    // section a second module wrote into the same file.
    std::fs::write(
        &path,
        "cpu_slots: 8\nport_block_size: 10\nup_timeout: 600\nguild:\n  withheld: []\n",
    )
    .unwrap();

    let repo = machine.repo("app", CONFIG);
    for verb in [
        &["manifest", "init", "--json"][..],
        &["manifest", "status", "--json"][..],
        &["fleet", "ls", "--json"][..],
    ] {
        let output = machine.run(&repo, verb);
        let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "`armada {}` failed: {payload}",
            verb.join(" ")
        );
    }

    // The machine's number was read rather than defaulted past: a block of ten
    // is what `port_block_size` asked for.
    let status = machine.run(&repo, &["manifest", "status", "--json"]);
    let payload: Value = serde_json::from_slice(&status.stdout).unwrap();
    let block = &payload["data"]["results"][0]["port_block"];
    assert_eq!(
        block["to"].as_u64().unwrap() - block["from"].as_u64().unwrap(),
        9,
        "{payload}"
    );

    // And nothing rewrote it: a read path leaves a hand-edited file alone.
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .starts_with("cpu_slots: 8"),
        "a read rewrote machine.yml"
    );
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
