---
id: 032
title: The Job drives, the Drone reports
status: RESERVED
module: fleet
raised: two Jobs stalled on step names a Drone invented, 2026-08-16
---

# 032 — The Job drives, the Drone reports

**The owner's words, and they restate [`021`](021-the-work-hierarchy.md)'s ontology as a tool
contract:**

> *"A Job has a task that needs completed. To complete it, it needs a workflow executed. The Drone
> is responsible for executing the workflow, but the Job is the one telling the Drone what the task
> is that it needs to complete. The ask of the Drone should be to report back to the fleet that it
> has completed its task for that step in the workflow. Then the Job runs the verification checks
> against its work to confirm that the step has been completed. PASS → it tells the Drone to
> continue to the next step with a prompt telling it what it needs to do. FAIL → reports the
> evidence back to the Drone."*

## What is built instead

`fleet.verdict` asks a Drone for three things, and it should be asked for none of them.

| Argument | Today | Who actually knows |
|---|---|---|
| `step: String` | **required** | The Job. It is on the record, and the gate evaluates *that* step's predicate |
| `verdict: String` | the Drone asserts `PASS`/`FAILED`/… | The gate, from evidence a command produced |
| evidence | the Drone supplies a check id and exit code | Fleet, which runs `armada manifest check` itself for `check_passes` |

`fleet.report` has the same shape one notch softer: `step: Option<String>`, documented as *"which
step, when it is not the one the Job is already on"* — a capability with no use case anybody has
named, and the route by which prose reached a state-machine field.

## What it cost, measured

| Symptom | Cause |
|---|---|
| `the plan workflow has no step called verify` — a sub-Job paused mid-flight | the Drone named a step that does not exist, and the name was stored rather than refused |
| `arm fleet ls` showing `step: 'Understand guild upgrade logic'` | free prose from `fleet.report` landed where a workflow step id belongs |

The second is the sharper one. **The Job's state machine displayed a string a model made up.**

## The contract this replaces it with

**A Drone reports that it finished its part. Nothing else.** No step, no verdict, no evidence.

1. The Job hands the Drone a step's task.
2. The Drone works, and says it is done.
3. **The Job gates** — runs the predicate, produces the evidence.
4. `PASS` → the Job starts the next step with a prompt describing it.
5. `FAIL` → the Job hands the evidence back and the step runs again.

**This is not a new mechanism; it is deleting one.** `check_passes` is already evaluated by Fleet
running the check, not by the Drone — so a Drone-supplied verdict was always a second answer to a
question the gate had already answered properly. `docs/reserved/016` built the gate that makes the
Drone's verdict redundant, and this is the other half of that change arriving.

## The one thing that is not obvious

**A Drone still needs to say when it cannot continue**, and that is not a verdict about the step —
it is a fact about itself. `BLOCKED` and `NEEDS_HUMAN` are real outcomes today and they must not be
lost by deleting the verdict.

The distinction to hold: *"the step passed"* is the Job's to say and a Drone must not; *"I stopped
and here is why"* is the Drone's to say and nothing else can. `fleet.ask_human` already covers a
question it needs answered. What is missing is the flat *"I am done"* and *"I am stuck"*, which is
what the tool should become — and the gate decides what either means for the workflow.

## What it touches

`crates/helm/src/mcp/drone.rs` (the two tools' schemas), `crates/core/src/fleet/drone.rs`'s
[`BRIEF`], which instructs a Drone to report through them, `advance`'s handling of a verdict, and
`019`'s account of what a Drone is told. The brief gets **shorter**, which is the tell that this is
the right direction: a contract a model cannot get wrong is one it is not asked to fill in.
