---
id: 028
title: Specialised Drones, and subagents a repository declares
status: RESERVED
module: cross-cutting
raised: an idea while watching a Job run, 2026-08-16
---

# 028 — Specialised Drones, and subagents a repository declares

**Raised in his words:** *"we need armada to support per manifest sub agents. Maybe armada comes
with a fleet of specialized drones that can be used for various tasks. That would be sick."*

Two ideas, and they are worth keeping apart because one is a mechanism and the other is content
shipped on top of it.

## 1 · A repository declares its own subagents

**The mechanism already has a precedent, and it is skills.** A repo-local skill is a named grant
plus a pointer to prose ([`PLAN.md`](../PLAN.md) §4.8): the mechanical half lives in `armada.yml`
and the prose is a markdown file Manifest never parses. A repo-local *subagent* is the same shape
with a different payload — a name, a persona, and the tools it may hold.

Claude Code reaches subagents through `--agent`, which Armada already uses: `armada helm` launches
`claude --agent helm`. So a specialised Drone is plausibly **not new machinery at all** — it is a
different `--agent`, a different posture, and a workflow step that names it.

That is the strongest argument for doing this: the three parts exist. A workflow step already
names a `skill:`; a step naming an `agent:` is the same sentence with a different noun.

## 2 · Armada ships a fleet of them

A reviewer, a test-writer, a migrator, a doc-writer — chosen per step rather than one general
Drone told to behave differently each time. **The argument for it is the same one
[`019`](019-the-brief-a-drone-reports-through.md) makes about the brief**: a prompt is guidance
and a mechanism is not, and *"you are reviewing"* in a task string is guidance. A reviewer agent
with `Edit` withheld cannot quietly start rewriting what it was asked to judge.

It also gives [`016`](016-what-the-gate-cannot-prove.md) something concrete. `review_clean` is one
of two predicates Fleet cannot settle because it needs a Job it does not spawn — and *"a reviewer
Job"* is exactly what a shipped reviewer Drone would be.

## What has to be settled first, and it is a naming problem

**Three things would be able to call themselves a subagent**, which is the defect
[`glossary.md`](../glossary.md) exists to prevent:

| Where | What it would be | Precedent |
|---|---|---|
| `~/.armada/guild/subagents/` | yours, machine-global, synced | the directory **already exists** |
| `armada.yml` | this repository's, committed with it | repo-local skills |
| compiled in | Armada's own, shipped | [`008`](008-armada-injects-its-own-skills.md)'s skill |

Skills already have exactly this three-way split and resolve it by ownership rather than by
mechanism, so the answer is probably the same — but it has to be *stated*, and the resolution
order has to be stated with it. Which wins when a repo declares a `reviewer` and the guild has
one is the question that will be asked five minutes after this ships.

## The part that is not obvious

**A specialised Drone probably wants its own posture**, and [`011`](011-what-a-drone-may-do-unattended.md)'s
`ALLOW`/`DENY` is currently one list for every Drone. A reviewer that cannot `Edit`, a doc-writer
that cannot run `Bash` — those are the point of specialising, and they are a posture per agent
rather than per machine. That is a real change to `011`'s shape, not a addition to it, and it is
the piece most likely to be discovered late.

**Nothing here is urgent.** It is worth writing down now because the two halves — the mechanism
and the fleet shipped on it — will look like one task later, and the naming question above is
cheap to answer before anything is built and expensive after.
