# Dispatch a milestone

**What it is:** Approving one Job that names a whole milestone, and staying in the middle of it without approving each piece.

Design fidelity: not set. Analysis: partial. UI/UX design: not started.

---

**Trigger:** A milestone or epic exists and you want its work done, rather than one of its issues.

**Concepts touched:** Job, Fleet, Drone, Job Board, Workflow.

**Milestone:** Throughput.

**Capability:** [`epic-as-one-job.md`](../capabilities/epic-as-one-job.md) — the reasoning, the dependencies, and what is not decided. This document covers only what a person does.

## Flow

1. **Pick the milestone.** The tracker's tickets are listed in Bridge, and one of them is the epic.
2. **Approve one Job.** It is the same approval card as Journey 1, and it is the only approval in this journey.
3. **Watch it decompose.** Its Drone reads the epic, writes a plan, and dispatches children that appear on the Board already queued.
4. **Answer the question it asks.** It arrives where a Job waiting on a person arrives, and the answer goes back into the running Drone.
5. **Read one report.** The parent converges when its children are done and says what landed.

## What I approve, and what I do not

| | |
|---|---|
| I approve | Once, the parent Job — its brief, its workflow, its acceptance criteria |
| I do not approve | Each child. A child inherits its parent's approval and enters at `queued` |
| I still see | Every child on the Board, tagged by origin, gated and judged like any other Job |
| I can still act | Redirect, Restart Step, Redispatch and Kill, on the parent or on any child |

The plan is a file in the parent's worktree before any child runs, so the split is readable and refusable while it is still only a plan.

## The question is an event, not a conversation

A Drone that does not know asks, and the ask is recorded on the Job beside its verdicts. Nothing waits on a chat window being open, and nothing is lost when it is closed.

That is the whole difference from the orchestrator shape `docs/scope.md` records as abandoned, and the capability document is where it is argued.

## What does not exist yet

Everything in the flow above except the approval card. Named here so the journey is not read as a description of something that runs.

| Missing | Consequence for this flow |
|---|---|
| A tool for a Drone to dispatch a Job | Step 3 has no mechanism |
| A tool for a Drone to ask a person | Step 4 does not happen; a Drone escalates or guesses |
| A status meaning waiting on an answer | A Drone that asks looks the same as one that stopped |
| Fleet calling `create_sub_dispatched` | The model has the constructor and nothing reaches it |
| Several Jobs at once, `#50` | A waiting parent holds the only working slot its children need |
| The tracker listed in Bridge, `#91` | Step 1 is done by reading the tracker yourself |

## What it must not become

**A queue of children to click through.** That is Journey 1 run several times, and removing it is the point of this journey.

**A chat.** There is one question and one structured answer, on a Job, and no thread.

## What this journey does not yet say

What a person sees when one child fails while the parent is still waiting. Not decided, and not chosen here.

The capability document names the rest — the gate on the dispatching step, how deep recursion may go, and whether this converges with the existing route from a request to a list of Jobs.

## Related

Journey 1 — Dispatch a Job. Its approval card is the one used here, and its strictly-one-by-one rule is what a sub-dispatched child is exempt from.

Read a failed Job, and Read the work and merge by hand — what a single child looks like at the end.

Monitor Active Work — the Board a decomposing Job fills.

This journey has no number because the design project has not drawn it. A number in a filename here means a `Journey N` drawing exists to match it; inventing one would assert a correspondence that does not.
