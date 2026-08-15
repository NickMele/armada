//! Helm's pure decisions: **the argv, the four configuration files, and the
//! record that makes it the same conversation tomorrow** (PLAN.md §15).
//!
//! Helm is a Claude Code session running an orchestrator persona from the
//! user's guild, with Armada's MCP server as its toolbelt. There is no session
//! mechanism to build — Claude Code already has one — so what lives here is a
//! uuid assigned before anything starts, an argv, and the JSON documents that
//! wire the inbox up.
//!
//! ```text
//! claude --agent helm
//!        --mcp-config  ~/.armada/helm/mcp.json
//!        --plugin-dir  ~/.armada/helm/plugin
//!        --settings    ~/.armada/helm/settings.json
//!        --session-id  <uuid>      # the first launch, which mints it
//!        --resume      <uuid>      # every launch after that
//! ```
//!
//! **This module is where the bugs are, which is why it is pure.** A `--resume`
//! where `--session-id` was meant starts tomorrow's conversation as a fresh one
//! and loses everything Helm knows about the fleet; a `--mcp-config` pointing at
//! a document Claude Code will not parse hands the orchestrator an empty
//! toolbelt and no error. Both are argv-and-bytes bugs, and a test that faked a
//! higher layer would catch neither (`ARCHITECTURE.md` §1.1).
//!
//! **No test in this repository spawns a real session or spends a token**
//! (PHASES.md §8.5). The argv is asserted here; [`FLAGS`] is what `armada
//! doctor` holds against `claude --help`, which starts nothing and costs
//! nothing.
//!
//! # The inbox is configuration, not code (§15.3)
//!
//! Two mechanisms deliver an inbox entry to the orchestrator, and Armada writes
//! neither as a daemon and neither as a poller:
//!
//! | Mechanism | Fires | Written by |
//! |---|---|---|
//! | A plugin **monitor** tailing `~/.armada/inbox.jsonl` | mid-turn, live | [`monitors_json`] in a `--plugin-dir` |
//! | A **`Stop` hook** on the orchestrator | at turn end, if anything is unread | [`settings_json`] plus [`stop_hook`] |
//!
//! The monitor is timely and the hook is complete, which is why both exist. A
//! monitor runs in interactive sessions only — that fits, because Helm is
//! interactive and Drones are headless, and it is why the hook is the one that
//! guarantees nothing is lost.

use crate::error::{ArmadaError, ErrClass};
use serde::{Deserialize, Serialize};

/// Where every file this module produces lives, under `~/.armada/`.
///
/// **A directory of its own rather than loose files beside `manifest.db`.**
/// Every one of them names a path on *this* machine — the inbox, the `armada`
/// binary, the guild — so all four are on the never-syncs side of PLAN.md
/// §13.1's line, and a directory says that once instead of four times.
pub const DIRECTORY: &str = "helm";

/// The persona a bare `armada helm` runs, and the file it is read from.
///
/// It lives at `~/.armada/guild/subagents/<AGENT>.md`, is seeded from
/// `templates/guild/subagents/helm.md`, and is **yours from that moment** —
/// Armada never touches it again (PLAN.md §15.4).
pub const AGENT: &str = "helm";

/// The name the MCP server is registered under, and therefore the prefix every
/// tool the persona names carries.
///
/// **It has to match `templates/guild/subagents/helm.md`'s `tools:` list**,
/// whose entries are `mcp__armada__fleet_spawn` and friends. A rename on one
/// side alone leaves the persona with a tool list naming nothing, which is a
/// Helm that cannot spawn and does not say why — asserted in this file's tests
/// against the template itself.
pub const SERVER: &str = "armada";

/// The program Helm is. The same binary a Drone is, because Helm is a session.
pub use crate::fleet::drone::CLAUDE;

/// Which conversation `armada helm` is, and whether it exists yet.
///
/// **Persisted, because resuming is the whole point.** "Resuming the same
/// conversation each day" (PLAN.md §15.1) is not a Claude Code feature Armada
/// switches on; it is a uuid Armada remembers. Losing this file loses Helm's
/// memory of the fleet, which is the one thing nothing else on the machine
/// could rebuild.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// The session id, minted before the first launch.
    pub uuid: String,
    /// Which persona this conversation belongs to.
    ///
    /// **Recorded rather than assumed**, because `--agent` may name another one
    /// (`commands/helm/helm.md`). A conversation held with one persona and
    /// resumed under a different one is two personas sharing a memory, and the
    /// second one inherits commitments it never made — so a change of agent
    /// starts a new conversation rather than borrowing this one.
    pub agent: String,
    /// Whether a launch has already minted it with `--session-id`.
    ///
    /// **The flag that decides `--session-id` against `--resume`**, and the one
    /// piece of state that cannot be derived: a transcript's location is Claude
    /// Code's business, not Armada's, so asking the filesystem would be Armada
    /// guessing at another tool's layout.
    pub started: bool,
}

impl Session {
    /// A conversation that has not happened yet.
    ///
    /// `seed` is hashed rather than randomised, for the reason
    /// [`crate::fleet::job::mint_uuid`] states: randomness is not one of the
    /// three seams, and a derived uuid lets a test assert the exact
    /// `--session-id` Armada passed.
    pub fn mint(seed: &str, agent: &str) -> Session {
        Session {
            uuid: crate::fleet::job::mint_uuid(seed),
            agent: agent.to_string(),
            started: false,
        }
    }

    /// Whether this record can be resumed **as the agent asked for**.
    ///
    /// Two ways it cannot: it has never run, and it belongs to another persona.
    pub fn resumable_as(&self, agent: &str) -> bool {
        self.started && self.agent == agent
    }
}

/// Everything the argv is built from, gathered by the caller.
///
/// **Paths arrive as values.** Nothing here reads `$HOME`, the environment or
/// the current directory (`ARCHITECTURE.md` §1.4) — which is also what lets the
/// whole suite point `armada helm` at a `TempDir` rather than at somebody's
/// real `~/.armada/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The persona, as `--agent` spells it.
    pub agent: String,
    /// The conversation.
    pub session: Session,
    /// `~/.armada/helm/mcp.json`.
    pub mcp_config: String,
    /// `~/.armada/helm/plugin`.
    pub plugin_dir: String,
    /// `~/.armada/helm/settings.json`.
    pub settings: String,
}

/// The command that starts, or resumes, the one conversation.
///
/// **`--session-id` on the first launch and `--resume` on every one after**,
/// and the two are not interchangeable. `--resume` against a uuid Claude Code
/// has never seen fails with *no conversation found*; `--session-id` against one
/// it has seen would be a second conversation wearing the first one's name. The
/// record is what tells them apart, which is why it is persisted rather than
/// derived.
pub fn launch_argv(launch: &Launch) -> Vec<String> {
    let mut argv = vec![
        CLAUDE.to_string(),
        // **The persona first, because it is what makes this Helm.** Everything
        // after it is the toolbelt and the wiring; without this flag the same
        // argv opens an ordinary session that happens to have Armada's tools.
        "--agent".to_string(),
        launch.agent.clone(),
        "--mcp-config".to_string(),
        launch.mcp_config.clone(),
        // **The monitor, and it is the only thing the plugin carries.** A
        // session-scoped `--plugin-dir` needs no marketplace and no install
        // step, and it namespaces nothing Helm relies on by name (PLAN.md
        // §13.3's third measurement).
        "--plugin-dir".to_string(),
        launch.plugin_dir.clone(),
        // **The `Stop` hook, registered for this session and not for the
        // machine.** Registering it in `~/.claude/settings.json` would fire it
        // in every session the reader ever opens, including the Drones — and a
        // Drone that refuses to stop while the inbox is unread is a Drone that
        // never stops.
        "--settings".to_string(),
        launch.settings.clone(),
    ];
    argv.push(
        match launch.session.resumable_as(&launch.agent) {
            true => "--resume",
            false => "--session-id",
        }
        .to_string(),
    );
    argv.push(launch.session.uuid.clone());
    argv
}

/// Every flag [`launch_argv`] uses, for `armada doctor` to hold against `claude
/// --help`.
///
/// **The point is the next version of Claude Code, not this one.** The failure
/// this exists to catch is the one Fleet's Drone shipped with: an argv every
/// test agreed was correct and the binary rejected at argument-parse time
/// (`docs/traps.md`). A flag renamed or removed under Armada would otherwise
/// surface as a Helm that will not start, in the one session the reader cannot
/// ask Armada about — because Helm is how they ask.
pub const FLAGS: [&str; 6] = [
    "--agent",
    "--mcp-config",
    "--plugin-dir",
    "--settings",
    "--session-id",
    "--resume",
];

/// The toolbelt, as a `--mcp-config` document.
///
/// **One server, over stdio, started by the session that needs it.** There is no
/// daemon and no port: `armada mcp serve` lives exactly as long as the Helm
/// session does (`commands/helm/mcp.md`).
///
/// `exe` is the `armada` binary Armada is *currently running as*, read once at
/// the entrypoint. Writing the bare word `armada` instead would resolve against
/// whatever is on the session's `PATH`, which on a machine with two
/// installations is the wrong one — and the failure is a toolbelt that answers
/// with another version's envelope.
pub fn mcp_json(exe: &str) -> String {
    format!(
        "{{\n  \"mcpServers\": {{\n    \"{SERVER}\": {{\n      \"type\": \"stdio\",\n      \
         \"command\": {},\n      \"args\": [\"mcp\", \"serve\", \"--stdio\"]\n    }}\n  }}\n}}\n",
        quote(exe)
    )
}

/// The plugin manifest for the session-scoped plugin that carries the monitor.
///
/// A plugin is the only load path a monitor has, and `--plugin-dir` is the only
/// way to load one without installing it — so this is the smallest plugin that
/// can exist: a name, a description, and no components but the monitor beside
/// it.
pub fn plugin_json() -> String {
    format!(
        "{{\n  \"name\": \"{MONITOR}\",\n  \"description\": \"{DESCRIPTION}\",\n  \
         \"version\": \"{}\",\n  \"author\": {{ \"name\": \"Armada\" }}\n}}\n",
        PLUGIN_VERSION
    )
}

/// The monitor: `tail -F` on the inbox, one line per event.
///
/// **Every stdout line a monitor writes is delivered to Claude as a live
/// notification during the session** (PHASES.md §9.1 F3), which is the whole of
/// "aware without polling". `tail -F` rather than `tail -f` because the inbox
/// may not exist yet on a machine whose first Job has not run, and `-F` waits
/// for the file to appear instead of exiting.
///
/// The path is absolute rather than `~/…`: a monitor's command is not
/// necessarily run through a shell that expands a tilde, and a monitor tailing
/// a file called `~` is a monitor that reports nothing forever.
pub fn monitors_json(inbox: &str) -> String {
    format!(
        "[\n  {{\n    \"name\": \"{MONITOR}\",\n    \"command\": {},\n    \
         \"description\": \"{DESCRIPTION}\"\n  }}\n]\n",
        quote(&format!("tail -F {inbox}"))
    )
}

/// The `Stop` hook, registered for **this session only**.
///
/// `--settings` layers this over whatever the reader's own settings say, for the
/// length of one session. Registering the same hook in `~/.claude/settings.json`
/// would fire it in every session on the machine — including a Drone's, and a
/// Drone held open until the inbox is read is a Drone that cannot finish the
/// work the inbox is about.
pub fn settings_json(hook: &str) -> String {
    format!(
        "{{\n  \"hooks\": {{\n    \"Stop\": [\n      {{\n        \"hooks\": [\n          \
         {{\n            \"type\": \"command\",\n            \"command\": {}\n          }}\n        \
         ]\n      }}\n    ]\n  }}\n}}\n",
        quote(hook)
    )
}

/// What the `Stop` hook runs: **nine lines of shell, and no daemon**.
///
/// The shape was verified in the M0 spike (PHASES.md §9.1 F3): a hook answering
/// `{"decision":"block","reason":"…"}` holds the turn open and feeds the reason
/// in, and the session relays it as a hook message rather than as its own
/// finding before handing control back.
///
/// **It reports a count and names the verb, rather than pasting the entries.**
/// That is PLAN.md §15.2's first structural rule arriving at the backstop: the
/// orchestrator reads summaries, and a hook that inlined every unread body would
/// put raw Drone output into Helm's window at the end of every single turn — the
/// exact way a context fills in three days. `fleet.inbox` is one tool call away
/// and answers properly.
///
/// **No `jq` and no `python`.** A hook that depends on a tool the machine may
/// not have is a backstop that silently stops backing anything up, so this is
/// `grep -c` and `printf` against a file of one JSON document per line.
pub fn stop_hook(inbox: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by `armada helm`. Regenerated on every launch; edit the verb, not this.\n\
         # The backstop of PLAN.md §15.3: a turn does not end while the inbox is unread.\n\
         inbox={inbox}\n\
         [ -f \"$inbox\" ] || exit 0\n\
         unread=$(grep -c '\"answered\":false' \"$inbox\" 2>/dev/null || echo 0)\n\
         [ \"$unread\" -gt 0 ] || exit 0\n\
         printf '{{\"decision\":\"block\",\"reason\":\"%s unread inbox %s. \
         Call fleet.inbox, then report what needs the user.\"}}\\n' \\\n\
         \"$unread\" \"$([ \"$unread\" -eq 1 ] && echo entry || echo entries)\"\n",
        inbox = shell_quote(inbox)
    )
}

/// The monitor's name, which is also the session-scoped plugin's.
pub const MONITOR: &str = "armada-inbox";

/// What the monitor is for, in the words the notification carries.
const DESCRIPTION: &str = "Fleet events needing you";

/// The plugin manifest's `version`.
///
/// **Fixed rather than Armada's own version.** The plugin carries one monitor
/// and no components; bumping it with every Armada release would say something
/// changed when nothing did.
const PLUGIN_VERSION: &str = "1.0.0";

/// A string as a JSON scalar.
///
/// **Hand-written rather than `serde_json::to_string`**, for the reason the
/// envelope gives for the same choice: these documents are written in reading
/// order, and a `serde_json::Value` map is a `BTreeMap` that alphabetises them
/// (`docs/traps.md`). Only the two escapes a path or a command can contain are
/// handled, because that is all that can arrive — and a control character in a
/// path is [`refuse`]'s business, not this function's.
fn quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// A path as one shell word.
///
/// Single quotes, with the one escape single quotes need. A `$HOME` containing a
/// space is ordinary on macOS and would otherwise split the hook's `inbox=` into
/// two words.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The refusal when a path could not be written into a generated document.
///
/// **Refused rather than escaped away.** A newline or a quote in a path that
/// reaches a `--mcp-config` produces a document Claude Code will not parse, and
/// the failure surfaces as an orchestrator with an empty toolbelt and no error
/// anywhere — so the honest answer is to say which path and stop.
pub fn refuse(what: &str, value: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: what.to_string(),
        message: format!("`{value}` cannot be written into Helm's configuration"),
        next_action: Some(
            "a path with a newline, a tab or a control character in it cannot be \
             registered; move ~/.armada elsewhere"
                .to_string(),
        ),
    }
}

/// Whether a path is safe to write into a generated JSON or shell document.
pub fn writable(value: &str) -> bool {
    !value.chars().any(|c| c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "15bfa340-33b1-4f81-bd7f-688f0f01dbb0";

    fn a_launch(started: bool) -> Launch {
        Launch {
            agent: AGENT.to_string(),
            session: Session {
                uuid: UUID.to_string(),
                agent: AGENT.to_string(),
                started,
            },
            mcp_config: "/scratch/.armada/helm/mcp.json".to_string(),
            plugin_dir: "/scratch/.armada/helm/plugin".to_string(),
            settings: "/scratch/.armada/helm/settings.json".to_string(),
        }
    }

    /// **The whole vector, exactly.** A test asserting on anything less specific
    /// would pass with `--agent` missing, which is a session that has Armada's
    /// tools and none of the four behaviours PLAN.md §15.4 decided.
    #[test]
    fn the_first_launch_mints_the_conversation_it_is_going_to_resume() {
        assert_eq!(
            launch_argv(&a_launch(false)),
            [
                "claude",
                "--agent",
                "helm",
                "--mcp-config",
                "/scratch/.armada/helm/mcp.json",
                "--plugin-dir",
                "/scratch/.armada/helm/plugin",
                "--settings",
                "/scratch/.armada/helm/settings.json",
                "--session-id",
                UUID,
            ]
        );
    }

    /// **The second launch resumes, and this is the assertion the whole feature
    /// turns on.** "The same conversation each day" is one flag; minting where
    /// resuming was meant loses everything Helm knows about the fleet, silently,
    /// and looks exactly like a Helm that works.
    #[test]
    fn every_launch_after_the_first_resumes_rather_than_minting() {
        let argv = launch_argv(&a_launch(true));
        assert_eq!(&argv[argv.len() - 2..], ["--resume", UUID]);
        assert!(
            !argv.iter().any(|word| word == "--session-id"),
            "{argv:?} mints a second conversation"
        );
        // Everything before the last pair is identical to the first launch's,
        // so the two differ by the one flag and by nothing else.
        let first = launch_argv(&a_launch(false));
        assert_eq!(argv[..argv.len() - 2], first[..first.len() - 2]);
    }

    /// **A different persona is a different conversation.** Resuming one
    /// persona's session under another's hands the second one commitments it
    /// never made, and the memory of a fleet it never ran.
    #[test]
    fn another_persona_does_not_borrow_helms_conversation() {
        let mut launch = a_launch(true);
        launch.agent = "skeptic".to_string();
        let argv = launch_argv(&launch);
        assert!(argv.iter().any(|word| word == "--session-id"));
        assert_eq!(argv[2], "skeptic");
    }

    /// Every flag the argv uses is one `armada doctor` knows to check for, and
    /// every flag it checks for is one the argv uses. A flag in one list and not
    /// the other is a flag nothing would notice disappearing.
    #[test]
    fn every_flag_the_launch_uses_is_one_doctor_checks_for() {
        let used: Vec<String> = launch_argv(&a_launch(false))
            .into_iter()
            .chain(launch_argv(&a_launch(true)))
            .filter(|word| word.starts_with("--"))
            .collect();
        for flag in &used {
            assert!(
                FLAGS.contains(&flag.as_str()),
                "`{flag}` is in the argv and not in FLAGS, so doctor would not \
                 notice it being removed"
            );
        }
        for flag in FLAGS {
            assert!(
                used.iter().any(|word| word == flag),
                "`{flag}` is checked for and used by nothing"
            );
        }
    }

    /// **Helm is interactive, so it carries none of the headless flags.**
    /// `--print` there would answer once and exit, which is not a conversation;
    /// this is the same rule `board_argv` follows and for the same reason.
    #[test]
    fn the_launch_is_interactive_and_carries_no_headless_flag() {
        let argv = launch_argv(&a_launch(true));
        for flag in ["--print", "--output-format", "--verbose", "--input-format"] {
            assert!(!argv.iter().any(|word| word == flag), "{argv:?} has {flag}");
        }
    }

    /// Every generated document parses as the JSON it claims to be. A
    /// `--mcp-config` Claude Code will not read hands Helm an empty toolbelt and
    /// reports nothing, which is the failure worth a test of its own.
    #[test]
    fn every_generated_document_is_the_json_it_claims_to_be() {
        for (what, text) in [
            ("mcp.json", mcp_json("/opt/armada/bin/armada")),
            ("plugin.json", plugin_json()),
            (
                "monitors.json",
                monitors_json("/scratch/.armada/inbox.jsonl"),
            ),
            (
                "settings.json",
                settings_json("/scratch/.armada/helm/stop.sh"),
            ),
        ] {
            serde_json::from_str::<serde_json::Value>(&text)
                .unwrap_or_else(|e| panic!("{what} is not JSON: {e}\n{text}"));
        }
    }

    /// The registration names the server the persona's `tools:` list names.
    #[test]
    fn the_toolbelt_is_registered_under_the_name_the_persona_calls_it() {
        let json: serde_json::Value = serde_json::from_str(&mcp_json("/usr/bin/armada")).unwrap();
        let server = &json["mcpServers"][SERVER];
        assert_eq!(server["command"], "/usr/bin/armada");
        assert_eq!(
            server["args"],
            serde_json::json!(["mcp", "serve", "--stdio"])
        );
        assert_eq!(server["type"], "stdio");
    }

    /// **The binary Armada is, not the word `armada`.** A bare name resolves
    /// against the session's own `PATH`, and on a machine with two installations
    /// that is the other one — answering with another version's envelope.
    #[test]
    fn the_toolbelt_is_started_by_the_binary_that_registered_it() {
        assert!(mcp_json("/opt/armada/bin/armada").contains("/opt/armada/bin/armada"));
        assert!(!mcp_json("/opt/armada/bin/armada").contains("\"command\": \"armada\""));
    }

    /// The monitor tails the inbox, with `-F` so a machine whose first Job has
    /// not run yet still gets the events when it does.
    #[test]
    fn the_monitor_waits_for_an_inbox_that_does_not_exist_yet() {
        let json: serde_json::Value =
            serde_json::from_str(&monitors_json("/scratch/.armada/inbox.jsonl")).unwrap();
        assert_eq!(json[0]["name"], MONITOR);
        assert_eq!(json[0]["command"], "tail -F /scratch/.armada/inbox.jsonl");
        assert!(
            !json[0]["command"].as_str().unwrap().contains("~"),
            "a monitor's command is not necessarily expanded by a shell"
        );
    }

    /// The `Stop` hook registered by `--settings` is the one the launch wrote.
    #[test]
    fn the_stop_hook_registered_is_the_script_that_was_generated() {
        let json: serde_json::Value =
            serde_json::from_str(&settings_json("/scratch/.armada/helm/stop-inbox.sh")).unwrap();
        let hook = &json["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(hook["type"], "command");
        assert_eq!(hook["command"], "/scratch/.armada/helm/stop-inbox.sh");
    }

    /// **The backstop's contract, as bytes.** The block decision is what holds
    /// the turn open; without the literal `"decision":"block"` the hook runs,
    /// succeeds, and changes nothing.
    #[test]
    fn the_backstop_blocks_the_turn_rather_than_merely_reporting() {
        let script = stop_hook("/scratch/.armada/inbox.jsonl");
        assert!(script.starts_with("#!/bin/sh\n"));
        assert!(script.contains(r#"{"decision":"block""#), "{script}");
        assert!(script.contains(r#""answered":false"#), "{script}");
        // It names the tool rather than pasting the bodies: §15.2's first rule.
        assert!(script.contains("fleet.inbox"), "{script}");
    }

    /// **No dependency the machine might not have.** A backstop that needs `jq`
    /// is a backstop that stops working on the machine where it was never
    /// installed, and says nothing about it.
    #[test]
    fn the_backstop_needs_nothing_but_a_posix_shell() {
        let script = stop_hook("/scratch/.armada/inbox.jsonl");
        for tool in ["jq", "python", "node", "awk", "perl"] {
            assert!(
                !script.contains(tool),
                "the Stop hook depends on `{tool}`: {script}"
            );
        }
    }

    /// A `$HOME` with a space in it is ordinary on macOS, and would otherwise
    /// split the hook's assignment into two words.
    #[test]
    fn a_path_with_a_space_survives_the_shell() {
        let script = stop_hook("/Users/a b/.armada/inbox.jsonl");
        assert!(
            script.contains("inbox='/Users/a b/.armada/inbox.jsonl'"),
            "{script}"
        );
    }

    /// A path Armada cannot write into a document is refused rather than
    /// escaped, because the failure it would otherwise produce — an empty
    /// toolbelt — reports nothing at all.
    #[test]
    fn a_path_that_would_corrupt_a_document_is_refused() {
        assert!(writable("/Users/nobody/.armada"));
        assert!(!writable("/Users/no\nbody/.armada"));
        let error = refuse("~/.armada", "/Users/no\nbody/.armada");
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
    }

    /// A quote in a path is escaped rather than terminating the JSON string.
    #[test]
    fn a_quote_in_a_path_does_not_end_the_string_it_is_in() {
        let text = mcp_json("/opt/ar\"mada/armada");
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            json["mcpServers"][SERVER]["command"],
            "/opt/ar\"mada/armada"
        );
    }

    /// **The persona's `tools:` list and [`SERVER`] must agree**, and the
    /// template is the thing asserted against rather than a copy of it. A rename
    /// on one side alone leaves Helm with a tool list naming nothing — a Helm
    /// that cannot spawn, and does not say why.
    #[test]
    fn the_starter_persona_names_the_server_this_module_registers() {
        let template = include_str!("../../../templates/guild/subagents/helm.md");
        assert!(
            template.contains(&format!("mcp__{SERVER}__fleet_spawn")),
            "the starter persona does not name the `{SERVER}` server"
        );
        assert!(
            template.contains(&format!("name: {AGENT}")),
            "the starter persona is not called `{AGENT}`"
        );
    }

    /// A minted session is a valid version-4 uuid, because that is what
    /// `claude --session-id` accepts — the same rule the Drone's probe follows.
    #[test]
    fn a_minted_conversation_has_an_id_claude_code_would_accept() {
        let session = Session::mint("boot-1|1754748131|helm", AGENT);
        let groups: Vec<&str> = session.uuid.split('-').collect();
        assert!(groups.iter().map(|g| g.len()).eq([8, 4, 4, 4, 12]));
        assert!(groups
            .iter()
            .all(|g| g.chars().all(|c| c.is_ascii_hexdigit())));
        assert_eq!(&session.uuid[14..15], "4", "not a version-4 uuid");
        assert!(!session.started);
        assert!(!session.resumable_as(AGENT), "nothing has run yet");
    }

    /// The record round-trips, because losing it loses Helm's memory of the
    /// fleet — the one thing nothing else on the machine could rebuild.
    #[test]
    fn the_record_survives_being_written_and_read_back() {
        let session = Session::mint("seed", AGENT);
        let text = serde_json::to_string_pretty(&session).unwrap();
        assert_eq!(serde_json::from_str::<Session>(&text).unwrap(), session);
    }
}
