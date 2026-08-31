---
capability: epic-as-one-job
issue: 215
milestone: Throughput
---

# One approval dispatches a milestone's work

A person approves one Job that names a milestone. That Job's Drone reads the
epic, decides the split, and dispatches the rest as Jobs — recorded, gated,
judged and merged without a further approval per piece.

**The approval question is already answered in the model.**
`Job::create_sub_dispatched` in `crates/core-model/src/job/record.rs` enters at
`queued` rather than at `awaiting_approval`, because a child is already approved
as part of its parent. `Origin::SubDispatched` and `DispatchOrigin` sit in
`crates/core-model/src/job/fields.rs`, `DependencyEdge` rides on `ProposeJob` in
`crates/ipc/src/job.rs`, and the store reads all of it back. Nothing in `fleet`
or `api` calls any of it.

## The medium is the difference, not the person

`scope.md` records that orchestrator agents with sub agents was abandoned
because having a conversation was not the tool that was wanted, and warns that a
design reaching for "ask the agent" is reaching for that attempt again.

**The distinction is not whether a person is involved — it is whether a
conversation is the medium.** The rejected shape put a conversation in the
middle and nothing was a Job: nothing was recorded, nothing was gated, nothing
was judged, and everything stalled when somebody stopped typing.

Here the output is Jobs. A question is an event on a Job — asked once, answered
once, recorded beside the verdicts — and it reaches a person through the
surfaces every other Job's events already reach them through.

## A waiting parent is a deadlock, not a slowness

The parent survives its own dispatch: it waits on its children and reports.
Fleet's working slot is a single `Option<Working>`, threaded through
`crates/fleet/src/dispatch.rs` and `crates/fleet/src/spawning.rs`, so a waiting
parent holds the only slot its children need.

**No amount of waiting frees it**, because the thing being waited on cannot
start. That is a deadlock, and it is why `#50` is a hard dependency rather than
a performance concern — this cannot ship in a reduced form where the parent
merely waits longer.

## Asking is a requirement, not a refinement

A Drone that does not know has two options today, and here neither one works.

| Option | What it does | Why it fails here |
|---|---|---|
| Escalate | Stops the Job; the worktree and port span are held until a person moves it | Decomposition is decision-dense, so the first ambiguity ends the run |
| Guess | Nothing prevents it | The output is Jobs that run and spend, not a bad verdict |

**Ask does not exist.** A Drone has three tools — `submit_evidence`,
`declare_scope` and `run_checks`, dispatched in `crates/api/src/mcp.rs`.
`fleet_inbox` and `fleet_answer` are Helm's, and there is nothing like them for
a Drone.

The delivery half is built: Redirect already injects a note into a live session.
What is missing is the other direction, and a status meaning *waiting on an
answer* rather than *stopped and needing a person to restart me*.

## The tracker is an adapter, not the tracker's own MCP server

**MCP tools exist only inside a running Drone, and there is no Drone when a
person is deciding whether to dispatch one.** Bridge talks to Fleet and nothing
else, so tickets in Bridge require Fleet itself to be able to read tickets. No
MCP arrangement substitutes for that, which is what settles `#91` as a
Fleet-side adapter.

The adapter also removes a dependency the MCP route adds. A tracker's own server
needs its token in the *Drone's* environment, which is `#65`; a Fleet-side
adapter keeps the credential in Fleet, which is what
`docs/contracts/adapters.md` already requires.

One fetch serves two consumers. Bridge lists tickets for a person, and this
workflow's Drone reads the same tickets through tools it calls on its own
timing — the shape `run_checks` already has, where the Drone decides when and
Fleet decides how.

**The adapter fetches; the Drone interprets.** Dependencies between issues are
prose rather than a typed relation, so inferring them is the Drone's work and
belongs in the plan it submits, where it can be read and refused.

## What one approval now authorises

This is the first tool whose effect is *other Jobs existing*, and one approval
becomes several Drones' worth of spend. `#51` is load-bearing from the moment
this exists rather than later.

The plan is written as a file in the Job's worktree, under `.armada/artifacts/`,
which is what makes the split reviewable before any child runs.

## What is not decided

Named here rather than chosen, because each is a person's call.

| Undecided | What turns on it |
|---|---|
| The gate on the dispatching step — Judge, person, or both | It must be settled before the dispatch tool is built |
| How deep recursion may go | A sub-dispatched Job can run this same workflow, and nothing bounds it |
| Whether this and `propose_from_request` converge | That route emits top-level Jobs each approved individually; these are pre-approved children |

## What it depends on

- `concepts/job.md` — Origin, and the approval and readiness axes a
  sub-dispatched Job inherits from its parent.
- `concepts/fleet.md` — what dispatch costs and what the working slot holds.
- `concepts/drone.md` — the tools a Drone has, and why it is not trusted to
  manage its own state.
- `concepts/workflow.md` — what a step is, and what a step that waits on other
  Jobs would be waiting on.
- `concepts/job-proposer.md` — the existing route from a request to a list of
  Jobs.
