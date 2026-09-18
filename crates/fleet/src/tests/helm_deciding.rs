//! What Fleet puts to a person and what it lets Helm run. `#1525`.
//!
//! **The table is the test.** The owner's rule is three sentences and this is
//! them, case by case: what runs is what he said should run, and what asks is
//! what he named. A case added here is a claim about his rule, not about the
//! code.

use ipc::AskingToRun;

use crate::helm::{because, Because};

/// One ask, **read the way the route reads it** — `helm_permission`'s rule and
/// its reason: `fleet` reads no JSON of its own, so a fixture here is the CLI's
/// own bytes rather than a literal.
fn asking(body: &str) -> AskingToRun {
    ipc::decode("a permission question", body.as_bytes()).expect("the fixture decodes")
}

/// A JSON string literal. The lines below carry quotes, backslashes and
/// newlines, which is the whole point of them.
fn quoted(said: &str) -> String {
    let mut out = String::from("\"");
    for c in said.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn shell(command: &str) -> AskingToRun {
    asking(&format!(
        "{{\"tool_name\":\"Bash\",\"input\":{{\"command\":{}}}}}",
        quoted(command)
    ))
}

fn tool(named: &str) -> AskingToRun {
    asking(&format!(
        "{{\"tool_name\":{},\"input\":{{}}}}",
        quoted(named)
    ))
}

fn writing(tool_name: &str, path: &str) -> AskingToRun {
    asking(&format!(
        "{{\"tool_name\":{},\"input\":{{\"file_path\":{}}}}}",
        quoted(tool_name),
        quoted(path)
    ))
}

/// The whole point: a person talking to Helm is not asked about the ordinary
/// work of answering them. Every line here was one the owner would have had a
/// card for on 18 Sep 2026.
#[test]
fn the_ordinary_work_of_answering_a_question_runs() {
    for line in [
        // The one from his own screenshot: the door cut a `get_diff` and Helm
        // shelled out to read what was cut.
        "python3 - <<'EOF'\nimport json\nEOF",
        "cat /tmp/out.txt",
        "rg --files-with-matches helm crates",
        "grep -rn AskingToRun crates/fleet",
        "ls -la crates/fleet/src/helm",
        "head -100 armada.yml",
        "git log --oneline -20",
        "git status --porcelain",
        "git diff main -- crates/",
        "git show 782186d1 --stat",
        "gh pr view 1524 --json title",
        "gh issue list --milestone Reach",
        "gh pr checks 1524",
        "cargo test -p fleet",
        "cargo build --workspace",
        "pnpm --dir packages/screens exec vitest run",
        "jq .name package.json",
        "curl -s http://localhost:40000/health",
        "cd crates/fleet && cargo check",
        "wc -l crates/fleet/src/helm/*.rs | sort -n",
    ] {
        assert_eq!(because(&shell(line)), None, "should have run: {line}");
    }
}

/// His first class. **Each of these takes something that does not come back.**
#[test]
fn a_shell_line_that_removes_or_overwrites_asks() {
    for line in [
        "rm -rf target",
        "rm crates/fleet/src/helm/deciding.rs",
        "cd crates && rm -rf target",
        "FOO=1 rm -rf /tmp/x",
        "/bin/rm -rf x",
        "git reset --hard origin/main",
        "git clean -fdx",
        "git checkout -- crates/",
        "git branch -D helm/some-branch",
        "git worktree remove --force /tmp/worktrees/x",
        "git stash",
        "sudo launchctl kickstart -k gui/501/com.armada.fleet",
        "kill -9 28658",
        "pkill -f armada",
        "mv armada.yml armada.yml.bak",
        "chmod -R 777 crates",
        "echo nothing > armada.yml",
        "cargo build > build.log",
    ] {
        assert_eq!(
            because(&shell(line)),
            Some(Because::Destructive),
            "should have asked: {line}"
        );
    }
}

/// **`>>` appends and `>&` is a descriptor**, so neither destroys what a file
/// held and neither asks.
#[test]
fn appending_and_redirecting_a_descriptor_are_not_overwriting() {
    for line in ["cargo build >> build.log", "cargo build 2>&1 | tail -5"] {
        assert_eq!(because(&shell(line)), None, "should have run: {line}");
    }
}

/// His second class.
#[test]
fn sending_code_where_other_people_read_it_asks() {
    for line in [
        "git push -u origin helm/some-branch",
        "git push --force",
        "gh pr merge 1524 --merge",
        "gh pr create --title x --body y",
        "gh release create v1",
        "cargo publish",
        "npm publish --access public",
        "scp build.tar host:/srv",
        "rsync -a out/ host:/srv",
        "ssh host 'systemctl restart armada'",
    ] {
        assert_eq!(
            because(&shell(line)),
            Some(Because::PushesToShared),
            "should have asked: {line}"
        );
    }
}

/// His third class. **A GET is a read and a body is a write**, which is the
/// line between the `curl` that runs and the `curl` that asks.
#[test]
fn writing_to_something_off_this_machine_asks() {
    for line in [
        "curl -X POST https://api.example.com/things",
        "curl -d '{\"a\":1}' https://api.example.com/things",
        "curl --json '{}' https://api.example.com",
        "gh issue create --title x",
        "gh api -X PATCH repos/o/r/issues/1",
        "gh repo delete o/r",
        "kubectl apply -f deploy.yaml",
        "aws s3 cp out.tar s3://bucket/",
    ] {
        assert_eq!(
            because(&shell(line)),
            Some(Because::WritesOffMachine),
            "should have asked: {line}"
        );
    }
}

/// **A classifier reading one word would run `cd x && rm -rf y`.** Every
/// segment is read, and the reason the whole line carries is the first segment
/// that asks.
#[test]
fn a_chained_line_is_read_segment_by_segment() {
    assert_eq!(
        because(&shell("git status && git push")),
        Some(Because::PushesToShared)
    );
    assert_eq!(
        because(&shell("cargo test; rm -rf target")),
        Some(Because::Destructive)
    );
    assert_eq!(because(&shell("cat x | jq . | head -5")), None);
}

/// **A line whose shape cannot be read asks on that ground**, because those are
/// the lines that can carry anything.
#[test]
fn a_line_this_cannot_read_asks_rather_than_guessing() {
    for line in [
        "eval \"$COMMAND\"",
        "`which rm` -rf target",
        "python3 -c \"$(cat script.py)\"",
        "exec rm -rf target",
    ] {
        assert_eq!(
            because(&shell(line)),
            Some(Because::Unreadable),
            "should have asked: {line}"
        );
    }
}

/// **`evaluate.sh` is not an `eval`.** A word inside a longer word is not that
/// word, or half the shell lines in this repository would ask.
#[test]
fn a_risky_word_inside_a_longer_one_is_not_that_word() {
    for line in [
        "./scripts/evaluate.sh",
        "cat execution-plan.md",
        "ls sudoku/",
    ] {
        assert_eq!(because(&shell(line)), None, "should have run: {line}");
    }
}

/// Armada's own door: **every read runs**, and only the acts in one of the
/// three classes ask. The Job control a person asks Helm for — pause, add a
/// task, redirect a Drone — is none of them.
#[test]
fn the_doors_reads_run_and_only_some_of_its_acts_ask() {
    for operation in [
        "get_job",
        "get_check_output",
        "get_diff",
        "list_jobs",
        "get_job_log",
        "pause_job",
        "add_task",
        "redirect_drone",
        "add_studio_node",
        "start_run",
        "propose_job",
        "restart_step",
    ] {
        let named = format!("mcp__{}__{operation}", ipc::door::SERVER);
        assert_eq!(because(&tool(&named)), None, "should have run: {operation}");
    }
    for (operation, why) in [
        ("merge_pull_request", Because::PushesToShared),
        ("approve_dispatch", Because::PushesToShared),
        ("kill_job", Because::Destructive),
        ("delete_branch", Because::Destructive),
        ("edit_manifest", Because::Destructive),
        ("file_finding_issue", Because::WritesOffMachine),
        ("clone_repository", Because::WritesOffMachine),
    ] {
        let named = format!("mcp__{}__{operation}", ipc::door::SERVER);
        assert_eq!(
            because(&tool(&named)),
            Some(why),
            "should have asked: {operation}"
        );
    }
}

/// A tool from a server Armada knows nothing about, read by its name — the only
/// thing there is to read. **Under *auto unless*, a name carrying none of the
/// verbs runs.**
#[test]
fn another_servers_tool_is_read_by_the_verb_in_its_name() {
    for named in [
        "mcp__gitnexus__query",
        "mcp__gitnexus__list_repos",
        "Read",
        "Grep",
        "Glob",
        "WebSearch",
        "TodoWrite",
    ] {
        assert_eq!(because(&tool(named)), None, "should have run: {named}");
    }
    assert_eq!(
        because(&tool("mcp__notion__delete_page")),
        Some(Because::Destructive)
    );
    assert_eq!(
        because(&tool("mcp__slack__send_message")),
        Some(Because::WritesOffMachine)
    );
}

/// **An overwrite has no undo and keeps its card; a new file does not.** The
/// owner's 17 Sep decision and his later rule agree on the first and differ
/// only on the second.
#[test]
fn overwriting_a_file_asks_and_writing_a_new_one_does_not() {
    let home = crate::tests::tmp::TempDir::new();
    let existing = home.path().join("already-here.txt");
    std::fs::write(&existing, "something").expect("the fixture writes");
    let fresh = home.path().join("not-yet.txt");

    for tool_name in ["Write", "Edit", "NotebookEdit"] {
        assert_eq!(
            because(&writing(tool_name, &existing.to_string_lossy())),
            Some(Because::Destructive),
            "{tool_name} over a file that is there"
        );
        assert_eq!(
            because(&writing(tool_name, &fresh.to_string_lossy())),
            None,
            "{tool_name} to a path that is not"
        );
    }
}

/// **Being unable to tell is not a reason to assume the harmless one.** A write
/// naming no path Fleet can read is an overwrite.
#[test]
fn a_write_naming_no_readable_path_asks() {
    assert_eq!(
        because(&tool("Write")),
        Some(Because::Destructive),
        "a write with no path in it"
    );
}
