//! The Drone: **the argv Fleet builds, and the ledger the transcript keeps**.
//!
//! A Job's conversation is an ordinary Claude Code session (PHASES.md §9.1 F1),
//! so there is no session mechanism here — only a uuid the caller assigns before
//! anything starts, and a subprocess.
//!
//! ```text
//! claude --session-id <uuid> <brief> <posture> --print --output-format stream-json --verbose <prompt>
//! claude --resume     <uuid> <brief> <posture> --print --output-format stream-json --verbose <answer>
//! claude --resume     <uuid>                                                                  # boarding
//! ```
//!
//! **`<brief>` is one `--append-system-prompt` carrying two things**: [`BRIEF`],
//! without which a Drone reported at whatever length it liked, and then
//! [`crate::skill::BODY`], without which it edited `armada.yml` rather than
//! saying what it had learned (`docs/reserved/019` and `docs/reserved/008`).
//! [`brief`] is where they are joined and why they are not one constant.
//!
//! **Neither is on the boarding line.** Boarding hands the conversation to a
//! person, and a person is the audience for neither *report in two sentences*
//! nor *do not edit `armada.yml` silently*.
//!
//! **`<posture>` is [`Posture`], and until it existed no Job could finish.** A
//! headless Drone that reaches a state-mutating tool call with no permission
//! for it is asked for one, and it has no terminal to answer with — so it waits
//! until the wall clock takes it. Read [`Posture`] for what is granted, what is
//! refused, and why each is on the side it is on.
//!
//! **A Drone runs detached and Armada does not wait for it.** That is the whole
//! point of Fleet: five Jobs at once with one thing to watch. It is the same
//! shape a `command` service already has — `setsid`, a log file rather than a
//! pipe, and the process group recorded as owned so `armada manifest clean`
//! reclaims it — and reusing it is what keeps an orphaned Drone reapable by the
//! path that already reaps an orphaned service, rather than by a second
//! mechanism nobody maintains.
//!
//! **This module is where the bugs are, which is why it is pure.** A missing
//! `--session-id` mints a session Fleet cannot find again; `--resume` where
//! `--session-id` was meant starts a Job's second turn as its first. Both are
//! argv bugs, and a test that faked a higher layer would catch neither
//! (`ARCHITECTURE.md` §1.1). **No test in this repository spawns a real session
//! or spends a token** (PHASES.md §8.5) — the argv is asserted here, and the
//! integration suite starts a harmless stub that records the vector it was
//! actually given.
//!
//! **Budgets need no accounting layer** (PHASES.md §9.1 F2). Every turn ends
//! with a `result` event carrying `total_cost_usd`, `usage`, `num_turns` and
//! `duration_api_ms`, and a resumed session appends its own — so the transcript
//! *is* the ledger and [`read`] sums it. Nothing here estimates anything, and
//! nothing needs a Drone to report home.

use super::job::Spend;
use crate::error::{ArmadaError, ErrClass};
use serde::{Deserialize, Serialize};

/// The program every Drone is.
pub const CLAUDE: &str = "claude";

/// The argv for a Job's **first** turn.
///
/// `--session-id` is what makes the caller the one who assigns identity, which
/// is the whole of PHASES.md §9.1 F1: the uuid exists before the process, so the
/// transcript's location is known before there is a transcript.
pub fn spawn_argv(uuid: &str, prompt: &str, posture: &Posture, settings: Option<&str>) -> Vec<String> {
    let mut argv = vec![
        CLAUDE.to_string(),
        "--session-id".to_string(),
        uuid.to_string(),
    ];
    argv.extend(headless(posture, settings));
    argv.push(prompt.to_string());
    argv
}

/// The argv for **continuing** a Job — `armada fleet answer`.
///
/// **An answer is a continuation, not a new run**, so the budget is not reset
/// and the session is resumed rather than minted. Resetting the ceiling here
/// would make budgets unenforceable for any Job that asks a question.
pub fn resume_argv(
    uuid: &str,
    prompt: &str,
    posture: &Posture,
    settings: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![CLAUDE.to_string(), "--resume".to_string(), uuid.to_string()];
    argv.extend(headless(posture, settings));
    argv.push(prompt.to_string());
    argv
}

/// The words a Job is continued with when a person resumed it and gave none.
///
/// **A prompt rather than an empty argument**, because [`resume_argv`] is the
/// headless form: `--print` needs something to print about, and an empty final
/// argument starts a turn that has nothing to answer. It is deliberately short
/// and deliberately not an instruction — resuming is "carry on with what you
/// were doing", and a longer prompt here would be Armada putting words in a
/// conversation it is not part of.
///
/// **`armada fleet answer` does not use it**, and that is the distinction the
/// two verbs draw: an answer is a person's words, a resume is the absence of
/// any.
pub const CONTINUE: &str = "Continue from where you left off.";

/// The argv for **resuming** a Job nobody said anything to —
/// `armada fleet resume`.
pub fn continue_argv(uuid: &str, posture: &Posture, settings: Option<&str>) -> Vec<String> {
    resume_argv(uuid, CONTINUE, posture, settings)
}

/// The argv `armada fleet board` prints, and execs under `--exec`.
///
/// **Interactive, and deliberately so.** Boarding hands you the conversation to
/// drive yourself; it does not stream a running Drone's output at you, which is
/// the pty work withdrawn in PHASES.md §9.1 F1.
pub fn board_argv(uuid: &str) -> Vec<String> {
    vec![CLAUDE.to_string(), "--resume".to_string(), uuid.to_string()]
}

/// A bounded headless turn with a live event stream.
///
/// **`--verbose` is not optional and is not decoration.** Claude Code refuses
/// `--print --output-format stream-json` without it:
///
/// ```text
/// Error: When using --print, --output-format=stream-json requires --verbose
/// ```
///
/// Measured 2026-08-14 against a real spawn, after every unit test in this file
/// passed against an argv the binary rejects. **Asserting on argv proves the
/// vector is the one Armada meant to build, not that it is one the program
/// accepts** — which is the whole limitation of the testing rule, recorded in
/// `docs/traps.md` and answered by `armada doctor`'s Drone-argv check.
///
/// [`STREAM_JSON_NEEDS`] states the requirement as data so that the test below,
/// and `doctor`, both read the same source.
///
/// **The posture comes first and the prompt comes last, and the order is load
/// bearing.** `--allowedTools` and `--disallowedTools` are variadic — measured
/// 2026-08-15, `claude --allowedTools Edit --unknown-xyz` answers `error:
/// unknown option '--unknown-xyz'`, so the list stops at the next `--` word and
/// nowhere else. Emitting the posture *after* `--verbose` would put the prompt
/// immediately behind a variadic list, and the Job's task would be read as one
/// more tool name. Emitting it before `--print` puts a flag behind every list
/// there is. [`the_prompt_never_follows_a_variadic_list`] holds it there.
///
/// **[`BRIEF`] comes before the posture**, for the reason Helm's own launch puts
/// the reader's words immediately after `--agent`: it says who this session is,
/// and everything after it is wiring. It is also the one position where it
/// cannot be mistaken for a value of anything variadic.
///
/// **The brief travels on a resumed turn too.** A Job's obligations do not lapse
/// halfway through it, and the turn a Drone is most likely to end without a
/// verdict is the one it was answered into. `--append-system-prompt` appends to
/// the session the flag is passed to, so a resumed turn that carries it is a
/// turn that still knows the contract; one that did not would be a Drone whose
/// instructions expired at its first question.
/// **`--settings` carries the `Stop` hook, and it comes before `--print`** for
/// the same reason the posture does: everything variadic has to be behind a
/// flag that ends it, and the prompt is last.
fn headless(posture: &Posture, settings: Option<&str>) -> Vec<String> {
    let mut argv = vec![APPEND.to_string(), brief()];
    argv.extend(posture.argv());
    if let Some(path) = settings {
        argv.push(SETTINGS.to_string());
        argv.push(path.to_string());
    }
    argv.push("--print".to_string());
    argv.push("--output-format".to_string());
    argv.push(STREAM_JSON.to_string());
    argv.extend(STREAM_JSON_NEEDS.iter().map(|flag| (*flag).to_string()));
    argv
}

/// **The whole of what a Drone is told before its task**: [`BRIEF`], then
/// Armada's own skill.
///
/// # One appended prompt, never two
///
/// `claude --help` spells the flag `--append-system-prompt <prompt>` — singular,
/// not variadic. A second occurrence is a Commander option with no collector, so
/// the **last one wins and the first is dropped without a word**: a Drone would
/// get its reporting contract and none of Armada's instructions, or the reverse,
/// with nothing anywhere saying which. So the two are one string.
///
/// # Why they are two constants and not one
///
/// They answer different questions and were written for different reasons.
/// [`BRIEF`] is *how you report* — the contract a worker owes an orchestrator,
/// `docs/reserved/019`. [`crate::skill::BODY`] is *how you use Armada* —
/// `docs/reserved/008` — and it goes to **Helm too**, where a reporting contract
/// would make no sense. Merging them would mean Helm's launch carrying a
/// paragraph about `fleet.verdict`, which Helm does not have.
///
/// **The brief comes first**, because it says who this session is; the skill is
/// about the tools it holds, and a session has to be somebody before it can be
/// told what it may do with them.
///
/// **Public because it is the artefact, and the two constants are its parts.**
/// A test holding [`BRIEF`] alone goes green against a prompt that lost the
/// skill, which is exactly the silent half of `--append-system-prompt` being
/// singular — so `armada doctor`'s probe assertion and the belt's own
/// tool-naming test both read this.
pub fn brief() -> String {
    format!("{BRIEF}\n\n{}", crate::skill::BODY)
}

/// The flag that carries an appended system prompt into a session.
///
/// **`--append-system-prompt` rather than `--system-prompt`**: the session keeps
/// the persona, the repository's own `CLAUDE.md` and everything Claude Code
/// normally is, and *gains* what it owes Armada — rather than being reduced to
/// it. `armada_core::helm::APPEND` is a re-export of this, because one spelling
/// of a flag is what `armada doctor` can hold against `claude --help` once.
pub const APPEND: &str = "--append-system-prompt";

/// **What a Drone owes, in the words the session is handed** (PLAN.md §15.2).
///
/// # Whose voice a Drone speaks in — and it is not the reader's
///
/// `armada helm` assembles the reader's `voice.md`, `expectations.md` and
/// `how-i-work.md` into an appended system prompt, because **Helm talks to
/// them**: its whole product is a sentence a person reads, so how that sentence
/// is written is theirs to decide.
///
/// **A Drone talks to Helm**, and its output has three destinations of which a
/// person is none:
///
/// | What a Drone writes | Who reads it |
/// |---|---|
/// | `fleet.report` bodies | the Job record — `fleet ls`, `fleet show`, and Helm aggregating many Jobs |
/// | `fleet.verdict` | the gate, which reads an enum and an exit code and never the prose |
/// | the transcript's last `result` | the cheap model that summarises it ([`super::probe`]), before Helm sees a word |
///
/// So the reader's 150-word rule is the wrong instrument twice over. It is
/// **too weak** where it would matter — a report is one or two sentences, not
/// a hundred and fifty words — and it is **wrong** in the one channel that does
/// reach a person: `fleet.ask_human` is read out of context and possibly hours
/// later, so it needs *more* context, not less. And a fleet of Drones each
/// imitating the reader's register would hand Helm a chorus of impersonations to
/// aggregate, when what Helm needs is the same shape from every Job.
///
/// **What a Drone owes is a contract, not a register.** That is what this says.
///
/// # Why it restates the tools rather than asking for brevity
///
/// Asking an agent to be terse in the abstract produces prose that is shorter
/// and no more useful. The three tools already have a contract — a step boundary
/// vocabulary, four verdict words, and evidence an external command produced —
/// and every part of it is a thing a Drone gets wrong silently: a step left
/// without a verdict cannot be advanced, a `PASS` without evidence is refused
/// after the work is done, and a Job that never reports `entered` is one whose
/// step nobody can see. Restating that contract is worth more than any
/// instruction about length, and it is the half of PLAN.md §15.2 that a Drone
/// can act on.
///
/// # Why it is a constant here and not a file in the guild
///
/// Every word of this describes the contract of Armada's **own** MCP tools. A
/// guild copy would be a description of a contract the guild does not own, and
/// `docs/reserved/006` is the reason that matters: a guild receives a template
/// change only through `armada guild upgrade`, which is a `git merge` somebody
/// has to run. A guild that never runs it would keep a stale description of
/// `fleet.verdict` — and a *wrong* description is worse than none, because a
/// Drone that believes it may report a step `completed` will try, be refused,
/// and spend a turn finding out.
///
/// **What is genuinely the reader's already reaches a Drone**, by paths that
/// needed no new mechanism: the worktree's own `CLAUDE.md` and `AGENTS.md`,
/// which a Drone reads because it is an ordinary Claude Code session in that
/// repository, and `~/.armada/guild/permissions.yml`, which says what it may do
/// ([`Posture`]). Adding a guild override later costs one `Option<&str>` at this
/// call site; adding one now would be a mechanism with no reader.
///
/// # The tools are named as the model sees them
///
/// The server advertises `fleet.report`; Claude Code exposes it to the model as
/// `mcp__armada__fleet_report` (`docs/traps.md`, *Claude Code renames a dotted
/// tool*). A prompt written with the documented name matches nothing and the
/// model reports it has no such tool — so this uses the client's spelling, and
/// `crates/helm/src/mcp/drone.rs` holds it against the router that serves them.
pub const BRIEF: &str = "\
# How a Drone reports

You are an Armada Drone: one Job, one git worktree, one branch, and nobody watching. Your \
account of the work is read by an orchestrator following a whole fleet, not by a person reading \
your transcript. Three tools are how you are read. Nothing else you write is.

- `mcp__armada__fleet_report` — one or two sentences, at a step boundary. Pass \
`event: \"entered\"` when you begin a step and `event: \"attempted\"` when you stop working on \
one. That is the only thing that makes which step you are on, and how long you have been on it, \
visible to anybody. It is not narration: a note per thought puts your transcript into the \
orchestrator's window, which is the one place it must never be. You cannot report a step \
`completed`.
- `mcp__armada__fleet_verdict` — how a step ended: `PASS`, `FAILED`, `BLOCKED` or \
`NEEDS_HUMAN`. Emit one for every step you entered, including the ones that went wrong; a step \
you leave silently is a Job nobody can advance. A `PASS` carries evidence an external command \
produced — a check id and its exit code — and is refused without it. Your own assertion that \
the tests pass is not evidence.
- `mcp__armada__fleet_ask_human` — only for a judgement that is genuinely the person's. Write \
the question in full: it is read out of context, possibly hours later, by somebody who has not \
seen your work. Brevity is the wrong instinct here and nowhere else.

You cannot spawn Jobs. If the task needs decomposing, say so with `mcp__armada__fleet_ask_human`.

Your closing message is read beside a dozen others. A few lines: what you did, what you proved \
it with, and what is left. Not a transcript — the record is already in your reports and your \
verdicts.";

/// **What a Drone may do unattended.**
///
/// A Drone runs headless under `--print`, in its own git worktree on its own
/// `armada/<job>` branch, with nobody watching. Claude Code's default posture
/// asks a person before the first state-mutating tool call — and a Drone has no
/// terminal to answer with, so it waits until the wall clock takes it. That is
/// what `STALLED` was: not a Job that went wrong, a Job that was never given
/// permission to start.
///
/// **Three decisions, and the first one is the fix.**
///
/// 1. **[`MODE`] is what happens when the posture does not cover something.**
///    `dontAsk` denies and carries on; every other mode that could grant enough
///    to work — `acceptEdits`, `manual` — still *prompts* for the tool calls it
///    does not cover, which is the stall arriving one flag later. A Drone that
///    is refused a command reports being refused; a Drone that is asked reports
///    nothing at all, and that difference is the whole bug.
/// 2. **[`ALLOW`] grants the tool classes the work is made of**, including
///    `Bash` whole. The set of commands a repository's checks are spelled with
///    is unbounded — `cargo test`, `npm run check`, `make`, `./scripts/ci` —
///    and every build system left off an enumeration is a Job that cannot
///    verify its own work.
/// 3. **[`DENY`] subtracts what escapes the worktree**, and deny beats allow.
///    The worktree is what makes granting `Bash` reasonable: a Drone cannot
///    reach the user's checkout, so the only capabilities worth naming are the
///    ones whose effect is not confined to a directory.
///
/// **`--dangerously-skip-permissions` is never any of this.** It would hand an
/// unattended model the caller's whole toolbelt, which is exactly what
/// `--strict-mcp-config` and `--disable-slash-commands` were added to prevent
/// one argv over ([`super::classify::argv`]).
///
/// **What this posture does not reach.** A deny rule is matched against each
/// subcommand of a compound command, but `bash -c "git push"` is one command
/// whose text is an argument, and the rule matches `bash` rather than what is
/// inside the quotes. The list narrows the blast radius; it is not a sandbox,
/// and the thing that actually bounds a Drone is the worktree it is confined
/// to. Stated rather than papered over, for the same reason the `--verbose`
/// residual is stated in `docs/traps.md`.
///
/// **It is a preference, so it is the guild's.** `armada_guild::permissions`
/// reads `~/.armada/guild/permissions.yml` and returns one of these; a guild
/// without that file gets [`Posture::default`], because a user who has never
/// thought about it must still get working Jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Posture {
    /// What Claude Code does with a tool call the two lists do not settle.
    #[serde(default = "default_mode")]
    pub mode: String,
    /// The tools a Drone may use. **Replaces** the default rather than adding
    /// to it — a posture you can only add to is one whose real contents are
    /// nowhere written down.
    #[serde(default = "default_allow")]
    pub allow: Vec<String>,
    /// The rules that are refused however broadly [`Posture::allow`] grants.
    /// Deny beats allow, so these are subtracted from it.
    #[serde(default = "default_deny")]
    pub deny: Vec<String>,
}

fn default_mode() -> String {
    MODE.to_string()
}

fn default_allow() -> Vec<String> {
    ALLOW.iter().map(|rule| (*rule).to_string()).collect()
}

fn default_deny() -> Vec<String> {
    DENY.iter().map(|rule| (*rule).to_string()).collect()
}

impl Default for Posture {
    fn default() -> Self {
        Posture {
            mode: default_mode(),
            allow: default_allow(),
            deny: default_deny(),
        }
    }
}

impl Posture {
    /// The flags this posture is, in the order [`headless`] needs them.
    ///
    /// **An empty list emits no flag at all.** `--allowedTools` with nothing
    /// after it would consume whatever came next as its first value, and what
    /// comes next is another flag or the Job's prompt.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = vec!["--permission-mode".to_string(), self.mode.clone()];
        for (flag, rules) in [(ALLOWED, &self.allow), (DISALLOWED, &self.deny)] {
            if rules.is_empty() {
                continue;
            }
            argv.push(flag.to_string());
            argv.extend(rules.iter().cloned());
        }
        argv
    }

    /// Why this posture cannot be used, if it cannot.
    ///
    /// **Three things a hand-edited `permissions.yml` gets wrong**, and each is
    /// silent rather than loud without this.
    ///
    /// A mode Claude Code does not have is a usage error a detached Drone dies
    /// of unseen. A rule starting with `-` is read as a flag, which ends the
    /// tool list early and hands the rest to argument parsing. And a rule whose
    /// space or comma is *outside* parentheses is read as two rules —
    /// `--allowedTools` is documented as taking a *"comma or space-separated
    /// list"*, and its own example, `"Bash(git *) Edit"`, is one argument
    /// holding two rules, one of which contains a space of its own. So the
    /// separator is paren-aware and `Bash(git push:*)` is a single rule;
    /// `Edit Write` written as one list entry is not.
    pub fn wrong(&self) -> Option<String> {
        if !MODES.contains(&self.mode.as_str()) {
            return Some(format!(
                "`{}` is not a permission mode — Claude Code offers {}",
                self.mode,
                MODES.join(", ")
            ));
        }
        for rule in self.allow.iter().chain(&self.deny) {
            if rule.trim().is_empty() {
                return Some("a rule is blank".to_string());
            }
            if rule.starts_with('-') {
                return Some(format!("`{rule}` starts with `-`, so it reads as a flag"));
            }
            let mut depth = 0usize;
            for character in rule.chars() {
                match character {
                    '(' => depth += 1,
                    ')' => depth = depth.saturating_sub(1),
                    ' ' | ',' if depth == 0 => {
                        return Some(format!(
                            "`{rule}` separates outside its parentheses, so it is two \
                             rules rather than one — write one rule per entry"
                        ))
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// What Claude Code does with a tool call the posture does not settle.
///
/// **`dontAsk`, because the alternative is the bug.** Every other mode that
/// grants enough to work still prompts for what it does not cover, and a
/// prompt is a stall when there is no terminal. Read off `claude --help` on
/// 2026-08-15; [`MODES`] is the choice list it printed.
pub const MODE: &str = "dontAsk";

/// Every value `--permission-mode` accepts, measured 2026-08-15:
///
/// ```text
/// claude --permission-mode bogus
/// -> error: option '--permission-mode <mode>' argument 'bogus' is invalid.
///    Allowed choices are acceptEdits, auto, bypassPermissions, manual, dontAsk, plan.
/// ```
///
/// **Free, and checked at argument-parse time** — which is why [`Posture::wrong`]
/// can refuse a bad mode before a Drone is started rather than after one has
/// died of it.
pub const MODES: [&str; 6] = [
    "acceptEdits",
    "auto",
    "bypassPermissions",
    "manual",
    "dontAsk",
    "plan",
];

/// The tool classes the work a Job is given is made of.
///
/// | Rule | Why it is granted |
/// |---|---|
/// | `Read`, `Glob`, `Grep` | reading the repository is the whole first half of any Job |
/// | `Edit`, `Write`, `NotebookEdit` | the change itself, and Claude Code confines these to the session's directory — which is the worktree |
/// | `TodoWrite` | the Drone's own scratchpad; it touches nothing outside the session |
/// | `Bash` | **the tool whole**, because the repository's checks are spelled in a language Armada does not know |
/// | `Skill` | a workflow step names the skill that runs it, so a Drone that cannot invoke one cannot perform its step |
/// | `mcp__armada__fleet_*` | the four tools [`BRIEF`] instructs the Drone to report through |
///
/// **`Bash` whole is the deliberate one.** An allowlist of commands would be an
/// enumeration of every build system there is, and each one missing is a Job
/// that edits code it cannot test. [`DENY`] is what makes that affordable: the
/// escapes are a finite list and the checks are not.
///
/// **The last five entries are a fix, and this is the evidence for it.** They
/// were absent, and the open question recorded here was whether an uncovered
/// tool is *denied* under [`MODE`]'s `dontAsk` or merely not pre-approved — not
/// provable in a test, because the only honest one spawns a real Drone and
/// spends a token (`PHASES.md` §8.5).
///
/// **Real use answered it.** A Drone was handed a brief naming the
/// `reproduce-failure` skill and reported that it could not access the skill it
/// had been told it had — with `Skill` absent from this list. An allowlist that
/// omits a tool denies it. The same mechanism governs the MCP tools, which is
/// why a Drone had already been observed answering its operator in *prose*
/// rather than calling `fleet_ask_human` (`docs/reserved/020` §2): it was not
/// ignoring the brief, it was refused the tools the brief names, silently. That
/// is 011's original bug one layer up, which
/// `docs/reserved/019-the-brief-a-drone-reports-through.md` recorded as a risk
/// and which is now closed.
///
/// **The four MCP tools are named individually rather than by a
/// `mcp__armada` wildcard.** `crates/helm/src/mcp/drone.rs` serves exactly these
/// four, so the enumeration is complete today — and it stays correct if that
/// router ever gains a fifth, because [`BRIEF`] promises *"you cannot spawn
/// Jobs"* and a wildcard would quietly grant whatever is added next.
///
/// The **client's** spelling, not the server's: Claude Code exposes
/// `fleet.report` as `mcp__armada__fleet_report`, and the dotted name matches
/// nothing the model can call (`docs/traps.md`).
pub const ALLOW: [&str; 13] = [
    "Read",
    "Glob",
    "Grep",
    "Edit",
    "Write",
    "NotebookEdit",
    "TodoWrite",
    "Bash",
    "Skill",
    "mcp__armada__fleet_report",
    "mcp__armada__fleet_verdict",
    "mcp__armada__fleet_ask_human",
    "mcp__armada__fleet_propose",
];

/// What a Drone may not do however broadly [`ALLOW`] grants — **the things
/// whose effect is not confined to the worktree**.
///
/// | Rule | Why it is refused |
/// |---|---|
/// | `Bash(git push:*)` | the one git operation nobody can undo for everybody else |
/// | `Bash(git remote:*)` | repointing the remote makes every later push escape somewhere new |
/// | `Bash(git config:*)` | `--global` writes the user's own git identity, which is in no worktree |
/// | `Bash(git worktree:*)` | the other Jobs' worktrees are this one's siblings; removing one kills a Drone still working |
/// | `Bash(git checkout:*)`, `Bash(git switch:*)` | leaving `armada/<job>` is how a Drone's commits land on somebody else's branch |
/// | `Bash(git branch:*)` | `-D` deletes another Job's branch, and this Job's branch already exists |
/// | `Bash(sudo:*)` | root is the definition of escaping a directory |
/// | `Bash(gh:*)` | the GitHub CLI opens pull requests, pushes and deletes repositories — none of it local |
/// | `Bash(armada:*)` | writes the user's **real** `~/.armada/` — other Jobs, other worktrees, the guild |
/// | `Bash(claude:*)` | a Drone spawning its own sessions is spend no budget counts, in the user's real `~/.claude/` |
/// | `Bash(npm publish:*)` and its four siblings | publishing is irreversible and public |
///
/// **What is deliberately *not* here.** `rm`, `git reset --hard` and
/// `git commit --amend` all destroy work, and all of it is the Drone's own, in
/// the Drone's own worktree, on the Drone's own branch. A posture that forbade
/// them would be protecting the Job from itself — which is what the worktree is
/// already for, and which would cost the Drone the ability to clean up after a
/// bad attempt.
///
/// **Giving a Drone Armada's own tools deliberately** is
/// `docs/reserved/008-armada-injects-its-own-skills.md`, and it arrives as MCP
/// rather than as a shell command. Denying the CLI here is what keeps that
/// decision open instead of making it by accident.
pub const DENY: [&str; 16] = [
    "Bash(git push:*)",
    "Bash(git remote:*)",
    "Bash(git config:*)",
    "Bash(git worktree:*)",
    "Bash(git checkout:*)",
    "Bash(git switch:*)",
    "Bash(git branch:*)",
    "Bash(sudo:*)",
    "Bash(gh:*)",
    "Bash(armada:*)",
    "Bash(claude:*)",
    "Bash(npm publish:*)",
    "Bash(pnpm publish:*)",
    "Bash(yarn publish:*)",
    "Bash(cargo publish:*)",
    "Bash(docker push:*)",
];

/// The flag [`ALLOW`] is passed as. Variadic — see [`headless`].
pub const ALLOWED: &str = "--allowedTools";

/// The flag [`DENY`] is passed as. Deny beats allow, and Claude Code decides
/// that, not Armada.
pub const DISALLOWED: &str = "--disallowedTools";

/// The flag that layers a Drone's own settings — and therefore its `Stop`
/// hook — over the reader's, **for the length of one exchange**.
///
/// Registering the hook in `~/.claude/settings.json` instead would fire it in
/// every session on the machine, including the reader's own and Helm's.
pub const SETTINGS: &str = "--settings";

/// How long the relay waits for its Drone to actually exit, in seconds.
///
/// **A cap rather than a deadline.** The hook cannot tick while the Drone it is
/// relaying for is still alive — a Job with a live process group observes as
/// `RUNNING` and is deliberately not gated ([`super::advance::attention`]) —
/// so it waits, and an exchange that never ends would otherwise leave one
/// `sh` per Job waiting forever. An hour is well past any real exchange, and a
/// Drone still going at that point is a Job its own wall-clock ceiling will
/// stop.
pub const RELAY_CAP_S: u32 = 3_600;

/// The `Stop` hook a Drone runs — **the event `020` §1 says nothing was
/// watching**.
///
/// # What it is for
///
/// A Drone runs one exchange under `--print` and exits. That is correct, and it
/// is why `spawn` can return and five Jobs can run at once. What was missing is
/// anything that *observed the exchange ending*: `armada fleet tick` existed
/// and nobody called it, so a Job sat `RUNNING` beside a dead Drone until
/// somebody typed the verb. The reader lost eight hours to exactly that.
///
/// **The hook is the relay because a hook cannot be forgotten.** PLAN.md §15.3:
/// *"an agent can forget to report progress, but it cannot forget to stop"*.
/// This is that argument applied to the loop rather than to the inbox.
///
/// # Why it waits, and why the wait is not a daemon
///
/// **A `Stop` hook runs while its session is still alive.** Ticking from inside
/// it would find a live process group, observe `RUNNING`, and decline to gate —
/// correctly, because gating a live exchange starts a check against a worktree
/// still being written to. Worse, advancing would `claude --resume` a session
/// that has not closed.
///
/// So the hook backgrounds one `sh` that waits for the Drone's own process to
/// go, and only then ticks. **It watches the process-group leader's pid**,
/// which under `setsid` *is* the Drone — read from `ps` rather than passed in,
/// because the hook is written before the process exists and therefore before
/// its pid does. It is one short-lived shell per exchange, it holds no lease,
/// it recovers nothing, and killing the Job's group kills it too — which is the
/// difference between this and the daemon `020` §1 refuses.
///
/// # It ticks the **whole fleet**, and that is the backstop
///
/// `020` §2's failure modes — a SIGKILLed Drone, a hook that could not run, a
/// crash between the two — all break *this Job's* relay, and no amount of care
/// inside one hook fixes them. What does is that every relay sweeps every Job:
/// a Job whose own hook was lost is picked up by the next Drone **anywhere on
/// the machine** to finish an exchange. `armada fleet tick` is idempotent and
/// cheap over an idle fleet — a directory listing, a transcript tail and a
/// `ps` — so the sweep costs the fleet nothing and is the second mechanism
/// PLAN.md §15.3 asks for.
///
/// **A read verb ticking would be the wrong repair.** `armada fleet ls`
/// advancing a Job behind the reader's back breaks PLAN.md §15.1, and `020` §1
/// rejects it by name; reporting a Job as `STALLED` is honest, doing the work
/// unasked is not.
///
/// # Shell, and what it may depend on
///
/// **No `jq` and no `python`**, for the reason Helm's hook gives: a backstop
/// that depends on a tool the machine may not have is a backstop that silently
/// stops backing anything up. This is `ps`, `kill -0` and `sleep`.
///
/// `exe` is the absolute path of the running `armada` — **not the bare word.**
/// A hook that resolved `armada` against the Drone's `PATH` is precisely `020`
/// §2's second failure mode, and it is the only one of the three that is fixed
/// rather than backstopped.
pub fn stop_hook(exe: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by Armada for one Job's Drone. Regenerated on every exchange;\n\
         # edit the verb, not this. See `020` §1: the exchange ending is the event.\n\
         armada={exe}\n\
         leader=$(ps -o pgid= -p $$ 2>/dev/null | tr -d ' ')\n\
         [ -n \"$leader\" ] || exit 0\n\
         {{\n\
         \x20 waited=0\n\
         \x20 while [ \"$waited\" -lt {cap} ] && kill -0 \"$leader\" 2>/dev/null; do\n\
         \x20   waited=$((waited + 1))\n\
         \x20   sleep 1\n\
         \x20 done\n\
         \x20 kill -0 \"$leader\" 2>/dev/null || \"$armada\" fleet tick\n\
         }} >/dev/null 2>&1 &\n\
         exit 0\n",
        exe = shell_quote(exe),
        cap = RELAY_CAP_S,
    )
}

/// The `--settings` document that registers [`stop_hook`].
///
/// **Fleet's, and Helm's is a re-export of it** — `crate::helm::settings_json`
/// calls this rather than the other way round, because `ARCHITECTURE.md` §1.9
/// says nothing points upward: Fleet may not reference Helm, and Helm may
/// reference Fleet. One document, one shape, in the lower module.
pub fn settings_json(hook: &str) -> String {
    format!(
        "{{\n  \"hooks\": {{\n    \"Stop\": [\n      {{\n        \"hooks\": [\n          \
         {{\n            \"type\": \"command\",\n            \"command\": {}\n          }}\n        \
         ]\n      }}\n    ]\n  }}\n}}\n",
        json_quote(hook)
    )
}

/// A string as a JSON scalar. Hand-written for the reason
/// `crate::helm::quote` gives: these documents are written in reading order and
/// a `serde_json::Value` map alphabetises them (`docs/traps.md`).
fn json_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// A path as one shell word. A `$HOME` containing a space is ordinary on macOS.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The output format a Drone's turn is read from.
pub const STREAM_JSON: &str = "stream-json";

/// The flags Claude Code requires alongside `--output-format stream-json`.
///
/// **Stated as data rather than inlined**, so the invariant test and `armada
/// doctor` cannot drift from the argv builder. A second entry here is a one-line
/// change that both of them pick up.
pub const STREAM_JSON_NEEDS: [&str; 1] = ["--verbose"];

/// The argv `armada doctor` validates the Drone's flags with, **without
/// spending a token**.
///
/// It is the real [`spawn_argv`] with the prompt replaced by `--input-format
/// stream-json`. Claude Code then waits for messages on stdin; the caller gives
/// it none, so it starts the session, gets EOF and exits **having made no API
/// call** — measured 2026-08-14: no `result` event is emitted at all, and a turn
/// that never happened has no ledger and no cost.
///
/// **Why this shape and not something simpler.** An invalid session id looks
/// like an obvious free probe and is useless: the uuid is validated *before* the
/// flag combination, so `Invalid session ID` masks the very error being looked
/// for. `--resume` against an unknown session is free too, and does not exercise
/// this rule at all — measured, the requirement applies to the `--session-id`
/// path and not to `--resume`. Only a valid uuid on the fresh-session path
/// reaches the check, and only closed stdin keeps it from costing anything.
///
/// The uuid is a fixed sentinel, so the probe reuses one transcript rather than
/// leaving a new one behind on every run.
pub fn probe_argv() -> Vec<String> {
    // **No `--settings`, because the probe must start nothing.** `doctor`'s
    // free probe reaches EOF and exits; a `Stop` hook on it would relay an
    // exchange that never happened. The flag is held against `claude --help`
    // by [`FLAGS`], which is where a renamed flag is caught.
    let mut argv = spawn_argv(PROBE_SESSION, "", &Posture::default(), None);
    argv.pop();
    argv.push("--input-format".to_string());
    argv.push(STREAM_JSON.to_string());
    argv
}

/// The session id [`probe_argv`] uses. Valid, so validation gets past it; fixed,
/// so the probe leaves one transcript rather than one per run.
pub const PROBE_SESSION: &str = "00000000-0000-4000-8000-0000000a2ada";

/// Every flag a Drone's argv uses, for `armada doctor` to hold against
/// `claude --help`.
///
/// **The point is the next version of Claude Code, not this one.** A flag
/// renamed or removed under Armada produces exactly the failure this list was
/// added after: a Job that spawns, records a worktree and a port block, and
/// whose Drone dies on a usage error nobody sees until `fleet ls` says
/// `STALLED`.
pub const FLAGS: [&str; 14] = [
    // The relay's, which carries the `Stop` hook that ticks the Job (`020`
    // §1). Its disappearance is the quietest failure of all — every Job still
    // spawns, and none of them ever advances a step again.
    SETTINGS,
    "--session-id",
    "--resume",
    "--print",
    "--output-format",
    "--verbose",
    "--model",
    "--input-format",
    // The classifier's two, which withhold capability rather than granting it —
    // and are therefore the two whose disappearance would silently hand an
    // unattended model the caller's whole toolbelt.
    "--strict-mcp-config",
    "--disable-slash-commands",
    // [`Posture`]'s three, which are the only ones that *grant* anything. Their
    // disappearance is the opposite failure and the more visible one: a Drone
    // back to asking a terminal that is not there, which is `STALLED`.
    "--permission-mode",
    ALLOWED,
    DISALLOWED,
    // [`brief`]'s, which grants nothing and withholds nothing — it says what the
    // Drone owes and how it uses Armada. Its disappearance is the quietest
    // failure of the three classes: every Job still runs, and every one of them
    // reports at whatever length it likes into an orchestrator's window and
    // edits `armada.yml` rather than proposing (`docs/reserved/008`).
    APPEND,
];

/// What one turn reported.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Turn {
    /// The ledger, summed from the `result` event.
    pub spend: Spend,
    /// Why the turn ended, as Claude Code spelled it.
    pub stop_reason: Option<String>,
    /// Whether the turn itself failed.
    pub is_error: bool,
    /// The turn's own text, when it produced one.
    pub result: Option<String>,
}

/// The rate-limit window a turn passed through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RateLimit {
    /// `allowed`, `allowed_warning`, `rejected` — whatever the event said.
    pub status: String,
    /// Which window: `five_hour`, and whatever else arrives.
    pub kind: String,
    /// When it resets, as seconds since the epoch.
    pub resets_at: Option<u64>,
}

/// Everything a Job's transcript says about what it has spent and how far it
/// got.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Reading {
    /// Every finished turn, in the order they finished.
    pub turns: Vec<Turn>,
    /// Their sum, which **is** the Job's spend. Nothing else adds it up.
    pub spend: Spend,
    /// The last rate-limit window seen.
    ///
    /// **Strictly better than a fixed concurrency cap**, which was only ever a
    /// proxy for the same thing: the orchestrator can decline to spawn when a
    /// window reset is close (PHASES.md §9.1 F2).
    pub rate_limit: Option<RateLimit>,
}

impl Reading {
    /// The turn that finished last, if any has.
    pub fn last(&self) -> Option<&Turn> {
        self.turns.last()
    }
}

/// Read a Job's `stream-json` transcript.
///
/// **The transcript is the ledger, and this is the whole of Fleet's
/// accounting.** A Drone runs detached and reports to nobody; a resumed session
/// appends its own `result` event to the same stream, so summing them is how a
/// Job's spend is known — without a Drone-side mechanism, without a hook, and
/// without a second number Armada maintains in parallel and can get wrong.
///
/// **One JSON document per line, and only two kinds matter.** Everything
/// between the `system` init and each `result` is the conversation, which is the
/// Drone's business and not Fleet's — PLAN.md §15.2's rule that the orchestrator
/// reads summaries rather than raw transcripts starts here, with Fleet declining
/// to parse them either.
///
/// **A stream with no `result` is a turn that has not finished**, which is the
/// ordinary state of a Job whose Drone is still working. That is emptiness
/// rather than failure, and the caller tells the two apart by asking whether the
/// process group is still alive — the one question a busy Drone cannot answer
/// about itself, which is why it belongs to the observer (PLAN.md §14.3).
pub fn read(stream: &str) -> Reading {
    let mut reading = Reading::default();

    for line in stream.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // **A line that does not parse is skipped rather than fatal.** Two
        // shapes reach here that are not events: the partial last line a killed
        // process leaves behind, and anything the Drone wrote to stderr — the
        // log holds both streams, because a child that outlives Armada cannot
        // be given a pipe ([`crate::ctx::StdioMode::Log`]).
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("result") => {
                let turn = read_result(&event);
                reading.spend.add(&turn.spend);
                reading.turns.push(turn);
            }
            Some("rate_limit_event") => reading.rate_limit = read_rate_limit(&event),
            _ => {}
        }
    }
    reading
}

fn read_result(event: &serde_json::Value) -> Turn {
    let usage = event.get("usage");
    let count = |key: &str| {
        usage
            .and_then(|u| u.get(key))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    };
    Turn {
        spend: Spend {
            cost_usd: event
                .get("total_cost_usd")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            // **Every kind of token, because every kind is billed.** Counting
            // input and output alone understates a cached turn by an order of
            // magnitude — the spike's own numbers were 4 input against 44357
            // cache reads — and a ceiling computed from the smaller number is a
            // ceiling that never stops anything.
            tokens: count("input_tokens")
                + count("output_tokens")
                + count("cache_creation_input_tokens")
                + count("cache_read_input_tokens"),
            turns: event
                .get("num_turns")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32,
            api_ms: event
                .get("duration_api_ms")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        },
        stop_reason: event
            .get("stop_reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        is_error: event
            .get("is_error")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        result: event
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    }
}

fn read_rate_limit(event: &serde_json::Value) -> Option<RateLimit> {
    let info = event.get("rate_limit_info").or(Some(event))?;
    Some(RateLimit {
        status: info.get("status")?.as_str()?.to_string(),
        kind: info
            .get("rateLimitType")
            .or_else(|| info.get("rate_limit_type"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        resets_at: info
            .get("resetsAt")
            .or_else(|| info.get("resets_at"))
            .and_then(serde_json::Value::as_u64),
    })
}

/// The failure a Drone that could not be started reports.
///
/// **`environment`, not `tool_failed`.** `claude` missing from `PATH` is the
/// machine being incomplete rather than the repository being wrong, and the
/// correct response is the identical command after a person fixes something
/// Armada cannot (`ARCHITECTURE.md` §1.7).
pub fn not_on_path() -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: CLAUDE.to_string(),
        message: "`claude` is not on PATH, so no Drone can be started".to_string(),
        next_action: Some("install Claude Code, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "15bfa340-33b1-4f81-bd7f-688f0f01dbb0";

    /// A posture small enough to write out in full, so the argv tests below
    /// assert on the *whole* vector rather than on a prefix of it.
    fn narrow() -> Posture {
        Posture {
            mode: "dontAsk".to_string(),
            allow: vec!["Edit".to_string(), "Bash".to_string()],
            deny: vec!["Bash(git push:*)".to_string()],
        }
    }

    /// **The argv PHASES.md §8.5 names, exactly.** A test asserting on anything
    /// less specific than the whole vector would pass with `--session-id`
    /// missing, which is the bug that loses a Job's transcript.
    #[test]
    fn a_first_turn_assigns_the_session_id_before_anything_starts() {
        assert_eq!(
            spawn_argv(UUID, "reproduce the flake", &narrow(), None),
            [
                "claude",
                "--session-id",
                UUID,
                APPEND,
                // **One appended prompt carrying both halves**: the reporting
                // contract, then Armada's own skill (`docs/reserved/008`). The
                // flag is singular, so a second occurrence would keep only the
                // last.
                &brief(),
                "--permission-mode",
                "dontAsk",
                "--allowedTools",
                "Edit",
                "Bash",
                "--disallowedTools",
                "Bash(git push:*)",
                "--print",
                "--output-format",
                "stream-json",
                "--verbose",
                "reproduce the flake",
            ]
        );
    }

    /// **A Drone is granted something, and this is the assertion that says so.**
    /// Every flag in the argv before this change either withheld capability or
    /// described the output; nothing granted any, which is why a Drone stalled
    /// on the first `git commit` it reached.
    #[test]
    fn a_drone_is_granted_permission_to_do_the_work_it_was_given() {
        let argv = spawn_argv(UUID, "fix the flake", &Posture::default(), None);
        assert!(
            argv.iter().any(|word| word == "--permission-mode"),
            "{argv:?} grants nothing, so the first state-mutating call stalls"
        );
        // Editing and committing are the job; both are granted.
        for granted in ["Edit", "Write", "Bash"] {
            assert!(
                argv.iter().any(|word| word == granted),
                "{granted} ungranted"
            );
        }
        // Pushing is not; and neither is the user's own `~/.armada/`.
        for refused in ["Bash(git push:*)", "Bash(armada:*)"] {
            assert!(argv.iter().any(|word| word == refused), "{refused} allowed");
        }
    }

    /// **The mode is the fix, and it is the one that cannot be swapped
    /// casually.** `acceptEdits` grants the edits and still *prompts* for the
    /// `cargo test` after them; `manual` prompts for everything. A prompt with
    /// no terminal behind it is the stall this whole module exists to end, so
    /// the default mode must be one that answers itself.
    #[test]
    fn the_default_mode_never_asks_a_terminal_that_is_not_there() {
        assert_eq!(MODE, "dontAsk");
        assert!(MODES.contains(&MODE), "{MODE} is not a mode the CLI offers");
        assert_eq!(Posture::default().mode, MODE);
        assert!(
            !MODE.starts_with("bypass") && MODE != "auto",
            "the posture must be stated, not delegated to a classifier"
        );
    }

    /// **Never `--dangerously-skip-permissions`, by any spelling.** An
    /// unattended model with the caller's whole toolbelt is what
    /// `--strict-mcp-config` and `--disable-slash-commands` were added to
    /// prevent, and undoing that one flag over is the easiest mistake here.
    #[test]
    fn nothing_a_drone_runs_bypasses_permissions_altogether() {
        let posture = Posture::default();
        let argvs = [
            spawn_argv(UUID, "go", &posture, None),
            resume_argv(UUID, "go", &posture, None),
            continue_argv(UUID, &posture, None),
            probe_argv(),
            board_argv(UUID),
        ];
        for argv in argvs {
            for word in &argv {
                assert!(
                    !word.contains("dangerously") && word != "bypassPermissions",
                    "{argv:?} skips permission checks"
                );
            }
        }
        assert!(!FLAGS.iter().any(|flag| flag.contains("dangerously")));
    }

    /// **The prompt is never behind a variadic list**, which would make the
    /// Job's task read as one more tool name.
    ///
    /// Measured 2026-08-15 — `claude --allowedTools Edit --unknown-xyz` answers
    /// `error: unknown option '--unknown-xyz'`, so the list ends at the next
    /// `--` word and at nothing else. This asserts there is always such a word
    /// between the last rule and the prompt, for every posture shape including
    /// the two where a list is empty.
    #[test]
    fn the_prompt_never_follows_a_variadic_list() {
        let postures = [
            Posture::default(),
            narrow(),
            Posture {
                mode: MODE.to_string(),
                allow: vec![],
                deny: vec!["Bash(git push:*)".to_string()],
            },
            Posture {
                mode: MODE.to_string(),
                allow: vec!["Bash".to_string()],
                deny: vec![],
            },
            Posture {
                mode: MODE.to_string(),
                allow: vec![],
                deny: vec![],
            },
        ];
        for posture in postures {
            for argv in [
                spawn_argv(UUID, "a prompt", &posture, None),
                resume_argv(UUID, "a prompt", &posture, None),
            ] {
                let variadic = argv
                    .iter()
                    .rposition(|word| word == ALLOWED || word == DISALLOWED);
                if let Some(at) = variadic {
                    let after = argv[at + 1..]
                        .iter()
                        .position(|word| word.starts_with("--"))
                        .expect("no flag closes the list, so the prompt joins it");
                    assert!(
                        at + 1 + after < argv.len() - 1,
                        "{argv:?} ends its tool list on the prompt"
                    );
                }
                assert_eq!(argv.last().unwrap(), "a prompt");
            }
        }
    }

    /// **The brief reaches the session as bytes, and the task still reaches it
    /// as the turn.** This is the assertion the whole change exists for, and it
    /// is written against the failure that has already shipped twice.
    ///
    /// The `config scan` hand-over asserted its flag, asserted its prose, and
    /// then asserted `argv.len() == 3` — so it went green against a session that
    /// opened with instructions and nothing to act on
    /// (`armada_guild::layout::skill_argv`). A Drone has the mirror-image
    /// hazard: `--append-system-prompt` takes a value, so a brief that were
    /// empty would consume `--permission-mode` and the Job's task would follow a
    /// posture that no longer exists. Both halves are asserted here, for every
    /// argv that carries a brief.
    #[test]
    fn the_brief_and_the_task_both_reach_the_argv() {
        for (argv, turn) in [
            (
                spawn_argv(UUID, "fix the flake", &narrow(), None),
                "fix the flake",
            ),
            (resume_argv(UUID, "yes, 90s", &narrow()), "yes, 90s"),
            (continue_argv(UUID, &narrow(), None), CONTINUE),
        ] {
            let at = argv
                .iter()
                .position(|word| word == APPEND)
                .unwrap_or_else(|| panic!("{argv:?} carries no brief"));
            // The value, and it is the prose itself rather than a path to it or
            // an instruction to go and read one.
            //
            // **`starts_with` rather than `==` since `docs/reserved/008`**: the
            // one appended prompt now carries [`BRIEF`] and then
            // [`crate::skill::BODY`], because the flag is singular and a second
            // occurrence would keep only the last. The equality this used to
            // make is now two assertions, one per half — a `contains` alone
            // would go green against an order that put the skill first, which
            // would tell a session what it may do before telling it who it is.
            let brief = &argv[at + 1];
            assert!(
                brief.starts_with(BRIEF),
                "{argv:?} appends something else, or appends it out of order"
            );
            assert!(
                brief.ends_with(crate::skill::BODY),
                "the reporting contract reached the session and Armada's own \
                 skill did not: {brief:?}"
            );
            assert!(
                !brief.trim().is_empty() && !brief.starts_with('-'),
                "an empty or flag-shaped brief eats the flag after it: {argv:?}"
            );
            // And the flag after it survived, which is the collision.
            assert_eq!(argv[at + 2], "--permission-mode", "{argv:?}");
            // The turn is still the last word, unflagged, and is not the brief.
            assert_eq!(argv.last().unwrap(), turn, "{argv:?} lost its turn");
            assert_ne!(argv.last().unwrap(), BRIEF);
            assert_ne!(argv.last().unwrap(), &brief.clone());
            assert!(!turn.starts_with('-'), "the turn reads as a flag");
        }
    }

    /// **The brief restates the tools' contract, which is what a Drone gets
    /// wrong.** A prompt asking for brevity in the abstract would pass a laxer
    /// test than this one and would leave every one of these silent: a step with
    /// no verdict cannot be advanced, and a `PASS` with no evidence is refused
    /// after the work is already done.
    ///
    /// The **client's** spelling of each tool, not the server's: Claude Code
    /// exposes `fleet.report` to the model as `mcp__armada__fleet_report`
    /// (`docs/traps.md`), and a prompt written with the documented name matches
    /// nothing at all. `crates/helm/src/mcp/drone.rs` holds these names against
    /// the router that actually serves them.
    /// **Every tool the brief instructs a Drone to use must be granted.**
    ///
    /// This is the assertion that would have caught the bug: `BRIEF` named three
    /// `mcp__armada__*` tools that [`ALLOW`] did not grant, so a Drone was
    /// refused — silently, under `dontAsk` — the only means it had of reporting
    /// anything. Observed in real use as a Drone answering its operator in prose
    /// and as a Job that never advanced.
    ///
    /// Derived from `BRIEF` rather than restated, so a tool added to the brief
    /// with no matching grant fails here rather than in a Job hours later.
    #[test]
    fn every_tool_the_brief_names_is_one_the_drone_is_granted() {
        let named: Vec<&str> = BRIEF
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|w| w.starts_with("mcp__"))
            .collect();
        assert!(
            !named.is_empty(),
            "the brief names no tools — this test would pass vacuously"
        );
        for tool in named {
            assert!(
                ALLOW.contains(&tool),
                "the brief tells a Drone to use `{tool}`, which ALLOW does not grant — \
                 under MODE `{MODE}` an uncovered tool is denied, and silently"
            );
        }
    }

    /// The skill tool, kept separate because nothing in `BRIEF` names it: a
    /// workflow step names its skill, and the brief is workflow-agnostic.
    ///
    /// A Drone was handed *"use the `reproduce-failure` skill"* and reported it
    /// could not access the skill it had been told it had. That is the
    /// measurement this list was missing.
    #[test]
    fn a_drone_may_invoke_a_skill_its_step_names() {
        assert!(
            ALLOW.contains(&"Skill"),
            "a workflow step names the skill that runs it; without this the step cannot run"
        );
    }

    /// A wildcard would grant whatever the router gains next, and `BRIEF`
    /// promises a Drone cannot spawn Jobs.
    #[test]
    fn the_mcp_grants_are_named_individually_rather_than_by_wildcard() {
        for tool in ALLOW.iter().filter(|t| t.starts_with("mcp__")) {
            assert!(
                !tool.ends_with('*') && tool.matches("__").count() == 2,
                "`{tool}` is not one fully-qualified tool name"
            );
        }
        assert!(
            !ALLOW.contains(&"mcp__armada__fleet_spawn"),
            "the brief promises a Drone cannot spawn Jobs"
        );
    }

    #[test]
    fn the_brief_states_the_contract_rather_than_asking_for_brevity() {
        for tool in [
            "mcp__armada__fleet_report",
            "mcp__armada__fleet_verdict",
            "mcp__armada__fleet_ask_human",
        ] {
            assert!(BRIEF.contains(tool), "the brief never names {tool}");
        }
        assert!(
            !BRIEF.contains("fleet.report"),
            "the dotted name matches nothing the model can call"
        );
        // The step-boundary vocabulary, and the two words that are not verdicts.
        for word in ["entered", "attempted"] {
            assert!(BRIEF.contains(word), "the brief omits `{word}`");
        }
        assert!(
            BRIEF.contains("completed"),
            "the brief does not say which boundary word is refused"
        );
        // Every verdict, so a Drone with a blocked step has a word for it.
        for verdict in ["PASS", "FAILED", "BLOCKED", "NEEDS_HUMAN"] {
            assert!(BRIEF.contains(verdict), "the brief omits {verdict}");
        }
        // And the rule the gate actually enforces (PLAN.md §14.3).
        assert!(BRIEF.contains("evidence"), "{BRIEF}");
        assert!(BRIEF.contains("not evidence"), "{BRIEF}");
        // It is not the reader's voice, and must never quietly become it: a
        // Drone that trimmed a question to a word count would be obeying the
        // one rule that is wrong in the one channel that reaches a person.
        assert!(
            !BRIEF.contains("150"),
            "the reader's rule for prose they read has reached a Drone: {BRIEF}"
        );
    }

    /// **A brief costs bytes on every turn of every Job**, so it is bounded here
    /// rather than discovered at `exec`. Helm's [`crate::helm::VOICE_BUDGET`]
    /// is 24 KiB because it carries prose somebody else wrote; this is Armada's
    /// own and there is no reason for it to be long.
    #[test]
    fn the_brief_is_small_enough_to_send_on_every_turn() {
        assert!(
            BRIEF.len() < 4096,
            "the brief is {} bytes, sent on every turn of every Job",
            BRIEF.len()
        );
        assert!(!BRIEF.ends_with('\n'), "a trailing newline is not content");
    }

    /// An empty list emits no flag, because `--allowedTools` with nothing after
    /// it consumes whatever came next — and what comes next is `--print`.
    #[test]
    fn an_empty_list_emits_no_flag_rather_than_a_flag_with_nothing_after_it() {
        let posture = Posture {
            mode: MODE.to_string(),
            allow: vec![],
            deny: vec![],
        };
        assert_eq!(posture.argv(), ["--permission-mode", MODE]);
    }

    /// The three ways a hand-edited `permissions.yml` is wrong, each caught
    /// before a Drone is started rather than after one has died of it.
    #[test]
    fn a_posture_that_would_not_parse_as_argv_is_refused_by_name() {
        assert_eq!(Posture::default().wrong(), None);
        let with = |allow: &[&str], mode: &str| Posture {
            mode: mode.to_string(),
            allow: allow.iter().map(|r| r.to_string()).collect(),
            deny: vec![],
        };
        assert!(with(&[], "acceptEdits").wrong().is_none());
        assert!(with(&[], "yolo")
            .wrong()
            .unwrap()
            .contains("not a permission mode"));
        // A space *inside* the parentheses is one rule — the CLI's own example
        // is `"Bash(git *) Edit"`, so the separator is paren-aware.
        assert!(with(&["Bash(git push:*)", "Bash(git *)"], MODE)
            .wrong()
            .is_none());
        // Outside them it is two rules typed as one, and the second half is
        // read as a tool name nothing has.
        for two in ["Edit Write", "Edit,Write", "Bash(git *) Edit"] {
            assert!(
                with(&[two], MODE).wrong().unwrap().contains("two rules"),
                "`{two}` was accepted as one rule"
            );
        }
        assert!(with(&["--print"], MODE)
            .wrong()
            .unwrap()
            .contains("reads as a flag"));
        assert!(with(&["  "], MODE).wrong().unwrap().contains("blank"));
    }

    /// **Every rule the shipped default names is one the argv can carry.** The
    /// defaults are hand-written constants, and a separator typed outside the
    /// parentheses of one of them would split a deny rule into two rules that
    /// deny nothing.
    #[test]
    fn the_shipped_default_is_a_posture_the_argv_can_actually_carry() {
        assert_eq!(Posture::default().wrong(), None);
        // And every deny rule is a `Bash(...)` rule, because the tools that are
        // *not* Bash are already confined to the session's own directory —
        // there is nothing to subtract from them.
        for rule in DENY {
            assert!(rule.starts_with("Bash("), "`{rule}` is not a Bash rule");
            assert!(rule.ends_with(":*)"), "`{rule}` matches one spelling only");
        }
    }

    /// **`stream-json` requires `--verbose`, and the binary refuses without
    /// it.** Measured against a real spawn: every test in this file passed while
    /// no Drone had ever run, because the argv Armada built was rejected at
    /// argument-parse time.
    ///
    /// This is the assertion that would have caught it — and it is written
    /// against [`STREAM_JSON_NEEDS`] rather than against the literal, so a
    /// second requirement added there is enforced here without anybody
    /// remembering to come back.
    #[test]
    fn every_stream_json_argv_carries_what_the_cli_requires_with_it() {
        for argv in [
            spawn_argv(UUID, "go", &Posture::default(), None),
            resume_argv(UUID, "carry on", &Posture::default(), None),
        ] {
            assert!(
                argv.iter().any(|word| word == STREAM_JSON),
                "{argv:?} does not stream"
            );
            for required in STREAM_JSON_NEEDS {
                assert!(
                    argv.iter().any(|word| word == required),
                    "{argv:?} streams without {required}, which the CLI refuses"
                );
            }
        }
    }

    /// **Boarding is the one argv that must *not* carry them.** It is
    /// interactive, `--verbose` there would change what a person sees, and
    /// nothing is streaming.
    #[test]
    fn boarding_carries_none_of_the_headless_flags() {
        let argv = board_argv(UUID);
        for flag in [
            "--print",
            "--output-format",
            "--verbose",
            // **And no brief.** [`BRIEF`] tells a session it is being read by an
            // orchestrator rather than by a person; boarding is a person, at a
            // terminal, reading it themselves.
            APPEND,
            // **And no posture either.** Boarding hands the conversation to a
            // person at a terminal, and a terminal is exactly the thing that
            // can answer a permission prompt. Granting there would take the
            // decision away from the one party entitled to make it.
            "--permission-mode",
            ALLOWED,
            DISALLOWED,
        ] {
            assert!(!argv.iter().any(|word| word == flag), "{argv:?} has {flag}");
        }
    }

    /// **The probe is the real argv, minus the prompt, plus closed input.**
    /// Anything less faithful would validate a combination Armada does not use,
    /// which is how a check ends up green on a Drone that cannot start.
    #[test]
    fn the_doctor_probe_is_the_spawn_argv_with_nothing_to_say() {
        let probe = probe_argv();
        let real = spawn_argv(PROBE_SESSION, "go", &Posture::default(), None);

        // Every flag of the real argv, in the same order.
        let flags = |argv: &[String]| -> Vec<String> {
            argv.iter()
                .take_while(|word| *word != "--input-format")
                .cloned()
                .collect()
        };
        assert_eq!(flags(&probe), flags(&real)[..real.len() - 1]);

        // And it says nothing: no prompt, and input it will never receive.
        assert_eq!(&probe[probe.len() - 2..], ["--input-format", "stream-json"]);
        assert!(
            !probe.iter().any(|word| word == "go"),
            "the probe carries a prompt, so it would run a turn: {probe:?}"
        );
    }

    /// **A valid uuid, because an invalid one is checked first and masks the
    /// answer.** Measured: `--session-id not-a-uuid` reports `Invalid session
    /// ID` whether or not the flag combination is legal, so a probe built that
    /// way proves nothing.
    #[test]
    fn the_probes_session_id_is_a_valid_uuid() {
        let groups: Vec<&str> = PROBE_SESSION.split('-').collect();
        assert_eq!(groups.len(), 5);
        assert!(groups.iter().map(|g| g.len()).eq([8, 4, 4, 4, 12]));
        assert!(groups
            .iter()
            .all(|g| g.chars().all(|c| c.is_ascii_hexdigit())));
    }

    /// Every flag the Drone argv uses is one `armada doctor` knows to check for.
    /// A flag added to the argv and not to the list is one nothing would notice
    /// disappearing.
    #[test]
    fn every_flag_the_drone_uses_is_one_doctor_checks_for() {
        let used: Vec<String> = spawn_argv(UUID, "go", &Posture::default(), None)
            .into_iter()
            .chain(resume_argv(UUID, "go", &Posture::default(), None))
            .chain(super::super::classify::argv("go"))
            .chain(probe_argv())
            .filter(|word| word.starts_with("--"))
            .collect();
        for flag in &used {
            assert!(
                FLAGS.contains(&flag.as_str()),
                "`{flag}` is in an argv and not in FLAGS, so doctor would not \
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

    /// **`--resume`, never a second `--session-id`.** Minting where continuing
    /// was meant starts a Job's next turn as its first, with none of the context
    /// the answer was an answer to.
    #[test]
    fn continuing_a_job_resumes_the_session_rather_than_minting_one() {
        let argv = resume_argv(UUID, "yes, raise it to 90s", &narrow());
        assert_eq!(
            argv,
            [
                "claude",
                "--resume",
                UUID,
                APPEND,
                // **One appended prompt carrying both halves**: the reporting
                // contract, then Armada's own skill (`docs/reserved/008`). The
                // flag is singular, so a second occurrence would keep only the
                // last.
                &brief(),
                "--permission-mode",
                "dontAsk",
                "--allowedTools",
                "Edit",
                "Bash",
                "--disallowedTools",
                "Bash(git push:*)",
                "--print",
                "--output-format",
                "stream-json",
                "--verbose",
                "yes, raise it to 90s",
            ]
        );
        assert!(!argv.iter().any(|a| a == "--session-id"));
    }

    /// Boarding is interactive: no `--print`, because the whole point is that
    /// you drive it.
    ///
    /// **And no skill either.** Boarding hands the conversation to a person, and
    /// *do not edit `armada.yml` silently, propose it instead* is an instruction
    /// to an unattended agent — telling a person at a keyboard not to edit their
    /// own repository would be Armada refusing them a thing it cannot refuse.
    #[test]
    fn boarding_is_the_interactive_resume_and_carries_no_prompt() {
        assert_eq!(board_argv(UUID), ["claude", "--resume", UUID]);
    }

    /// **The skill reaches every headless turn, first and resumed.**
    ///
    /// This is `docs/reserved/008`'s injection, asserted where the bugs are. A
    /// Drone changes what a repository runs; until this flag existed nothing
    /// told it that noticing a stale `armada.yml` was part of the job, so it
    /// either edited the file inside a diff about something else or said nothing
    /// at all.
    ///
    /// **Argv alone does not prove the flag is accepted**, which is the whole
    /// lesson of the `--verbose` trap — so [`crate::skill::APPEND`] is also in
    /// [`FLAGS`], and [`probe_argv`] is built from [`spawn_argv`], which is what
    /// makes `armada doctor` exercise it against the real binary for free.
    #[test]
    fn every_headless_turn_carries_armadas_own_skill() {
        for argv in [
            spawn_argv(UUID, "fix the flake", &Posture::default(), None),
            resume_argv(UUID, "yes", &Posture::default(), None),
            continue_argv(UUID, &Posture::default(), None),
        ] {
            let at = argv
                .iter()
                .position(|word| word == APPEND)
                .unwrap_or_else(|| panic!("no appended system prompt: {argv:?}"));
            assert!(
                argv[at + 1].contains(crate::skill::BODY),
                "the brief reached the session and the skill did not, which is \
                 what appending the flag twice would silently produce: {:?}",
                argv[at + 1]
            );
            assert!(
                argv[at + 1].starts_with(BRIEF),
                "the reporting contract went missing: {:?}",
                argv[at + 1]
            );
            // **One occurrence, never two.** `--append-system-prompt <prompt>`
            // is singular in `claude --help`, so a second would keep only the
            // last and drop the first without a word.
            assert_eq!(argv.iter().filter(|word| *word == APPEND).count(), 1);
            // **The prose is one argument, whatever is in it.** A body split
            // across argv elements would be read as flags and tool names.
            assert!(argv[at + 1].contains("mcp__armada__fleet_propose"));
            // **Before `--print`**, so the value cannot be swallowed by the
            // variadic tool lists that come before it.
            assert!(
                at + 1 < argv.iter().position(|word| word == "--print").unwrap(),
                "{argv:?}"
            );
        }
        // Inverted once: the assertions above would all hold vacuously against
        // a boarding argv that has neither flag, and it must not have them.
        assert!(!board_argv(UUID).iter().any(|word| word == APPEND));
    }

    /// The prompt is the last element and is never split. A task arrives as free
    /// text and a shell has already had its turn with it.
    #[test]
    fn the_prompt_is_one_argument_however_many_words_it_has() {
        let argv = spawn_argv(UUID, "add rate limiting to the API --json", &narrow(), None);
        assert_eq!(argv.last().unwrap(), "add rate limiting to the API --json");
        assert_eq!(argv.len(), 17);
        // And a prompt that *looks* like a flag is still the prompt, because a
        // flag closes the tool list before it and nothing reopens one.
        assert_eq!(argv[argv.len() - 2], "--verbose");
    }

    /// **The measured values from the spike** (PHASES.md §9.1 F2), read back off
    /// a recorded stream. Nothing here is estimated and no test spends a token.
    const RECORDED: &str = r#"
{"type":"system","subtype":"init","session_id":"15bfa340-33b1-4f81-bd7f-688f0f01dbb0"}
{"type":"assistant","message":{"content":[{"type":"text","text":"working"}]}}
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour","resetsAt":1754748131}}
{"type":"result","subtype":"success","is_error":false,"num_turns":2,"duration_api_ms":2956,"total_cost_usd":0.1724735,"stop_reason":"end_turn","result":"done","usage":{"input_tokens":4,"output_tokens":85,"cache_creation_input_tokens":14815,"cache_read_input_tokens":44357}}
"#;

    #[test]
    fn the_turns_ledger_is_read_straight_off_the_result_event() {
        let reading = read(RECORDED);
        let turn = reading.last().expect("a finished turn has a result event");
        assert_eq!(turn.spend.turns, 2);
        assert_eq!(turn.spend.api_ms, 2_956);
        assert!((turn.spend.cost_usd - 0.1724735).abs() < 1e-9);
        assert_eq!(turn.stop_reason.as_deref(), Some("end_turn"));
        assert!(!turn.is_error);
        assert_eq!(turn.result.as_deref(), Some("done"));
    }

    /// **Every kind of token, because every kind is billed.** Input plus output
    /// alone is 89 against a real 59,261 — a ceiling computed from the smaller
    /// number never stops anything.
    #[test]
    fn a_cached_turn_counts_its_cache_tokens_and_not_only_its_input() {
        let reading = read(RECORDED);
        assert_eq!(reading.spend.tokens, 4 + 85 + 14_815 + 44_357);
        assert_ne!(reading.spend.tokens, 4 + 85, "the cache was not counted");
    }

    /// The window travels with the reading, because "may I spawn another one" is
    /// one question rather than two.
    #[test]
    fn the_rate_limit_window_is_reported_alongside_the_turn() {
        let limit = read(RECORDED).rate_limit.expect("a window");
        assert_eq!(limit.status, "allowed");
        assert_eq!(limit.kind, "five_hour");
        assert_eq!(limit.resets_at, Some(1_754_748_131));
    }

    /// **A Drone that has not finished a turn yet reads as empty, not as
    /// broken.** That is the ordinary state of a Job whose Drone is still
    /// working — which, now that a Drone runs detached, is what `armada fleet
    /// ls` sees most of the time.
    #[test]
    fn a_stream_with_no_finished_turn_reads_as_empty_rather_than_failing() {
        for stream in [
            "",
            "{\"type\":\"system\",\"subtype\":\"init\"}\n{\"type\":\"assis",
        ] {
            let reading = read(stream);
            assert!(reading.turns.is_empty());
            assert!(reading.last().is_none());
            assert_eq!(reading.spend, Spend::default());
        }
    }

    /// A turn that ended in an error still carries its ledger: the spend
    /// happened whether or not the work did.
    #[test]
    fn a_failed_turn_still_reports_what_it_cost() {
        let reading = read(
            r#"{"type":"result","is_error":true,"num_turns":1,"total_cost_usd":0.02,"usage":{"input_tokens":10,"output_tokens":2}}"#,
        );
        let turn = reading.last().unwrap();
        assert!(turn.is_error);
        assert_eq!(reading.spend.tokens, 12);
        assert!((reading.spend.cost_usd - 0.02).abs() < 1e-9);
    }

    /// **A resumed session appends its own `result`, so the transcript is the
    /// ledger and the ledger is a sum.**
    ///
    /// This is the assertion the change to a detached Drone turns on. While
    /// `spawn` blocked, Fleet added each turn's spend to a running total it
    /// maintained itself; a detached Drone reports to nobody, so the file is the
    /// only account there is — and reading only the *last* turn would reset a
    /// Job's spend to zero every time it was answered, which is the failure that
    /// makes a budget unenforceable for exactly the Jobs that ask questions.
    #[test]
    fn two_turns_in_one_transcript_sum_rather_than_replace() {
        let reading = read(
            "{\"type\":\"result\",\"num_turns\":2,\"total_cost_usd\":0.1,\"usage\":{\"input_tokens\":100}}\n\
             {\"type\":\"result\",\"num_turns\":3,\"total_cost_usd\":0.2,\"usage\":{\"input_tokens\":200}}\n",
        );
        assert_eq!(reading.turns.len(), 2);
        assert_eq!(reading.spend.turns, 5, "not 3 — the turns are summed");
        assert_eq!(reading.spend.tokens, 300);
        assert!((reading.spend.cost_usd - 0.3).abs() < 1e-9);
        // The last turn is still reachable on its own, because *how the Job
        // ended* is a different question from *what it has spent*.
        assert_eq!(reading.last().unwrap().spend.turns, 3);
    }

    /// A missing `claude` is the machine's problem and not the repository's, so
    /// the class is the one whose correct response is *fix the machine, then
    /// retry unchanged*.
    #[test]
    fn a_missing_claude_is_an_environment_failure() {
        let error = not_on_path();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
    }
}
