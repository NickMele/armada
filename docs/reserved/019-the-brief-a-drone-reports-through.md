---
id: 019
title: The brief a Drone reports through
status: BUILT
module: fleet
raised: real use, 2026-08-15 — a Drone reported at whatever length it liked, and nothing had ever told one what it owed
---

# 019 — The brief a Drone reports through

> **Built and refined.** `armada_core::fleet::drone::BRIEF`, appended by
> [`headless`](../../crates/core/src/fleet/drone.rs) on every turn a Job takes, and held against
> the router that serves the tools by
> [`crates/helm/src/mcp/drone.rs`](../../crates/helm/src/mcp/drone.rs). 
>
> **Updated by [032](032-the-job-drives-the-drone.md):** The brief is now ~40% shorter. A Drone
> no longer reports which step ended, which verdict was reached, or what evidence was gathered —
> the Job determines all three by gating. The brief now simply states: report your status (`done`
> or `stuck`), and the Job decides what comes next. This simplification is exactly what the
> section below prophesied — *a contract a model cannot get wrong is one it is not asked to fill
> in*.
>
> **`BRIEF` is no longer the whole of what `headless` appends.**
> [008](008-armada-injects-its-own-skills.md) landed alongside it and shares the flag:
> `drone::brief()` joins `BRIEF` — *how you report* — with `skill::BODY` — *how you use Armada* —
> because `--append-system-prompt <prompt>` is singular and a second occurrence would keep only
> the last. They stay two constants: this one is a worker's contract with an orchestrator and
> Helm has no use for it, while `008`'s skill goes to Helm too. A test holds the split, so a
> sentence does not end up in both and get bought twice on every turn.

**The gap.** `armada helm` assembles the reader's `voice.md`, `expectations.md` and
`how-i-work.md` into an appended system prompt, so Helm speaks in the reader's register. A Drone
had no equivalent and no instruction of any kind: it ran headless, did the work, and wrote
whatever it wrote. [011](011-what-a-drone-may-do-unattended.md) gave a Drone permission to
*act*; nothing told it what it owed once it had.

## 1. A Drone talks to Helm, so the reader's voice is the wrong one to give it

Copying Helm's mechanism was the obvious move and it is the wrong one. Helm's product **is** a
sentence a person reads, so how that sentence is written is theirs. A Drone's output has three
destinations and a person is none of them:

| What a Drone writes | Who reads it |
|---|---|
| `fleet.report` bodies | the Job record — `fleet ls`, `fleet show`, Helm aggregating many Jobs |
| `fleet.verdict` | the gate, which reads an enum and an exit code and never the prose |
| the transcript's last `result` | the cheap model that summarises it, before Helm sees a word |

The reader's 150-word rule was therefore left out **deliberately**, and it is worth saying why in
both directions. It is **too weak** where it would apply — a report is one or two sentences, not
a hundred and fifty words — and it is **wrong** in the one channel that does reach a person:
`fleet.ask_human` is read out of context, possibly hours later, by somebody who has not seen the
work, and it needs *more* context, not less. A fleet of Drones each imitating the reader's
register would also hand Helm a chorus of impersonations to aggregate, when what Helm needs is
the same shape from every Job.

**What a Drone owes is a contract, not a register.** So the brief restates the contract instead
of asking for brevity: the step-boundary vocabulary, the four verdict words, and the rule that a
`PASS` carries evidence an external command produced. Each of those is a thing a Drone gets
wrong *silently* — a step left without a verdict cannot be advanced, a `PASS` without evidence is
refused after the work is already done, and a Job that never reports `entered` is one whose step
nobody can see. Asking an agent to be terse in the abstract produces prose that is shorter and no
more useful.

## 2. The tools are named as the model sees them

The server advertises `fleet.report`; Claude Code exposes it to the model as
`mcp__armada__fleet_report` ([`docs/traps.md`](../traps.md), *Claude Code renames a dotted tool*).
A brief written with the documented name matches nothing, and the model answers that it has no
such tool. The brief uses the client's spelling, and `mcp::drone`'s suite holds those three
strings against `TOOLS` so a rename on either side fails a test rather than a Job.

## 3. The risk this leaves open — a brief may name tools the posture denies

**This is a finding about [011](011-what-a-drone-may-do-unattended.md), not a defect in the
brief, and it was deliberately not fixed here.** The permission lists are the user's decision and
shipped as their own reserved item; adjusting them in passing while building something else is
exactly how a decision gets unmade by accident.

`drone::MODE` is `dontAsk`, which **denies and carries on** rather than prompting — that choice
is the whole of 011 §*The mode is the fix*. `drone::ALLOW` grants eight tool classes: `Read`,
`Glob`, `Grep`, `Edit`, `Write`, `NotebookEdit`, `TodoWrite` and `Bash`. **Not one of them
matches `mcp__armada__*`.** So the brief instructs a Drone to report through three tools that its
own posture may refuse it, and under `dontAsk` that refusal is silent.

The failure it would produce is the one this whole file exists to prevent, arriving one layer
lower: every step entered and none reported, every step ended and no verdict, and a Job that
`fleet ls` cannot advance — reported by the Drone, if at all, as a tool it was denied rather than
as a contract it could not meet. It is the same shape as 011's original bug, where *a missing
capability does not fail, it waits*.

**It is not proved, and proving it costs a real session.** Whether an MCP tool absent from
`--allowedTools` is *denied* or merely *not pre-approved* is Claude Code's behaviour, not
Armada's, and the only honest test spawns a Drone and spends a token — which
[PHASES.md](../PHASES.md) §8.5 forbids the suite from doing. So it is recorded here rather than
asserted anywhere.

Whoever revisits 011 owns the call, and there are two ways to make it:

| fix | what it costs |
|---|---|
| **Add the three tool names to `ALLOW`** — `mcp__armada__fleet_report`, `mcp__armada__fleet_verdict`, `mcp__armada__fleet_ask_human` | three strings, and a test that the allowlist and `mcp::drone::TOOLS` cannot drift apart; it grants exactly what the brief already asks for and nothing wider |
| **Confirm `dontAsk` never gates MCP tools** and record that in `docs/traps.md` | one deliberate session against a stub server, and the answer expires with the next Claude Code release |

Naming the tools is the cheaper of the two and does not depend on behaviour that can change under
Armada — but it is still an edit to a list [011](011-what-a-drone-may-do-unattended.md) owns.

## 4. Why it is a constant and not a file in the guild

Every word of the brief describes the contract of Armada's **own** MCP tools, so a guild copy
would be a description of a contract the guild does not own.
[006](006-guild-has-no-way-to-learn.md) is why that matters: a guild receives a template change
only through `armada guild upgrade`, which is a `git merge` somebody has to run. A guild that
never ran it would keep a stale description of `fleet.verdict`, and a *wrong* description is worse
than none — a Drone that believes it may report a step `completed` will try, be refused, and
spend a turn finding out.

**What is genuinely the reader's already reaches a Drone**, by paths that needed no new mechanism:
the worktree's own `CLAUDE.md` and `AGENTS.md`, which a Drone reads because it is an ordinary
Claude Code session in that repository, and `~/.armada/guild/permissions.yml`, which says what it
may do. A guild override costs one `Option<&str>` at the call site whenever somebody wants one;
adding it now would be a mechanism with no reader.

## Hook

The first Drone to run a Job end to end and be denied `mcp__armada__fleet_verdict` — or the first
person to read a `fleet show` and find the reports the same shape across every Job in the fleet.
