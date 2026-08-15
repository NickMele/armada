---
id: 012
title: A Drone's progress through its workflow
status: BUILT
module: fleet
raised: real use, 2026-08-15
---

# 012 — A Drone's progress through its workflow

> **Built.** `Job.transitions` on disk, `fleet.report` extended with `event:`, a `STEP` column
> on the Bridge, and a `GATED` row plus a transition history in the detail pane. What follows is
> the design, recorded because the two decisions a later change is most likely to get wrong are
> *why this is allowed to exist at all* and *who is allowed to write the word `completed`*.

**The complaint this exists to fix.** A Job record stored the current step name and nothing
else — a single scalar, `"step": "plan"`. No history, no timing, no per-step attempt count. A
Job that had been on `implement` for forty minutes and a Job that entered it thirty seconds ago
were the same row on the Bridge, and the difference is the whole of what you want to know.

**What was asked for**, in the words it was asked in: *"It would be sick if in the bridge, I
could see the step that the drone was on in the workflow and how long they've been on that
specific step … as drones work through a set of work, they must report back the status. It
doesn't have to be in real time, but when they start a step in the workflow and when they have
completed it, or if it has failed or if they're starting over. And we could see the number of
iterations that they have done."*

And, sharpening it a day later: *"Doesn't each step in the workflow have tasks that it needs to
complete to verify and move on to the next step? … So couldn't we use those kind of as the gate
to say whether or not it is done? And not just rely on a judgment from the drone."*

## Why this does not break the ban on a progress column

[`PHASES.md`](../PHASES.md) §9.1 F2 and the Bridge's own doc comment say it plainly:

> There is no progress column, deliberately. Nothing emits percent-complete, and a bar computed
> from a turn count is a guess drawn as a measurement.

That rule is intact, and the distinction is the reason this feature was allowed.

**A step transition is not a guess — it is a fact somebody recorded.** *"Entered `implement` at
14:02"* is measured, exactly as spend and turn count already are. What stays banned is inferring
*how far through the work is* from it. A workflow with five steps sitting on step three is not
"60% done" and nothing may ever say so.

So the whole surface obeys one sentence: **report what happened, what gated it, and when — never
how much is left.** No percentage, no bar, no estimated completion, and no step *index*, which is
the percentage in a different notation.

## Who writes which word

The user's second question is the load-bearing one, and it changed the design. A Drone does not
get to say a step is done. **Completion is the step's `verify: { must: <predicate> }` holding**,
and [`workflow.rs`](../../crates/core/src/fleet/workflow.rs) already states the rule this
enforces:

> A step advances when its predicate holds *and* the verdict carries evidence an external
> command produced — an agent asserting that tests pass is not evidence, and an
> `armada manifest check` exit code is.

That is the three-layer sandwich ([`PLAN.md`](../PLAN.md) §5) applied to a step: Armada reports
facts, an agent authors, **Armada verifies**. So the five words split by author:

| word | written by | means |
|---|---|---|
| `entered` | the Drone, via `fleet.report` | an attempt at the step began |
| `restarted` | **derived** by Armada | the same, on a step already attempted |
| `attempted` | the Drone, via `fleet.report` | *"I believe I am finished"* — an assertion |
| `completed` | the gate, via `fleet.verdict` | the predicate held, with the evidence it rested on |
| `failed` | the gate, via `fleet.verdict` | the predicate did not hold |

**The split is structural, not a sentence in a prompt.** `StepEvent::is_a_drones_to_report` is
what `fleet.report` checks, and a Drone writing `completed` gets a refusal that names
`fleet.verdict` and the word *evidence* — the same shape as `fleet.spawn` being *absent* from
the Drone's toolbelt rather than filtered out of it.

**`attempted` and `completed` are two words on purpose.** A Drone that says it is done and a
check that says otherwise must be distinguishable in what is stored, afterwards, by somebody
reading the record cold. A detail pane whose last two rows are `ATTEMPTED` then `FAILED` is a
Drone that thought it was finished and a gate that disagreed four minutes later — and that is
unrecoverable from any record that stores one word for both.

**`restarted` is derived rather than reported.** Armada already holds the fact that decides it:
whether this step has been entered before. A word the Drone chooses is a word the Drone can
choose wrongly, so it reports `entered` both times and the record makes the second one a
restart. The `fleet.report` envelope says which word it became, so the Drone finds out it is
going round again.

**`BLOCKED` and `NEEDS_HUMAN` write no boundary at all.** Neither is the predicate holding or
not — the step is still open, the inbox says why, and the attempt that was under way is still
under way. Recording `failed` for them would report a gate that never ran.

## What is on disk

`Job.transitions`, append-only, in the Job record — for the reason the inbox is on disk: it has
to survive a crash, **including the crash being the thing recorded**. A history re-derived from
the transcript would be gone the moment `armada fleet kill` took the worktree.

Each entry carries when, which step, what happened, which attempt it belongs to, and — on the
two events a gate writes — a `Gate` holding the predicate, what the predicate named, and the
evidence. Additive and defaulted, so `schema_version` stays 1 and every record written before
the field existed still parses.

**The predicate is `Option`, and the absence is recorded rather than defaulted.** A workflow
lives in the guild, and a guild can be absent, half-synced or renamed; answering `always` for a
step nobody could look up would invent the one fact the record exists to carry.

## Nothing polls

*"It doesn't have to be in real time"* — so a report at each boundary is enough, and there is no
polling to add. **The probe never interrupts a Drone** ([`PLAN.md`](../PLAN.md) §15.2), and
nothing here opens a second channel that would: recording a boundary is a read of the index, an
append and a write, and a test asserts that it starts no subprocess.

## Two counts, kept apart

The detail view used to read `implement, attempt 2 of 15`. That paired a **per-step** attempt
count with the **Job-wide** iteration ceiling, which counts turns across every step — two
different quantities in one phrase, answering neither question. It now reads `implement, attempt
2`, and the ceiling stays on the `budget` row as `4 of 15 turns`, beside the thing it bounds.

The per-step count is bumped **when an attempt begins**, by the boundary that begins it.
`fleet.verdict` still counts one for a Drone that never reported entering — the count is what a
ceiling is enforced against, and a Drone that forgets to report must not earn unlimited retries
— but it checks `attempt_open` first, because counting at both ends would halve the rope a
workflow declared.

## What the two surfaces show, and the trade

**The Bridge gets `STEP`: `implement 12m`.** One column rather than two, and the width for it
comes out of the flexible `TASK` column — which is the trade, stated. At eighty columns the
table went from 68 to 80 and `TASK` gave up three characters; the task is already a truncation
of a sentence the detail pane carries whole, and the step is the fact no other column can be
read for. `fleet ls`'s `DETAIL` is not a substitute: it folds the step together with an open
inbox body, so it goes blank on exactly the rows somebody is looking at.

**The duration is omitted rather than zeroed when nothing measured it.** It is a subtraction
from a boundary a Drone reported crossing; a Drone that never reported one leaves no boundary,
and `implement 0s` would be a measurement nobody took — the same reason `ls` draws a dash
instead of a zero.

**The predicate did not fit on the Bridge and is not there.** A `GATE` column would have cost
another fourteen columns on a table already at eighty. So the Bridge answers *which step, how
long*, and the detail pane answers *gated on what, with what evidence* — which is where the room
is.

**`human_approves` did not become a second signal.** It is the one predicate whose answer is
yours, and a step waiting on it is the same *needs you* the inbox raises and the `NEEDS YOU`
column already draws ([`PLAN.md`](../PLAN.md) §15.4). The detail pane names it as the reason —
*"approval advances on human_approves, which is yours to answer"* — and nothing anywhere raises
it a second time in different words.

**`failing_test_exists` shows what it named.** Its own comment gives the reason: *"Without it a
Drone 'fixes' a bug it never reproduced and closes green."* A pane that drew the predicate
without the test would be hiding the half that makes it a gate, so the test and the artifact
travel beside the word.
