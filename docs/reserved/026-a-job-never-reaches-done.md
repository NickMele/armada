---
id: 026
title: A Job never reaches `DONE`
status: BUG
module: fleet
raised: driving a `design` Job to its last step, 2026-08-16
---

# 026 — A Job never reaches `DONE`

**`DONE` is a legal Job state, is rendered, is matched on in five places — and no Job has ever
been in it.** The same shape as [`022`](022-docker-hygiene.md)'s finding about the `owned` table's
unwritten `kind` values, one layer up.

## The measurement

A `design` Job was driven through every step on 2026-08-16. It reached `hand-over`, the workflow's
last step, and its own detail says so:

```text
STATE=PAUSED  step=hand-over
`hand-over` finished its last step and the `design` workflow ends with you
```

It then stayed there through four `armada fleet answer` / `armada fleet tick` cycles, spending
$0.52, and halted on its iterations ceiling. Each cycle spawned a fresh Drone exchange, raised a
**new** inbox entry, and asked again.

## What is and is not wrong

**The machinery to finish exists and is correct.** `release_on_finish`
(`crates/helm/src/verbs/fleet.rs`) writes `JobState::Done`, keeps the branch, and removes the
worktree only when git says there is nothing to lose. It is called on `Next::Finish`. Nothing is
missing at that end.

**What no Job reaches is `Next::Finish`.** The terminal step gates on `human_approves`, and the
gate looks for *"the entry this attempt asked, by id — never an open entry"*, with the attempt
number carried in `job::Pending`. That rule is right, and [`gate.rs`](../../crates/core/src/fleet/gate.rs)
argues it well: *"yes, ship it" about the second thing you were asked is not approval of the
third.*

**The cause, traced in code rather than guessed.** `armada fleet answer`
(`crates/helm/src/verbs/fleet.rs`) does two things: it records the answer with `inbox::answer`,
and then it **immediately resumes the Drone** — `start_drone` with `resume_argv`, and
`record.state = JobState::Running`. That is right for the question it was built for, where a Drone
is stuck and needs input to carry on. It is wrong for a gate.

`human_approves` is not a Drone's question. It is the *workflow* asking whether the step is
accepted, and the answer belongs to the gate. What happens instead:

1. The gate halts and asks. `pending` remembers that entry's id for this attempt.
2. `fleet answer` records the answer — **and starts a Drone exchange**.
3. That Drone does more work and, per [`BRIEF`], asks its own question. A **new** entry is raised
   and `pending` now names it.
4. The next `tick` reads `pending`, finds the new entry unanswered, and halts to ask again.

The approval is recorded every time and read never, because a fresh question always overtakes it.
The Job cannot settle its last step, so `Next::Finish` is unreachable and `release_on_finish`
never runs.

**The earlier suspicion recorded here was wrong**, and is kept because a rejected diagnosis with
its reason attached does not get proposed again: it guessed the attempt counter had advanced past
the pending entry. It has not. `job::step_failures` counts only `StepEvent::Failed`, and the only
writer of that is `Verdict::Failed` — asking a person is not a failure and never moves the
attempt.

**What has to be decided, and it is not a one-line fix.** Answering a gate and answering a Drone
are two different acts that share one verb. Either `fleet answer` learns the difference — settle
the gate and tick, rather than resume — or `human_approves` stops routing through the inbox that
Drones also write to. The first is smaller; the second is the one that stops the two kinds of
question sharing a queue at all.

## Why this was invisible

Until 2026-08-16 no Job had ever reached its last step. Each earlier blocker hid this one — a
posture that denied the Drone its tools, an MCP server that was never attached
([`024`](024-the-relay-does-not-fire.md) is the third), and an `artifact_exists` that could not
match its own pattern. A terminal-state bug needs a Job at the terminal state.

## Also recorded

`armada fleet answer` refuses an 8-character id prefix with *"give more of the id"* while
`fleet inbox` prints exactly 8 characters in its `ID` column. The verb refuses what the listing
shows. Small, and it cost two cycles of the loop above.

## Established 2026-08-16, by driving a Job rather than reading

**The `fleet answer` half is fixed and merged.** Answering a gate now settles it
rather than resuming the Drone, and a real Job proved it: its transitions record
`explore`, `articulate` and `hand-over` all `completed`, where before every
answer produced a fresh question and the approval was never read.

Three things had to change together, and only the first was the one predicted
above — the other two were found by running it:

| Change | Why it was needed |
|---|---|
| Settle the gate rather than resume the Drone | the diagnosis above |
| Leave `PAUSED` before ticking | `advance::attention` reads `PAUSED` as *"it is waiting on you"* and declines to gate. The old path reached `RUNNING` only as a side effect of resuming — so removing the resume removed the thing that made the tick work |
| Pass the Job's name to `tick`, not the caller's handle | this verb accepts an entry id too, so `tick` was handed an inbox id and refused with `no Job called a058890c` — **after** the answer had been written, naming the wrong noun |

**What still stops a Job finishing, and it is now narrow.** After the final step
records `completed`, the Job rests `PAUSED` and `tick` will not gate a `PAUSED`
Job. `Next::Finish` is unreachable, so `release_on_finish` — which writes
`DONE`, keeps the branch and removes a clean worktree — still never runs.

The remaining question is what should move a Job out of `PAUSED` when its last
step has completed and nothing is open. Answering is no longer it: the entry is
closed and there is nothing left to answer. **Either the pass that completes a
final step must go on to `Finish` in the same tick, or `PAUSED` with no open
entry must not read as "waiting on you".** The second is the more general
statement and probably the right one — `PAUSED` currently means two things, and
only one of them wants a person.

**Do not fix this by making `tick` gate every `PAUSED` Job.** That would gate
Jobs genuinely waiting on a person, which is the case the state exists for and
the reason the rule reads the way it does.
