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

**Ask is built, and it is not a status.** `ask_question` is the Drone's fourth
tool and `answer_question` is the Bridge command that answers it; the question
rides beside the state on the working slot, crosses on `get_job` as `asking`,
and moves as `job.asking`. A step whose Drone is waiting is still `running`, for
the reason `crates/ipc/operations.toml` gives about a Judge call in flight — a
seventh step state or a twelfth Job status would be a variant the other side
matches on, which is a major bump and Bridge falling back to the /v0 lifeboat.

**The answer is one of the labels the Drone itself offered.** Two to four of
them, each saying what it commits to, and there is no field for prose anywhere
on the path — which is what makes this an event on a Job rather than the
conversation `scope.md` rejected. Redirect stays the one route a person's own
words reach a Drone by.

Neither vigil counts a waiting Drone as a stopped one: `crates/fleet/src/silence.rs`
and `crates/fleet/src/converging.rs` decline on it exactly as they decline on
evidence sitting at the gate.

**What it still costs is a slot.** A waiting Drone occupies a place under
`#50`'s bound and nothing gives it back, so a question left unanswered
overnight is a fraction of the fleet idle. And a question does not reach the
Board: `who_is_acting` is `Drone` on a `running` Job, so a Job waiting on an
answer sits under Running rather than Needs you, and a question on a Job nobody
has open is invisible.

## The tracker is an adapter for Bridge, and this workflow does not wait on it

**This workflow does not depend on `#91`.** The argument below settles what `#91`
is when it is built; it was read for a while as settling that this cannot start
until it is, and that was wrong. Nothing in this capability fetches a ticket:
its Drone reads the epic the way it reads anything else, through tools it calls
inside its own session.

**Where the adapter is necessary is Bridge.** MCP tools exist only inside a
running Drone, and there is no Drone when a person is looking at a ticket list
deciding whether to dispatch one. Bridge talks to Fleet and nothing else, so
tickets *in Bridge* require Fleet itself to be able to read tickets. No MCP
arrangement substitutes for that, which is what settles `#91` as a Fleet-side
adapter — and the credential stays in Fleet rather than needing `#65`, which the
tracker's own stdio server would have.

**A Drone is the case the MCP route does serve.** That is why one fetch serving
two consumers is a saving rather than a prerequisite: when `#91` exists this
workflow's Drone can read the same tickets through it, on its own timing, in the
shape `run_checks` already has. Until then it reads them itself, and the only
thing missing is a person browsing tickets in Bridge — which is `#91`'s own
feature and not this one's.

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
