//! **M2's done-when, run rather than reasoned about** (`PHASES.md` §8.4):
//!
//! > on a machine that has never seen Armada, `armada init` → pull → a working
//! > setup, and a `git diff` in the guild repo shows what changed since the
//! > other machine.
//!
//! Two scratch `$HOME`s, one bare repository standing in for the private
//! remote, and the real binary against real `git` — because the whole of Guild
//! is argv, and a fake that answers `0` to everything proves only that the code
//! path was entered.
//!
//! # Every test here points `$HOME` at a `TempDir`
//!
//! `support::Machine` does it for every invocation, and it is not a nicety:
//! `armada init` creates `~/.armada/` and `armada guild init --force` replaces
//! a guild. Destroying somebody's real one is unrecoverable, and the defence is
//! that nothing in this file can name a path it did not create.

mod support;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use support::Machine;

/// A bare repository, standing in for the private remote the interview names.
fn remote(at: &Path) -> String {
    std::fs::create_dir_all(at).unwrap();
    git(at, &["init", "-q", "--bare", "-b", "main"]);
    at.display().to_string()
}

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "Armada")
        .env("GIT_AUTHOR_EMAIL", "armada@example.test")
        .env("GIT_COMMITTER_NAME", "Armada")
        .env("GIT_COMMITTER_EMAIL", "armada@example.test")
        .output()
        .expect("git runs");
    assert!(
        status.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn guild_of(machine: &Machine) -> PathBuf {
    machine.home.path().join(".armada/guild")
}

/// A `~/.claude/` for a machine to adopt, with one credential in it.
fn a_claude_setup(machine: &Machine) {
    let claude = machine.home.path().join(".claude");
    std::fs::create_dir_all(claude.join("skills/add-migration")).unwrap();
    std::fs::create_dir_all(claude.join("hooks")).unwrap();
    std::fs::write(claude.join("skills/add-migration/SKILL.md"), "# add\n").unwrap();
    std::fs::write(claude.join("hooks/stop-notify.sh"), "#!/bin/sh\n").unwrap();
    std::fs::write(
        claude.join("settings.json"),
        format!(
            // Assembled, not written: a literal token here blocks the push.
            // See `guild::secrets`'s `shaped` for the measurement.
            r#"{{"model":"opus","env":{{"EDITOR":"nvim","GITHUB_TOKEN":"{}{}"}}}}"#,
            "ghp", "16C7e42F292c6912E7710c838347Ae178B4a"
        ),
    )
    .unwrap();
    std::fs::write(
        claude.join("CLAUDE.md"),
        "## Verbosity\n\n150 words.\n\n## Branching\n\nTrunk based.\n",
    )
    .unwrap();
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn envelope(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "not an envelope ({e}):\nstdout: {}\nstderr: {}",
            stdout(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Every file under a directory, recursively, guild-relative.
fn files(root: &Path) -> Vec<String> {
    fn walk(root: &Path, at: &Path, out: &mut Vec<String>) {
        let Ok(listing) = std::fs::read_dir(at) else {
            return;
        };
        for entry in listing.filter_map(Result::ok) {
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.path().is_dir() {
                walk(root, &entry.path(), out);
            } else if let Ok(relative) = entry.path().strip_prefix(root) {
                out.push(relative.display().to_string());
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// **The failure `PHASES.md` §8.4 records, closed end to end.** A guild skill
/// used to be invisible to the tool Armada hands you to: guild skills live in
/// `~/.armada/guild/skills/` and Claude Code reads `~/.claude/skills/`, and
/// nothing copied between them — so `/onboard-repo` answered `Unknown command`.
///
/// Run against the real binary and a scratch `$HOME`, because the whole of this
/// is filesystem layout and a fake that answers `0` proves only that the code
/// path was entered.
#[test]
fn a_guild_init_leaves_the_guild_where_claude_code_reads_it() {
    let machine = Machine::new();
    a_claude_setup(&machine);
    let outside = machine.outside();

    let built = machine.run(&outside, &["guild", "init", "--defaults", "--json"]);
    assert!(
        built.status.success(),
        "guild init failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let claude = machine.home.path().join(".claude");
    // The starter skill `guild init` copies, now on the load path under the
    // name `armada manifest config scan` hands over by.
    assert!(
        claude.join("skills/onboard-repo/SKILL.md").is_file(),
        "the starter skill is not where Claude Code reads skills: {:?}",
        files(&claude)
    );
    // And the orchestrator persona, under Claude Code's word for it.
    assert!(
        claude.join("agents/helm.md").is_file(),
        "`subagents/` did not land in `agents/`: {:?}",
        files(&claude)
    );
    // **What Claude Code has no load path for stays out of it.** Workflows are
    // Armada's own to read, and a `voice.md` in `~/.claude/` is read by nothing.
    assert!(!claude.join("workflows").exists());
    assert!(!claude.join("voice.md").exists());

    let envelope = envelope(&built);
    assert_eq!(envelope["data"]["projected"]["at"], "~/.claude/");
    assert_eq!(envelope["data"]["projected"]["kept"], 0);
}

/// **The rule that must not break.** Place, edit by hand, project again: the
/// edit survives, and it is reported rather than silently kept.
///
/// Getting this wrong destroys work somebody did by hand, silently, on a
/// machine they were not looking at — which is why `PLAN.md` §13.2 specifies a
/// hash of each file rather than a copy.
#[test]
fn a_file_you_edited_survives_a_re_projection_and_is_reported() {
    let machine = Machine::new();
    a_claude_setup(&machine);
    let outside = machine.outside();
    machine.run(&outside, &["guild", "init", "--defaults", "--json"]);

    let claude = machine.home.path().join(".claude");
    let skill = claude.join("skills/onboard-repo/SKILL.md");
    let mine = "# onboard\n\nAnd always ask about the database.\n";
    std::fs::write(&skill, mine).unwrap();
    // The guild moves on underneath it, exactly as a `guild pull` would leave
    // it.
    std::fs::write(
        guild_of(&machine).join("skills/onboard-repo/SKILL.md"),
        "# onboard\n\nSomebody else's version.\n",
    )
    .unwrap();

    let again = machine.run(&outside, &["guild", "project", "--json"]);
    assert!(
        again.status.success(),
        "guild project failed: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&skill).unwrap(),
        mine,
        "the edit was overwritten — this is the failure the manifest exists to prevent"
    );
    let envelope = envelope(&again);
    assert_eq!(
        envelope["data"]["kept"], 1,
        "the file that was left alone was not reported: {envelope}"
    );
    let rows = envelope["data"]["results"].as_array().unwrap();
    assert!(
        rows.iter()
            .any(|row| row["status"] == "CONFLICT" && row["item"] == "skills"),
        "no conflict row: {envelope}"
    );
}

/// **`--remove` reverses exactly what was placed, and nothing else.** A skill
/// the reader had before Armada ever ran is still there afterwards.
#[test]
fn removing_a_projection_takes_back_only_what_it_placed() {
    let machine = Machine::new();
    a_claude_setup(&machine);
    let outside = machine.outside();

    let claude = machine.home.path().join(".claude");
    // `a_claude_setup` already put `skills/add-migration` here, and `guild
    // init` adopts it — so a projection would legitimately claim it. This one
    // is written after the import and is therefore never the guild's.
    std::fs::create_dir_all(claude.join("skills/only-mine")).unwrap();
    machine.run(&outside, &["guild", "init", "--defaults", "--json"]);
    std::fs::write(claude.join("skills/only-mine/SKILL.md"), "mine\n").unwrap();

    let removed = machine.run(&outside, &["guild", "project", "--remove", "--json"]);
    assert!(
        removed.status.success(),
        "guild project --remove failed: {}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(
        !claude.join("skills/onboard-repo/SKILL.md").exists(),
        "what was placed was not taken back"
    );
    assert_eq!(
        std::fs::read_to_string(claude.join("skills/only-mine/SKILL.md")).unwrap(),
        "mine\n",
        "--remove reached a file it never placed"
    );
    // The reader's own `settings.json` and `CLAUDE.md` are not projection's to
    // touch in either direction.
    assert!(claude.join("settings.json").is_file());
    assert!(claude.join("CLAUDE.md").is_file());
}

/// **The first half of the done-when**, and the constraint the milestone would
/// not be allowed to ship without: a machine that has never seen Armada gets a
/// working setup, and no credential-shaped value reaches the guild.
#[test]
fn a_machine_that_has_never_seen_armada_gets_a_working_guild() {
    let machine = Machine::new();
    a_claude_setup(&machine);
    let outside = machine.outside();

    let output = machine.run(&outside, &["init", "--defaults", "--json"]);
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let envelope = envelope(&output);
    assert_eq!(envelope["status"], "READY");
    assert_eq!(envelope["data"]["questions"], 5);
    assert_eq!(
        envelope["data"]["answered"], 0,
        "--defaults takes every default"
    );

    // The starters, the fragments, and what was adopted.
    let guild = guild_of(&machine);
    let contents = files(&guild);
    for expected in [
        "voice.md",
        "expectations.md",
        "how-i-work.md",
        "workflows/bug.yml",
        "workflows/design.yml",
        "workflows/feature.yml",
        "workflows/plan.yml",
        "workflows/workflow.schema.json",
        "skills/onboard-repo/SKILL.md",
        "subagents/helm.md",
        "skills/add-migration/SKILL.md",
        "hooks/stop-notify.sh",
    ] {
        assert!(
            contents.contains(&expected.to_string()),
            "`{expected}` is not in the guild: {contents:?}"
        );
    }

    // **The constraint.** Not in any file, in any spelling, anywhere under the
    // directory that syncs — including the git objects, which is why this walks
    // the whole tree rather than reading the two files it expects.
    let leaked = Command::new("grep")
        .args(["-rq", "ghp_16C7e42F"])
        .arg(&guild)
        .output()
        .expect("grep runs");
    assert!(
        !leaked.status.success(),
        "a credential-shaped value reached the guild"
    );

    // And it is recorded, by key, in the file that never syncs.
    let machine_yml =
        std::fs::read_to_string(machine.home.path().join(".armada/machine.yml")).unwrap();
    assert!(machine_yml.contains("env.GITHUB_TOKEN"), "{machine_yml}");
    assert!(
        !machine_yml.contains("ghp_16C7e42F"),
        "machine.yml carries the value: {machine_yml}"
    );

    // The three never-syncing entries sit outside the repository, so they
    // cannot be committed even by a bug.
    for never in ["manifest.db", "jobs", "workspaces", "machine.yml"] {
        assert!(
            !guild.join(never).exists(),
            "`{never}` is inside the directory that syncs"
        );
    }
}

/// **The second half of the done-when**: a second machine pulls, and a `git
/// diff` in the guild repo shows what changed since the first.
#[test]
fn a_second_machine_pulls_the_first_machines_guild() {
    let first = Machine::new();
    a_claude_setup(&first);
    let hub = remote(&first.root.path().join("hub.git"));
    let outside = first.outside();

    // Machine one: build a guild and push it.
    let built = first.run(
        &outside,
        &["guild", "init", "--defaults", "--remote", &hub, "--json"],
    );
    assert!(
        built.status.success(),
        "guild init failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let pushed = first.run(&outside, &["guild", "push", "--json"]);
    assert!(
        pushed.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&pushed.stderr)
    );

    // Machine two, which has never seen Armada, pulls it.
    let second = Machine::new();
    let elsewhere = second.outside();
    let cloned = second.run(&elsewhere, &["init", "--guild", &hub, "--json"]);
    assert!(
        cloned.status.success(),
        "the second machine could not pull: {}",
        String::from_utf8_lossy(&cloned.stderr)
    );

    // The same guild, file for file.
    assert_eq!(files(&guild_of(&first)), files(&guild_of(&second)));
    assert!(guild_of(&second).join("subagents/helm.md").is_file());

    // **And a working setup, not a directory.** The done-when says *a working
    // setup*, and a guild in `~/.armada/guild/` that nothing has projected is
    // one Claude Code cannot see (`PHASES.md` §8.4).
    assert!(
        second
            .home
            .path()
            .join(".claude/skills/onboard-repo/SKILL.md")
            .is_file(),
        "the cloned guild was never put on Claude Code's load path: {:?}",
        files(&second.home.path().join(".claude"))
    );
    assert!(second.home.path().join(".claude/agents/helm.md").is_file());
    assert_eq!(envelope(&cloned)["data"]["projected"]["at"], "~/.claude/");

    // Machine two edits, commits and pushes; machine one pulls and sees it.
    std::fs::write(guild_of(&second).join("voice.md"), "answer first\n").unwrap();
    let sent = second.run(&elsewhere, &["guild", "push", "--json"]);
    assert!(
        sent.status.success(),
        "the second machine could not push: {}",
        String::from_utf8_lossy(&sent.stderr)
    );

    let pulled = first.run(&outside, &["guild", "pull", "--json"]);
    assert!(
        pulled.status.success(),
        "pull failed: {}",
        String::from_utf8_lossy(&pulled.stderr)
    );
    let envelope = envelope(&pulled);
    assert_eq!(
        envelope["data"]["applied"], true,
        "a fast-forward is applied: {envelope}"
    );
    assert_eq!(
        std::fs::read_to_string(guild_of(&first).join("voice.md")).unwrap(),
        "answer first\n",
        "the pull did not take effect"
    );
    // **And a `git diff` shows what changed** — the done-when's own words.
    let diff = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(guild_of(&first))
        .output()
        .unwrap();
    assert!(diff.status.success());
}

/// **Conflicts surface as conflicts, never a silent overwrite.** Both machines
/// edit `voice.md`; the pull reports it and changes nothing.
#[test]
fn a_diverged_guild_is_reported_and_nothing_is_changed() {
    let first = Machine::new();
    a_claude_setup(&first);
    let hub = remote(&first.root.path().join("hub.git"));
    let outside = first.outside();

    first.run(
        &outside,
        &["guild", "init", "--defaults", "--remote", &hub, "--json"],
    );
    first.run(&outside, &["guild", "push", "--json"]);

    let second = Machine::new();
    let elsewhere = second.outside();
    second.run(&elsewhere, &["init", "--guild", &hub, "--json"]);

    // Each machine edits the same fragment, and the second gets there first.
    std::fs::write(guild_of(&second).join("voice.md"), "theirs\n").unwrap();
    second.run(&elsewhere, &["guild", "push", "--json"]);
    std::fs::write(guild_of(&first).join("voice.md"), "mine\n").unwrap();
    first.run(&outside, &["guild", "push", "--json"]);

    let pulled = first.run(&outside, &["guild", "pull", "--json"]);
    let envelope = envelope(&pulled);

    assert_eq!(
        pulled.status.code(),
        Some(1),
        "a divergence is `tool_failed`, and nothing changed: {envelope}"
    );
    assert_eq!(envelope["data"]["applied"], false);
    assert_eq!(envelope["data"]["headline"], "NEEDS ATTENTION");
    assert!(
        envelope["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["status"] == "CONFLICT"),
        "the conflict was not reported: {envelope}"
    );
    assert_eq!(
        std::fs::read_to_string(guild_of(&first).join("voice.md")).unwrap(),
        "mine\n",
        "the pull overwrote a file this machine had edited"
    );
}

/// **A folder is a remote, and it is the same remote a URL is.**
///
/// The honest answer to *do you have a private git remote?* is, for most
/// people, no — and they are not going to make one to finish setting a tool up.
/// A folder that is already on every machine they own is: iCloud Drive, a NAS,
/// a stick. Git speaks a filesystem remote natively, so this is not a lesser
/// mode, and this test proves it by running the whole two-machine path through
/// a plain directory that was never a repository: init it, push, clone from it,
/// edit on the second machine, push, pull on the first.
///
/// Run against a `TempDir` rather than an actual iCloud folder for the obvious
/// reason. What is iCloud-specific — eviction and a half-replicated push — is
/// `armada_guild::remote`'s, and is tested there.
#[test]
fn a_plain_folder_works_as_a_sync_remote_end_to_end() {
    let first = Machine::new();
    a_claude_setup(&first);
    let outside = first.outside();

    // **Not a repository, and not even created.** Naming a folder that does not
    // exist yet is what somebody typing a path into the interview does.
    let folder = first.root.path().join("Drive/guild");
    let named = folder.display().to_string();
    assert!(!folder.exists());

    let built = first.run(
        &outside,
        &["guild", "init", "--defaults", "--remote", &named, "--json"],
    );
    assert!(
        built.status.success(),
        "guild init against a folder failed: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(
        folder.join("HEAD").is_file() && folder.join("objects").is_dir(),
        "the folder was not made a bare repository"
    );

    let pushed = first.run(&outside, &["guild", "push", "--json"]);
    assert!(
        pushed.status.success(),
        "push to a folder failed: {}",
        String::from_utf8_lossy(&pushed.stderr)
    );

    // A second machine treats it as any other remote.
    let second = Machine::new();
    let elsewhere = second.outside();
    let cloned = second.run(&elsewhere, &["init", "--guild", &named, "--json"]);
    assert!(
        cloned.status.success(),
        "the second machine could not pull from a folder: {}",
        String::from_utf8_lossy(&cloned.stderr)
    );
    assert_eq!(files(&guild_of(&first)), files(&guild_of(&second)));

    // **Real merges and real history**, which is the whole argument for a git
    // remote over a file copy: the second machine's edit fast-forwards onto the
    // first rather than replacing it.
    std::fs::write(guild_of(&second).join("voice.md"), "answer first\n").unwrap();
    second.run(&elsewhere, &["guild", "push", "--json"]);
    let pulled = first.run(&outside, &["guild", "pull", "--json"]);
    assert!(
        pulled.status.success(),
        "pull from a folder failed: {}",
        String::from_utf8_lossy(&pulled.stderr)
    );
    assert_eq!(envelope(&pulled)["data"]["applied"], true);
    assert_eq!(
        std::fs::read_to_string(guild_of(&first).join("voice.md")).unwrap(),
        "answer first\n"
    );
}

/// A bundle, for a machine that will never hold your credentials — and the
/// rule that `machine.yml` does not travel unless it is asked for by name.
#[test]
fn a_bundle_carries_the_guild_and_not_this_machine() {
    let first = Machine::new();
    a_claude_setup(&first);
    let outside = first.outside();
    first.run(&outside, &["init", "--defaults", "--json"]);

    let exported = first.run(&outside, &["guild", "export", "--json"]);
    assert!(
        exported.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let envelope = envelope(&exported);
    assert_eq!(envelope["data"]["secrets"], false);
    let bundle = outside.join("guild.tar.zst");
    assert!(bundle.is_file(), "no bundle was written");

    // A second machine restores from it, with no network and no git credential.
    let second = Machine::new();
    let elsewhere = second.outside();
    let imported = second.run(
        &elsewhere,
        &["guild", "import", bundle.to_str().unwrap(), "--json"],
    );
    assert!(
        imported.status.success(),
        "import failed: {}",
        String::from_utf8_lossy(&imported.stderr)
    );

    assert_eq!(files(&guild_of(&first)), files(&guild_of(&second)));
    assert!(
        !second.home.path().join(".armada/machine.yml").exists()
            || !std::fs::read_to_string(second.home.path().join(".armada/machine.yml"))
                .unwrap()
                .contains("GITHUB_TOKEN"),
        "the first machine's withheld record travelled in the bundle"
    );
}

/// `armada doctor` reports the fragments a skipped interview left as imported —
/// which is the promise that keeps `--defaults` from finishing silently in a
/// state that looks configured and is not.
#[test]
fn doctor_names_the_fragments_a_skipped_interview_left_alone() {
    let machine = Machine::new();
    a_claude_setup(&machine);
    let outside = machine.outside();
    machine.run(&outside, &["init", "--defaults", "--json"]);

    let output = machine.run(&outside, &["doctor", "--json"]);
    let envelope = envelope(&output);
    let rows = envelope["data"]["results"].as_array().unwrap();

    for fragment in ["voice.md", "expectations.md", "how-i-work.md"] {
        assert!(
            rows.iter().any(|row| {
                row["status"] == "PARTIAL"
                    && row["detail"].as_str().is_some_and(|d| d.contains(fragment))
            }),
            "`{fragment}` was not reported as still imported: {envelope}"
        );
    }
    // **A warning alone does not fail**, so `doctor` stays safe in a prompt.
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(envelope["data"]["headline"], "NEEDS ATTENTION");
}

/// The human render of `armada doctor` carries the `→` lines that are the point
/// of it — folded to ASCII, because a captured stdout is an agent's.
#[test]
fn doctors_human_render_names_the_command_that_fixes_each_problem() {
    let machine = Machine::new();
    let outside = machine.outside();

    let output = machine.run(&outside, &["doctor"]);
    let text = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        stdout(&output)
    };
    assert!(text.contains("NEEDS ATTENTION"), "{text}");
    assert!(text.contains("-> armada init"), "{text}");
    assert!(
        !text.contains('→'),
        "a captured stdout is an agent's: {text}"
    );
}
