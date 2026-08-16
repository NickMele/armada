---
id: 008
title: Armada injects its own skills
status: BUILT
module: cross-cutting
raised: real use — user request
---

# 008 — Armada injects its own skills

> **Built.** `armada_core::skill`, the `--append-system-prompt` on both the Drone's argv and
> Helm's launch, and the `fleet.propose` tool. What follows is the design, and the last section
> is what was deliberately left.

**The ask, and it generalises past where it started.** *"Armada should inject custom skills into
Helm and the subagents that are dispatched so that they can properly use Armada if needed,
including the manifest, and propose changes to the manifest or propose changes to the guild."*

**Why this is the missing half of the three-layer sandwich (PLAN.md §5).** Armada reports facts,
an agent authors, Armada verifies — and the middle layer was expected to know how to hold the
tools. A Drone that changed what a repository runs had learned something the `armada.yml` does
not say, and it had no instruction telling it that saying so is part of the job. So it had two
bad options and no good one: edit the manifest itself, inside a diff about something else, or
say nothing.

## One skill, and the restraint is the answer to the first open question

*Which skills exist.* **One.** It holds only what an agent cannot infer, and everything else it
needs is already somewhere it will look — the workflow says what the steps are, the persona says
how to talk, the repository says what it is.

| In the skill | Because it cannot be inferred |
|---|---|
| Armada arrives as MCP tools, and the `armada` CLI is denied on purpose | a session that cannot find a tool reaches for the shell, and the shell writes the user's **real** `~/.armada/` ([011](011-what-a-drone-may-do-unattended.md)) |
| evidence is an exit code, never an assertion | `fleet.verdict` refuses a `PASS` without evidence and nothing said why |
| a stale manifest is a **finding**, and findings are raised rather than fixed | this is the whole of this item. Nothing in a repository says that noticing is part of the work |
| `fleet.propose` exists, returns at once, and is not `fleet.ask_human` | a tool description is read *after* a model has decided to reach for a tool; the skill is what makes it decide to |

**The failure mode this item names is writing a lot of prose no agent reads**, so the skill is
capped by a test at 4 KiB and is currently about two. It is prepended to the system prompt of
every Drone turn and every Helm launch: a paragraph added to it is a paragraph bought again on
every turn any Job ever runs.

## Compiled in, not projected — and the projection was the obvious wrong answer

This item originally named the projection Guild already performs (PLAN.md §4.8,
[`PHASES.md`](../PHASES.md) §8.4) as the natural home. It is the wrong one, for four reasons and
the first is sufficient.

| Not `~/.claude/skills/` | Because |
|---|---|
| **it is the user's directory** | a projected file is one the user may edit or delete and a `guild upgrade` merge may overwrite. Armada's contract with the agents it spawns cannot be a file that is sometimes there |
| **projection is machine-scoped** | `projection.json` records what was placed in *this* `~/.claude/`, so a Drone on a machine where `guild project` never ran loses the instruction silently — a capability present or absent depending on a prior command, which is the `--verbose` class of failure |
| **a Drone would not load it** | skills are *discovered*, and discovery is exactly what `armada_guild::layout::skill_argv` already measured does not work — `claude /onboard-repo` answered *unknown command* |
| **it would leak** | a skill there is loaded by every session the user ever opens, including the ones with nothing to do with Armada |

So the prose is a constant and it reaches a session as an appended system prompt. **This is the
third use of one mechanism, not a third mechanism**: `skill_argv` proved it for the `config
scan` hand-over and `armada_core::helm::voice` proved it for the user's own three files.

**One appended prompt, never two.** `claude --help` spells the flag `--append-system-prompt
<prompt>` — singular. A second occurrence is a Commander option with no collector, so the last
wins and the first is dropped without a word. Helm's launch therefore composes Armada's skill
and the reader's voice into one string rather than emitting the flag twice, and the file it
writes for `"$(cat …)"` was renamed `guild-voice.md` → `system-prompt.md`, because it is no
longer only the guild's.

## May a Drone propose a guild change? Both, and the asymmetry is written down

*A guild is the user* — their voice, their workflows, what they are willing to let an unattended
agent do. A manifest is a repository's description of itself. The blast radius of an agent
**editing** those two is not remotely the same.

**But nothing here edits either.** A proposal writes one inbox entry and changes nothing at all,
so *may it* collapses into *is it worth reading* — and a Drone that has watched the same
workflow step be wrong in three repositories has noticed something about the person. Refusing
that outright would lose it to preserve a distinction the mechanism does not need.

So `Subject` has two values and both are allowed. The asymmetry lives in the skill's own prose,
which tells a Drone to be **sparing** with `guild` because it is looking at one repository, and
the enforcement is that applying a proposal is somebody else's verb.

## How a proposal comes back: the inbox, and there is no fifth origin

This item is downstream of [001](001-raised-items-need-identity.md), which is **BUILT** — one id
space over four origins, and `Origin::Raised` already means *a Drone asked for you*. A proposal
is an inbox entry, so `armada fleet answer <id>`, `armada failures show <id>` and the Bridge all
reach it on the day it is written. Nothing new is stored and nothing new is resolved.

**What Armada verifies is the shape, not the claim.** The subject is one of two words and the
body is not empty; a third subject is refused rather than guessed at, for the reason
`word_to_verdict` refuses a fifth verdict. Whether the proposal is *true* Armada cannot check —
the Drone is the only thing that ran the command — so verification of the content is the person
reading the row, or a Job they start from it, through a path that already exists.

## What was left

- **`manifest.status` and `manifest.check` are still out of a Drone's reach.** The Drone belt
  carries `fleet.*` only, and `Bash(armada:*)` is denied ([011](011-what-a-drone-may-do-unattended.md)).
  So the skill tells an agent what to do when it learns the manifest is wrong, and cannot tell it
  to go and look. A Drone learns from what it runs. Widening the belt is a separate decision with
  a separate blast radius and is not made here.
- **A proposal has no structure.** It is a sentence, not a diff. Something that could be applied
  by a `--yes` would be a scanner proposal ([007](007-scanner-should-propose.md)) rather than an
  observation, and the two are not the same shape: `007` proposes what it can *prove* from a file
  that already exists, and a Drone proposes what it *saw*.
- **Helm is given the skill and no new tool.** It already reads the inbox; a proposal arrives
  there like everything else, and the skill's last line says to bring it with its id rather than
  apply it. Nothing in Helm needed building.
