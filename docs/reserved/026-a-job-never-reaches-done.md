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

The suspicion — **not proved, and it should be proved before it is fixed** — is that answering
the entry lets the step run again, the re-run raises a new question under a new attempt number,
and the gate then waits on the newer id. The approval is always one attempt behind the question.
If that is right, the terminal step can never settle, because settling requires an approval that
arrives without the step running again first.

**A cheap experiment settles it**: answer the pending entry, then tick *without* letting a Drone
exchange run in between, and see whether the gate reaches `Finish`.

## Why this was invisible

Until 2026-08-16 no Job had ever reached its last step. Each earlier blocker hid this one — a
posture that denied the Drone its tools, an MCP server that was never attached
([`024`](024-the-relay-does-not-fire.md) is the third), and an `artifact_exists` that could not
match its own pattern. A terminal-state bug needs a Job at the terminal state.

## Also recorded

`armada fleet answer` refuses an 8-character id prefix with *"give more of the id"* while
`fleet inbox` prints exactly 8 characters in its `ID` column. The verb refuses what the listing
shows. Small, and it cost two cycles of the loop above.
