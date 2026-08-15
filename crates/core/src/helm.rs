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
/// It lives at `~/.armada/guild/subagents/<AGENT>.md` and is seeded from
/// `templates/guild/subagents/helm.md`. It is **yours** — but not frozen:
/// `armada guild upgrade` merges a later release's version into it, because
/// what is in it is operating knowledge rather than anything personal, and a
/// guild that could never receive it was `docs/reserved/006`. The three memory
/// fragments are the files no release ever touches.
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

/// The flag that carries the reader's own words into the session.
///
/// **`--append-system-prompt` rather than `--system-prompt`**: the session keeps
/// the persona and everything Claude Code normally is, and *gains* the reader's
/// standing instructions, rather than being reduced to them. It is in [`FLAGS`],
/// so `armada doctor` holds it against `claude --help` like every other flag the
/// launch uses.
pub const APPEND: &str = "--append-system-prompt";

/// The document [`voice`] assembles, written under `~/.armada/helm/`.
///
/// **Named so it cannot be mistaken for its sources.** `~/.armada/guild/voice.md`
/// is the reader's, hand-edited and synced; this is a generated file rewritten on
/// every launch, exactly like `mcp.json` beside it. A generated file called
/// `voice.md` would be edited by somebody once and silently regenerated the next
/// time.
pub const VOICE: &str = "guild-voice.md";

/// How many bytes of the reader's prose reach one launch.
///
/// **A real limit exists whether or not Armada names one, and discovering it at
/// `exec` is the bad way to find out.** `MAX_ARG_STRLEN` caps a single argv
/// element at 128 KiB on Linux, and macOS caps argv plus environ together at
/// 256 KiB; past either, `armada helm --exec` fails with *Argument list too
/// long* at the moment the reader was expecting a session, with nothing on
/// screen naming the file that caused it.
///
/// 24 KiB is far enough under both to leave the rest of the argv and the
/// environment room, and is several times the size of any of the three fragments
/// a person actually writes. Past it, [`voice`] cuts at a line boundary and says
/// so in the prompt and in the row `armada helm` prints — because a reader whose
/// instructions were silently halved would attribute the result to the model.
pub const VOICE_BUDGET: usize = 24 * 1024;

/// What the assembled prompt says before it says anything of the reader's.
///
/// **Three sentences, and the third is the load-bearing one.** The persona at
/// `templates/guild/subagents/helm.md` already states that the reader's files
/// win where they disagree with it; this says the same thing from the other
/// side, so the two halves of one system prompt cannot be read as competing
/// authorities. Anything longer would be Armada spending the reader's context to
/// introduce the reader.
const VOICE_HEADER: &str = "\
# Your user's own standing instructions

The sections below are the memory fragments of their guild. They wrote them; Armada did not.

**They are binding, and where anything in your persona disagrees with them, these win.**";

/// The reader's own words, assembled into one appended system prompt.
///
/// # Why this is injected rather than read
///
/// The persona used to *ask* Helm to read `~/.armada/guild/voice.md`,
/// `expectations.md` and `how-i-work.md` at the start of a session. It could
/// not: its `tools:` list is `mcp__armada__*` and holds no `Read`, so the
/// instruction was inert and Helm spoke in nobody's voice. Granting `Read` was
/// the cheaper repair and the worse one — it costs a turn per file, it widens a
/// toolbelt that is deliberately narrow ("never do the work" is enforced by the
/// absence of `Read`, `Edit` and `Bash`, not by a paragraph), and a session that
/// forgets to read them is a session that ignores the reader. Injection makes it
/// structural: the words are in the system prompt before the first turn, or the
/// launch says out loud that they are not.
///
/// This is the same repair `armada_guild::layout::skill_argv` already made one
/// file over, for the same reason — a session cannot be relied on to find a file
/// Armada could simply hand it.
///
/// # A fragment nobody has written is not injected
///
/// `fragments` is what the caller judged to be the reader's own — `armada guild
/// ls` already distinguishes a fragment that is *still Armada's example text*
/// from one that has been written. Injecting the examples would make Armada's
/// boilerplate binding in the reader's name, and the examples are generic advice
/// the persona's own defaults already cover. So a guild nobody has edited yields
/// [`None`] and no flag at all, which is honest and costs nothing.
///
/// # Order and precedence
///
/// The fragments are appended in the caller's order, under headings naming the
/// file each came from, so a reader can see which file a sentence came out of.
/// **Order inside the prompt is not precedence** — the header settles that
/// once, above all three — because three files by the same author do not
/// out-rank each other.
pub fn voice(fragments: &[(&str, &str)]) -> Option<Voice> {
    let mut prompt = String::from(VOICE_HEADER);
    let mut carried = Vec::new();
    let mut truncated = Vec::new();

    for (name, body) in fragments {
        let body = body.trim();
        if body.is_empty() {
            continue;
        }
        let heading = format!("\n\n## ~/.armada/guild/{name}\n\n");
        // The reservation is for the truncation note, which is written *after*
        // the budget has already been spent and must not be what overruns it.
        let room = VOICE_BUDGET.saturating_sub(prompt.len() + heading.len() + NOTE);
        prompt.push_str(&heading);
        if body.len() <= room {
            prompt.push_str(body);
        } else {
            let kept = cut_at_line(body, room);
            if !kept.is_empty() {
                prompt.push_str(kept);
                prompt.push('\n');
            }
            prompt.push_str(&format!(
                "\n[armada: `{name}` is {} bytes and a launch carries at most {VOICE_BUDGET}; \
                 the first {} of it reached this session. Shorten it, or move the detail into \
                 a skill.]",
                body.len(),
                kept.len()
            ));
            truncated.push((*name).to_string());
        }
        carried.push((*name).to_string());
    }

    match carried.is_empty() {
        true => None,
        false => Some(Voice {
            prompt,
            carried,
            truncated,
        }),
    }
}

/// Room held back for a truncation note, so that saying the prose was cut is
/// not itself what overruns [`VOICE_BUDGET`].
const NOTE: usize = 256;

/// The reader's words, and what had to be done to fit them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    /// The appended system prompt, exactly as it reaches the argv.
    ///
    /// **No trailing newline**, so that `"$(cat …)"` in [`launch_line`] produces
    /// this string byte for byte — command substitution strips trailing
    /// newlines, and a printed command that is not the argv is wrong in the one
    /// place a reader cannot check it.
    pub prompt: String,
    /// The fragments that reached it, in the order they appear.
    pub carried: Vec<String>,
    /// The fragments that reached it only in part.
    pub truncated: Vec<String>,
}

impl Voice {
    /// The bytes to write to `~/.armada/helm/`[`VOICE`].
    ///
    /// The prompt plus the newline every text file ends with — and the newline
    /// `"$(cat …)"` then strips back off, which is what keeps the file and the
    /// argv the same string.
    pub fn document(&self) -> String {
        format!("{}\n", self.prompt)
    }

    /// The one line `armada helm` prints about it.
    pub fn said(&self) -> String {
        let carried = self.carried.join(", ");
        match self.truncated.is_empty() {
            true => format!("{carried}: your words, and they outrank the persona"),
            false => format!(
                "{carried}: your words, and they outrank the persona; {} cut at {} KB",
                self.truncated.join(", "),
                VOICE_BUDGET / 1024
            ),
        }
    }
}

/// The largest prefix of `body` that is whole lines and fits in `room`.
///
/// Cutting mid-sentence would hand the session half an instruction, which reads
/// as a complete one; a line boundary is the coarsest cut that cannot invent a
/// rule the reader never wrote.
fn cut_at_line(body: &str, room: usize) -> &str {
    if room >= body.len() {
        return body;
    }
    let mut end = room;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    match body[..end].rfind('\n') {
        Some(newline) => &body[..newline],
        None => "",
    }
}

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
    /// The reader's own standing instructions, assembled by [`voice`].
    ///
    /// **[`None`] is a guild nobody has written yet**, not an error and not a
    /// degraded launch: the persona's own defaults already produce a terse Helm,
    /// and injecting the example text would make Armada's boilerplate binding in
    /// the reader's name.
    pub voice: Option<String>,
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
    ];
    // **Immediately after the persona, because it qualifies the persona.**
    // Everything below this is wiring — a toolbelt, a plugin, a hook — and none
    // of it has anything to say about how Helm talks. Position is not
    // precedence: what outranks what is settled in the prose ([`voice`]), and
    // has to be, because a flag's place in an argv is not something a model
    // reads.
    if let Some(voice) = &launch.voice {
        argv.push(APPEND.to_string());
        argv.push(voice.clone());
    }
    argv.extend([
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
    ]);
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
pub const FLAGS: [&str; 7] = [
    "--agent",
    APPEND,
    "--mcp-config",
    "--plugin-dir",
    "--settings",
    "--session-id",
    "--resume",
];

/// The same launch, written as a line a person can read and paste.
///
/// **Two renderings of one thing, and both have to work** — the rule
/// `armada_guild::layout::skill_command_line` already follows for the hand-over
/// it prints. [`launch_argv`] is what `--exec` becomes, with the reader's prose
/// already inlined; this is what `armada helm` *prints*, and inlining twenty
/// kilobytes of somebody's `voice.md` into that line would produce an
/// unreadable, unpasteable screenful in place of the one line this verb exists
/// to produce.
///
/// **Derived from the argv rather than built beside it**, so the two cannot
/// drift: every word is the argv's own except the appended prompt, which becomes
/// `"$(cat …)"` against the file the launch just wrote. That substitution
/// reproduces the argv byte for byte — [`Voice::document`] is the prompt plus
/// the single trailing newline that command substitution strips back off.
///
/// `voice_at` is the path as a person writes it, `~/…` rather than absolute:
/// this line is pasted and screen-shared, and an absolute home is both longer to
/// read and somebody's name in a terminal.
pub fn launch_line(argv: &[String], voice_at: &str) -> String {
    let mut out: Vec<String> = Vec::with_capacity(argv.len());
    let mut prose = false;
    for word in argv {
        if prose {
            out.push(format!("\"$(cat {voice_at})\""));
            prose = false;
            continue;
        }
        prose = word == APPEND;
        out.push(word.clone());
    }
    out.join(" ")
}

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
            voice: None,
        }
    }

    /// The same launch on a machine whose reader has written their guild.
    fn a_spoken_launch() -> Launch {
        Launch {
            voice: Some(a_voice().prompt),
            ..a_launch(false)
        }
    }

    /// Three fragments in the reader's own words — **written here rather than
    /// copied from anybody's machine.** This repository is public and a fixture
    /// is the easiest place for somebody's real memory file to end up in it.
    fn a_voice() -> Voice {
        voice(&[
            ("voice.md", "# Voice\n\nAnswer in one line. No preamble.\n"),
            (
                "expectations.md",
                "# Expectations\n\nGreen suite, or say so.\n",
            ),
        ])
        .expect("two written fragments")
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
            .chain(launch_argv(&a_spoken_launch()))
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

    /// **The reader's own words are in the argv, not merely referred to by it.**
    ///
    /// This is the assertion the whole change exists for. The persona used to
    /// *ask* Helm to read the three files and Helm had no `Read` tool, so the
    /// instruction was inert and nothing on the machine said so. A launch that
    /// named the paths — or that granted `Read` and hoped — would pass a laxer
    /// test than this one and would still be able to start a session that had
    /// never seen a word the reader wrote.
    #[test]
    fn the_readers_own_words_reach_the_session_as_bytes() {
        let argv = launch_argv(&a_spoken_launch());
        let at = argv
            .iter()
            .position(|word| word == APPEND)
            .expect("the reader's words are not in the launch at all");
        // Immediately after the persona it qualifies, and before the wiring.
        assert_eq!(&argv[..at], ["claude", "--agent", "helm"]);
        let prompt = &argv[at + 1];
        assert!(prompt.contains("Answer in one line."), "{prompt}");
        assert!(prompt.contains("Green suite, or say so."), "{prompt}");
        // Each is attributed to the file it came out of.
        assert!(prompt.contains("~/.armada/guild/voice.md"), "{prompt}");
        assert!(
            prompt.contains("~/.armada/guild/expectations.md"),
            "{prompt}"
        );
        // **The persona does not have to be argued with.** It already says the
        // reader's files win; the appended half says the same from its side, so
        // one system prompt cannot read as two competing authorities.
        assert!(
            prompt.contains("these win"),
            "the prompt does not settle precedence: {prompt}"
        );
        assert_eq!(&argv[at + 2..at + 4], ["--mcp-config", MCP]);
    }

    const MCP: &str = "/scratch/.armada/helm/mcp.json";

    /// **A guild nobody has written injects nothing at all** — no flag, no
    /// empty prompt, no heading with nothing under it. Armada's example text
    /// made binding in the reader's name is worse than the persona's own
    /// defaults, which already produce a terse Helm.
    #[test]
    fn an_unwritten_guild_adds_no_flag_and_no_empty_prompt() {
        assert_eq!(voice(&[]), None);
        assert_eq!(
            voice(&[("voice.md", "   \n\n")]),
            None,
            "whitespace is not a voice"
        );
        let argv = launch_argv(&a_launch(false));
        assert!(!argv.iter().any(|word| word == APPEND), "{argv:?}");
    }

    /// **Prose with no length bound meets a launch with one.** A single argv
    /// element is capped at 128 KiB on Linux and argv plus environ at 256 KiB on
    /// macOS; past either, `--exec` fails with *Argument list too long* and
    /// names no file. So the cut happens here, at a line boundary, and says so
    /// in the prompt itself — a reader whose instructions were silently halved
    /// would blame the model.
    #[test]
    fn prose_past_the_budget_is_cut_at_a_line_and_says_so() {
        let long = "A line of somebody's standing instructions.\n".repeat(2_000);
        let voice = voice(&[("how-i-work.md", &long)]).expect("it is written");
        assert!(long.len() > VOICE_BUDGET, "the fixture is not over budget");
        assert!(
            voice.prompt.len() <= VOICE_BUDGET,
            "{} bytes would be handed to exec",
            voice.prompt.len()
        );
        assert!(voice.prompt.ends_with(']'), "{}", voice.prompt);
        assert!(voice.prompt.contains("Shorten it"), "{}", voice.prompt);
        assert_eq!(voice.truncated, ["how-i-work.md"]);
        assert!(voice.said().contains("cut at 24 KB"), "{}", voice.said());
        // Whole lines only: half a sentence reads as a whole instruction, and
        // the one that would be cut here ends mid-word.
        assert!(
            !voice.prompt.contains("A line of somebody's standing in\n"),
            "a line was cut mid-word"
        );
        let partial = voice
            .prompt
            .lines()
            .filter(|line| line.starts_with("A line"))
            .find(|line| *line != "A line of somebody's standing instructions.");
        assert_eq!(partial, None, "a line of the reader's prose was halved");
    }

    /// **The printed line and the argv are the same launch.** A reader who
    /// pastes what `armada helm` printed and a reader whose `--exec` was
    /// exec'd must land in the same session; `"$(cat …)"` is what makes the
    /// second reproduce the first, and it reproduces it exactly because
    /// [`Voice::document`] adds the one newline the substitution strips.
    #[test]
    fn the_printed_line_reproduces_the_argv_it_stands_for() {
        let voice = a_voice();
        let argv = launch_argv(&a_spoken_launch());
        let line = launch_line(&argv, "~/.armada/helm/guild-voice.md");

        assert!(
            line.contains("--append-system-prompt \"$(cat ~/.armada/helm/guild-voice.md)\""),
            "{line}"
        );
        assert!(
            !line.contains("Answer in one line."),
            "the prose is inlined: {line}"
        );
        assert!(
            !line.contains('\n'),
            "the printed command is one line: {line}"
        );
        // The substitution's own guarantee: the file is the prompt plus the
        // newline `$(cat …)` removes.
        assert_eq!(voice.document(), format!("{}\n", voice.prompt));
        assert_eq!(voice.document().trim_end_matches('\n'), voice.prompt);
        // Every other word is the argv's own, untouched.
        for word in ["claude", "--agent", "helm", "--mcp-config", MCP] {
            assert!(line.contains(word), "{word} is missing from {line}");
        }
    }

    /// A launch with nothing to append prints exactly what it did before, so
    /// the substitution cannot appear on a machine that has no voice file.
    #[test]
    fn a_silent_launch_prints_the_plain_argv() {
        let argv = launch_argv(&a_launch(false));
        assert_eq!(
            launch_line(&argv, "~/.armada/helm/guild-voice.md"),
            argv.join(" ")
        );
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
