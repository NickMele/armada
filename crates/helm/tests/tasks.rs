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
    // **The reviewer too, because `bug` reaches it.** `fleet spawn` walks every
    // workflow the chosen one can reach and refuses before spending if one is
    // missing — a `bug` Job whose guild has no `review` would otherwise buy
    // `reproduce` and `fix` before finding out. A fixture carrying only `bug`
    // is a guild that cannot run `bug`.
    for name in ["bug.yml", "review.yml"] {
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../templates/guild/workflows")
                .join(name),
            workflows.join(name),
        )
        .unwrap();
    }

    // **And the skills those workflows name.** `fleet spawn` refuses a Job
    // whose named skill is in neither the guild nor the repository — a guild
    // with workflows and no skills is not a state `guild init` produces, it is
    // the state that shipped until 2026-08-17.
    let skills = machine.home.path().join(".armada/guild/skills");
    for name in [
        "reproduce-failure",
        "implement-change",
        "land-branch",
        "review-diff",
    ] {
        let to = skills.join(name);
        std::fs::create_dir_all(&to).unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../templates/guild/skills")
                .join(name)
                .join("SKILL.md"),
            to.join("SKILL.md"),
        )
        .unwrap();
    }

    // **And projected**, because a Drone reads `~/.claude/`, not the guild, and
    // `fleet spawn` refuses when the two have diverged. A guild edited and not
    // projected is a change no Drone will ever see.
    armada_guild::projector::project(
        &armada_guild::layout::Guild::at(&machine.home.path().join(".armada")),
        &machine.home.path().join(".claude"),
        &machine.home.path().join(".armada"),
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

/// **`armada tasks` is scoped to the repository it is run from, by default.**
/// A task written in one checkout does not clutter the listing of a completely
/// unrelated one — the gap he found: *"I was hoping they could be scoped to
/// the directory that it was run in."*
#[test]
fn a_task_is_scoped_to_the_repository_it_was_written_in_by_default() {
    let machine = Machine::new();
    let orders = machine.repo("orders", "version: 1\nname: orders\n");
    let payments = machine.repo("payments", "version: 1\nname: payments\n");
    let id = capture(&machine, &orders, "rename the port allocator");

    let elsewhere: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&payments, &["tasks", "--json"]))).unwrap();
    assert!(
        elsewhere["data"]["results"].as_array().unwrap().is_empty(),
        "a task from another repository showed up unscoped: {elsewhere}"
    );

    let home: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&orders, &["tasks", "--json"]))).unwrap();
    assert_eq!(home["data"]["results"][0]["id"], id.as_str(), "{home}");

    // **`--all` is the same word `armada manifest status` already uses to
    // widen past its own narrow default** — reused rather than a second flag.
    let all: serde_json::Value = serde_json::from_str(&stdout(
        &machine.run(&payments, &["tasks", "--all", "--json"]),
    ))
    .unwrap();
    assert!(
        all["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == id.as_str()),
        "--all did not reach a task from another repository: {all}"
    );
}

/// **A task written outside any repository is not scoped out of existence.**
/// It has no project to be excluded from, so it is reachable from the same
/// place it was written, and `--all` reaches it from anywhere.
#[test]
fn a_task_written_outside_any_workspace_is_reachable_by_default_and_under_all() {
    let machine = Machine::new();
    let outside = machine.outside();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let id = capture(&machine, &outside, "set this machine up properly");

    let same_place: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&outside, &["tasks", "--json"]))).unwrap();
    assert!(
        same_place["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == id.as_str()),
        "a task written outside any repository vanished from its own directory: {same_place}"
    );

    let from_a_repo: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--json"]))).unwrap();
    assert!(
        from_a_repo["data"]["results"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{from_a_repo}"
    );
    let all: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--all", "--json"]))).unwrap();
    assert!(
        all["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == id.as_str()),
        "--all did not reach a task written outside any repository: {all}"
    );
}

/// **A task written where capture finds an `armada.yml` records the workspace
/// as a column, separate from `cwd`.** `capture` reuses
/// `armada_manifest::discovery::resolve` — the same walk `armada manifest
/// status` resolves against — so this is the wiring test: the entry the CLI
/// actually writes carries what that resolver actually found, not a value a
/// unit test on `armada_core::failure` constructed by hand.
#[test]
fn a_task_written_inside_a_workspace_records_it_as_a_column() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let id = capture(&machine, &repo, "rename the port allocator");

    let shown: serde_json::Value = serde_json::from_str(&stdout(
        &machine.run(&repo, &["tasks", "show", &id, "--json"]),
    ))
    .unwrap();
    let row = &shown["data"]["results"][0];
    assert!(
        row["workspace"]
            .as_str()
            .is_some_and(|w| w.ends_with("orders")),
        "the resolved workspace root is on the row: {row}"
    );
}

/// **The column is empty rather than guessed when capture finds no
/// `armada.yml`.** `machine.outside()` is a real directory with no config at
/// all — a candidate, never a workspace — so `workspace` is absent from the
/// `--json` payload entirely rather than holding a nearest directory that
/// does not own it.
#[test]
fn a_task_written_outside_any_workspace_leaves_the_column_absent() {
    let machine = Machine::new();
    let outside = machine.outside();
    let id = capture(&machine, &outside, "set this machine up properly");

    let shown: serde_json::Value = serde_json::from_str(&stdout(
        &machine.run(&outside, &["tasks", "show", &id, "--json"]),
    ))
    .unwrap();
    let row = &shown["data"]["results"][0];
    assert!(
        row.get("workspace").is_none(),
        "a candidate directory was recorded as a workspace: {row}"
    );
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

// ------------------------------------------------- one id space, four origins
//
// `docs/reserved/001-raised-items-need-identity.md`. Three origins already
// shared `~/.armada/failures.jsonl`; the inbox was the fourth kind of item and
// the only one outside, so an id off `armada fleet inbox` resolved nowhere else
// and an id off `armada failures` resolved nowhere there.

/// Plant one open inbox entry, the way a Drone's `fleet.ask_human` writes it.
///
/// **Written as a line rather than through the API**, because the file is what
/// the reader reads and a helper that used the writer would not prove the two
/// agree on the shape.
fn plant_inbox(machine: &Machine, uuid: &str, job: &str, body: &str) {
    let path = machine.home.path().join(".armada/inbox.jsonl");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(
            "{{\"type\":\"raised\",\"uuid\":\"{uuid}\",\
             \"job_uuid\":\"c19d0a34-3069-4f6a-9d1e-2b7c8a5f0e11\",\
             \"job\":\"{job}\",\"kind\":\"needs_human\",\
             \"raised_at\":\"2026-08-09T14:02:11Z\",\"raised_ms\":1,\
             \"body\":\"{body}\"}}\n"
        ),
    )
    .unwrap();
}

/// **An inbox entry resolves in the same id space, and is listed in neither
/// lens** — the two halves of `001`'s unification, and the second is what keeps
/// it from being a fifth store wearing a fourth listing.
///
/// A raised item has its own listing already: `armada fleet inbox`. Drawing the
/// same row in `armada tasks` as well is how two tables start disagreeing about
/// it. One id space is not the same claim as one listing, and `001` asks for
/// the first.
#[test]
fn an_inbox_entry_resolves_by_id_and_is_listed_in_neither_lens() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    let task = capture(&machine, &repo, "rename the port allocator");
    plant_inbox(
        &machine,
        "4f2a91c8-6b03-4d17-8e5a-91c30b6f2d84",
        "nightly-flake",
        "raise the CI timeout to 90s?",
    );

    // **Four characters of the id**, which is what a person retypes off the
    // table — and it resolves through a verb that has never read the inbox.
    let shown = machine.run(&repo, &["failures", "show", "4f2a", "--json"]);
    assert!(shown.status.success(), "{}", why(&shown));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&shown)).unwrap();
    let row = &payload["data"]["results"][0];
    assert_eq!(row["id"], "4f2a91c8", "{payload}");
    assert_eq!(row["origin"], "raised", "{payload}");
    assert_eq!(row["message"], "raise the CI timeout to 90s?", "{payload}");
    assert_eq!(row["job"], "nightly-flake", "{payload}");
    // **No task**, because its Job exists and is stopped in front of the
    // question. There is nothing to hand over.
    assert_eq!(payload["data"]["task"], "", "{payload}");

    // Inverted from both sides, the rule this suite already holds for tasks and
    // failures: absent from each listing, present in the id space.
    for verb in ["tasks", "failures"] {
        let listed: serde_json::Value =
            serde_json::from_str(&stdout(&machine.run(&repo, &[verb, "--json"]))).unwrap();
        let ids: Vec<String> = listed["data"]["results"]
            .as_array()
            .expect("results[]")
            .iter()
            .map(|row| row["id"].as_str().unwrap().to_string())
            .collect();
        assert!(
            !ids.contains(&"4f2a91c8".to_string()),
            "a raised item was drawn in `armada {verb}`: {listed}"
        );
    }
    // And the assertion that makes the one above mean something: the store the
    // listing does read is still being read.
    let tasks: serde_json::Value =
        serde_json::from_str(&stdout(&machine.run(&repo, &["tasks", "--json"]))).unwrap();
    assert_eq!(tasks["data"]["results"][0]["id"], task.as_str(), "{tasks}");
}

/// **A raised item is refused by the two verbs that would lie about it**, and
/// the refusal names the verb that works.
///
/// `start` would spawn a second Job for a question a Drone is already stopped
/// in front of; `clear` would append a line to `failures.jsonl` about an id
/// that file has never held, producing a row that reads as cleared and is not.
/// Silently forwarding either to `armada fleet answer` would be worse: clearing
/// a row and unblocking an agent are different acts.
#[test]
fn a_raised_item_is_refused_by_start_and_clear_and_told_where_to_go() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    plant_inbox(
        &machine,
        "4f2a91c8-6b03-4d17-8e5a-91c30b6f2d84",
        "nightly-flake",
        "raise the CI timeout to 90s?",
    );

    for verb in [
        vec!["tasks", "start", "4f2a", "--json"],
        vec!["failures", "fix", "4f2a", "--json"],
        vec!["failures", "clear", "4f2a", "--json"],
        vec!["tasks", "clear", "4f2a", "--json"],
    ] {
        let refused = machine.run(&repo, &verb);
        assert_eq!(
            refused.status.code(),
            Some(2),
            "`{}` should be a bad_invocation: {}",
            verb.join(" "),
            why(&refused)
        );
        let payload: serde_json::Value = serde_json::from_str(&stdout(&refused)).unwrap();
        assert_eq!(payload["error"]["class"], "bad_invocation", "{payload}");
        assert!(
            payload["error"]["next_action"]
                .as_str()
                .unwrap_or_default()
                .contains("armada fleet answer 4f2a91c8"),
            "the refusal should name the verb that works: {payload}"
        );
    }

    // **Inverted**: the same verbs still work on a row that is theirs, so the
    // refusal above is about the origin and not about the id being unknown.
    let task = capture(&machine, &repo, "rename the port allocator");
    let cleared = machine.run(&repo, &["tasks", "clear", &task, "--json"]);
    assert!(cleared.status.success(), "{}", why(&cleared));
}

/// **`clear --all` never reaches a raised item**, whichever lens asks. It
/// sweeps what the listing drew, and the listing draws none of them — so the
/// one mistake an append-only log cannot make quiet stays impossible.
#[test]
fn clearing_everything_leaves_the_inbox_untouched() {
    let machine = Machine::new();
    let repo = machine.repo("orders", "version: 1\nname: orders\n");
    capture(&machine, &repo, "rename the port allocator");
    plant_inbox(
        &machine,
        "4f2a91c8-6b03-4d17-8e5a-91c30b6f2d84",
        "nightly-flake",
        "raise the CI timeout to 90s?",
    );
    let before = std::fs::read_to_string(machine.home.path().join(".armada/inbox.jsonl")).unwrap();

    for verb in [["tasks", "clear", "--all"], ["failures", "clear", "--all"]] {
        let swept = machine.run(&repo, &verb);
        assert!(swept.status.success(), "{}", why(&swept));
    }

    assert_eq!(
        std::fs::read_to_string(machine.home.path().join(".armada/inbox.jsonl")).unwrap(),
        before,
        "a clear --all wrote to the inbox"
    );
    // And the raised item is still open and still resolvable, which is the fact
    // the byte comparison above is a proxy for.
    let shown = machine.run(&repo, &["failures", "show", "4f2a", "--json"]);
    assert!(shown.status.success(), "{}", why(&shown));
    let payload: serde_json::Value = serde_json::from_str(&stdout(&shown)).unwrap();
    assert_eq!(
        payload["data"]["results"][0]["state"], "FIXING",
        "{payload}"
    );
}
