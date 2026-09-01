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

The parent survives its own dispatch: it waits on its children and reports. So
a parent that holds a working slot while it waits holds one its children need,
and **no amount of waiting frees it** — the thing being waited on cannot start.
That is a deadlock rather than a performance concern, and it is why `#50` was a
hard dependency: it cannot ship in a reduced form where the parent merely waits
longer.

**`#50` moved the shape without removing it.** The single `Option<Working>` is
a roster now, bounded provisionally at two — which a parent and one child fill
between them. So the answer is not a larger bound; it is that a waiting parent
holds nothing. See *What waiting costs* below.

## Asking is a requirement, not a refinement

A Drone that does not know has two options today, and here neither one works.

| Option | What it does | Why it fails here |
|---|---|---|
| Escalate | Stops the Job; the worktree and port span are held until a person moves it | Decomposition is decision-dense, so the first ambiguity ends the run |
| Guess | Nothing prevents it | The output is Jobs that run and spend, not a bad verdict |

**Ask is built, and it is not a status.** `ask_question` is a Drone tool in
every toolbelt and `answer_question` is the Bridge command that answers it; the question
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

**A question reaches the Board, and it is the one tab rule that is not a
lifecycle row.** `who_is_acting` on `running` is `Drone`, so the registry cannot
place a Job that is waiting on a person; the row carries `asking` and
`docs/concepts/job-board.md` is the rule. Without it a question on a Job nobody
had open would be invisible.

**What it still costs is a slot.** A waiting Drone occupies a place under
`#50`'s bound and nothing gives it back, so a question left unanswered overnight
is a fraction of the fleet idle. That is filed rather than fixed.

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

## The shape is a loop, and the slate is not written at the start

Owner, 31 Aug 2026. **A milestone plan is a hypothesis**, so dispatching the
whole slate before anything runs briefs the later waves against what was
believed at the beginning.

That is not a worry, it is what happened. Building Throughput by hand on
31 Aug: `#50` landing rewrote the two briefs after it, and `#47`'s entire
premise turned out to be false once `write_targets` was known to be null at
every gate. A slate written at the start would have dispatched `#47` against an
impossible specification and paid a Drone to find out.

So the workflow is `plan` -> `dispatch` -> `assess` -> back to `plan`, or
forward. **What closes the loop is `verdict_routing`, and what bounds it is
`iteration_cap`** — both in the designed schema, both refused today as
deferred. Carrying them changes the workflow language for every workflow rather
than for this one, which is why it is its own piece of work.

**How often a person approves is a property of the milestone, not of the
system.** It is set when they approve it: a low-risk milestone runs wave to
wave on its own, a risky one stops each round. The gate is therefore one that
is *read* rather than one that is fixed in a definition — which is also the
shape `#206` needs, where a unanimous confident Judge panel clears a plan and
only a flagged one reaches a person.

## What is built, and what it is waiting for

**Everything below the workflow exists.** The tool, the grant that withholds
it, the refusals, the constructor, the wait and the return from it. What does
not exist is a workflow that uses any of it.

| Built | Where |
|---|---|
| `dispatch_job`, and `after` naming siblings only | `crates/ipc/src/mcp/dispatch.rs`, `crates/fleet/src/sub_dispatch.rs` |
| The grant, withheld off the dispatching step | `Grant::DispatchAJob`, `fleet::spawning`'s `toolbelt(job, step)` |
| Depth 1 | `Dispatching::at`, built from `Origin::top_level` |
| `create_sub_dispatched` reached | `crates/fleet/src/sub_dispatch.rs`, its only call site |
| The parent standing down and coming back | `running -> queued`, `fleet::admitting`, `fleet::readmitting` |

**No shipped workflow sets `may_dispatch_jobs`**, and
`crates/config/tests/shipped.rs` asserts that none does. The step key is
carried by the parser and by the frozen record, so a definition that wanted the
grant could have it; the definition that will want it is the loop above.

## What was decided, and how each one is held

A decision held by a convention is one that gets re-opened by accident, so what
holds each is named beside it.

| Decided | How it is held |
|---|---|
| **Recursion is refused outright — depth 1** | `Dispatching::at` is built from `Origin::top_level`, which answers `None` for a sub-dispatched Job. There is no constructor that reaches a grandchild, so there is no bound to keep in step with anything. It matters more under a loop, where rounds repeat |
| **The two routes do not converge** | Written down in `concepts/fleet.md` beside the approval rule. One route is the gate and the other is the exemption from it; a shared path would put the exemption one refactor away from the rule |
| **A Drone cannot name the parent, or sequence a stranger** | The tool has no parameter for a parent and refuses `parent_id` by name; an `after` id is looked up in this parent's own children and nowhere else |

## What waiting costs, and who gives up the slot

**A parent that has dispatched gives its Drone back.** The dispatching step
advances, the Drone stands down, and the Job goes to `queued` — a new edge,
`running -> queued`, recorded in `job-transitions.toml` for this and nothing
else. Admission holds it there until every child is terminal, and
`crate::readmitting` then puts a fresh Drone on the step after the dispatch,
with every child's outcome in its opening brief.

**A parent that kept its slot would be the deadlock `#50` just removed.** The
bound is provisionally two: a parent plus one child fills it, and the parent is
waiting for work that cannot start. No amount of waiting frees it.

**The wait belongs to the step that dispatched, not to the step after it.**
Whatever follows a step that created Jobs is work about those Jobs, so nothing
downstream has to be marked — which is what makes the wait survive the linear
shape becoming a loop.

**Terminal, not successful.** It waits for children to *stop*, not to succeed,
because a child that failed is the thing the next step is most needed for. That
is deliberately weaker than the dependency-edge rule admission uses between
peers, which requires an upstream to have landed.

## What is still not decided

| Undecided | What turns on it |
|---|---|
| The loop keys — `verdict_routing`, `iteration_cap`, `structure: loop` | The shape above cannot be written until they are carried, and carrying them changes the language every workflow is written in |
| What a spent budget does to a parent with children queued | `#51`. A cap that stops the parent leaves children queued against an approval whose author is gone; a cap that stops the children leaves a parent waiting for Jobs that will not run |
| What Fleet does about a child that failed | The brief names it and Fleet does nothing else. Whether the parent's own step should fail, and whether a person is asked, is a person's call |

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
