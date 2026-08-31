---
name: orchestrate-milestone
description: Running a whole milestone by dispatching agents at its issues — sequencing, briefing, verifying what comes back, merging, and what to decide versus what to bring the owner. Load before starting a milestone, not before starting an issue.
---

# Orchestrating a milestone

**One agent per issue, and you are not one of them.** Your work is sequencing,
briefing, checking what comes back, and merging. `work-issue` is what each agent
loads; this is what you do.

Focus was run this way on 30–31 Aug 2026 — eight issues, merged in a night, and
the milestone's own acceptance test passing at the end. Everything here is what
that cost to learn.

## Before dispatching anything

**A milestone whose issues are one-liners is not ready and no amount of briefing
fixes it.** Focus ran clean because every issue named the file, the failure each
decision existed for, and the wrong fix to avoid. Throughput's five said
*"Migrated from the project's design notes"* and nothing else; they had to be
rewritten before anyone could start.

**Settle the open questions first, with the owner.** Not during. An agent that
hits an undecided question stops, and a stopped queue at 3am is a wasted night.
Read every issue and its comments, list what is genuinely undecided, and put them
to the owner in one pass.

**Sequence by dependency and say what blocks what.** Focus was: the claim and the
acceptance test alone, then the schema and the artifact in parallel, then the
brief, then the lifecycle switch, then the model dial. Each one made the next
buildable. Getting this wrong wastes a whole agent.

## The brief

Every brief carries these, and each is here because leaving it out cost something:

- **The skill to load.** `work-issue` first, always.
- **The current verification numbers**, exactly: tests with `--exclude
  acceptance`, acceptance separately, and what `verify-foundations` reads on
  `main`. Ask for the delta, not the absolute — an agent reporting "86 failing"
  has told you nothing without the baseline.
- **What has changed since the issue was written.** Issues written a week ago
  describe a codebase that has moved, and an agent trusting a stale `## In`
  builds the wrong thing. Name what merged and tell it to verify rather than
  trust your summary.
- **Scope discipline**, including who else is working where — see `work-issue`.
- **The `**QUESTION:**` rule**, and explicitly: *if something needs deciding that
  nobody has decided, put it in your question line rather than deciding it.*
- **The Bridge half**, if the change crosses the wire. A feature means both
  halves unless the owner says otherwise.

**Brief them to push back.** Say what you believe and where you are unsure. Agents
caught real errors in briefs on every one of these: a stale claim about which
documents needed correcting, a wrong baseline count, and an entire premise —
that a step's deliverable should reach the Judge through git — which the owner
then rejected outright.

## Reading what comes back

**The report is not the work.** Read the diff. An agent's summary of its own
change is written by the thing with an interest in it being right, and a summary
that says "handled" is not a change you have seen.

This is not hypothetical: a gaming flag was relayed to the owner as a Drone
cheating, and reading the diff showed both flags were false positives and the
Drone had done nothing wrong.

**Verify a claim before repeating it.** Particularly a claim about what exists.
Check the file. `grep` is cheap and a wrong claim in a merge message or an issue
survives.

**Take the corrections.** When an agent's finding contradicts your brief, it is
usually right — it has just read the code and you were working from memory.

## The merge bar

All of it, every time:

| | |
|---|---|
| The gating check passes | `cargo nextest run --workspace --exclude acceptance` |
| The acceptance tests pass | separately; a milestone's own claim may be red on purpose while it is in flight |
| Both halves build | Bridge too, if it was touched |
| `verify-foundations` is no worse | it is red on purpose — read the delta, never chase it green |
| `verify-docs` is green | a stale `docs/OPEN.md` fails it |
| You have read the diff | not the report |

**Then give the worktree back** — see `agent-worktrees`. At the merge, not later.

**Restart Fleet when the protocol moves**, and after a store migration. A running
Fleet is a stale binary the moment you merge, and a major bump means Bridge
refuses to connect until it is rebuilt.

## What you decide, and what you bring the owner

**Decide** anything the code or an existing decision settles. Read the record
first — a decision already made and forgotten is the most common thing to get
wrong.

**Stop and ask** when two readings are both defensible, when a document's promise
would change, or when there is a cost trade that is the owner's. Unsupervised,
the safe failure is a stalled queue, not a wrong merge.

Use `AskUserQuestion` with concrete options and a recommendation first. A question
buried in prose is a question that gets missed.

**Do not file an issue for a problem the owner has not seen.** Bring it, work it
out, then file what was decided. See `armada-bug`.

## If you say you are watching something, arm something

A sentence about intending to check back is not a mechanism. When a Job or a
build is running and you have told the owner you will report on it, use `Monitor`
with a poll that emits on **every** terminal state — not just the good ones.

Silence looks identical to still-running. A filter that matches only success stays
quiet through a crash, and the owner is left believing something is being watched
when nothing is.

## What it looks like when it works

The Focus run: eight issues, five agents in flight at once at the peak, every
merge green, the milestone's acceptance test written first and passing last. Then
a real Job dispatched through the result — three Drones, one per step, seven and
a half minutes, no intervention.

The thing that made it work was not speed. It was that every question was settled
before an agent met it.
