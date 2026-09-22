# Plan

**What it is:** A Job's own record of what it means to do the work — an
approach and an ordered list of tasks, recorded by one step and kept current
by the steps that follow it.

---

**Kind:** Entity. Owned by [Job](job.md), one per Job.

Designed with the owner as the Plan milestone. #893 built the record and the
Drone tools that write it, #894 put it in a Drone's brief, and #895 wired the
workflow fields. #896 draws it in job detail, #897 lets a person change it,
and #898 draws it on the Board — none of the three built yet. #1006 let any
step record it beside its own product, so all eight carried workflows keep
one.

## What it is

**One Plan per Job, not per step.** [Job](job.md) owns it the way it owns
`facts[]` and `escalations[]` — a record the Job carries, not an actor.

A Plan holds an `approach` — a paragraph — and an ordered list of Tasks.

**Tasks sit in groups, and a group is what runs.** *Not yet built.* The plan's tasks are ordered into groups; the groups run one at a time in plan order, in the Job's one worktree, and the step's Checks run at the end of each group rather than once over the whole step. A verdict then names the group that broke, and a retry re-runs only that group.

Groups exist because a parallel schedule cannot be derived. Intersecting the tasks' `scope` lists finds every edge where two tasks write one file, and finds none of the edges where one task uses what another made — four of the eight tasks on the Job the owner killed on 18 Sep were linked that way and shared no declared path. Plan order carries those edges, so running in plan order honours them; scope intersection catches a plan whose order contradicts its own paths.

| Field | On | Meaning |
| --- | --- | --- |
| `approach` | Plan | The step's own account of how it means to do the work |
| `id` | Task | Stable, assigned once, never reused |
| `title` | Task | One line |
| `note` | Task | One line for what the other fields cannot hold. Optional |
| `scope` | Task | The repository-relative paths this task touches |
| `expects` | Task | What should prove it, written by the step that plans |
| `shown` | Task | What did prove it, written by the step that does the work |
| `state` | Task | `open`, `working`, `done`, `dropped`. A fifth, `failed`, is decided and not built |
| `reason` | Task | Required when `state` is `dropped` |
| `concurrent_with` | Task | Which tasks in its group may run at the same time, declared by the planner. *Not yet built* |

**`scope` is a list because the step after the planning one reads it.** It
was prose inside `note` until `#1421`, and a step handed prose went looking
for the same files again — the Job the owner killed on 17 Sep had recorded
`packages/screens/src/overview.ts` against the task that changes it, an hour
before its next Drone grepped for the same file.

**`expects` and `shown` are two fields and are never reconciled into one.**
The planner names an artifact before the work starts; whoever does the work
finds out what actually proved it. The pair disagreeing is the fact worth
reading, so nothing collapses them — the same shape declared `scope` has
against the files a task actually touched.

**A Judge reads the pair, and refuses silence rather than difference.**
Feature, Bug and Refactor ask `the_evidence_accounts_for_itself` on the step
that follows the plan: for each task set `done`, does its `shown` demonstrate
its `expects`, or say why the work proved it another way? A task that proved
its work differently and says so passes — that is how work finds the real
seam. What is refused is a `shown` that neither matches nor accounts for
itself. Epic plans too and asks nothing, because its steps that follow the
plan are handed no `reference_docs` and a task there is a dispatched Job.

**Every change records who made it** — a step, or a person — and **every
change is an appended row**. The current list is derived; history is never
overwritten.

## Who may change it

| Who | May do |
| --- | --- |
| The step that records the plan | Record it whole, while its step runs. A retry, or a loop's return to that step, replaces the plan |
| A step declaring `follows_plan: true` | Add a task, and move one to `working`, `done` or `dropped` with a reason. A `done` task may move back; a `dropped` one stays dropped. Legal on the recording step itself, so one step may plan and keep its own tasks current |
| A person | Add a task, or drop one with a reason, from Bridge, while the Job runs |
| Any other step | Read the plan. Change nothing |

A retry of a step that follows the plan keeps task states — the work behind
them is still on the branch, so the plan does not reset with the step.

## Fleet writes a task's state, and a Drone stops claiming it

**Not yet built.** Fleet marks a task `working` when it dispatches a Drone for it and `done` when the group's Checks come back green, in place of a Drone claiming both afterwards. On the Job of 18 Sep three tasks flipped to `done` within 1.6 seconds of each other and five never entered `working` at all, which is what a self-reported state is worth.

**`failed` is the state that has nowhere to go today.** A task whose group's Checks went red is not `open`, not `working`, not `done` and not `dropped`. Fleet writes it, and the retry that re-runs the group is what clears it.

**A done task a later task edits stays done, and is flagged.** The work behind it is still on the branch, so nothing reopens it; what a person needs is to know that somebody wrote into its files afterwards, which the flag says and the state does not.

## Tasks that may run at once are declared, never inferred

**Not yet built.** The planning Drone names which tasks in a group may run at the same time. It is a declaration because it cannot be a derivation: intersecting declared paths finds the write-write edges and none of the read edges, and a planner that has just written the plan knows both.

**They share the Job's one worktree.** Two Drones writing different files in one checkout do not collide. What needs care is anything that *reads* the tree, which is the group boundary the design already has: the concurrent tasks join, then the Checks run on a still tree, then the group commits once. Tasks that ran at once cannot each have their own commit.

**The cost is an undeclared write.** Two writes to one file neither task declared are last-write-wins and silent, and `scope_diff_check` finds the drift after the fact — which is enough when tasks run in order and is not enough when they run together. Nothing yet catches it.

## A task's cost appears when its agent stops

**Turns while it runs, cost when it ends.** Cost arrives on a session's last line — see [Machine](machine.md) — so a live figure would be invented. It is the task's own agent stopping that settles it, not the group passing.

## Task state is never a gate

**Rule.** Fleet never refuses a step's submission because a task is open.
Why: while task state is a Drone's own claim, `docs/scope.md` treats
self-report as a signal and not a source of truth — and once Fleet writes the
state itself, the state says what Fleet already knows rather than adding a
second gate over it.

A step declaring `follows_plan: true` may still be judged against the plan's
task states — whether the diff does what a task set `done` claims, whether a
`dropped` task's reason holds, and whether a done task's `shown` accounts for
itself against its `expects`. Those are Judge criteria, asked of the diff and
of the Drone's own account of its evidence, never a mechanical gate on the
plan itself. `plan_recorded` is the one mechanical check touching a Plan, and
it asks only that one exists with at least `min_tasks` tasks — see
[Workflow](workflow.md).

## How a workflow declares it

A workflow names the step that records the plan one of two ways: giving it
`evidence.submitted.type: "plan"`, where the plan is the whole of the
step's product, or giving it `records_plan: true` beside another product —
Code Review's `read` keeps `read.md` and records the plan too. Either way
the step needs a `plan_recorded` mechanical check. A later step that keeps
the plan current declares `follows_plan: true`, legal on the recording step
itself as well as on every step after it. [Workflow](workflow.md) owns the
field-by-field schema and the refusals a definition can trip — two steps
recording a plan, a recording step missing `plan_recorded`, `follows_plan`
on a step before the recording step or in a workflow with none.

## The plan never replaces a step's own product

A step that records the plan beside another product keeps both. The Judge
reads the step's own deliverable or diff and the plan, labelled apart, never
one folded into the other — Epic's `plan` step keeps its split drawing this
way, not just the tasks it broke the wave into. A later step naming
`<recording_step>.evidence` in `reference_docs` reads the same pairing: the
recording step's own submission, with the plan appended after it.

## Distinct from the declared scope's "plan"

**The word "plan" already names something else in this codebase.** A
step's declared file scope — what `declare_plan_at: step_start` and the
`job_step_plans` table hold, read against by the drift check — is a list of
paths a Drone said it would touch, unrelated to this Plan's tasks. The two
must not collide in name: this document's Plan and Task are the Job's own
record of its work, not a file-scope declaration.
