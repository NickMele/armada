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
//! # Two bodies, because there are two agents and they are not variants of one
//!
//! There was one constant, [`DRONE`] under the name `BODY`, appended to both
//! launches on the argument that it was *"the half both audiences need"*. That
//! argument was wrong, and the shape of the wrongness is worth keeping:
//!
//! **A Drone and Helm are different kinds of thing, not two settings of one.** A
//! Drone is started *by* Armada, headless, in a worktree, on a budget, with
//! nobody to answer a question — everything it may do is a grant Armada wrote,
//! and Armada is the only thing that can tell it what those grants are. Helm is a
//! session a **person** starts and sits in front of; it orchestrates, it holds
//! read-only tools, and its operating knowledge is its persona
//! (`templates/guild/subagents/helm.md`), which is the *user's* file.
//!
//! **The bundling produced an instruction one audience could not obey.** The
//! propose paragraph read *"Call `mcp__armada__fleet_propose`"* with no audience
//! named — and `fleet.propose` is on the Drone's belt and no other, so every Helm
//! session ever launched was told to reach for a tool it has never been offered.
//! That is `docs/reserved/031`'s *a grant is not a connection* family from the
//! other side: not a grant with no tool behind it, but an instruction with no
//! grant behind it. It could not have been caught by reading either constant,
//! because there was only one and it was correct for one of its readers.
//!
//! **And the tell was in the prose.** [`DRONE`] opened by asking its reader which
//! of two things it was. A body that has to begin *"either you are X or you are
//! Y"* is two bodies that have not been separated yet.
//!
//! | Constant | Reader | Holds |
//! |---|---|---|
//! | [`DRONE`] | a Drone, appended after [`crate::fleet::drone::BRIEF`] | what a headless worker in a worktree cannot infer about the grants it was given |
//! | [`HELM`] | Helm, appended before the reader's own fragments | the three things Armada's *orchestrator* cannot infer, and nothing its persona already says |
//!
//! # The restraint is still the design
//!
//! `008` asks *which skills exist* and warns against a library. Splitting one
//! constant in two is not the library it warns about: neither restates the other,
//! and the test that keeps that honest asserts it. Each holds only what its own
//! reader cannot infer:
//!
//! | In [`DRONE`] | Because a Drone cannot infer it |
//! |---|---|
//! | Armada arrives as MCP tools, and the `armada` CLI is denied on purpose | a session that cannot find a tool reaches for the shell, and the shell writes the user's **real** `~/.armada/` ([`docs/reserved/011`](../../../docs/reserved/011-what-a-drone-may-do-unattended.md)) |
//! | a stale manifest is a **finding**, and findings are raised rather than fixed | this is the whole of `008`. Nothing in a repository says that noticing is part of the work |
//! | `mcp__armada__fleet_propose` exists, returns at once, and is not `fleet.ask_human` | a tool description is read after a model has decided to reach for a tool; this is what makes it decide to |
//!
//! | In [`HELM`] | Because Helm cannot infer it |
//! |---|---|
//! | it cannot change a file, and that is deliberate rather than an oversight | a model refused an `Edit` looks for another way to do it — measured, through `jq` and `mv`, onto the operator's own `~/.claude/settings.json` (`docs/reserved/031` §1) |
//! | a Drone's summary is not evidence; the exit code its Job recorded is | Helm's job is to aggregate and report, and the thing it is aggregating is written by models that believe themselves |
//! | a proposal in its inbox is carried to the person with its id and never applied | the entry looks exactly like a change somebody already agreed to |
//!
//! Everything else either agent needs is already somewhere it will look: the
//! workflow says what the steps are, the persona says how to talk, and the
//! repository says what it is. Prose restating any of that is prose nobody reads,
//! which is `008`'s own stated failure mode.
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
//! # One appended prompt per launch, never two — and neither body is the whole of it
//!
//! `claude --help` spells the flag `--append-system-prompt <prompt>` — singular,
//! not variadic. Passing it twice is a Commander option without a collector,
//! where the **last one wins and the first is dropped without a word**. So each
//! launch composes one string, and each of these is one part of one of them:
//!
//! | Launch | Appends |
//! |---|---|
//! | a **Drone** | [`crate::fleet::drone::BRIEF`] — *how you report* — then [`DRONE`] |
//! | **Helm** | [`HELM`], then the reader's own three fragments ([`crate::helm::appended`]) |
//!
//! **The flag itself is [`crate::fleet::drone::APPEND`]**, defined once beside
//! the brief and re-exported by [`crate::helm`]. Nothing here redefines it: two
//! spellings of one flag are two things `armada doctor` would have to be told
//! about separately, and it is already in both `FLAGS` lists.
//!
//! # What [`DRONE`] does *not* say, because the brief already does
//!
//! [`crate::fleet::drone::BRIEF`] names the three reporting tools, states that a
//! `PASS` carries evidence an external command produced, and settles how long a
//! Drone's closing message should be. That is a worker's contract with an
//! orchestrator (`docs/reserved/019`), and it travels in the same appended prompt
//! — so [`DRONE`] says none of it.

/// What the skill is called, where a reader has to name it.
///
/// **Not a file on disk anywhere**, which is the point of the module above. It
/// is the name `armada doctor` and the docs use so that "Armada's own skill" is
/// a thing with one spelling rather than a description.
pub const NAME: &str = "working-under-armada";

/// **What a Drone cannot infer**, appended after
/// [`crate::fleet::drone::BRIEF`] to every headless turn of every Job.
///
/// **Roughly two kilobytes, and that ceiling is deliberate.** It is paid for on
/// every turn any Job ever runs, so every sentence is bought again forever.
/// [`crate::helm::VOICE_BUDGET`] is 24 KiB for the *reader's* prose because a
/// person's standing instructions are worth that; Armada's own are worth what
/// they cannot be inferred, and no more.
///
/// **Addressed to a Drone throughout, in the second person.** It used to open by
/// asking its reader which of two agents it was, because one constant served
/// both — see the module doc for what that cost.
///
/// **Written to be read by a model, not by a person browsing docs.** Short
/// headings, one table, and the rule stated before its rationale — the opposite
/// of this repository's own prose style, on purpose.
pub const DRONE: &str = "\
# Working under Armada

You are a **Drone**: one Armada Job, in its own git worktree, on its own `armada/<job>` branch,
headless, on a budget, with nobody watching. This is what Armada expects of you and cannot work
out for itself.

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
should know that you are not blocked on.";

/// **What Helm cannot infer**, appended before the reader's own three fragments
/// to the session `armada helm` opens.
///
/// **Three things, and its shortness is the argument.** Helm's operating
/// knowledge already has a home — `templates/guild/subagents/helm.md`, seeded
/// into the reader's guild, merged forward by `armada guild upgrade` and editable
/// by them. Anything Armada writes here is prose the reader cannot change, so it
/// holds only what must not be editable: two facts about Helm's own grants, and
/// one about what an inbox entry is not.
///
/// **It says nothing to a Drone**, which is the whole point of it being a second
/// constant. See the module doc.
pub const HELM: &str = "\
# Working under Armada

You are **Helm**: the session a person opens to run a fleet of Armada Jobs. You decompose what
they ask into Jobs, delegate those, aggregate what comes back, and bring them the decisions that
are theirs. This is what Armada expects of you and cannot work out for itself.

## You read; you do not write

You hold `Read`, `Grep` and `Glob` and no `Edit`, `Write` or `Bash`, **deliberately**. Authorship
happens inside a Job — in its own worktree, on its own branch, against a budget, reviewed. If the
right answer is a change to a file, that is a Job to spawn, not a refusal to work around. Say
what the change should be and who should make it.

## A Drone's summary is not evidence

What comes back from a Job is written by a model that believes itself. The evidence is the exit
code its checks recorded and the verdict Armada stored — those are facts, and everything else a
Drone said about its own work is a claim. Report the two differently to the person. Never upgrade
*it says it fixed it* into *it is fixed*.

## A proposal in your inbox is not a decision

Jobs raise proposals about this repository's `armada.yml` or about how this person works. They
change nothing. Bring one to the person **with its id** and let them settle it; do not apply one
yourself, and do not treat one as already agreed because it is written down.";

#[cfg(test)]
mod tests {
    use super::*;

    /// Both constants, wherever an assertion is about *prose Armada hands a
    /// model* rather than about one reader's contract.
    const BOTH: [(&str, &str); 2] = [("DRONE", DRONE), ("HELM", HELM)];

    /// **The three facts the module doc says are the whole reason this exists.**
    ///
    /// Inverted once, as `ARCHITECTURE.md` §2.1.1 requires: deleting the
    /// `fleet.propose` paragraph from [`DRONE`] fails this, which is the check
    /// that the skill still carries `008`'s point rather than only its
    /// formatting.
    #[test]
    fn the_drones_skill_says_the_things_a_drone_cannot_infer() {
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
                DRONE.contains(phrase),
                "the skill no longer says `{phrase}`, which is one of the four \
                 things `docs/reserved/008` exists to tell an agent"
            );
        }
    }

    /// **The three facts Helm cannot infer**, and they are three because Helm's
    /// operating knowledge lives in its persona — a file the reader owns and
    /// edits. What is here is only what must not be editable.
    ///
    /// Inverted once: deleting any of the three paragraphs fails this.
    #[test]
    fn helms_own_body_says_the_three_things_its_persona_cannot_own() {
        for (phrase, why) in [
            // It reads and does not write, and that is deliberate — a model
            // refused an `Edit` looks for another way (`docs/reserved/031` §1).
            (
                "do not write",
                "Helm is left to discover its own grants by being refused",
            ),
            // A Drone's summary is not evidence.
            (
                "is not evidence",
                "Helm reports a Drone's claim to the person as a fact",
            ),
            // A proposal is not a decision.
            ("not a decision", "Helm applies a proposal nobody agreed to"),
        ] {
            assert!(HELM.contains(phrase), "`{phrase}` is gone, so {why}");
        }
    }

    /// **Neither body addresses the other's reader**, which is the whole of why
    /// there are two.
    ///
    /// One constant served both and opened by asking its reader which it was;
    /// the propose paragraph then told *whoever was reading* to call
    /// `mcp__armada__fleet_propose`, a tool on the Drone's belt and no other. So
    /// every Helm session ever launched was instructed to reach for a tool it
    /// has never held. A body that has to say *"if you are Helm"* is two bodies
    /// that have not been separated yet, and this is the assertion that keeps
    /// them apart.
    #[test]
    fn neither_body_is_addressed_to_the_others_reader() {
        assert!(
            !DRONE.contains("Helm"),
            "the Drone's body speaks to Helm: {DRONE}"
        );
        assert!(
            !HELM.contains("If you are") && !DRONE.contains("Either you are"),
            "a body that asks its reader which agent it is, is two bodies"
        );
        // The two tools only a Drone holds are named only where a Drone reads.
        for drones_own in ["fleet_propose", "fleet_ask_human"] {
            assert!(
                DRONE.contains(drones_own),
                "`{drones_own}` is the Drone's and it is not told about it"
            );
            assert!(
                !HELM.contains(drones_own),
                "Helm is told to call `{drones_own}`, which is on the Drone's belt \
                 and no other — the defect the split exists to close"
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
        assert!(DRONE.contains("`manifest`"));
        assert!(DRONE.contains("`guild`"));
        assert!(
            DRONE.contains("Be sparing"),
            "the guild is the user, and a skill that offered it as an equal \
             option would be inviting a Drone to edit them"
        );
    }

    /// **Each has to fit in a system prompt that is paid for on every exchange.**
    ///
    /// Not a style rule: [`DRONE`] is prepended to every headless turn of every
    /// Job and [`HELM`] to every Helm launch, so a paragraph added to either is a
    /// paragraph bought again every time.
    ///
    /// **The ceiling is per body rather than for the pair**, because no launch
    /// ever carries both — splitting one constant in two did not double what
    /// anybody pays.
    #[test]
    fn each_body_stays_small_enough_to_pay_for_on_every_turn() {
        for (name, body) in BOTH {
            assert!(
                body.len() < 4 * 1024,
                "`{name}` is {} bytes; past 4 KiB it is a library, which is what \
                 `docs/reserved/008` says not to build",
                body.len()
            );
            assert!(!body.is_empty(), "`{name}` is empty");
        }
    }

    /// A trailing newline would make the appended prompt and the file written
    /// beside it differ by one byte, which is exactly the drift
    /// [`crate::helm::launch_line`]'s `"$(cat …)"` substitution cannot survive —
    /// command substitution strips trailing newlines.
    #[test]
    fn neither_body_carries_a_trailing_newline() {
        for (name, body) in BOTH {
            assert!(
                !body.ends_with('\n'),
                "`{name}`: {:?}",
                &body[body.len() - 40..]
            );
        }
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
                !DRONE.contains(owned),
                "`{owned}` is the brief's (`docs/reserved/019`) and is now said twice"
            );
        }
        // Inverted once: the brief says nothing about proposing, which is why
        // this constant exists at all.
        assert!(!crate::fleet::drone::BRIEF.contains("fleet_propose"));
        assert!(DRONE.contains("mcp__armada__fleet_propose"));
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
    fn no_tool_in_either_body_is_spelled_the_way_the_wire_spells_it() {
        for (name, body) in BOTH {
            for dotted in [
                "fleet.propose",
                "fleet.ask_human",
                "fleet.verdict",
                "fleet.report",
            ] {
                assert!(
                    !body.contains(dotted),
                    "`{name}` says `{dotted}`, which matches nothing the model can call"
                );
            }
        }
        assert!(DRONE.contains("mcp__armada__fleet_propose"));
        assert!(DRONE.contains("mcp__armada__fleet_ask_human"));
    }

    /// **No absolute home anywhere in prose Armada ships.** The privacy gate
    /// checks tracked files; these are tracked files that are also handed to a
    /// model, so it is worth asserting rather than assuming.
    #[test]
    fn neither_body_names_anyones_home_directory() {
        for (name, body) in BOTH {
            assert!(!body.contains("/Users/"), "`{name}`: {body}");
            assert!(!body.contains("/home/"), "`{name}`: {body}");
        }
    }
}
