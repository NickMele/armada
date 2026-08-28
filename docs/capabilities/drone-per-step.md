---
capability: drone-per-step
issue: 142
milestone: Focus
---

# A Drone does the step it was given, and nothing after it

A Job's Drone runs every step of its workflow in one process and one session.
When a step passes its gate, Fleet writes a line into that live session telling
it to carry on. Everything the Drone learned in the step it just finished is
still in front of it during the next one.

`scope.md` names the result in the owner's pain table: *"This one got ahead of
itself and did all of the work in one PR instead of three"* — **not yet
solved**. A Drone is asked not to do the next step's work by a sentence in its
prompt, and `concepts/drone.md` opens by saying that asking is not enough:
Drones cannot be trusted to manage their own state.

**A Drone belongs to a workflow step.** It ends when its step ends, and the next
step starts a fresh one on the same worktree. What crosses the boundary is the
record rather than the session — so whatever the next step needs is something
that was written down and checked.

## What does not change

| | |
|---|---|
| The worktree and the branch | One per Job, held for its life |
| The commit | One, at the last step |
| The Checks a step is measured against | Frozen when the Job is created |
| A Drone's toolset | Resolved once, from the same snapshot |

## The two rules

Both are constraints on the change rather than consequences of it, and each
replaces a protection that the per-Job lifetime was providing for free.

**The snapshot is taken once, at Job creation.** Fleet snapshots the rules a
Drone works under — its Commands and its Checks — when the Job is created, and
hands that same snapshot to every step's Drone. A step boundary never
re-resolves it; only a human-approved scope change re-snapshots.

Without it a Drone could weaken a Check in one step and be measured against the
weakened one in the next. A mid-Job respawn re-resolves today, and that is safe
only because the one case where it happens needs a person to approve it first. A
step boundary has no person in it.

**The intervention act is decided by where the Job stands.** Which of Redirect,
Restart Step, Redispatch and Kill applies follows from whether the Job is
mid-step or at a boundary, not from whether a process exists at that instant. A
Redirect arriving at a boundary waits and goes into the next Drone's opening
brief.

Without it the acts collapse, which `concepts/job.md` forbids: a redirect that
respawns is a restart that threw away the session. They are told apart today by
whether the Drone is alive — a signal that stops meaning anything once a Drone's
absence is the ordinary state between steps.

## What this rules out

**A commit at every step boundary.** `crates/fleet/src/landing.rs` holds the
reasoning: a per-step commit puts commits on the branch of a Job that then
failed, and uncommitted work is what makes a failed Job's branch unmistakably
not mergeable. It would also change nothing the gate sees, because
`crates/adapters/src/work_product.rs` reads the diff from the merge base to the
working directory whether or not anything is committed.

**`reference_docs` in a Drone's brief.** It is a Judge field —
`crates/core-model/domain/workflowdef-fields.toml` gives it
`concern = "Verification"`, and `contracts/agent-prompt.md` calls it the
yardstick. A Drone is never told what the Checks are.

**Loop workflows, until a step that runs twice keeps both attempts.** Every
per-step table in `crates/store/src/schema.rs` is keyed by step alone, so a
second visit erases the first — and the handoff would read evidence that no
longer exists.

## What the cost is

Unknown, and stated as unknown. Nothing is billed per token on a subscription;
`spikes/005-what-does-a-job-cost.md` records that the five-hour window is what
actually stops a fleet, and that it reports no quantity a Drone can read.

Two measurements would settle whether a Drone per step is cheaper or dearer than
one long session: what a cold start costs, and how many turns a fresh Drone
spends re-orienting. Neither exists. The claim to avoid is that either direction
is obvious.

## What it depends on

- `concepts/drone.md` — the lifetime this changes, and the frozen-versus-live
  table the first rule amends.
- `contracts/agent-prompt.md` — the six layers, and the freeze-time ordering
  whose stated justification changes when all six are re-assembled.
- `concepts/job.md` — the four intervention acts the second rule governs.
- `concepts/workflow.md` — what a step is, and what advancing one means.
