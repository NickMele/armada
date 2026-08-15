//! `armada helm` end to end, against a scratch `$HOME`.
//!
//! **No test here starts a Claude Code session, and none can.** The verb under
//! test assembles a launch and reports it; `--exec` is refused by name, and
//! there is no path in the binary that opens one. That refusal is asserted
//! rather than assumed — see [`entering_is_refused_by_name_and_says_why`], which
//! is the deliverable of this file and not a detail of it. A gate with no test
//! is a comment, and the first thing anybody would learn about it silently
//! coming back on is a bill.
//!
//! [`entering_is_refused_by_name_and_says_why`]: entering_is_refused_by_name_and_says_why
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
/// Claude Code's load path — the three things `armada helm` refuses without.
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
    project(machine, "helm");
}

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
    assert_eq!(
        argv_of(&envelope),
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
            "--session-id".to_string(),
            data["uuid"].as_str().unwrap().to_string(),
        ]
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
    // The reason, and that it is temporary.
    assert!(
        said.contains("switched off until the Bridge is fixed"),
        "{said}"
    );
    // And a way forward, because `armada helm` has already printed the command.
    assert!(said.contains("armada helm"), "{said}");
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
    // and has not built** — `doctor --fix` and `check --detach` are both this.
    // A class invented for the occasion would say the refusal is a different
    // kind of thing from those two, and it is not.
    assert_eq!(error["class"], "bad_invocation");
    assert_eq!(error["where"], "helm --exec");
    assert!(error["next_action"].as_str().is_some_and(|n| !n.is_empty()));
}

/// **A refused `--exec` changes nothing at all.** It is turned away before the
/// verb runs, so it cannot leave four configuration files behind as the price of
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

/// **The one constant, read by every surface.** The parser's flag, the help
/// page's row, the render's summary line and the refusal all have to agree, and
/// they agree because they read the same two strings rather than four copies of
/// the same sentence.
#[test]
fn every_surface_reads_the_gate_from_the_same_place() {
    use armada_helm::verbs::helm::{ENTER, ENTER_IS_OFF};

    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let refusal =
        String::from_utf8_lossy(&machine.run(machine.root.path(), &["helm", "--exec"]).stderr)
            .to_string();
    let page =
        String::from_utf8_lossy(&machine.run(machine.root.path(), &["helm", "--help"]).stdout)
            .to_string();
    let report =
        String::from_utf8_lossy(&machine.run(machine.root.path(), &["helm"]).stdout).to_string();

    for (surface, text) in [
        ("refusal", &refusal),
        ("help page", &page),
        ("report", &report),
    ] {
        assert!(
            text.contains(ENTER),
            "the {surface} does not name `{ENTER}`"
        );
    }
    for (surface, text) in [("refusal", &refusal), ("report", &report)] {
        assert!(
            text.contains(ENTER_IS_OFF),
            "the {surface} does not give the reason: {text}"
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

/// The default is free. `armada helm` with no flags starts nothing, and the
/// human render says so in words rather than leaving a reader waiting for a
/// prompt that is never coming.
#[test]
fn the_default_says_out_loud_that_it_started_nothing() {
    let machine = Machine::new();
    a_machine_ready_for_helm(&machine);

    let out = machine.run(machine.root.path(), &["helm"]);
    assert!(out.status.success());
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains("nothing started"), "{said}");
    assert!(said.contains("--exec"), "{said}");
    assert!(said.contains("enter with claude --agent helm"), "{said}");
}

/// `armada helm --help` tells the truth about all three: the launch is
/// assembled, entering is off, and there is still no `helm` binary.
#[test]
fn the_page_says_the_launch_is_built_and_entering_is_off() {
    let machine = Machine::new();
    let out = machine.run(machine.root.path(), &["helm", "--help"]);
    assert!(out.status.success());
    let page = String::from_utf8_lossy(&out.stdout);
    assert!(
        page.contains("--exec"),
        "the flag is not on its own page: {page}"
    );
    assert!(page.contains("assembled and verified"), "{page}");
    assert!(
        page.contains("switched off until the Bridge is fixed"),
        "{page}"
    );
    assert!(
        page.contains("no\n  path in this binary opens a session")
            || page.contains("path in this binary opens a session"),
        "{page}"
    );
    assert!(page.contains("There is no `helm` binary"), "{page}");
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
