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

## Three things this document got wrong — corrected 2026-08-17

**The naming above stands. The change list under it did not survive contact with the code.** Each
of the three was load-bearing, so they are corrected here rather than quietly rewritten.

**1 · "Five things now share one store and one id space" is half true.** They share one id space
and **three files**. `~/.armada/failures.jsonl` holds fault, report and task; a raised entry lives
in `~/.armada/inbox.jsonl` and is projected into the same shape at read time; `untried.jsonl` is a
third file with a different record entirely. `crates/helm/src/verbs/failures.rs` says it in its own
words — *"the unification is in the reader, not the store"* — and
[`001`](001-raised-items-need-identity.md) is the document that had it right: *"One id space is the
claim `001` makes; one file is not."* Two files is also **forced** rather than accidental: Helm's
Stop hook and its monitor both read `inbox.jsonl` at a hardcoded path, so merging the stores breaks
the mechanism that makes a raised item reach anybody.

**2 · `untried` must not become a view over signals.**
[`017`](017-what-you-have-not-tried-yet.md) argues against exactly that, under the heading *"Why it
is a counter and not a fourth list of raised items"*, and its reasoning holds: an untried verb's
identity is its own name, so there is no id to act on one at a time; there is nothing to
acknowledge; and the row deletes itself the moment you type the verb, which nothing else in the
store does. It shares no field with `Entry` except a count, and folding it in would mean inventing
an id, a state and a promotion path — *"inventing the parallel list rather than avoiding one"*, in
`017`'s words. **Four origins, not five.**

**3 · The Bridge's menu has no `failures`/`tasks`/`untried` rows to collapse.** `verbs/menu.rs`'s
`MODULES` is five module rows — helm, fleet, inbox, manifest, guild — and the golden confirms it.
Those three rows are on **`armada --help`**, in its `THIS MACHINE` section
(`crates/helm/src/render/help.rs`). The Bridge has neither. The word `signals` appears in no `.rs`
file in the workspace.

**And "nothing new is built by this" was wrong.** Every *box in the model* does exist, which is what
that sentence was reaching for, but the merged listing cannot be assembled out of what ships: the
reader never sorts, one row carries two different status words depending on which listing draws it,
and the screen it belongs on cannot scroll.

## The design, decided 2026-08-17

Taken with the owner over a rendered mock-up, the same way
[`033`](033-the-command-centre-designed.md) was. Eleven decisions.

| # | Decided | Rather than |
|---|---|---|
| 1 | One listing, origin as a filter — [`020`](020-the-tui-decided.md) §3 stands | a ranked *what needs me now* queue, which stays available later |
| 2 | Keep the append-only log; fix the reader | moving signals into `manifest.db` |
| 3 | `untried` stays out — four origins | five, per the correction above |
| 4 | The verb is **`arm inbox`**, replacing `failures`, `tasks` **and `fleet inbox`** | `arm signals`, or keeping the old verbs as aliases |
| 5 | Four states: `OPEN` · `NEEDS_HUMAN` · `FIXING` · `CLEARED` | three, with `FIXING` meaning two different things |
| 6 | A dense list plus a preview pane that follows the cursor | one truncated line per row |
| 7 | `ID` first, following `033`'s `NAME → STATUS → DETAIL → TIME` | `STATUS` first, as three of four shipped surfaces do |
| 8 | Side by side above `render::WIDE`, stacked below it | one layout at every width |
| 9 | The Bridge's `INBOX` panel becomes the inbox — four origins, verbs wired | a sixth panel, or leaving it raised-only |
| 10 | The Bridge becomes a ratatui application — [`035`](035-the-bridge-becomes-a-ratatui-application.md) | keeping every box hand-composed |
| 11 | Golden coverage of the interactive surfaces lands **with** the listing; the six recorded TUI complaints are their own pass | shipping the listing onto a screen no fixture can see |

**Why the verb is `inbox` while the concept stays `Signal`.** The collective noun this document
chose is right and `glossary.md` already carries it. The **verb** is what the owner reaches for
unprompted — *"a generic `armada inbox` for all tasks, reports and failures"* — and a verb nobody
has to learn beats one that matches the glossary. `arm fleet inbox` is absorbed rather than left
beside it, because the same raised rows under a narrower verb is the *"running in circles"*
complaint reappearing one level down.

**Why storage did not move.** The argument for the log is specific to what is being recorded —
`crates/manifest/src/failures.rs`: *"the thing being recorded **is** the crash, and a store that had
to be rewritten consistently would be at its least trustworthy at the one moment it is written."*
`manifest::failures::append` returns `bool` and never errors upward, so a lock-wait must not land on
the error path. The case for SQLite in [`PLAN.md`](../PLAN.md) §4.3 is about **leases** — a
heartbeat renewed every few seconds for ten minutes — and signals are append-once. No passage
anywhere argues signals belong in the database.

## What has to change, corrected

| Where | What |
|---|---|
| `docs/glossary.md` | **done** — Fleet redefined, Signal added, Job's state-machine role stated, Pilot/Run/Voyage recorded as rejected |
| `docs/PLAN.md` | §14–§15 describe Fleet as an actor in places; reword to match |
| `crates/core/src/failure.rs` | `Entry` → `Signal`, and `State` gains `NeedsHuman`. Internal, so mechanical |
| `crates/helm/src/verbs/failures.rs` | Becomes `inbox.rs`: `Lens` grows a raised arm, and **the merged read sorts** — today it does not sort at all |
| `crates/helm/src/args.rs` | `inbox` replaces `failures` and `tasks` in `TOP_LEVEL_VERBS`; `fleet inbox` goes |
| `crates/helm/src/render/help.rs` | The three `THIS MACHINE` rows become one |
| `crates/fleet/src/inbox.rs` | `as_entry` maps an open raised entry to `NeedsHuman` rather than `Fixing` |
| `crates/core/src/failure.rs` (`Line::Promoted`) | Carries the Job's **uuid**, not its name — [`005`](005-inbox-label-not-identity.md)'s defect, reproduced here |
| The Bridge | [`035`](035-the-bridge-becomes-a-ratatui-application.md), which is the larger half |

**What is new, and it is not naming.** Four things the model does not describe and the code does not
have: the missing sort, the fourth state, a preview pane, and a screen that can scroll. The plan is
[`PLAN.md`](../../PLAN.md).
