//! **Armada's own instructions to the agents it runs**
//! ([`docs/reserved/008`](../../../docs/reserved/008-armada-injects-its-own-skills.md)).
//!
//! `PLAN.md` §5's sandwich is *Armada reports facts, an agent authors, Armada
//! verifies* — and until this module existed the middle layer was **expected to
//! know how to hold the tools**. A Drone that changed what a repository runs had
//! learned something the `armada.yml` did not say, and nothing told it that
//! saying so was part of the job. So it either edited the manifest inside a diff
//! about something else, or it said nothing at all. Both are silent.
//!
//! # One skill, and the restraint is the design
//!
//! `008` asks *which skills exist* and warns against a library. The answer is
//! **one**, and it holds only what an agent cannot infer:
//!
//! | In [`BODY`] | Because it cannot be inferred |
//! |---|---|
//! | Armada arrives as MCP tools, and the `armada` CLI is denied on purpose | a session that cannot find a tool reaches for the shell, and the shell writes the user's **real** `~/.armada/` ([`docs/reserved/011`](../../../docs/reserved/011-what-a-drone-may-do-unattended.md)) |
//! | evidence is an exit code, never an assertion | every agent believes its own summary; `fleet.verdict` refuses a `PASS` without evidence and nothing said why |
//! | a stale manifest is a **finding**, and findings are raised rather than fixed | this is the whole of `008`. Nothing in a repository says that noticing is part of the work |
//! | `mcp__armada__fleet_propose` exists, returns at once, and is not `fleet.ask_human` | a tool description is read after a model has decided to reach for a tool; this is what makes it decide to |
//!
//! Everything else an agent needs is already somewhere it will look: the
//! workflow says what the steps are, the persona says how to talk, and the
//! repository says what it is. A second skill restating any of that would be
//! prose nobody reads, which is `008`'s own stated failure mode.
//!
//! # Why it is compiled in rather than projected into `~/.claude/`
//!
//! `armada guild project` already puts a guild's skills where Claude Code reads
//! them (`PHASES.md` §8.4), and `008` names that projection as the natural home.
//! It is the wrong one, for four reasons and the first is sufficient:
//!
//! 1. **`~/.claude/skills/` is the user's, and this is Armada's.** A projected
//!    file is one the user may edit, delete or have overwritten by a `guild
//!    upgrade` merge. Armada's own contract with the agents it spawns cannot be
//!    a file that is sometimes there.
//! 2. **Projection is machine-scoped.** `projection.json` records what was
//!    placed in *this* `~/.claude/`, so a Drone on a machine where `guild
//!    project` never ran would silently lose the instruction — a capability
//!    present or absent depending on a prior command, which is the `--verbose`
//!    class of failure `docs/traps.md` exists for.
//! 3. **A Drone would not load it anyway.** Skills are discovered, and discovery
//!    is exactly what [`armada_guild::layout::skill_argv`] already found does not
//!    work: `claude /onboard-repo` answered *unknown command* because guild
//!    skills live where Claude Code does not look.
//! 4. **It would leak.** A skill in `~/.claude/skills/` is loaded by every
//!    session the user ever opens, including the ones that have nothing to do
//!    with Armada.
//!
//! So the prose is a constant, and it reaches a session as an appended system
//! prompt — the mechanism [`armada_guild::layout::skill_argv`] proved for the
//! `config scan` hand-over and [`crate::helm::voice`] proved for the user's own
//! three files. This is the third use of one mechanism, not a third mechanism.
//!
//! # One appended prompt, never two — and this body is never the whole of it
//!
//! `claude --help` spells the flag `--append-system-prompt <prompt>` — singular,
//! not variadic. Passing it twice is a Commander option without a collector,
//! where the **last one wins and the first is dropped without a word**. So each
//! launch composes one string, and this body is one half of it:
//!
//! | Launch | Appends |
//! |---|---|
//! | a **Drone** | [`crate::fleet::drone::BRIEF`] — *how you report* — then this |
//! | **Helm** | this, then the reader's own three fragments ([`crate::helm::appended`]) |
//!
//! **The flag itself is [`crate::fleet::drone::APPEND`]**, defined once beside
//! the brief and re-exported by [`crate::helm`]. Nothing here redefines it: two
//! spellings of one flag are two things `armada doctor` would have to be told
//! about separately, and it is already in both `FLAGS` lists.
//!
//! # What this body does *not* say, because the brief already does
//!
//! [`crate::fleet::drone::BRIEF`] names the three reporting tools, states that a
//! `PASS` carries evidence an external command produced, and settles how long a
//! Drone's closing message should be. That is a worker's contract with an
//! orchestrator (`docs/reserved/019`) and Helm has no use for it. This is the
//! half both audiences need, so it says nothing the brief says.

/// What the skill is called, where a reader has to name it.
///
/// **Not a file on disk anywhere**, which is the point of the module above. It
/// is the name `armada doctor` and the docs use so that "Armada's own skill" is
/// a thing with one spelling rather than a description.
pub const NAME: &str = "working-under-armada";

/// The whole of it.
///
/// **Roughly two kilobytes, and that ceiling is deliberate.** It is prepended to
/// the system prompt of every Drone turn and every Helm launch, so every
/// sentence is paid for on every exchange forever. [`crate::helm::VOICE_BUDGET`]
/// is 24 KiB for the *reader's* prose because a person's standing instructions
/// are worth that; Armada's own are worth what they cannot be inferred, and no
/// more.
///
/// **Written to be read by a model, not by a person browsing docs.** Short
/// headings, one table, and the rule stated before its rationale — the opposite
/// of this repository's own prose style, on purpose.
pub const BODY: &str = "\
# Working under Armada

You are running under Armada. Either you are **Helm**, the session that orchestrates a fleet of
Jobs, or you are a **Drone** — one Job, in its own git worktree, on its own `armada/<job>`
branch, with nobody watching. This is what Armada expects of you and cannot work out for itself.

## Armada reaches you as tools, not as a command line

Armada's verbs arrive as MCP tools named `mcp__armada__…`. **Running `armada` in a shell is
denied to a Job on purpose**: the CLI writes the user's real `~/.armada/`, which holds every
other Job's record, every other worktree, and their guild. A tool you have not been given is a
tool you were not meant to use. Say so rather than reaching around it.

## The manifest is a claim, and claims go stale

`armada.yml` is the workspace's own description of itself: its services, its commands, its
checks. You will sometimes learn something it does not say — you ran a command it does not list,
a check it declares no longer exists, a service wanted a port it never claimed. **That is a
finding, and reporting it is part of the job.**

## Raise it; do not fix it silently

**Do not edit `armada.yml` to make it match what you learned.** A manifest edited by an agent is
a claim nobody checked, arriving inside a diff about something else — and Armada verifies rather
than taking an agent's word for anything.

Call `mcp__armada__fleet_propose`. It writes one inbox entry with an id, hands the id back, and
returns at once: you are not waiting for an answer, and you carry on with the step you were on. A
proposal is not a change, and nothing you propose takes effect until the person says so.

| `subject` | Use it for |
|---|---|
| `manifest` | something about **this repository** — a command, a check, a service, a port. This is the one you will use. |
| `guild` | something about **how this person works** — a standing preference, a workflow step that is wrong for them every time. Be sparing: their guild is them rather than their code, and you are looking at one repository. |

`mcp__armada__fleet_ask_human` is the other half of the pair and is not the same thing. It is for
a question you cannot proceed without an answer to, and it waits. A proposal is something they
should know that you are not blocked on.

**If you are Helm**: proposals arrive in your inbox like anything else a Job raised. Bring them
to the person with their id, and do not apply one yourself.";

#[cfg(test)]
mod tests {
    use super::*;

    /// **The three facts the module doc says are the whole reason this exists.**
    ///
    /// Inverted once, as `ARCHITECTURE.md` §2.1.1 requires: deleting the
    /// `fleet.propose` paragraph from [`BODY`] fails this, which is the check
    /// that the skill still carries `008`'s point rather than only its
    /// formatting.
    #[test]
    fn the_skill_says_the_things_an_agent_cannot_infer() {
        for phrase in [
            // Armada is tools, and the CLI is denied on purpose.
            "denied to a Job on purpose",
            // A stale manifest is a finding.
            "part of the job",
            // And the tool that carries it back, spelled the way the model
            // sees it rather than the way the wire does (`docs/traps.md`,
            // *Claude Code renames a dotted tool*).
            "mcp__armada__fleet_propose",
        ] {
            assert!(
                BODY.contains(phrase),
                "the skill no longer says `{phrase}`, which is one of the four \
                 things `docs/reserved/008` exists to tell an agent"
            );
        }
    }

    /// **Both subjects are named, because `008` asks whether a Drone may propose
    /// a guild change and the answer is in the prose rather than in a type.**
    ///
    /// A Drone may propose either; the difference is blast radius, and blast
    /// radius is a thing you write down rather than a thing you enforce — the
    /// enforcement is that a proposal changes nothing at all.
    #[test]
    fn both_subjects_are_named_and_the_guild_one_is_hedged() {
        assert!(BODY.contains("`manifest`"));
        assert!(BODY.contains("`guild`"));
        assert!(
            BODY.contains("Be sparing"),
            "the guild is the user, and a skill that offered it as an equal \
             option would be inviting a Drone to edit them"
        );
    }

    /// **It has to fit in a system prompt that is paid for on every exchange.**
    ///
    /// Not a style rule: this string is prepended to every Drone turn and every
    /// Helm launch, so a paragraph added here is a paragraph bought again on
    /// every turn any Job ever runs.
    #[test]
    fn the_skill_stays_small_enough_to_pay_for_on_every_turn() {
        assert!(
            BODY.len() < 4 * 1024,
            "Armada's own skill is {} bytes; past 4 KiB it is a library, which \
             is what `docs/reserved/008` says not to build",
            BODY.len()
        );
        assert!(!BODY.is_empty());
    }

    /// A trailing newline would make the appended prompt and the file written
    /// beside it differ by one byte, which is exactly the drift
    /// [`crate::helm::launch_line`]'s `"$(cat …)"` substitution cannot survive —
    /// command substitution strips trailing newlines.
    #[test]
    fn the_body_carries_no_trailing_newline() {
        assert!(!BODY.ends_with('\n'), "{:?}", &BODY[BODY.len() - 40..]);
    }

    /// **The skill and the Drone's brief do not restate each other.**
    ///
    /// They are two constants in one appended prompt, so an overlap is paid for
    /// on every turn of every Job — and the brief is the one that already owns
    /// the reporting contract. This is the assertion that keeps the split
    /// honest rather than merely alphabetical.
    #[test]
    fn the_skill_leaves_the_reporting_contract_to_the_drones_brief() {
        for owned in ["fleet_verdict", "fleet_report", "PASS"] {
            assert!(
                !BODY.contains(owned),
                "`{owned}` is the brief's (`docs/reserved/019`) and is now said twice"
            );
        }
        // Inverted once: the brief says nothing about proposing, which is why
        // this constant exists at all.
        assert!(!crate::fleet::drone::BRIEF.contains("fleet_propose"));
        assert!(BODY.contains("mcp__armada__fleet_propose"));
    }

    /// **Every tool is named the way the model sees it, never the way the wire
    /// does** (`docs/traps.md`, *Claude Code renames a dotted tool*).
    ///
    /// The server advertises `fleet.propose`; Claude Code exposes it as
    /// `mcp__armada__fleet_propose`. A skill written with the dotted name
    /// matches nothing the model can call, and the model answers that it has no
    /// such tool — the same class of inert instruction that had Helm's persona
    /// asking for files it held no `Read` to open.
    #[test]
    fn no_tool_in_the_skill_is_spelled_the_way_the_wire_spells_it() {
        for dotted in [
            "fleet.propose",
            "fleet.ask_human",
            "fleet.verdict",
            "fleet.report",
        ] {
            assert!(
                !BODY.contains(dotted),
                "`{dotted}` matches nothing the model can call"
            );
        }
        assert!(BODY.contains("mcp__armada__fleet_propose"));
        assert!(BODY.contains("mcp__armada__fleet_ask_human"));
    }

    /// **No absolute home anywhere in prose Armada ships.** The privacy gate
    /// checks tracked files; this one is a tracked file that is also handed to a
    /// model, so it is worth asserting rather than assuming.
    #[test]
    fn the_skill_names_no_ones_home_directory() {
        assert!(!BODY.contains("/Users/"), "{BODY}");
        assert!(!BODY.contains("/home/"), "{BODY}");
    }
}
