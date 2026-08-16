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
declared in [`workflow.rs`](../../crates/core/src/fleet/workflow.rs). **Seven it decides. One it
asks you. Two of the seven decide something narrower than their names suggest.** This file
records those narrowings, because *"a partial evaluator honest about its coverage beats one that
guesses"* — and because a gate that guesses produces the exact failure
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
| `review_clean` | a committed branch **and** a reviewer Job's verdict | decided, narrowly — §2 |
| `subjob_passed` | a sub-Job's verdict | decided — §2 |

**§2 was a refusal until 2026-08-16** and is now the longest section here, because the thing it
was waiting for — a workflow step that spawns a Job and waits on it — was built, and what it
decides is worth stating precisely.

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

## 2. `review_clean` and `subjob_passed` are settled by a Job the gate spawns

Both are settled by **another Job's** verdict — a reviewer for the first, a sub-Job running a
named workflow for the second. For the length of M4 nothing in Fleet spawned a Job from inside a
running Job's gate, so the loop refused both and stopped once rather than retrying: *yes* would
have been the false pass the predicate exists to prevent, and *no* would have retried until the
budget was gone and then reported a **ceiling**, sending the reader to hunt a failing test
instead of telling them a reviewer was never started.

That refusal is closed. `armada fleet tick` starts the child
([`verbs/fleet.rs`](../../crates/helm/src/verbs/fleet.rs), `spawn_child`), records its uuid on
the parent's `pending`, and advances the step on the child's own verdict.

### Why a child Job is evidence when the parent's Drone is not

[`Verify`](../../crates/core/src/fleet/workflow.rs)'s rule is *"evidence an external command
produced"*, and a Job is not a command. The distinction that actually matters is not the process
boundary but **who decided**: the parent's Drone saying *"I reviewed it and it is fine"* is the
worker grading its own work, while a child Job's verdict is Armada's own gate run against its own
predicates in a session that never saw the parent's reasoning. [`PLAN.md`](../PLAN.md) §14.6
puts the same thing as *"a reviewer that shares the implementer's context shares its blind
spots"*, and names the verdict envelope as the evidence.

### The decisions, and what each costs

| decision | what it costs |
|---|---|
| **A reviewer runs the `review` workflow** ([`templates/guild/workflows/review.yml`](../../templates/guild/workflows/review.yml)), and a step may name `workflow:` to choose a different one. The name is a constant rather than the step's, because a `review_clean` step names no runner at all — that is how the schema says *Fleet satisfies this* — so the Job under review has no say in who examines it. | A fifth shipped workflow and a shipped `review-diff` skill. It is deliberately **not** one of the four classification labels: `armada fleet spawn "review this"` should still classify as `bug` or `feature`. |
| **The reviewer's worktree starts at the parent's branch**, so the diff it reads is the work. `git worktree add` gained a start point. | The reviewer sees **only what is committed**, which is why the gate checks the commit first — see below. |
| **`subjob_passed` takes its workflow from the step's own `workflow:`**, and a step that gates on it without one no longer parses. | Nothing. The key already existed and `feature`'s plan step already used it; the parse-time refusal is the same one `failing_test_exists` without a test gets. |
| **Iterations and tokens are carved out of the parent's remaining**, and the child's spend is added to the parent's ledger when it settles (`Kin::spend`). | A parent's `armada fleet ls` figure now includes work done in another Job's transcript. Without it a parent spends no turns of its own while a child works, so its ceiling never moves — a child able to exhaust its parent in silence, which is the failure this was asked to design out. |
| **The wall clock is the child's own, and the parent's is suspended while it runs** (`Kin::suspended_ms`). | Two fields on the record and a `run_time_ms` that is no longer *how long the Job has existed*. It has to be this way: [`PLAN.md`](../PLAN.md) §14.6 is explicit that `feature` waits on a `plan` sub-Job that ends at your approval, and a clock that kept ticking would kill a Job because you went to lunch. |
| **One child per attempt**, keyed exactly as a detached check run is. | A failed review starts a second reviewer rather than re-reading the first one's verdict. That is the point: the first read the diff *before* the fix. |
| **A step Fleet satisfies starts no Drone** — except on a retry, when the gate's words are handed to the parent's own session, which is the only thing that can act on them. | The first step of a workflow is still always a Drone's, because `--resume` needs a session that exists. |
| **And it is gated in the pass that enters it**, rather than on the next one. | One more gate per pass that advances into such a step. It is not an optimisation — see below. |

### The step that is never *due* a tick

[`020`](020-the-tui-decided.md)'s *"the Stop hook drives tick"* decision gave the
loop a watermark: a Job is due a gate when its transcript
holds more finished turns than the last tick gated (`finished > ticked_turns`),
and a Job with no live Drone, nothing pending and nothing due is `STALLED`. That
is what makes a dead Job provable, and it is right.

**A step no Drone runs produces no exchange, so it can never satisfy it.** A
parent that advanced into `review` and stopped there was measured as
`state=Stalled due=false -> Idle { "its Drone stopped without finishing a turn" }`
— a Job nothing would ever look at again, including the sweep whose whole
purpose is to rescue the ones nothing looked at. The two features are each
correct and the gap is exactly between them.

The fix is to close the window rather than to add a second watermark: there is
genuinely nothing to wait for, and the pass that advances into the step already
holds everything the gate needs. It terminates because `step_after` only moves
forward.

**The same seam cost the ledger, in the other direction.** `tear_down` settles a
finishing Job's spend from its transcript, *"because the transcript is about to
be the only thing left that knows"* — true before a Job could have children, and
false after: a child runs in a session of its own. A `feature` Job that finished
with a `plan` sub-Job had that sub-Job's entire spend subtracted at the last
moment, and the figure written there is the one `armada fleet ls --all` shows
for ever.

### What `armada fleet kill` on a parent does to a live child

**It ends the children first, deepest first, and then the parent.** Left behind, a child keeps a
Drone, a worktree and a port block, and spends a budget producing a verdict for a record that
says `ABORTED`.

Killing a **child** on its own is a different act and means a different thing: the parent's gate
reads a Job that ended without a verdict, stops **once** and asks you, rather than starting a
second child over the same work. Retrying there would spend again on a decision a person has
already made by hand.

### Cycles, bounded where the edge is taken

`workflow.schema.json` says the graph *"must be acyclic; `armada guild verify` rejects a
cycle"*, and `guild verify` is not built — so `feature → plan → feature` would have been a fleet
that grows until every ceiling in it is reached. A sub-Job whose workflow already appears in the
chain above it is refused by name, with the chain in the message, and a fourth generation is
refused as a backstop for the non-cyclic shape. It is checked against the **records on disk**
rather than against the guild's documents, which is stricter: it catches a cycle somebody
introduced by editing a workflow while a Job was running through it.

### What `review_clean` still does not prove — and it is the name

It proves **an independent Job read the committed branch and produced its findings**. It does not
prove those findings say *clean*, because nothing reads the document: `review.yml` gates on
`artifact_exists`, and a reviewer that wrote *"three things block this"* passes the same gate as
one that wrote *"nothing does"*.

**What carries a blocking finding today is the reviewer stopping.** The skill tells it to raise a
genuinely blocking judgement through `fleet.ask_human`; that leaves its Job short of `DONE`, the
parent reads *waiting on you* rather than a pass, and the question is in your inbox. So the
mechanism exists — but it rests on the reviewer choosing to use it, which is an instruction and
not a gate.

Closing it needs one of two things, and neither is small:

| design | what it costs |
|---|---|
| **A findings grammar.** `REVIEW.md` becomes structured, and a `findings_clean` predicate reads it. | Armada owns a document format and every reviewer skill has to produce it — the thing §4 of `PLAN.md` refuses for test runners, arriving in a new place. |
| **A Drone that can fail its own Job.** A reviewer emits `FAILED` for its own last step, and the child ends `DONE` with a non-`PASS` verdict. | `fleet.verdict` is deliberately absent from a Drone's toolbelt (`job::StepEvent`), because a step is done when its predicate holds and not when the worker says so. Granting it here would be granting it everywhere. |

Until one exists, the predicate is worth what it costs: it stops the branch that nobody
independent ever opened, which is the failure that was actually shipping.

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

The first person to read a `REVIEW.md` that lists three blocking findings on a branch that
landed, or to write a workflow whose `artifact:` starts with `/` and watch it never pass.

## What is still open

**§2's refusal is closed and its narrowing is not.** The section is kept rather than deleted
because what `review_clean` decides is not what its name claims, and a reader who only saw the
predicate list would take *"a reviewer Job returned no blocking findings"* at face value. §1 and
§3 are unchanged: `failing_test_exists` still cannot link the red suite to the named test, and an
absolute `artifact:` is still `armada guild verify`'s to refuse — and that verb is still not
built.
