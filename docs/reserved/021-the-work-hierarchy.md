---
id: 021
title: The work hierarchy
status: RESERVED
module: cross-cutting
raised: a design session on the ontology, 2026-08-15
---

# The work hierarchy

**What this settles.** Three naming decisions, taken after the user said *"something is not quite
right about our hierarchy here"* and drew it out himself. `glossary.md` is already updated; this
records the reasoning and what still has to change in code and docs.

## The complaint

> *"Fleet → Job → Drone. In my head the fleet is a set of drones that are executing on Jobs. We
> have this problem with reports/failures/tasks feeling disconnected and I think this might be
> part of it. All of those ultimately lead to a job. Task / Reports / Failures → Jobs → Fleet
> dispatches a drone to tackle a Job → … Something here manages the Job + Drone + Manifest +
> Workflow required to get the job done. Is that Fleet?"*

He had found two gaps, and diagnosing them explained a third thing he had complained about
separately.

## 1 · Fleet is a collection, not a manager

**The defect: one word, four meanings.** `Fleet` was simultaneously a crate (`armada-fleet`), a
layer in `ARCHITECTURE.md` §1.5, a CLI noun (`armada fleet ls`), and — to any English speaker — a
collective. `glossary.md` said *"the module that mints Jobs and runs Drones"*, which is a
description of the **crate**, not the concept.

That is the same collision that kept `helm` off `PATH`, and `glossary.md` exists *"specifically so
two modules cannot invent two names for one concept."* It had one name for four.

**Decided.** Fleet is **every Job on this machine, and the Drones executing them.** `ls`, `reap`
and `inbox` are operations on a set; `spawn` adds to it. The crate keeps its name — [`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.5's layering
is about dependency direction and does not require the user-facing word to mean the same thing.

## 2 · The Job is the state machine, and nothing drives it

**The gap.** The thing that evaluates a step's gate and decides advance / retry / stop had no noun.
It is `armada fleet tick`, a verb whose logic lives in `crates/core/src/fleet/advance.rs` and
`gate.rs`.

**Decided: it needs no noun, because it is not a separate thing.** A Job already carries its
`workflow`, its `step`, its `budget`, its `spend`, its `verdict`, its `worktree`, its `branch` and
its `port_block`. **That is a state machine's entire state.** So `tick` is not a manager acting
*on* a Job — it is the Job taking one transition, and `tick` stays a verb exactly as `check` is.

**`Pilot` was proposed and tested, then rejected** — his proposal, and he withdrew it himself once
the cost was visible. The metaphor was exact: a harbour pilot boards a ship she does not own,
guides it through the part needing local knowledge, and steps off carrying no cargo. It failed on
one point: **a Pilot would hold no state.** Everything it would own, the Job already owns, so the
word would name a function rather than a thing. Recorded in `glossary.md`'s *"Words deliberately
not used"* table with that reasoning, alongside `Run` and `Voyage`, which fail the same way and
additionally collide with Manifest's check runs.

## 3 · `Signal` — the collective that was missing

**The gap, and it explains an earlier complaint.** Five things now share one store and one id
space — a **task** he intends, a **report** he filed, a **failure** Armada noticed, an **untried**
verb, and a question a Drone **asked** — and no word covered them. He had already complained that
this family felt like *"running in circles"*; the missing collective noun is why.

**Decided: `Signal`.** Something raised that may or may not be acted on — which is exactly true of
all five, and fits the fleet metaphor rather than sitting beside it. A signal is raised and then
ignored, dismissed, or promoted; only the last makes work.

**Why the loop stops feeling circular once it is named:** a Job that needs the user **raises a
signal**, and answering it advances the Job. Signals are both the input to work and its interrupt.
That is why one id space was right ([`001`](001-raised-items-need-identity.md)) and why it felt
strange without a word.

## The model, entire

```
Signals — task · report · failure · untried · asked
   │  one store, one id space
   ▼  promote
Job    — a state machine: workflow · step · budget · verdict
   │  a step needs an exchange
   ▼
Drone  — one exchange, then exits
   │  its Stop hook ticks the Job
   └──▶ back to the Job, which advances, retries, stops,
        or raises a signal because it needs you

Fleet  — the set of all Jobs on this machine
```

## What still has to change

| Where | What |
|---|---|
| `docs/glossary.md` | **done** — Fleet redefined, Signal added, Job's state-machine role stated, Pilot/Run/Voyage recorded as rejected |
| `docs/PLAN.md` | §14–§15 describe Fleet as an actor in places; reword to match |
| The verbs | `armada failures` / `tasks` / `untried` should read as views over signals. [`020`](020-the-tui-decided.md) §3 already decided one listing with origin as a filter — this gives that listing its name |
| `crates/core/src/failure.rs` | The record is called a failure and is now four other things. Renaming it `Signal` is the honest change; it is internal, so the cost is mechanical |
| The Bridge | `020`'s menu gains a `signals` row rather than three separate ones |

**Nothing new is built by this.** Every box in the model exists in code today. This is naming, one
glossary rewrite, and the renames above.
