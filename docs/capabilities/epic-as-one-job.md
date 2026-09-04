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

**The plan draws the split, and the drawing is what is approved.** Owner, 2 Sep
2026, on reading one produced by hand. A wave is a set of Jobs that may run at
once and an ordering between the sets, and prose asking a person to hold that in
their head is asking them to redraw it themselves before they can answer. The
plan carries a Mermaid flowchart beside the prose: a node per child Job, an edge
per reason one waits.

**Two kinds of edge, told apart, because only one of them is a fact about the
work.** A dependency edge is one Job needing what another produced —
`DependencyEdge` on `ProposeJob` already carries it. A sequencing edge is two
Jobs that would write the same paths, held apart by the plan rather than by
anything in Fleet: `#47` settled that an overlap is *surfaced, never serialised*,
so a parent's own split is the only thing separating them and a reader has to be
able to see it doing so.

**What a person approves is the graph, and approving it is the dispatch.** The
first wave leaves as Jobs on the approval, which is what makes this one approval
rather than a plan followed by a second act.

## The shape is a loop, and the slate is not written at the start

Owner, 31 Aug 2026. **A milestone plan is a hypothesis**, so dispatching the
whole slate before anything runs briefs the later waves against what was
believed at the beginning.

That is not a worry, it is what happened. Building Throughput by hand on
31 Aug: `#50` landing rewrote the two briefs after it, and `#47`'s entire
premise turned out to be false once `write_targets` was known to be null at
every gate. A slate written at the start would have dispatched `#47` against an
impossible specification and paid a Drone to find out.

So the workflow is `plan` -> `dispatch` -> `roll_up` -> back to `plan`, or
forward. **What closes the loop is `verdict_routing`, and what bounds it is
`iteration_cap`** — both carried since `#263`, and `.armada/workflows/epic.json`
is what declares them. `roll_up` is the step this page called `assess` while it
was still being drawn; the issue's name is the one that ships, because
`fleet::converging` owns the other candidate.

`iteration_cap` is five, which buys five waves. It is `design-plan.json`'s
designed cap taken for want of a better-argued number, and it is not what bounds
the spend — `#51` is.

**How often a person approves is fixed in the definition today, and that is not
where it belongs.** The owner's decision, 3 Sep 2026: he reads every plan, every
time. A Judge-first variant was rejected because it needs `#206`, which is open,
and because the first thing a Judge would wave through is eight Jobs' worth of
spend. What the shape wants is a gate that is *read* per Job rather than
declared per workflow — a low-risk milestone running wave to wave, a risky one
stopping each round — and that is `#264`'s, unbuilt. `#206` is the other half:
which plans reach a person at all, once a Judge panel can say how sure it is.

## What is built, and what it is waiting for

**The workflow exists now, and everything below it did first.** The tool, the
grant that withholds it, the refusals, the constructor, the wait and the return
from it were all built before anything used any of them;
`.armada/workflows/epic.json` is the definition that does, and
`crates/fleet/src/tests/epic.rs` drives it off disk.

**Where the gate sits is the one thing the shape above did not settle.**
`human_always` is on `plan` and not on `dispatch`, and the placement is forced
rather than chosen: an `advance_gate` is read *after* a step's Drone has
submitted, so on the dispatching step it would be a person approving a spend
that had already happened. On the step before it, approving is what advances
into the step holding the tool — which is what makes the approval the dispatch.
`crates/config/tests/shipped.rs` asserts the pair.

| Built | Where |
|---|---|
| `dispatch_job`, and `after` naming siblings only | `crates/ipc/src/mcp/dispatch.rs`, `crates/fleet/src/sub_dispatch.rs` |
| The grant, withheld off the dispatching step | `Grant::DispatchAJob`, `fleet::spawning`'s `toolbelt(job, step)` |
| Depth 1 | `Dispatching::at`, built from `Origin::top_level` |
| `create_sub_dispatched` reached | `crates/fleet/src/sub_dispatch.rs`, its only call site |
| The parent standing down and coming back | `running -> queued`, `fleet::admitting`, `fleet::readmitting` |

**Exactly one shipped workflow sets `may_dispatch_jobs`**, and
`crates/config/tests/shipped.rs` asserts that it is the only one — the
assertion used to be that *none* did, and flipping it was part of shipping this.
The grant is `epic.json`'s `dispatch` step and nothing else in
`.armada/workflows/` has it, which is the property worth holding: the step key
is carried by the parser and by the frozen record, so any definition could ask
for it, and a test is what makes a second one a deliberate act rather than a
merge nobody read.

## What was decided, and how each one is held

A decision held by a convention is one that gets re-opened by accident, so what
holds each is named beside it.

| Decided | How it is held |
|---|---|
| **Recursion is refused outright — depth 1** | `Dispatching::at` is built from `Origin::top_level`, which answers `None` for a sub-dispatched Job. There is no constructor that reaches a grandchild, so there is no bound to keep in step with anything. It matters more under a loop, where rounds repeat |
| **The two routes do not converge** | Written down in `concepts/fleet.md` beside the approval rule. One route is the gate and the other is the exemption from it; a shared path would put the exemption one refactor away from the rule |
| **A Drone cannot name the parent, or sequence a stranger** | The tool has no parameter for a parent and refuses `parent_id` by name; an `after` id is looked up in this parent's own children and nowhere else |
| **A person reads every plan before its Jobs spend** | `human_always` on `plan`, which is the step *before* the one holding the tool. A gate is read after its Drone submits, so the same key on the dispatching step would approve a spend that had happened. `crates/config/tests/shipped.rs` asserts the pair rather than either half |

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
| Whether a person reads every plan, or only a flagged one | `#264` carries the mechanism — a gate read off the Job rather than fixed in the definition — and `#206` carries the question of which plans a Judge would let through. Today the gate is declared per workflow, so every milestone stops each round whatever its risk |
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
