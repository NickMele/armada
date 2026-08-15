---
id: 016
title: What the gate cannot prove
status: RESERVED
module: fleet
raised: building the M4 loop, 2026-08-15 — five predicates are decided from the machine, three are not, and the honest shape is a gate that says so rather than one that guesses
---

# 016 — What the gate cannot prove

M4's evaluator ([`crates/core/src/fleet/gate.rs`](../../crates/core/src/fleet/gate.rs)) decides
a step's `verify: { must: … }` from evidence an external command produced. Eight predicates are
declared in [`workflow.rs`](../../crates/core/src/fleet/workflow.rs). **Five it decides. One it
asks you. Two it refuses.** This file records the refusals and the one predicate that decides
something narrower than its name suggests, because *"a partial evaluator honest about its
coverage beats one that guesses"* — and because a gate that guesses produces the exact failure
[`PLAN.md`](../PLAN.md) §14.3 built the predicates to prevent: a Drone that closes green on its
own assertion.

| predicate | decided by | status |
|---|---|---|
| `always` | the exchange ending cleanly | decided |
| `check_passes` | `armada manifest check --detach` / `--status` | decided |
| `failing_test_exists` | a search of the tree **and** a red check | decided, narrowly — §1 |
| `artifact_exists` | a path under the Job's worktree | decided, narrowly — §3 |
| `branch_exists` | `git rev-parse` **and** a clean `git status` | decided |
| `human_approves` | you, through the inbox | asked |
| `review_clean` | — | **refused** — §2 |
| `subjob_passed` | — | **refused** — §2 |

## 1. `failing_test_exists` proves the suite is red, not that *that* test is why

The predicate's two halves are each backed by their own command: a fixed-string search of the
worktree for the test's name, and an `armada manifest check` run that exits **non-zero**. Both
are real evidence and both are recorded on the transition.

**What is not proved is the link between them.** The suite being red and a test of that name
being in the tree are two facts; that the named test is the one that failed is a third, and
nothing reads it. A Drone could write `regression_bad_parse`, leave it passing, and break
something else — and this gate would hold.

Closing it needs one of two things, and neither is small:

| design | what it costs |
|---|---|
| **Read the failure list out of the run.** `check` records a verdict per check, not per test; parsing a test runner's output means a parser per runner, which is the thing Armada has refused to own since `PLAN.md` §4. | a grammar for test-level results, and a way for a repository to declare how its runner reports them |
| **Run the one test.** `armada manifest check --only <test>` — a selector below check granularity. | a new axis in the selector grammar (`PLAN.md` §4.1.1), and every repository has to be able to express it |

Until one exists, the predicate is worth what it costs: it stops the Drone that never reproduced
anything at all, which is the failure that actually happened.

## 2. `review_clean` and `subjob_passed` need a Job that Fleet does not spawn

Both are settled by **another Job's** verdict — a reviewer for the first, a sub-Job running a
named workflow for the second. `armada fleet spawn` creates one Job from one task; nothing in
Fleet spawns a Job from inside a running Job's gate, and nothing joins one Job's completion to
another's step.

**The loop refuses them rather than deciding them**, and stops once instead of retrying:

- Answering *yes* would be exactly the false pass the predicate exists to prevent.
- Answering *no* would retry until the Job's budget was gone and then report a **ceiling** —
  sending the reader to hunt a failing test instead of telling them a reviewer Job was never
  started.

So the gate raises `NEEDS_HUMAN` naming what is missing, and
`the_shipped_bug_workflow_runs_to_its_review_step_and_stops_there` holds that boundary as a fact
in the suite rather than a sentence in a document. **The shipped four-step `bug` workflow
therefore reproduces, fixes and stops at `review`.** PHASES.md §8.6's done-when is met by the
three steps before it: reproduce, fix, land, no human turn in the middle.

What would close it is a **workflow step that spawns a Job and waits on it** — child Job
identity on the parent's record, the parent's ceilings bounding the child's, and an answer to
what `armada fleet kill` on the parent does to a child mid-exchange. It wants its own pass.

## 3. An `artifact:` outside the worktree reads as *not on disk*

`artifact_exists` joins the step's `artifact:` onto the Job's worktree. An **absolute** path
makes `Path::join` discard the worktree entirely, so a workflow naming `/etc/passwd` would be
gating on a file outside the Job.

It is reported as *not on disk* rather than allowed, which is safe — but the honest answer is a
refusal at the point the workflow is read, not a false negative at the point it runs. That is
`armada guild verify`'s finding to make, and **`guild verify` is not built**
([`AGENTS.md`](../../AGENTS.md)). When it is, an absolute `artifact:` is one of the things it
should refuse by name.

## Hook

The first person to write a workflow with a `review` step and watch it stop, or to write one
whose `artifact:` starts with `/` and watch it never pass.
