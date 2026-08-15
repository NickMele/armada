//! `armada task` and `armada tasks` end to end, against a scratch `$HOME`.
//!
//! **What this suite is really holding is one claim**: a task, a report and a
//! recorded failure are one record with one id space
//! (`docs/reserved/002-tasks.md`, `docs/reserved/001-raised-items-need-identity.md`).
//! So the assertions that matter are the crossing ones — a task id resolves
//! under `armada failures show`, a task never appears in `armada failures`, and
//! promotion is the verb `failures fix` already was.
//!
//! **Nothing here spends a token or starts a `claude` session.** The stub on
//! `PATH` answers the probes; every promotion names its workflow, so nothing
//! reaches the classifier.

mod support;

use support::{why, Machine};

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Write a task down and answer with its id.
fn capture(machine: &Machine, at: &std::path::Path, what: &str) -> String {
    let filed = machine.run(at, &["task", what, "--json"]);
    assert!(filed.status.success(), "{}", why(&filed));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&filed)).expect("an envelope");
    payload["data"]["results"][0]["id"]
        .as_str()
        .expect("an id")
        .to_string()
}

/// Put the `bug` workflow in the scratch guild, so a promotion that names it
/// has one to read.
fn seed_bug_workflow(machine: &Machine) {
    let workflows = machine.home.path().join(".armada/guild/workflows");
    std::fs::create_dir_all(&workflows).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/guild/workflows/bug.yml"),
        workflows.join("bug.yml"),
    )
    .unwrap();
}

/// **The whole feature in one test.** A sentence goes in, an id comes back, and
/// the row is in `armada tasks` — with no class, no diagnostics and nothing
/// gathered, because capture is free.
#[test]
fn a_task_is_written_down_with_an_id_and_nothing_else_happens() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");

    let said = machine.run(&repo, &["task", "rename the port allocator"]);
    assert!(said.status.success(), "{}", why(&said));
    assert!(
        stdout(&said).contains("rename the port allocator"),
        "the sentence comes back: {}",
        stdout(&said)
    );

    let listed = machine.run(&repo, &["tasks", "--json"]);
    assert!(listed.status.success(), "{}", why(&listed));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&listed)).expect("an envelope");
    let rows = payload["data"]["results"].as_array().expect("results[]");
    assert_eq!(rows.len(), 1, "{payload}");
    assert_eq!(rows[0]["origin"], "written");
    assert_eq!(rows[0]["state"], "OPEN");
    assert_eq!(payload["data"]["open"], 1);
    assert!(
        rows[0]["class"].is_null(),
        "Armada noticed nothing, so it attributed nothing: {payload}"
    );
    assert!(
        rows[0]["diagnostics"].is_null(),
        "capture gathered nothing, which is what makes it free: {payload}"
    );
}

/// **One store, one id space, two lenses** — the claim `002` and `001` together
/// make, and the one thing this feature would be wrong without.
///
/// Inverted deliberately in both directions: the task must be **absent** from
/// `armada failures` and the failure **absent** from `armada tasks`, because a
/// filter asserted only from the side it keeps is a filter that passes when it
/// does nothing.
#[test]
fn tasks_and_failures_share_one_store_and_are_never_listed_together() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");

    let task = capture(&machine, &repo, "rename the port allocator");
    // A real failure, recorded by the recorder rather than planted: an unknown
    // manifest verb is Armada's own refusal to answer.
    let broke = machine.run(&repo, &["manifest", "check", "--nonsense"]);
    assert!(!broke.status.success(), "{}", why(&broke));
    machine.run(&repo, &["report", "the dry-run made nothing"]);

    let tasks: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--json"]))).unwrap();
    let failures: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["failures", "--json"]))).unwrap();

    let ids = |payload: &serde_json::Value| -> Vec<String> {
        payload["data"]["results"]
            .as_array()
            .expect("results[]")
            .iter()
            .map(|row| row["id"].as_str().unwrap().to_string())
            .collect()
    };
    assert!(ids(&tasks).contains(&task), "{tasks}");
    assert!(
        !ids(&failures).contains(&task),
        "a task listed as a failure: {failures}"
    );
    assert!(
        !ids(&failures).is_empty(),
        "the failure half has to be non-empty for the exclusion to mean anything: {failures}"
    );
    for row in failures["data"]["results"].as_array().unwrap() {
        assert_ne!(row["origin"], "written", "{failures}");
        assert!(!ids(&tasks).contains(&row["id"].as_str().unwrap().to_string()));
    }

    // **And the id crosses.** A person holding an id has already said which row
    // they mean, so the other verb's `show` answers rather than refusing.
    let shown = machine.run(&repo, &["failures", "show", &task[..4], "--json"]);
    assert!(shown.status.success(), "{}", why(&shown));
    let one: serde_json::Value = serde_json::from_str(&stdout(&shown)).expect("an envelope");
    assert_eq!(one["data"]["results"][0]["id"], task.as_str());
}

/// **The prompt a Job gets is the sentence**, which is what
/// `docs/reserved/002-tasks.md` asks for — not Armada's paraphrase of it.
#[test]
fn the_prompt_a_started_task_hands_a_job_is_the_sentence_itself() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let id = capture(&machine, &repo, "rename the port allocator");

    let shown = machine.run(&repo, &["tasks", "show", &id[..4], "--json"]);
    assert!(shown.status.success(), "{}", why(&shown));
    let one: serde_json::Value = serde_json::from_str(&stdout(&shown)).expect("an envelope");
    let task = one["data"]["task"].as_str().expect("a prompt");
    assert!(
        task.starts_with("rename the port allocator"),
        "the sentence leads: {task}"
    );
    assert!(
        !task.contains("Armada failed"),
        "nothing went wrong, and the Job is not told otherwise: {task}"
    );
}

/// **Promotion is the path `failures fix` already was**, and `--workflow` keeps
/// it free of a model call — which is what makes it testable at all.
#[test]
fn a_task_starts_a_job_through_the_promotion_the_store_already_had() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    seed_bug_workflow(&machine);
    let id = capture(&machine, &repo, "the port allocator picks a used block");

    let started = machine.run(
        &repo,
        &[
            "tasks",
            "start",
            &id,
            "--workflow",
            "bug",
            "--dry-run",
            "--json",
        ],
    );
    assert!(started.status.success(), "{}", why(&started));
    let spawned: serde_json::Value = serde_json::from_str(&stdout(&started)).expect("an envelope");
    assert_eq!(
        spawned["data"]["workflow"], "bug",
        "named rather than classified, so nothing is spent: {spawned}"
    );
    assert!(
        spawned["data"]["task"]
            .as_str()
            .is_none_or(|task| !task.contains("Armada failed")),
        "{spawned}"
    );

    // **A dry run leaves the task open.** A promotion line for a Job that never
    // started would put `FIXING` on a row nobody is working.
    let listed: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--json"]))).unwrap();
    assert_eq!(listed["data"]["results"][0]["state"], "OPEN", "{listed}");
    assert!(listed["data"]["results"][0]["job"].is_null(), "{listed}");
}

/// **Discarding is one keystroke and the row stays on disk.** A list you cannot
/// clear stops being read; a log you rewrite is a log that can lose the rest.
#[test]
fn a_cleared_task_leaves_the_default_listing_and_comes_back_under_all() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let id = capture(&machine, &repo, "rename the port allocator");

    let cleared = machine.run(&repo, &["tasks", "clear", &id]);
    assert!(cleared.status.success(), "{}", why(&cleared));

    let listed: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--json"]))).unwrap();
    assert!(
        listed["data"]["results"].as_array().unwrap().is_empty(),
        "{listed}"
    );
    let all: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--all", "--json"]))).unwrap();
    assert_eq!(all["data"]["results"][0]["id"], id.as_str(), "{all}");
    assert_eq!(all["data"]["results"][0]["state"], "CLEARED", "{all}");
}

/// **`tasks clear --all` clears tasks and nothing else.** The store is shared,
/// so sweeping the other lens would discard rows the person never had on
/// screen — the one mistake an append-only log cannot make quiet.
#[test]
fn clearing_every_task_leaves_the_failures_alone() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    capture(&machine, &repo, "rename the port allocator");
    let filed = machine.run(&repo, &["report", "the dry-run made nothing"]);
    assert!(filed.status.success(), "{}", why(&filed));

    let cleared = machine.run(&repo, &["tasks", "clear", "--all"]);
    assert!(cleared.status.success(), "{}", why(&cleared));

    let failures: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["failures", "--json"]))).unwrap();
    let rows = failures["data"]["results"].as_array().unwrap();
    assert!(!rows.is_empty(), "{failures}");
    for row in rows {
        assert_eq!(row["state"], "OPEN", "a failure was swept: {failures}");
    }
}

/// **A task can be written from anywhere**, including a directory that is no
/// workspace — the thought arrives where it arrives.
#[test]
fn a_task_can_be_written_from_outside_any_workspace() {
    let machine = Machine::new();
    let outside = machine.outside();
    let written = machine.run(&outside, &["task", "set this machine up properly"]);
    assert!(written.status.success(), "{}", why(&written));
}

/// **A task written inside a worktree names the checkout, not the worktree.**
///
/// `docs/reserved/002-tasks.md` asks for tasks keyed by project *"so a Drone in
/// a throwaway worktree sees the same tasks the checkout you wrote them in
/// does"* — and the sharper case is the reverse: a worktree under
/// `.claude/worktrees/` is deleted when the work that made it ends, so a row
/// recording that path would name a directory that is gone before anybody reads
/// it. `git rev-parse --git-common-dir` is what makes both views agree.
#[test]
fn a_task_written_in_a_worktree_belongs_to_the_checkout() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let worktree = repo.join(".claude/worktrees/agent-a1b2c3");
    let git = |args: &[&str]| -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&repo)
            .env("HOME", machine.home.path())
            .output()
            .expect("git runs")
    };
    // A worktree needs a commit to branch from. The harness may already have
    // made one, so this is allowed to answer "nothing to commit".
    git(&["add", "-A"]);
    git(&["commit", "-qm", "first"]);
    let added = git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "scratch",
        worktree.to_str().unwrap(),
    ]);
    assert!(added.status.success(), "git worktree add: {added:?}");

    let id = capture(&machine, &worktree, "rename the port allocator");
    let shown: serde_json::Value = serde_json::from_str(&stdout(
        &machine.run(&worktree, &["tasks", "show", &id, "--json"]),
    ))
    .unwrap();
    let cwd = shown["data"]["results"][0]["cwd"].as_str().expect("a cwd");
    assert!(
        !cwd.contains("worktrees"),
        "the task named a directory that will not outlive it: {cwd}"
    );
    assert!(cwd.ends_with("orders"), "{cwd}");
}

/// **A task without a sentence is refused rather than written empty**, and the
/// refusal names the verb that lists them — because a bare `armada task` is far
/// more often somebody reaching for the list.
#[test]
fn a_task_with_nothing_to_do_is_refused_and_points_at_the_listing() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let refused = machine.run(&repo, &["task"]);
    assert!(!refused.status.success(), "{}", why(&refused));
    let said = why(&refused);
    assert!(said.contains("armada task \""), "{said}");
    assert!(said.contains("armada tasks"), "{said}");
}

/// **No absolute `$HOME` reaches disk**, the rule `docs/reserved/010` states for
/// the failure log — and a task is a line in that same file.
#[test]
fn no_absolute_home_path_reaches_the_store_from_a_task() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    capture(&machine, &repo, "rename the port allocator");
    let written = std::fs::read_to_string(machine.home.path().join(".armada/failures.jsonl"))
        .expect("written");
    assert!(
        !written.contains(&machine.home.path().display().to_string()),
        "an absolute $HOME reached disk:\n{written}"
    );
}
