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

A Plan holds an `approach` — a paragraph — and an ordered list of Tasks. A
Task carries a stable id (`T1`, `T2`, …), a one-line `title`, an optional
`detail`, and a state: `open`, `working`, `done` or `dropped`. `dropped`
requires a `reason`.

| Field | On | Meaning |
| --- | --- | --- |
| `approach` | Plan | The step's own account of how it means to do the work |
| `id` | Task | Stable, assigned once, never reused |
| `title` | Task | One line |
| `detail` | Task | Optional, longer |
| `state` | Task | `open`, `working`, `done`, `dropped` |
| `reason` | Task | Required when `state` is `dropped` |

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

## Task states are a Drone's claim, never a gate

**Rule.** Fleet never refuses a step's submission because a task is open.
Why: task state is self-report, and `docs/scope.md` treats self-report as a
signal, not a source of truth.

A step declaring `follows_plan: true` may still be judged against the plan's
task states — whether the diff does what a task set `done` claims, and
whether a `dropped` task's reason holds. That is a Judge criterion asked of
the diff, never a mechanical gate on the plan itself. `plan_recorded` is the
one mechanical check touching a Plan, and it asks only that one exists with
at least `min_tasks` tasks — see [Workflow](workflow.md).

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
