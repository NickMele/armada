//! `armada helm` — **assemble the conversation; do not enter it**.
//!
//! Every decision this file appears to make was made somewhere else: the argv,
//! the four documents and the record are `armada_core::helm`'s, and the guild
//! that holds the persona is `armada_guild`'s. What lives here is the order the
//! adapter calls go in, and the one refusal — no guild, no persona — that has to
//! happen before any of them (`ARCHITECTURE.md` §1.3).
//!
//! # Entering is a separate act, gated by a switch you control
//!
//! `armada helm` writes the configuration, reports the command, and **starts
//! nothing**. Entering the session is [`ENTER`], and the split is not caution
//! for its own sake:
//!
//! | | Costs |
//! |---|---|
//! | Assembling the command | nothing |
//! | Entering the session | a real budget, against a real account, for as long as it is open |
//!
//! A verb that opened a Claude Code session as a side effect of being run can be
//! reached by a script, by a shell alias, by a test harness and by a mistyped
//! line — and each of those spends.
//!
//! **So [`ENTER`] is refused unless this machine has said otherwise.** Whether
//! it may become a session lives in [`crate::machine`] — `helm.enter` in
//! `~/.armada/machine.yml`, off on a fresh install, flipped by `armada helm
//! enable` and put back by `armada helm disable`. Refused, it is refused by
//! name and with a reason, via [`entering_is_off`], never as an unknown flag:
//! "unknown flag" reads as a bug and invites somebody to go looking for the
//! spelling that works.
//!
//! **This used to be an unconditional refusal, pending the Bridge.** The Bridge
//! is now built, which is what made a real decision possible here instead of a
//! blanket no. What replaced the blanket refusal is a switch rather than an
//! unconditional yes, because assembling costs nothing and entering spends a
//! real budget for as long as it is open — a decision that deserves an
//! explicit yes on each machine, not a default that changed out from under
//! whoever had not been reading release notes.
//!
//! **The argv, the four documents and the conversation's record are built and
//! verified the same way whether the switch is on or off.** [`mark_started`] is
//! the writer the exec path calls — first, because the process is replaced and
//! there is no after.
//!
//! **The same reasoning is why bare `armada` is not wired to it.** PLAN.md §15.1
//! says typing `armada` with no arguments enters Helm; that is the intended end
//! state and it is deliberately not this milestone's, because the bare word is
//! the single most typeable thing on the machine and the failure mode of getting
//! it wrong is a session nobody meant to open.
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

/// The flag that would turn the assembled command into a session.
///
/// Named here rather than typed in four files, because the parser, the help
/// page, the render's summary line and the refusal all have to say the same
/// word.
pub const ENTER: &str = "--exec";

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

/// The refusal [`ENTER`] gets when [`entering_allowed`] says no.
///
/// **`bad_invocation`, which is the class this CLI already uses for a flag it
/// knows and has not built** — `armada doctor --fix` and `armada manifest check
/// --detach` are both refused this way. No new class: an invented one would say
/// this refusal is a different kind of thing from those two, and it is not.
///
/// **Refused by name, never as an unknown flag.** A caller told "unknown flag"
/// concludes Armada is broken or that they typed it wrong, and goes looking for
/// the spelling that works. Told that it is off and how to turn it on, they
/// either run [`ENABLE`] or paste the printed command themselves — both of
/// which `next_action` offers, because `armada helm` has already printed the
/// command.
pub fn entering_is_off() -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: format!("helm {ENTER}"),
        message: format!(
            "`armada helm {ENTER}` is {ENTER_IS_OFF}: entering opens a Claude Code \
             session, and this machine has not said yes to that yet"
        ),
        next_action: Some(format!(
            "`{ENABLE}` turns it on here; `armada helm` alone still only assembles and \
             prints the command"
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

/// `armada helm enable` — let [`ENTER`] become a session on this machine.
///
/// **Writes one boolean, and nothing else.** It does not touch the guild, the
/// persona or any of the four documents `armada helm` wires — whether a
/// machine is *allowed* to open a session and whether it is currently *able*
/// to (a guild, a persona, a projection) are different questions, answered by
/// different commands. `armada helm --exec` still needs both.
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
/// **Nothing here starts a process.** The four documents are written, the
/// conversation's record is read or minted, and the argv is returned as a
/// string for a person to read or a script to parse.
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

    // **The refusal comes first, before anything is written.** A `helm` that
    // laid down four configuration files and then admitted it has no persona to
    // run would have changed the machine to report a failure.
    persona(place, &guild, &agent)?;

    let (session, minted) = conversation(now, place, &agent, options.new)?;
    let wiring = wire(place, &session, minted)?;

    let launch = Launch {
        agent: agent.clone(),
        session: session.clone(),
        mcp_config: paths(place).mcp_config.display().to_string(),
        plugin_dir: paths(place).plugin_dir.display().to_string(),
        settings: paths(place).settings.display().to_string(),
    };
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
            argv: helm::launch_argv(&launch),
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
    }
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
/// **It has no caller while [`ENTER`] is refused, and is kept deliberately.**
/// That gate is on entering and not on the work underneath it ([`ENTER_IS_OFF`]),
/// so the writer the exec path needs stays here, stays tested, and turning
/// entering back on is a deleted refusal and a call — rather than rediscovering
/// which end of the exec this has to be written at. It is exercised directly by
/// the suite for exactly that reason: a function kept for later that nothing
/// runs is a function that has rotted by the time later arrives.
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
fn wire(place: &Where, session: &Session, minted: Wiring) -> Result<Vec<Wired>, ArmadaError> {
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
        &helm::stop_hook(&inbox.display().to_string()),
    )?;
    write(
        &paths.settings,
        &helm::settings_json(&paths.stop_hook.display().to_string()),
    )?;
    wired.push(Wired {
        what: "backstop".to_string(),
        at: place.shown(&paths.stop_hook),
        state: hook,
        detail: "Stop hook: a turn does not end while the inbox is unread".to_string(),
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
