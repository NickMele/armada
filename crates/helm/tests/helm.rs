//! `armada helm` end to end, against a scratch `$HOME`.
//!
//! **No test here starts a Claude Code session, and none can.** Entering is
//! refused unless this machine has run `armada helm enable`, and a fresh scratch
//! machine never has — so every test that wants an envelope asks for one with
//! `--json` or `--print-command`, neither of which enters on any machine. That
//! refusal is asserted rather than assumed — see
//! [`entering_is_refused_by_name_and_says_why`], which is the deliverable of
//! this file and not a detail of it. A gate with no test is a comment, and the
//! first thing anybody would learn about it silently coming back on is a bill.
//!
//! **The suite proves the switch decides, and stops there — deliberately.**
//! [`enable_and_disable_flip_the_switch_machine_yml_records`] proves `armada
//! helm enable` turns [`armada_helm::verbs::helm::entering_allowed`] on and
//! `disable` turns it back, and that every surface that reports the state —
//! `armada helm --json`'s envelope, `armada doctor`'s row — agrees with it.
//!
//! **One test does run the entering path**, and it is safe for one reason:
//! [`support`]'s stub `claude` is first on this suite's `PATH`, so the `exec`
//! lands on nine lines of `sh` that exit 0 and talk to nothing. See
//! [`a_machine_that_has_said_yes_is_not_asked_a_second_time`], which exists
//! because proving the decision alone was what let a second lock sit behind the
//! first one unnoticed.
//!
//! [`entering_is_refused_by_name_and_says_why`]: entering_is_refused_by_name_and_says_why
//! [`enable_and_disable_flip_the_switch_machine_yml_records`]: enable_and_disable_flip_the_switch_machine_yml_records
//!
//! The scratch machine is [`support::Machine`]: its own `$HOME`, so the guild,
//! the projection and `~/.armada/helm/` are all a `TempDir` and never the
//! developer's own. That property is what makes it safe to run a verb whose
//! whole subject is the reader's `~/.claude/`.
//!
//! **What this suite is for that the unit tests are not.** `armada_core::helm`
//! proves the argv is the vector Armada meant to build. This proves the *driver
//! feeds it*: that the verb reads the record it wrote last time, that the four
//! documents land where the argv says they are, and that the `Stop` hook it
//! generated is a script `/bin/sh` actually runs. A green reducer whose driver
//! never calls it is the failure this tier exists to catch.

mod support;

use serde_json::Value;
use std::path::{Path, PathBuf};
use support::Machine;

/// A machine with a guild, a Helm persona, and that persona projected onto
/// Claude Code's load path — the three things `armada helm` refuses without —
/// **and three memory fragments their owner has actually written**.
///
/// The fragments are here rather than in one test because a guild whose owner
/// has written it is the ordinary case, and the launch is different on such a
/// machine: their words are in the argv. The other case — a guild still holding
/// Armada's example text — is [`an_unwritten_guild_injects_nothing_and_says_so`].
fn a_machine_ready_for_helm(machine: &Machine) {
    let home = machine.home.path();
    let guild = home.join(".armada/guild");
    std::fs::create_dir_all(guild.join(".git")).unwrap();
    std::fs::create_dir_all(guild.join("subagents")).unwrap();
    std::fs::write(
        guild.join("subagents/helm.md"),
        include_str!("../../../templates/guild/subagents/helm.md"),
    )
    .unwrap();
    for (name, body) in FRAGMENTS {
        std::fs::write(guild.join(name), body).unwrap();
    }
    project(machine, "helm");
}

/// Three memory fragments as somebody who had written them would leave them:
/// no `armada:unedited` marker, and a sentence each that appears nowhere else.
///
/// **Written for this suite and belonging to nobody.** This repository is
/// public, and a fixture is the easiest place for a real person's own memory
/// file to end up in it; a distinctive sentence is all a test needs to prove the
/// prose travelled.
const FRAGMENTS: [(&str, &str); 3] = [
    ("voice.md", "# Voice\n\nOne line, then stop.\n"),
    (
        "expectations.md",
        "# Expectations\n\nA green suite, or name the red test.\n",
    ),
    (
        "how-i-work.md",
        "# How I work\n\nNever on the default branch.\n",
    ),
];

/// Put a persona where Claude Code reads it, which is what `armada guild
/// project` does.
fn project(machine: &Machine, agent: &str) {
    let agents = machine.home.path().join(".claude/agents");
    std::fs::create_dir_all(&agents).unwrap();
    std::fs::write(agents.join(format!("{agent}.md")), "---\nname: x\n---\n").unwrap();
}

fn helm_json(machine: &Machine, args: &[&str]) -> Value {
    let mut argv = vec!["helm", "--json"];
    argv.extend_from_slice(args);
    let out = machine.run(machine.root.path(), &argv);
    assert!(
        out.status.success(),
        "`armada {argv:?}` failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("an envelope")
}

fn argv_of(envelope: &Value) -> Vec<String> {
    envelope["data"]["argv"]
        .as_array()
        .expect("the launch argv")
        .iter()
        .map(|word| word.as_str().unwrap().to_string())
        .collect()
}

fn helm_home(machine: &Machine) -> PathBuf {
    machine.home.path().join(".armada/helm")
}

/// **The whole launch, exactly, against a real `$HOME`.** The unit tests assert
/// the vector for given paths; this asserts that the paths the verb chose are
/// the ones it wrote files to, which is the half a pure test cannot make.
#[test]
fn the_first_launch_is_assembled_and_nothing_is_started() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let envelope = helm_json(&machine, &[]);
    let data = &envelope["data"];
    assert_eq!(envelope["status"], "OK");
    assert_eq!(data["agent"], "helm");
    assert_eq!(data["conversation"], "NEW");
    assert_eq!(
        data["launched"], false,
        "a verb reported starting a session"
    );

    let root = helm_home(&machine);
    let argv = argv_of(&envelope);

    // **The reader's own words, in the argv, as bytes.** This is the assertion
    // the change exists for: the persona instructs Helm to read the three
    // fragments and Helm has no `Read` tool, so until they were injected the
    // instruction was inert and nothing said so.
    assert_eq!(argv[3], "--append-system-prompt", "{argv:?}");
    for (name, body) in FRAGMENTS {
        let sentence = body.lines().last().unwrap();
        assert!(
            argv[4].contains(sentence),
            "`{name}` did not reach the launch: {}",
            argv[4]
        );
    }

    // Everything else is the launch it always was, in the same order.
    let mut without_voice = argv.clone();
    without_voice.drain(3..5);
    assert_eq!(
        without_voice,
        [
            "claude".to_string(),
            "--agent".to_string(),
            "helm".to_string(),
            "--mcp-config".to_string(),
            root.join("mcp.json").display().to_string(),
            "--plugin-dir".to_string(),
            root.join("plugin").display().to_string(),
            "--settings".to_string(),
            root.join("settings.json").display().to_string(),
            // **The mode the session enters under, which the launch used to
            // pass not at all.** Without it Helm inherited Claude Code's own
            // default and the reader approved every tool call by hand.
            "--permission-mode".to_string(),
            "auto".to_string(),
            // **And the model, for the same reason.** Left off, the session ran
            // on whatever the account default happened to be — a choice nobody
            // made and nobody could see. `helm.model` overrides it.
            "--model".to_string(),
            "sonnet".to_string(),
            "--session-id".to_string(),
            data["uuid"].as_str().unwrap().to_string(),
        ]
    );
}

/// **The printed line is the argv, and the file is what makes it so.**
///
/// `armada helm` prints one line for a person to paste. It cannot paste twenty
/// kilobytes of their own prose, so the appended prompt is written to
/// `~/.armada/helm/system-prompt.md` and the line reads it back with `"$(cat …)"`
/// — which reproduces the argv byte for byte, or the two renderings of one
/// hand-over disagree in the one place a reader has no way to check.
#[test]
fn the_line_a_reader_pastes_reproduces_the_launch_exactly() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let envelope = helm_json(&machine, &[]);
    let argv = argv_of(&envelope);
    let command = envelope["data"]["command"].as_str().expect("the line");
    let document = helm_home(&machine).join("system-prompt.md");

    assert!(
        command.contains("--append-system-prompt \"$(cat ~/.armada/helm/system-prompt.md)\""),
        "{command}"
    );
    assert!(
        !command.contains("One line, then stop."),
        "the prose was pasted into the line: {command}"
    );
    assert!(!command.contains('\n'), "{command}");

    // `$(cat …)` strips the trailing newline every text file ends with, which
    // is why the document is the prompt plus exactly one.
    let on_disk = std::fs::read_to_string(&document).expect("the launch wrote it");
    assert_eq!(on_disk.trim_end_matches('\n'), argv[4]);
    assert_eq!(on_disk, format!("{}\n", argv[4]));
    // And no absolute home in a line meant to be pasted and screen-shared.
    assert!(!command.contains("/Users/"), "{command}");
    assert!(!command.contains("/home/"), "{command}");
}

/// **A guild nobody has written yet injects nothing, and the launch says so.**
///
/// The three fragments ship holding Armada's example text, which `armada guild
/// ls` reports as *still Armada's example text*. Appending it would make
/// Armada's boilerplate binding in the reader's name, and the persona's own
/// defaults already cover the same ground. The row is the half that matters: a
/// launch that quietly appended nothing looks exactly like one that appended
/// everything, and that is how the instruction sat unread in the persona for as
/// long as it did.
#[test]
fn an_unwritten_guild_injects_none_of_the_readers_words_and_says_so() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let guild = machine.home.path().join(".armada/guild");
    for (name, _) in FRAGMENTS {
        // What `armada guild init` leaves behind when import found nothing.
        std::fs::write(
            guild.join(name),
            "<!-- armada:unedited example -->\n\n# Voice\n\n- Lead with the answer.\n",
        )
        .unwrap();
    }

    let envelope = helm_json(&machine, &[]);
    let argv = argv_of(&envelope);
    assert!(
        !argv
            .iter()
            .any(|word| word.contains("Lead with the answer")),
        "Armada's own example text was made binding: {argv:?}"
    );
    // **The flag is there, carrying Armada's own skill and nothing of theirs**
    // (`docs/reserved/008`). It used to be absent entirely, which is what this
    // test asserted; Armada's instructions to the agents it runs do not wait on
    // the reader describing themselves.
    let at = argv
        .iter()
        .position(|word| word == "--append-system-prompt")
        .unwrap_or_else(|| panic!("{argv:?}"));
    assert_eq!(argv[at + 1], armada_core::skill::HELM);

    let row = envelope["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["what"] == "voice")
        .expect("a launch that appended none of their words still says so");
    let detail = row["detail"].as_str().unwrap();
    assert!(detail.contains("none of yours yet"), "{detail}");
    assert!(detail.contains("armada guild edit voice.md"), "{detail}");
    // **The document is written either way**, because `"$(cat …)"` in the
    // printed line has to find it — it used to be removed here, back when an
    // unwritten guild meant no flag and nothing to substitute.
    assert!(helm_home(&machine).join("system-prompt.md").exists());
}

/// A fragment the reader deleted, or one that is nothing but whitespace, is
/// skipped as quietly as one that is still Armada's — and the two that remain
/// still travel. A missing file is not a reason to refuse a launch.
#[test]
fn the_fragments_that_exist_travel_and_the_missing_ones_are_skipped() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let guild = machine.home.path().join(".armada/guild");
    std::fs::remove_file(guild.join("expectations.md")).unwrap();
    std::fs::write(guild.join("how-i-work.md"), "   \n\n").unwrap();

    let envelope = helm_json(&machine, &[]);
    let argv = argv_of(&envelope);
    assert!(argv[4].contains("One line, then stop."), "{}", argv[4]);
    assert!(!argv[4].contains("how-i-work.md"), "{}", argv[4]);

    let row = envelope["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["what"] == "voice")
        .expect("the row is always there");
    assert_eq!(
        row["detail"], "voice.md: your words, and they outrank the persona",
        "{row:#}"
    );
}

/// **Every path the argv names is a file that exists.** An argv pointing at a
/// `--mcp-config` nothing wrote hands the orchestrator an empty toolbelt and
/// reports no error at all, which is the failure worth its own assertion.
#[test]
fn every_path_the_launch_names_was_actually_written() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let argv = argv_of(&helm_json(&machine, &[]));

    for (flag, expected) in [
        ("--mcp-config", "file"),
        ("--settings", "file"),
        ("--plugin-dir", "dir"),
    ] {
        let at = argv
            .iter()
            .position(|word| word == flag)
            .map(|index| PathBuf::from(&argv[index + 1]))
            .unwrap_or_else(|| panic!("{argv:?} has no {flag}"));
        match expected {
            "dir" => assert!(at.is_dir(), "{flag} names {at:?}, which is not a directory"),
            _ => assert!(at.is_file(), "{flag} names {at:?}, which is not a file"),
        }
    }
    // The plugin's two documents, at the two paths Claude Code looks for them.
    let plugin = helm_home(&machine).join("plugin");
    assert!(plugin.join(".claude-plugin/plugin.json").is_file());
    assert!(plugin.join("monitors/monitors.json").is_file());
}

/// The toolbelt registers **this** binary, and it is the one the suite just
/// built — not whatever `armada` a session's `PATH` would find.
#[test]
fn the_toolbelt_registers_the_binary_that_wrote_it() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    helm_json(&machine, &[]);

    let text = std::fs::read_to_string(helm_home(&machine).join("mcp.json")).unwrap();
    let json: Value = serde_json::from_str(&text).expect("mcp.json is JSON");
    assert_eq!(
        json["mcpServers"]["armada"]["command"],
        support::armada_binary().display().to_string()
    );
    assert_eq!(
        json["mcpServers"]["armada"]["args"],
        serde_json::json!(["mcp", "serve", "--stdio"])
    );
}

/// The monitor tails **this machine's** inbox, absolutely, with `-F` so it
/// waits for a file the first Job has not created yet.
#[test]
fn the_monitor_tails_this_machines_inbox() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    helm_json(&machine, &[]);

    let text =
        std::fs::read_to_string(helm_home(&machine).join("plugin/monitors/monitors.json")).unwrap();
    let json: Value = serde_json::from_str(&text).expect("monitors.json is JSON");
    let inbox = machine.home.path().join(".armada/inbox.jsonl");
    assert_eq!(
        json[0]["command"],
        format!("tail -F {}", inbox.display()),
        "the monitor is not watching this machine's inbox"
    );
}

/// **The backstop is run, not merely written.** Asserting on generated shell
/// proves the bytes are the ones intended; it does not prove `/bin/sh` accepts
/// them — the same gap `docs/traps.md` records for argv, arriving at a second
/// kind of generated command.
///
/// It costs nothing and starts no session: this is `/bin/sh`, `grep` and
/// `printf` against a scratch inbox.
#[test]
fn the_stop_hook_that_was_written_actually_blocks_a_turn() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    helm_json(&machine, &[]);

    let settings: Value = serde_json::from_str(
        &std::fs::read_to_string(helm_home(&machine).join("settings.json")).unwrap(),
    )
    .expect("settings.json is JSON");
    let hook = settings["hooks"]["Stop"][0]["hooks"][0]["command"]
        .as_str()
        .expect("a Stop hook command")
        .to_string();
    let hook = Path::new(&hook);
    assert!(hook.is_file(), "the registered Stop hook is not there");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(hook).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "a Stop hook that is not executable is a backstop that silently is not one"
        );
    }

    let run = || {
        let out = std::process::Command::new("/bin/sh")
            .arg(hook)
            .output()
            .expect("/bin/sh runs");
        assert!(out.status.success(), "the hook exited non-zero");
        String::from_utf8(out.stdout).unwrap()
    };

    // No inbox at all is silence: a machine whose first Job has never run must
    // not have every turn held open.
    assert_eq!(run(), "", "an absent inbox blocked a turn");

    let inbox = machine.home.path().join(".armada/inbox.jsonl");
    std::fs::create_dir_all(inbox.parent().unwrap()).unwrap();
    std::fs::write(&inbox, "{\"job\":\"a\",\"answered\":true}\n").unwrap();
    assert_eq!(run(), "", "an answered inbox blocked a turn");

    std::fs::write(
        &inbox,
        "{\"job\":\"a\",\"answered\":true}\n{\"job\":\"b\",\"answered\":false}\n",
    )
    .unwrap();
    let blocked: Value = serde_json::from_str(run().trim()).expect("the hook emits JSON");
    assert_eq!(
        blocked["decision"], "block",
        "the backstop reported without blocking, which changes nothing"
    );
    let reason = blocked["reason"].as_str().unwrap();
    assert!(reason.starts_with("1 unread inbox entry."), "{reason}");
    // **A count and the tool, never the bodies** — PLAN.md §15.2's first rule
    // arriving at the backstop. A hook that pasted every unread entry would put
    // raw Drone output into Helm's window at the end of every turn.
    assert!(reason.contains("fleet.inbox"), "{reason}");
    assert!(
        !reason.contains("\"job\""),
        "the hook pasted the entries: {reason}"
    );
}

/// **The same conversation each day** (PLAN.md §15.1). A record that says the
/// session exists produces `--resume`, and the id is the one on disk — which is
/// the whole feature, and the one place a wrong flag silently costs Helm
/// everything it knows about the fleet.
#[test]
fn a_conversation_that_has_run_is_resumed_rather_than_minted() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let first = helm_json(&machine, &[]);
    let uuid = first["data"]["uuid"].as_str().unwrap().to_string();

    // What `--exec` records the moment it hands the process over.
    let record = helm_home(&machine).join("session.json");
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(&record, text.replace("false", "true")).unwrap();

    let second = helm_json(&machine, &[]);
    assert_eq!(
        second["data"]["uuid"],
        uuid.as_str(),
        "the conversation moved"
    );
    assert_eq!(second["data"]["conversation"], "RESUMED");
    let argv = argv_of(&second);
    assert_eq!(&argv[argv.len() - 2..], ["--resume", uuid.as_str()]);
    assert!(
        !argv.iter().any(|word| word == "--session-id"),
        "{argv:?} minted a second conversation"
    );
}

/// Running the verb twice changes nothing the second time, and says so. A
/// reader who edited one of these files by hand can only tell that Armada
/// overwrote it if the ordinary case reports `unchanged`.
#[test]
fn a_second_run_rewrites_nothing_and_reports_that() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let first = helm_json(&machine, &[]);
    assert!(
        first["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["state"] == "WRITTEN"),
        "{first:#}"
    );

    let second = helm_json(&machine, &[]);
    for row in second["data"]["results"].as_array().unwrap() {
        assert_eq!(row["state"], "UNCHANGED", "{row:#} was rewritten");
    }
    assert_eq!(first["data"]["uuid"], second["data"]["uuid"]);
}

/// `--new` puts yesterday's conversation down. The id changes, and the flag
/// goes back to `--session-id` because the new one has never run.
#[test]
fn new_mints_another_conversation_rather_than_reusing_the_one_on_disk() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let first = helm_json(&machine, &[]);
    let record = helm_home(&machine).join("session.json");
    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(&record, text.replace("false", "true")).unwrap();

    let fresh = helm_json(&machine, &["--new"]);
    assert_ne!(
        fresh["data"]["uuid"], first["data"]["uuid"],
        "--new resumed the conversation it was told to replace"
    );
    assert_eq!(fresh["data"]["conversation"], "NEW");
    assert!(argv_of(&fresh).iter().any(|word| word == "--session-id"));
}

/// **A different persona is a different conversation.** Resuming one persona's
/// session under another's hands the second one commitments it never made.
#[test]
fn another_persona_gets_its_own_conversation() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let helm = helm_json(&machine, &[]);

    let guild = machine.home.path().join(".armada/guild/subagents");
    std::fs::write(guild.join("skeptic.md"), "---\nname: skeptic\n---\n").unwrap();
    project(&machine, "skeptic");

    let other = helm_json(&machine, &["--agent", "skeptic"]);
    assert_eq!(other["data"]["agent"], "skeptic");
    assert_ne!(other["data"]["uuid"], helm["data"]["uuid"]);
    assert_eq!(argv_of(&other)[2], "skeptic");
}

/// Three refusals, because three different things fix them. Collapsing them
/// into "no persona" would send a reader to the wrong command twice out of
/// three times.
#[test]
fn a_machine_that_cannot_run_helm_says_which_of_the_three_is_missing() {
    // No guild at all.
    let machine = Machine::new();
    let out = machine.run(machine.root.path(), &["helm", "--json"]);
    assert_eq!(out.status.code(), Some(3), "bad_config is exit 3");
    let envelope: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(envelope["error"]["class"], "bad_config");
    assert!(envelope["error"]["next_action"]
        .as_str()
        .unwrap()
        .contains("armada init"));

    // A guild, and no such persona.
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let out = machine.run(
        machine.root.path(),
        &["helm", "--json", "--agent", "nobody"],
    );
    assert_eq!(out.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains("nobody"));

    // The persona is in the guild and not on Claude Code's load path, which is
    // the one failure that would otherwise open an ordinary session wearing
    // Helm's name and none of its rules.
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    std::fs::remove_file(machine.home.path().join(".claude/agents/helm.md")).unwrap();
    let out = machine.run(machine.root.path(), &["helm", "--json"]);
    assert_eq!(out.status.code(), Some(3));
    let envelope: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(envelope["error"]["next_action"]
        .as_str()
        .unwrap()
        .contains("armada guild project"));
}

/// **A refusal changes nothing.** A `helm` that laid down four configuration
/// files and then admitted it has no persona to run would have modified the
/// machine in order to report a failure.
#[test]
fn a_refusal_writes_nothing() {
    let machine = Machine::new();
    let out = machine.run(machine.root.path(), &["helm", "--json"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(
        !helm_home(&machine).exists(),
        "a refused launch wrote {:?}",
        helm_home(&machine)
    );
}

/// **The gate, asserted. This test is the deliverable.**
///
/// Entering opens a Claude Code session, and no path in this binary may start
/// one. `--exec` is known to the parser and refused — by name, with a reason,
/// and as an ordinary error with a class and a `next:` line.
///
/// A gate with no test is a comment: without this, re-enabling entering is a
/// silent one-line change, and the first thing anybody would learn about it is a
/// bill.
#[test]
fn entering_is_refused_by_name_and_says_why() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let out = machine.run(machine.root.path(), &["helm", "--exec"]);
    assert_eq!(out.status.code(), Some(2), "bad_invocation is exit 2");

    let said = String::from_utf8_lossy(&out.stderr);
    // **By name, never as an unknown flag.** A caller told "unknown flag"
    // concludes they typed it wrong and goes looking for the spelling that
    // works — which is the reading this refusal exists to prevent.
    assert!(
        !said.to_lowercase().contains("unknown"),
        "the refusal reads as a typo rather than a decision: {said}"
    );
    assert!(said.contains("--exec"), "{said}");
    // The reason, and the exact verb that lifts it — named, not paraphrased,
    // so a caller can paste it rather than guess at the spelling.
    assert!(said.contains("off on this machine"), "{said}");
    assert!(said.contains("armada helm enable"), "{said}");
}

/// **A fresh install cannot exec, on the machine that has never asked.** This
/// is the requirement the switch exists to satisfy, stated as a test on its
/// own rather than folded into the refusal test above: `--json` and the
/// exit code, not just the words on stderr.
#[test]
fn a_fresh_install_refuses_exec_by_default() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let out = machine.run(machine.root.path(), &["helm", "--exec", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("an envelope");
    assert_eq!(envelope["error"]["class"], "bad_invocation");
    assert!(
        !helm_home(&machine).exists(),
        "a refused --exec on a fresh install wrote {:?}",
        helm_home(&machine)
    );
}

/// **The deliverable for the switch itself.** `armada helm enable` flips
/// `helm.enter` in `~/.armada/machine.yml` and reports it; `armada helm
/// disable` puts it back; every surface that answers "is entering on"
/// agrees, including `armada helm`'s own envelope. This is as far as a test
/// may safely go — see this file's header for why `--exec` itself is never
/// run once the switch is on.
#[test]
fn enable_and_disable_flip_the_switch_machine_yml_records() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let armada_home = machine.home.path().join(".armada");

    assert!(
        !armada_helm::verbs::helm::entering_allowed(&armada_home),
        "a fresh machine already allows entering"
    );

    let enabled = machine.run(machine.root.path(), &["helm", "enable", "--json"]);
    assert!(enabled.status.success());
    let envelope: Value = serde_json::from_slice(&enabled.stdout).expect("an envelope");
    assert_eq!(envelope["data"]["entering"], true);
    assert_eq!(envelope["data"]["changed"], true);
    assert!(armada_helm::verbs::helm::entering_allowed(&armada_home));

    // **A second `enable` changes nothing and says so** — the same rule
    // every other idempotent write in this suite follows.
    let again = machine.run(machine.root.path(), &["helm", "enable", "--json"]);
    let envelope: Value = serde_json::from_slice(&again.stdout).expect("an envelope");
    assert_eq!(envelope["data"]["changed"], false, "{envelope:#}");

    // **`armada helm` itself now reports entering as on**, without having
    // been asked to enter — the read side agrees with the write side.
    let helm = machine.run(machine.root.path(), &["helm", "--json"]);
    let envelope: Value = serde_json::from_slice(&helm.stdout).expect("an envelope");
    assert_eq!(envelope["data"]["entering"], true, "{envelope:#}");
    assert_eq!(
        envelope["data"]["launched"], false,
        "reporting the switch as on must not itself start anything"
    );

    let disabled = machine.run(machine.root.path(), &["helm", "disable", "--json"]);
    let envelope: Value = serde_json::from_slice(&disabled.stdout).expect("an envelope");
    assert_eq!(envelope["data"]["entering"], false);
    assert_eq!(envelope["data"]["changed"], true);
    assert!(!armada_helm::verbs::helm::entering_allowed(&armada_home));

    // Refused again, now that it is back off.
    let out = machine.run(machine.root.path(), &["helm", "--exec"]);
    assert_eq!(out.status.code(), Some(2));
}

/// **`enable`/`disable` need none of Helm's own readiness.** A machine with
/// no guild at all can still flip the switch — whether a session is *allowed*
/// here and whether the guild and persona currently exist to run one are
/// different questions, and this proves the first does not accidentally
/// require the second.
#[test]
fn enabling_needs_no_guild_and_touches_nothing_helm_itself_writes() {
    let machine = Machine::new();
    let armada_home = machine.home.path().join(".armada");

    let out = machine.run(machine.root.path(), &["helm", "enable", "--json"]);
    assert!(out.status.success(), "{:?}", out.stderr);
    assert!(armada_helm::verbs::helm::entering_allowed(&armada_home));
    assert!(
        !helm_home(&machine).exists(),
        "enabling the switch wrote {:?}, which only `armada helm` should",
        helm_home(&machine)
    );
}

/// The same refusal in the envelope: a class an agent can branch on, a `where`
/// naming the flag, and a `next_action`.
#[test]
fn the_refusal_is_an_ordinary_error_with_a_class_and_a_next_action() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let out = machine.run(machine.root.path(), &["helm", "--exec", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("an envelope");
    let error = &envelope["error"];
    // **`bad_invocation`, the class this CLI already uses for a flag it knows
    // and has not built** — `doctor --fix` is this. `check --detach` was, until
    // it shipped; a flag leaving the list changes nothing about the class the
    // ones still on it answer with. A class invented for the occasion would say
    // this refusal is a different kind of thing, and it is not.
    assert_eq!(error["class"], "bad_invocation");
    // **`helm`, not `helm --exec`.** One refusal answers both spellings, because
    // they are one act; a `where` naming the flag would say the flag was the
    // thing refused, which is the second lock all over again.
    assert_eq!(error["where"], "helm");
    assert!(error["next_action"].as_str().is_some_and(|n| !n.is_empty()));
}

/// **A refused `--exec` changes nothing at all.** It is turned away before the
/// verb runs, so it cannot leave a configuration file behind as the price of
/// saying no — the rule `armada doctor --fix` already follows.
#[test]
fn a_refused_entry_writes_nothing() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    machine.run(machine.root.path(), &["helm", "--exec"]);
    assert!(
        !helm_home(&machine).exists(),
        "a refused --exec wrote {:?}",
        helm_home(&machine)
    );
}

/// **The constants, read by every surface that has anything to say about
/// them.** The parser's flags, the help page's rows, the render's summary line
/// and the refusal all have to agree, and they agree because they read the same
/// strings rather than four copies of the same sentence.
///
/// **The reason is what the surfaces share, not the flag.** `--exec` is now a
/// synonym for the default rather than the lock, so the surface that used to
/// have to name it — the report — names the *state* instead, and it reads that
/// state from the same constant the refusal does.
#[test]
fn every_surface_reads_the_gate_from_the_same_place() {
    use armada_helm::verbs::helm::{ENABLE, ENTER, ENTER_IS_OFF, PRINT};

    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let refusal =
        String::from_utf8_lossy(&machine.run(machine.root.path(), &["helm", "--exec"]).stderr)
            .to_string();
    let page =
        String::from_utf8_lossy(&machine.run(machine.root.path(), &["helm", "--help"]).stdout)
            .to_string();
    let report =
        String::from_utf8_lossy(&machine.run(machine.root.path(), &["helm", PRINT]).stdout)
            .to_string();

    // Both spellings of entering are on the page, and neither reads as the
    // permission — the machine switch is that, and the page names it too.
    for flag in [ENTER, PRINT] {
        assert!(page.contains(flag), "the help page does not name `{flag}`");
    }
    // The refusal names the flag it also answers for, so a caller who typed it
    // is not told about a different command.
    assert!(
        refusal.contains(ENTER),
        "the refusal does not name `{ENTER}`"
    );

    // The state, and the verb that changes it, in the two places a reader meets
    // them — both read from the constants rather than retyped.
    for (surface, text) in [("refusal", &refusal), ("report", &report)] {
        assert!(
            text.contains(ENTER_IS_OFF),
            "the {surface} does not give the reason: {text}"
        );
        assert!(
            text.contains(ENABLE),
            "the {surface} does not name the verb that lifts it: {text}"
        );
    }
}

/// **The record-writer the exec path needs, exercised directly.**
///
/// `mark_started` has no caller while entering is refused. It is kept so that
/// turning entering back on is a deleted refusal and a call rather than a
/// rediscovery — and it is run here for the reason anything kept for later is
/// run: a function nothing exercises has rotted by the time later arrives.
///
/// It also closes the one link the rest of the suite cannot: that the flag which
/// makes the next launch say `--resume` is the flag this function writes.
#[test]
fn the_writer_that_turns_a_launch_into_a_resume_still_works() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    let first = helm_json(&machine, &[]);
    let uuid = first["data"]["uuid"].as_str().unwrap().to_string();
    assert_eq!(first["data"]["conversation"], "NEW");

    armada_helm::verbs::helm::mark_started(
        &armada_helm::verbs::helm::Where {
            home: machine.home.path().to_path_buf(),
            armada_home: machine.home.path().join(".armada"),
            claude_home: machine.home.path().join(".claude"),
            exe: support::armada_binary(),
            boot_id: "test-boot".to_string(),
        },
        &armada_core::helm::Session {
            uuid: uuid.clone(),
            agent: "helm".to_string(),
            started: true,
        },
    )
    .expect("the record is writable");

    let second = helm_json(&machine, &[]);
    assert_eq!(second["data"]["conversation"], "RESUMED");
    let argv = argv_of(&second);
    assert_eq!(&argv[argv.len() - 2..], ["--resume", uuid.as_str()]);
}

/// `--print-command` is free, and says out loud that it started nothing —
/// rather than leaving a reader waiting for a prompt that is never coming.
///
/// **The line it prints is the point of the flag.** This is where the old
/// default went when the bare verb started entering, and it is how a person
/// reads the argv Armada built without spending anything to see it.
#[test]
fn print_command_says_out_loud_that_it_started_nothing() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let out = machine.run(machine.root.path(), &["helm", "--print-command"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains("nothing started"), "{said}");
    assert!(said.contains("entering is off on this machine"), "{said}");
    assert!(said.contains("enter with claude --agent helm"), "{said}");
    // The whole launch is on that line, mode and all.
    assert!(said.contains("--permission-mode auto"), "{said}");
}

/// **The defect this change exists for, as a test.**
///
/// `helm.enter` was already on and `armada helm` still printed a command to
/// paste, because entering was gated a second time behind `--exec`. A machine
/// that has said yes must not be asked again — and the only safe way to assert
/// that here is on the *decision*, because entering for real would exec into
/// whatever `claude` is on this developer's `PATH`.
///
/// So: with the switch off, the bare verb is refused and writes nothing. With
/// it on, the same bare verb assembles the launch and then **becomes it** —
/// which is safe here and nowhere else, because the `claude` on this suite's
/// `PATH` is [`support`]'s stub: nine lines of `sh` that answer Armada's probes,
/// exit 0, and talk to nothing. No turn, no token.
///
/// The three assertions are the three ways this could still be wrong: a refusal
/// (the second lock still there), a printed command and an envelope (the old
/// default still there), or no configuration at all (a launch that never ran).
#[test]
fn a_machine_that_has_said_yes_is_not_asked_a_second_time() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let refused = machine.run(machine.root.path(), &["helm"]);
    assert_eq!(
        refused.status.code(),
        Some(2),
        "a bare `armada helm` on a machine that has not said yes did not refuse"
    );
    assert!(
        !helm_home(&machine).exists(),
        "a refused launch wrote {:?}",
        helm_home(&machine)
    );

    machine.run(machine.root.path(), &["helm", "enable"]);
    let entered = machine.run(machine.root.path(), &["helm"]);
    assert_eq!(
        entered.status.code(),
        Some(0),
        "the machine said yes and `armada helm` did not enter: {}",
        String::from_utf8_lossy(&entered.stderr)
    );
    let said = String::from_utf8_lossy(&entered.stdout);
    // **The process was replaced, so there is no envelope and no line to
    // paste.** This is the assertion the reader's complaint turns on: a machine
    // that has said yes must not be handed its own launch command.
    assert!(
        !said.contains("enter with"),
        "`armada helm` printed a command instead of entering: {said}"
    );
    assert!(
        helm_home(&machine).join("mcp.json").is_file(),
        "`armada helm` neither refused nor assembled a launch"
    );
    // And the record says the conversation exists, which is what `mark_started`
    // writes *before* the exec — because there is no after.
    let record = std::fs::read_to_string(helm_home(&machine).join("session.json")).unwrap();
    assert!(record.contains("\"started\": true"), "{record}");
}

/// **`--print-command` and `--exec` are refused together**, rather than one
/// silently winning. A precedence rule would do one of the two things the
/// caller asked for and drop the other, and the one it dropped is either a
/// session they did not want or a session they did.
#[test]
fn asking_to_print_and_to_enter_at_once_is_refused() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    machine.run(machine.root.path(), &["helm", "enable"]);

    let out = machine.run(
        machine.root.path(),
        &["helm", "--print-command", "--exec", "--json"],
    );
    assert_eq!(out.status.code(), Some(2));
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("an envelope");
    assert_eq!(envelope["error"]["class"], "bad_invocation");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("opposite")),
        "{envelope:#}"
    );
}

/// **A `helm.mode` Claude Code has never heard of is refused before a launch is
/// assembled**, by name, naming the file and the key.
///
/// `--permission-mode` is checked at argument-parse time, so an unrecognised
/// value is a session that dies the instant it is entered, printing Claude
/// Code's usage error over whatever the reader was doing. This is the check
/// that costs nothing and names the file instead.
#[test]
fn a_mode_this_machine_invented_is_refused_and_nothing_is_written() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    std::fs::write(
        machine.home.path().join(".armada/machine.yml"),
        "helm:\n  enter: true\n  mode: yolo\n",
    )
    .unwrap();

    let out = machine.run(machine.root.path(), &["helm", "--json"]);
    assert_eq!(out.status.code(), Some(3), "bad_config is exit 3");
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("an envelope");
    let error = &envelope["error"];
    assert_eq!(error["class"], "bad_config");
    assert_eq!(error["where"], "machine.yml helm.mode");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|m| m.contains("yolo")),
        "{envelope:#}"
    );
    assert!(
        error["next_action"]
            .as_str()
            .is_some_and(|n| n.contains("retry unchanged")),
        "{envelope:#}"
    );
    assert!(
        !helm_home(&machine).exists(),
        "a refused mode wrote {:?}",
        helm_home(&machine)
    );
}

/// **The mode is the machine's, not a constant one layer down.** A reader who
/// chose `acceptEdits` gets it in the argv.
#[test]
fn the_mode_this_machine_chose_reaches_the_launch() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);
    std::fs::write(
        machine.home.path().join(".armada/machine.yml"),
        "helm:\n  mode: acceptEdits\n",
    )
    .unwrap();

    let argv = argv_of(&helm_json(&machine, &[]));
    let at = argv
        .iter()
        .position(|word| word == "--permission-mode")
        .expect("no mode in the launch");
    assert_eq!(argv[at + 1], "acceptEdits");
}

/// `armada helm --help` tells the truth about all four: entering is what the
/// verb does, one machine switch is what permits it, `--print-command` is how
/// to get the line instead, and there is still no `helm` binary.
#[test]
fn the_page_says_entering_is_the_default_and_names_its_one_switch() {
    let machine = Machine::new();
    let out = machine.run(machine.root.path(), &["helm", "--help"]);
    assert!(out.status.success());
    let page = String::from_utf8_lossy(&out.stdout);
    for flag in ["--exec", "--print-command"] {
        assert!(page.contains(flag), "{flag} is not on its own page: {page}");
    }
    // **One switch, named, and named as the only one.** The page a reader
    // reaches for after `armada helm` refused them has to send them somewhere,
    // and a page that implied a second gate is what this change removed.
    assert!(page.contains("helm.enter"), "{page}");
    assert!(page.contains("nothing else"), "{page}");
    assert!(page.contains("armada helm enable"), "{page}");
    assert!(page.contains("armada helm disable"), "{page}");
    assert!(
        page.contains("off on a fresh install"),
        "the page does not say what a caller who has never run enable gets: {page}"
    );
    // The mode the session enters under is a machine setting too, and the page
    // is where a reader learns the key exists.
    assert!(page.contains("helm.mode"), "{page}");
    assert!(page.contains("There is no `helm` binary"), "{page}");
}

/// `armada helm enable --help` and `armada helm disable --help` are their own
/// pages, not the launch's — a caller asking about the switch must not be
/// handed the unrelated `--agent`/`--new` page.
#[test]
fn enable_and_disable_answer_their_own_help() {
    let machine = Machine::new();
    for verb in ["enable", "disable"] {
        let out = machine.run(machine.root.path(), &["helm", verb, "--help"]);
        assert!(out.status.success());
        let page = String::from_utf8_lossy(&out.stdout);
        assert!(
            page.contains(&format!("armada helm {verb}")),
            "`armada helm {verb} --help` did not draw its own page: {page}"
        );
        assert!(
            !page.contains("--agent"),
            "`armada helm {verb} --help` drew the launch's page: {page}"
        );
    }
}

/// **Bare `armada` is the orientation page and starts nothing.** PLAN.md §15.1
/// gives the bare word to Helm eventually; until it does, the most typeable
/// thing on the machine must not be the one that opens a session.
#[test]
fn the_bare_word_does_not_enter_a_conversation() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let out = machine.run(machine.root.path(), &[]);
    assert!(out.status.success());
    let page = String::from_utf8_lossy(&out.stdout);
    assert!(page.contains("armada helm"), "{page}");
    assert!(
        !helm_home(&machine).exists(),
        "bare `armada` wired a launch nobody asked for"
    );
}
