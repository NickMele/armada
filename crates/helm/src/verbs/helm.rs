//! `armada helm` — **assemble the conversation, and enter it if this machine
//! allows it**.
//!
//! Every decision this file appears to make was made somewhere else: the argv,
//! the documents and the record are `armada_core::helm`'s, and the guild
//! that holds the persona is `armada_guild`'s. What lives here is the order the
//! adapter calls go in, and the two refusals — no guild or no persona, and a
//! `helm.mode` Claude Code does not have — that both have to happen before any
//! of them (`ARCHITECTURE.md` §1.3).
//!
//! # One lock, and the machine holds it
//!
//! Entering spends, and assembling does not:
//!
//! | | Costs |
//! |---|---|
//! | Assembling the command | nothing |
//! | Entering the session | a real budget, against a real account, for as long as it is open |
//!
//! So there is a lock: `helm.enter` in `~/.armada/machine.yml`, read through
//! [`entering_allowed`], off on a fresh install, flipped by `armada helm enable`
//! and put back by `armada helm disable`. Off, `armada helm` is refused by name
//! and with a reason, via [`entering_is_off`], never as an unknown flag —
//! "unknown flag" reads as a bug and invites somebody to go looking for the
//! spelling that works.
//!
//! **There used to be a second lock, and it was the defect.** Entering was also
//! behind [`ENTER`], so a reader who had already set `helm.enter: true` typed
//! `armada helm`, got a command printed for them to paste, and reported it:
//! *"it didn't dump me into a Claude Code session. It asked me to run the
//! command, which was weird."* They were right. Flipping the machine switch **is**
//! the yes; asking for it twice does not make the second answer more
//! considered, it makes the first one look ignored.
//!
//! | Invocation | What happens |
//! |---|---|
//! | `armada helm`, `helm.enter` on | enters the session |
//! | `armada helm`, `helm.enter` off | [`entering_is_off`], and nothing is written |
//! | `armada helm --print-command` ([`PRINT`]) | prints the pasteable line; never enters |
//! | `armada helm --json` | the envelope, argv and all; never enters |
//! | `armada helm --exec` ([`ENTER`]) | enters — the explicit spelling, kept for the scripts and the fingers that already have it |
//!
//! **The printed line is kept, and moved rather than deleted.** It is how a
//! person reads the argv Armada built for them without spending anything to see
//! it, and it is what a caller that wants to open the session *somewhere else* —
//! the workspace hand-off of `docs/reserved/020` §9 — pastes. [`PRINT`] is where
//! it lives now.
//!
//! **`--json` does not enter either, and that is the same rule.** A caller
//! asking for the envelope is asking for output, and a process replaced by
//! `claude` never prints one — so a script that read the argv yesterday reads it
//! today rather than finding itself inside somebody's conversation.
//!
//! **The argv, the documents and the conversation's record are built and
//! verified the same way whichever of those it is.** [`mark_started`] is the
//! writer the entering path calls — first, because the process is replaced and
//! there is no after.
//!
//! **Bare `armada` is still not wired to this.** `docs/reserved/020` §8 gives
//! the bare word a menu rather than Helm, and that is a different verb.
//!
//! # Machine-scoped, like Fleet
//!
//! Helm runs before workspace resolution: its whole subject is the fleet across
//! every repository, and routing it through a `armada.yml` would refuse to
//! start the orchestrator from any directory that is not a workspace — which is
//! most directories, including the one a person opens a terminal in.

use armada_core::envelope::{Conversation, Envelope, HelmData, HelmSwitchData, Wired, Wiring};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::helm::{self, Launch, Session};
use armada_guild::layout::Guild;
use std::path::{Path, PathBuf};

use crate::verbs::Output;

/// The explicit spelling of entering — **a synonym now, not a second lock**.
///
/// `armada helm` on a machine whose `helm.enter` is on does exactly what this
/// does. It is kept because it is typed in scripts and in muscle memory, and
/// because a flag that stops working is a worse answer than a flag that agrees
/// with the default.
///
/// Named here rather than typed in four files, because the parser, the help
/// page, the render's summary line and the refusal all have to say the same
/// word.
pub const ENTER: &str = "--exec";

/// The flag that asks for the line and nothing else.
///
/// **Where the old default went.** Printing the command used to be what bare
/// `armada helm` did, which is how a machine that had already said yes ended up
/// being asked to paste its own launch. The behaviour is worth keeping and the
/// default is not: it is how a person reads the argv, and it costs nothing.
///
/// Named beside [`ENTER`] for the same reason [`ENTER`] is named at all.
pub const PRINT: &str = "--print-command";

/// Why entering is refused, in **the one place every surface reads it from**.
///
/// A gate whose reason is retyped per call site is a gate that says three
/// different things by the third edit, and the one that reads as an accident is
/// the one somebody works around. The parser, [`entering_is_off`], the help page
/// and the render all read this string, so the decision moves in one place or
/// not at all.
pub const ENTER_IS_OFF: &str = "off on this machine";

/// The verb that turns [`ENTER`] on, read by [`entering_is_off`], the help
/// page and `armada doctor`'s row — so the three cannot drift into naming
/// three different commands for the same switch.
pub const ENABLE: &str = "armada helm enable";

/// The verb that puts it back — the state a fresh install is already in.
pub const DISABLE: &str = "armada helm disable";

/// The refusal entering gets when [`entering_allowed`] says no — whether it was
/// asked for by bare `armada helm` or by [`ENTER`].
///
/// **`bad_invocation`, which is the class this CLI already uses for a flag it
/// knows and has not built** — `armada doctor --fix` and `armada manifest check
/// --detach` are both refused this way. No new class: an invented one would say
/// this refusal is a different kind of thing from those two, and it is not.
///
/// **Refused by name, never as an unknown flag.** A caller told "unknown flag"
/// concludes Armada is broken or that they typed it wrong, and goes looking for
/// the spelling that works. Told that it is off and how to turn it on, they
/// either run [`ENABLE`] or ask for the line with [`PRINT`] and paste it
/// themselves — both of which `next_action` offers.
///
/// **One refusal for both spellings, because they are one act.** `armada helm`
/// and `armada helm --exec` enter the same session; two refusals worded
/// differently would be the second lock growing back as prose.
pub fn entering_is_off() -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: "helm".to_string(),
        message: format!(
            "entering is {ENTER_IS_OFF}: `armada helm` — and `armada helm {ENTER}`, which \
             is the same act spelled out — opens a Claude Code session, and this machine \
             has not said yes to that yet"
        ),
        next_action: Some(format!(
            "`{ENABLE}` turns it on here; `armada helm {PRINT}` prints the line to paste \
             and enters nothing"
        )),
    }
}

/// Whether [`ENTER`] would become a session **on this machine, right now**.
///
/// Reads `helm.enter` from [`crate::machine`] — off on a fresh install, so a
/// machine nobody has said yes to cannot open one.
pub fn entering_allowed(armada_home: &Path) -> bool {
    crate::machine::read(armada_home).enter
}

/// The `--permission-mode` this machine's Helm enters under, checked before it
/// can reach an argv.
///
/// **Read here and validated here, one layer above the argv.** `helm.mode` is
/// free text in a file a person edits, and `--permission-mode` is checked by
/// Claude Code at argument-parse time (`docs/traps.md`) — so an unrecognised
/// value is a session that dies the instant it is entered, with Claude Code's
/// usage error printed over whatever the reader was doing and no mention of the
/// file that caused it. Held against `armada_core::helm::MODES`, which is the
/// same list `crates/core/src/fleet/drone.rs` refuses a Drone's posture against,
/// the failure is a named `bad_config` naming the file, the key and the six
/// values.
pub fn mode(armada_home: &Path) -> Result<String, ArmadaError> {
    let mode = crate::machine::read(armada_home).mode;
    match helm::MODES.contains(&mode.as_str()) {
        true => Ok(mode),
        false => Err(helm::bad_mode(&mode)),
    }
}

/// The `--model` this machine's Helm enters under.
///
/// **Read but not validated**, which is the difference from [`mode`] above and
/// is argued at [`armada_core::helm::MODEL`]: Claude Code publishes a closed set
/// of permission modes and an open set of models, so a list here would refuse a
/// model released after this build. The binary's own refusal is the check, and
/// it is the current one.
pub fn model(armada_home: &Path) -> String {
    crate::machine::read(armada_home).model
}

/// `armada helm enable` — let `armada helm` become a session on this machine.
///
/// **This is the yes, and the only one.** Nothing downstream asks again: with
/// it on, the bare verb enters.
///
/// **Writes one boolean, and nothing else.** It does not touch the guild, the
/// persona or any of the documents `armada helm` wires — whether a
/// machine is *allowed* to open a session and whether it is currently *able*
/// to (a guild, a persona, a projection) are different questions, answered by
/// different commands. Entering still needs both.
pub fn enable(armada_home: &Path) -> Result<Output, ArmadaError> {
    switch(armada_home, true, "helm enable")
}

/// `armada helm disable` — put the switch back where a fresh install leaves
/// it.
pub fn disable(armada_home: &Path) -> Result<Output, ArmadaError> {
    switch(armada_home, false, "helm disable")
}

/// The one write path [`enable`] and [`disable`] share: read, compare, write
/// only on a change, report the result either way.
fn switch(armada_home: &Path, enter: bool, verb: &str) -> Result<Output, ArmadaError> {
    let before = entering_allowed(armada_home);
    crate::machine::set_enter(armada_home, enter).map_err(|error| ArmadaError {
        class: ErrClass::Environment,
        r#where: armada_home.join("machine.yml").display().to_string(),
        message: format!(
            "cannot write {}: {error}",
            armada_home.join("machine.yml").display()
        ),
        next_action: Some("check the permissions on ~/.armada/".to_string()),
    })?;
    Ok(Output::HelmSwitch(Box::new(Envelope::ok(
        verb,
        None,
        Status::Ok,
        HelmSwitchData {
            entering: enter,
            changed: before != enter,
        },
    ))))
}

/// Everything `armada helm` needs from the machine, gathered at the entrypoint.
///
/// **`$HOME` and the current directory arrive as values** — nothing below the
/// entrypoint reads either (`ARCHITECTURE.md` §1.4) — which is also what lets
/// the suite point this verb at a `TempDir` rather than at somebody's real
/// `~/.armada/` and real `~/.claude/`.
pub struct Where {
    /// `$HOME`, for writing a path the way a person writes one.
    pub home: PathBuf,
    /// `~/.armada/`.
    pub armada_home: PathBuf,
    /// `~/.claude/` — where the persona has to have been projected to.
    pub claude_home: PathBuf,
    /// The `armada` binary itself, which is what the toolbelt registration
    /// names.
    pub exe: PathBuf,
    /// This boot, which seeds the conversation's id.
    pub boot_id: String,
}

impl Where {
    /// `~/.armada/helm/`.
    pub fn helm_home(&self) -> PathBuf {
        self.armada_home.join(helm::DIRECTORY)
    }

    /// A path as a person writes it.
    fn shown(&self, path: &Path) -> String {
        armada_fleet::home::tilde(path, &self.home)
    }
}

/// What the line asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// `--agent <name>`: a persona other than `helm`.
    pub agent: Option<String>,
    /// `--new`: start a fresh conversation instead of resuming.
    pub new: bool,
}

/// Assemble the launch, wire the inbox, and report.
///
/// **Nothing here starts a process**, on any path. The documents are written,
/// the conversation's record is read or minted, and the argv is returned — as a
/// vector for the caller to become, and as a string for a person to read. Which
/// of those happens is the entrypoint's decision, not this function's.
pub fn run<C: armada_core::ctx::Clock>(
    now: &C,
    place: &Where,
    options: &Options,
) -> Result<Output, ArmadaError> {
    let agent = options
        .agent
        .clone()
        .unwrap_or_else(|| helm::AGENT.to_string());
    let guild = Guild::at(&place.armada_home);

    // **Both refusals come first, before anything is written.** A `helm` that
    // laid down its configuration files and then admitted it has no persona to
    // run — or a mode Claude Code would reject at argument-parse time — would
    // have changed the machine in order to report a failure.
    persona(place, &guild, &agent)?;
    let mode = mode(&place.armada_home)?;

    let (session, minted) = conversation(now, place, &agent, options.new)?;
    let voice = voice(&guild);
    let wiring = wire(place, &session, minted, voice.as_ref())?;

    let launch = Launch {
        agent: agent.clone(),
        session: session.clone(),
        mcp_config: paths(place).mcp_config.display().to_string(),
        plugin_dir: paths(place).plugin_dir.display().to_string(),
        settings: paths(place).settings.display().to_string(),
        mode,
        model: model(&place.armada_home),
        voice: voice.as_ref().map(|voice| voice.prompt.clone()),
    };
    let argv = helm::launch_argv(&launch);
    let command = helm::launch_line(&argv, &place.shown(&paths(place).voice));
    Ok(Output::Helm(Box::new(Envelope::ok(
        "helm",
        None,
        Status::Ok,
        HelmData {
            agent,
            uuid: session.uuid.clone(),
            conversation: match session.started {
                true => Conversation::Resumed,
                false => Conversation::New,
            },
            argv,
            command,
            results: wiring,
            launched: false,
            entering: entering_allowed(&place.armada_home),
        },
    ))))
}

/// Where each generated document goes.
struct Paths {
    session: PathBuf,
    mcp_config: PathBuf,
    plugin_dir: PathBuf,
    plugin_manifest: PathBuf,
    monitors: PathBuf,
    settings: PathBuf,
    stop_hook: PathBuf,
    voice: PathBuf,
}

fn paths(place: &Where) -> Paths {
    let root = place.helm_home();
    Paths {
        session: root.join("session.json"),
        mcp_config: root.join("mcp.json"),
        plugin_dir: root.join("plugin"),
        plugin_manifest: root.join("plugin/.claude-plugin/plugin.json"),
        monitors: root.join("plugin/monitors/monitors.json"),
        settings: root.join("settings.json"),
        stop_hook: root.join("stop-inbox.sh"),
        voice: root.join(helm::VOICE),
    }
}

/// **The reader's own three files, read here and handed to the launch.**
///
/// The persona at `templates/guild/subagents/helm.md` used to *instruct* Helm to
/// read `~/.armada/guild/voice.md`, `expectations.md` and `how-i-work.md` at the
/// start of a session. It could not: its `tools:` list is `mcp__armada__*` and
/// contains no `Read` — deliberately, because "never do the work" is enforced by
/// the absence of `Read`, `Edit` and `Bash` rather than by a paragraph. So the
/// instruction was inert, and Helm spoke in nobody's voice while a file on the
/// machine said exactly how to speak.
///
/// **Granting `Read` was the cheaper repair and the worse one.** It would have
/// widened a toolbelt that is narrow on purpose, spent a turn per file, and left
/// the outcome to a session remembering to do it — and a session that forgets is
/// a session that ignores the reader. Reading them here makes it structural: the
/// words are in the system prompt before the first turn or `armada helm` says
/// out loud that they are not. It is the repair `armada_guild::layout::skill_argv`
/// already made for `config scan`'s hand-over, one file over.
///
/// **A fragment still holding Armada's example text is not the reader's, and is
/// skipped** — [`armada_guild::memory::state`] is the same test `armada guild
/// ls` reports as *still Armada's example text*. Injecting the examples would
/// make Armada's boilerplate binding in the reader's name, and they are generic
/// advice the persona's own defaults already carry. Missing, unreadable and
/// blank files are skipped for the same reason and reported the same way.
///
/// **This is the only place Armada reads those three files**, and it reads them
/// off the guild it was given — never off `$HOME`, which nothing below the
/// entrypoint may look at (`ARCHITECTURE.md` §1.4). That is also what lets the
/// suite point it at a scratch guild rather than at somebody's real one.
fn voice(guild: &Guild) -> Option<helm::Voice> {
    let written: Vec<(&str, String)> = armada_guild::memory::FRAGMENTS
        .iter()
        .filter_map(|name| {
            let body = std::fs::read_to_string(guild.path(name)).ok()?;
            match armada_guild::memory::state(&body) {
                Some(_) => None,
                None => Some((*name, body)),
            }
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = written
        .iter()
        .map(|(name, body)| (*name, body.as_str()))
        .collect();
    helm::voice(&borrowed)
}

/// What the wired row says when none of the three is the reader's yet.
///
/// **Said rather than left silent, and this is the row that would have caught
/// the bug.** A launch that quietly appended nothing looks exactly like a launch
/// that appended everything; the reader's complaint — *"Helm is going to be very
/// verbose"* — was true for months with the instruction sitting unread in the
/// persona. A row naming the files and the command that fills them turns that
/// into something visible on every single launch.
///
/// **It says *none of yours* rather than *nothing*** since `docs/reserved/008`:
/// the file at this path is no longer empty when the reader has written nothing,
/// because Armada's own skill is in it, and a row saying `none yet` beside a
/// document with two kilobytes in it would be the row lying about the file it
/// names.
fn no_voice_yet() -> String {
    format!(
        "none of yours yet — {} are Armada's words; `armada guild edit voice.md`",
        armada_guild::memory::FRAGMENTS.join(", ")
    )
}

/// Refuse a machine that has no guild, or a guild with no such persona.
///
/// **Three separate refusals, because three different things fix them.** No
/// guild is `armada init`; a guild without the persona is a file the reader
/// deleted; a persona the guild has and Claude Code cannot see is a projection
/// that has not run. Collapsing them into "no persona" would send a reader to
/// the wrong command in two cases out of three.
fn persona(place: &Where, guild: &Guild, agent: &str) -> Result<(), ArmadaError> {
    if !guild.exists() {
        return Err(bad_config(
            "guild",
            "this machine has no guild, so there is no Helm persona to run",
            "`armada init` sets up this machine and seeds your guild",
        ));
    }

    let source = guild.path(&format!("subagents/{agent}.md"));
    if !source.is_file() {
        return Err(bad_config(
            &format!("subagents/{agent}.md"),
            &format!("your guild has no `{agent}` persona"),
            match agent == helm::AGENT {
                true => "`armada guild init` seeds it; it is yours to edit afterwards".to_string(),
                false => format!(
                    "`armada helm` runs `{}`; --agent names a file in \
                     ~/.armada/guild/subagents/",
                    helm::AGENT
                ),
            }
            .as_str(),
        ));
    }

    // **Claude Code reads `~/.claude/agents/`, and the guild is not on that
    // path until projection puts it there** (`PHASES.md` §8.4). `--agent` names
    // a persona Claude Code has to be able to find, so a guild that has it and
    // a `~/.claude/` that does not is the one failure that would otherwise
    // surface as an ordinary session wearing Helm's name and none of its rules.
    let projected = place.claude_home.join(format!("agents/{agent}.md"));
    if !projected.is_file() {
        return Err(bad_config(
            &place.shown(&projected),
            &format!("`{agent}` is in your guild and not on Claude Code's load path"),
            "`armada guild project` puts it there",
        ));
    }
    Ok(())
}

/// The conversation this launch is, read from disk or minted.
///
/// **A different persona is a different conversation**, and so is `--new`.
/// Resuming one persona's session under another's would hand the second one
/// commitments it never made and the memory of a fleet it never ran.
fn conversation<C: armada_core::ctx::Clock>(
    now: &C,
    place: &Where,
    agent: &str,
    new: bool,
) -> Result<(Session, Wiring), ArmadaError> {
    let path = paths(place).session;
    let existing = std::fs::read_to_string(&path)
        .ok()
        // **A record that will not parse is replaced rather than fatal.** It is
        // a uuid and a boolean; refusing to start the orchestrator because one
        // line of JSON is corrupt would cost the reader the one tool that can
        // tell them what the fleet is doing.
        .and_then(|text| serde_json::from_str::<Session>(&text).ok());

    if !new {
        if let Some(session) = existing {
            if session.agent == agent {
                return Ok((session, Wiring::Unchanged));
            }
        }
    }

    let session = Session::mint(
        &format!("{}|{}|{agent}", place.boot_id, now.wall_ms()),
        agent,
    );
    write(&path, &record(&session)?)?;
    Ok((session, Wiring::Written))
}

/// The record, as the bytes that go on disk.
fn record(session: &Session) -> Result<String, ArmadaError> {
    serde_json::to_string_pretty(session)
        .map(|text| text + "\n")
        .map_err(|error| ArmadaError {
            class: ErrClass::ArmadaBug,
            r#where: "helm/session.json".to_string(),
            message: format!("the conversation record would not serialise: {error}"),
            next_action: None,
        })
}

/// Record that the conversation now exists, so the next launch resumes it.
///
/// **Written by the one path that hands the process over, and by nothing
/// else.** Whether a session exists is Claude Code's fact, not Armada's: the
/// only moment Armada knows for certain that a uuid has been minted is the
/// moment it becomes the process that mints it. Setting the flag anywhere
/// earlier — when the command is merely reported, say — would make the next
/// launch `--resume` a conversation nobody ever started, which fails with *no
/// conversation found* and is indistinguishable from a lost transcript.
///
/// **Its caller is the entering path, and no test may be that caller.** No test
/// in this repository starts a session (`PHASES.md` §8.5), so the suite
/// exercises this function directly instead — which is what closes the one link
/// the rest of the suite cannot: the flag that makes the next launch say
/// `--resume` is the flag this function writes.
pub fn mark_started(place: &Where, session: &Session) -> Result<(), ArmadaError> {
    let started = Session {
        started: true,
        ..session.clone()
    };
    write(&paths(place).session, &record(&started)?)?;
    Ok(())
}

/// Write the four documents the inbox and the toolbelt need, and say what
/// changed.
///
/// **Rewritten on every launch rather than written once.** Each one names a path
/// on this machine — the `armada` binary, the inbox — and a machine whose home
/// directory moved, or whose Armada was reinstalled elsewhere, would otherwise
/// keep a registration pointing at a binary that is not there. Regenerating is
/// cheap and the diff is reported, so a reader who edited one by hand sees that
/// it was replaced.
fn wire(
    place: &Where,
    session: &Session,
    minted: Wiring,
    voice: Option<&helm::Voice>,
) -> Result<Vec<Wired>, ArmadaError> {
    let paths = paths(place);
    let inbox = armada_fleet::home::inbox(&place.armada_home);
    let exe = paths_ok(place, &inbox)?;

    let mut wired = Vec::new();
    wired.push(Wired {
        what: "toolbelt".to_string(),
        at: place.shown(&paths.mcp_config),
        state: write(&paths.mcp_config, &helm::mcp_json(&exe))?,
        detail: format!("{} over stdio: fleet.* and manifest.*", helm::SERVER),
    });
    write(&paths.plugin_manifest, &helm::plugin_json())?;
    wired.push(Wired {
        what: "monitor".to_string(),
        at: place.shown(&paths.plugin_dir),
        state: write(
            &paths.monitors,
            &helm::monitors_json(&inbox.display().to_string()),
        )?,
        detail: "live push: every inbox line arrives mid-turn".to_string(),
    });
    let hook = write_executable(
        &paths.stop_hook,
        &helm::stop_hook(&inbox.display().to_string(), &exe),
    )?;
    write(
        &paths.settings,
        &helm::settings_json(&paths.stop_hook.display().to_string()),
    )?;
    wired.push(Wired {
        what: "backstop".to_string(),
        at: place.shown(&paths.stop_hook),
        state: hook,
        detail: "Stop hook: the inbox is read, and the fleet is caught up".to_string(),
    });
    // **The appended system prompt, and it is now always written.** The document
    // is what `"$(cat …)"` in the printed command reads, so it has to be on disk
    // before that line is printed and it has to be byte-identical to the argv
    // element — a printed command that is not the argv is wrong in the one place
    // a reader has no way to check.
    //
    // **It used to be removed when the reader had written nothing**, because
    // then the launch emitted no flag at all. Since `docs/reserved/008` the
    // launch always appends Armada's own skill, so there is always something to
    // write, and a removal here would leave `"$(cat …)"` pointing at a file that
    // is not there.
    let appended = helm::appended(voice.map(|voice| voice.prompt.as_str()));
    wired.push(Wired {
        what: "voice".to_string(),
        at: place.shown(&paths.voice),
        state: write(&paths.voice, &format!("{appended}\n"))?,
        detail: match voice {
            // **Armada's half is not mentioned when the reader has a half.** The
            // row exists to answer *did my words reach the session*, which is
            // the question the bug was about; naming Armada's own instructions
            // beside them would put Armada's name on the line that is meant to
            // be about theirs.
            Some(voice) => voice.said(),
            None => no_voice_yet(),
        },
    });
    wired.push(Wired {
        what: "conversation".to_string(),
        at: place.shown(&paths.session),
        state: minted,
        detail: match session.started {
            true => format!("resumed: {}", armada_core::fleet::job::short(&session.uuid)),
            false => "not started yet: the next launch mints it".to_string(),
        },
    });
    Ok(wired)
}

/// Every path that reaches a generated document, checked before one is written.
///
/// A newline or a control character in `$HOME` produces a `--mcp-config` Claude
/// Code will not parse, and that failure surfaces as an orchestrator with an
/// empty toolbelt and no error anywhere — so it is refused by name instead.
fn paths_ok(place: &Where, inbox: &Path) -> Result<String, ArmadaError> {
    let exe = place.exe.display().to_string();
    for (what, value) in [
        ("armada", exe.as_str()),
        ("~/.armada/inbox.jsonl", &inbox.display().to_string()),
        ("~/.armada/helm", &place.helm_home().display().to_string()),
    ] {
        if !helm::writable(value) {
            return Err(helm::refuse(what, value));
        }
    }
    Ok(exe)
}

/// Write a file, and say whether it changed.
fn write(path: &Path, body: &str) -> Result<Wiring, ArmadaError> {
    if std::fs::read_to_string(path).is_ok_and(|existing| existing == body) {
        return Ok(Wiring::Unchanged);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(unwritable(path))?;
    }
    std::fs::write(path, body).map_err(unwritable(path))?;
    Ok(Wiring::Written)
}

/// The same, plus the mode a hook has to have to be run at all.
///
/// **A `Stop` hook that is not executable is a backstop that silently is not
/// one.** Claude Code runs it as a command; a file without the bit set fails to
/// start, and nothing on the machine reports that the thing guaranteeing
/// nothing is lost is doing nothing.
fn write_executable(path: &Path, body: &str) -> Result<Wiring, ArmadaError> {
    let state = write(path, body)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(unwritable(path))?;
    }
    Ok(state)
}

fn unwritable(path: &Path) -> impl Fn(std::io::Error) -> ArmadaError + '_ {
    move |error| ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot write {}: {error}", path.display()),
        next_action: Some("check the permissions on ~/.armada/".to_string()),
    }
}

fn bad_config(r#where: &str, message: &str, next_action: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadConfig,
        r#where: r#where.to_string(),
        message: message.to_string(),
        next_action: Some(next_action.to_string()),
    }
}
