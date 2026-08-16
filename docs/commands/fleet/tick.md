# `armada fleet tick`

One pass of the workflow loop: notice an exchange ended, gate the step it rests on, then
advance, retry or stop.

> **Status: built — M4.**

**A Drone runs one exchange under `--print` and exits.** That is correct — it is what lets
[`spawn.md`](spawn.md) return and several Jobs run at once — but until this verb existed
**nothing observed the exchange ending**. A Job went `RUNNING` and stayed there for ever beside a
process group that was gone, which is what a person hit on their first real spawn. This is the
thing that observes it.

## Who runs it

**Every Drone runs it, when its own exchange ends** — the relay of
[`020`](../../reserved/020-the-tui-decided.md) §1. Each Drone is started with a `--settings`
document registering a `Stop` hook; the hook waits for its own process to actually go, then runs
`armada fleet tick`. *"An agent can forget to report progress, but it cannot forget to stop"*
([`PLAN.md`](../../PLAN.md) §15.3), and the exchange ending **is** the event.

**The hook waits, and it has to.** A `Stop` hook fires while its session is still alive; a tick
from inside it would find a live process group and decline to gate — correctly, because gating a
live exchange starts a check against a worktree still being written to.

**It ticks the whole fleet rather than its own Job, and that is the backstop.** A relay is lost
three ways — a SIGKILL, a hook that could not run, a crash in between — and none of them can be
fixed from inside the hook that was lost. What fixes them is that the *next* Drone anywhere on
the machine to finish an exchange sweeps every Job, including that one. **Helm's own `Stop` hook
sweeps too**, so asking the orchestrator anything at all catches the fleet up.

**A read verb still never ticks.** `armada fleet ls` advancing a Job behind your back would break
[`PLAN.md`](../../PLAN.md) §15.1; what it does instead is *report* the Job as `STALLED`
([`ls.md`](ls.md)), which is the honest half of the same repair.

**One pass at a time on a machine.** Five exchanges ending together would start five passes, and
two passes gating one step would both `claude --resume` one session. `~/.armada/tick.lock`
serialises them; a second pass **declines** rather than queueing, because the pass holding the
lock is walking the same records and a queued one would only repeat it. A lock left behind by a
killed pass is taken over after ten minutes, so nothing can stop the fleet permanently.

## Synopsis

```sh
armada fleet tick [<job>] [--watch] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name | every Job | Which Job to look at. Absent means the whole fleet, in name order. |
| `--watch` | flag | off | Keep going, every two seconds, until nothing in scope could move again. |

## How it works

Five steps per Job, and the decisions are all in `armada-core` rather than here:

| | |
|---|---|
| **1. Look** | The transcript and the process table, reconciled once — the same read [`ls.md`](ls.md) does. |
| **2. Attention** | Is there anything to do at all? A Job whose Drone is mid-exchange is `working` and nothing is gathered for it, which is what keeps a pass over twenty Jobs cheap. A Job past one of its ceilings halts here, before any gate. |
| **3. Gather** | The **only** I/O in the gate, and it is driven by what the step's predicate asks for: start or poll an `armada manifest check`, search the tree for a test, stat a path, read the inbox. |
| **4. Decide** | Does the predicate hold. |
| **5. Act** | Advance to the next step, retry this one, ask you, or stop. |

**A step advances only on evidence an external command produced.** An agent asserting that the
tests pass is not evidence and an exit code is, and
[`answer.md`](answer.md)'s `fleet.verdict` refuses a `PASS` that carries none. The loop is the
one caller that legitimately reaches that verb: `fleet.report` still writes only `entered` and
`attempted`, and the two words a gate owns are still written by one thing.

**Checks are run detached and polled, never held open.** `armada manifest check --detach` hands
back a run id and returns; a later pass reads it with `--status`. An attached run would hold this
command open for however long a repository's suite takes — minutes, once per Job, for a loop
meant to come round every couple of seconds ([`../manifest/check.md`](../manifest/check.md)).

**One run per attempt, and the attempt travels with the id.** A check started for attempt one
never settles attempt two: the Drone has rewritten the worktree in between, and a stale green
would advance a step on a run that predates the work it was judging. The same holds for an
answer — *"yes, ship it"* about the second thing you were asked is not approval of the third.

**A retry is told what was wrong with the last attempt.** The gate's own words go into the next
exchange's prompt, because a retry that started with the identical prompt is an agent asked to do
the same thing again with no idea what failed.

### Why a verb and not a daemon

Armada owns no long-lived process and M4 does not add one. A pass is idempotent and cheap, so a
timer, a `Stop` hook, [`../helm/bridge.md`](../helm/bridge.md) or a person typing it are all valid
drivers — `--watch` is one of them rather than the only one, and the `Stop` hook is the one that
actually runs on every exchange. A daemon would need its own lease,
its own crash recovery and its own answer to *"what happened while it was down"*, all to replace
a command you can simply run again.

### What it cannot decide

Five of the eight predicates are decided from the machine, one is asked of you, and **two are
refused**: `review_clean` and `subjob_passed` both need another Job's verdict, and Fleet spawns no
Job from inside a running Job's gate. The loop stops once and names what is missing rather than
guessing — answering *yes* would be the false pass the predicate exists to prevent, and answering
*no* would spend the budget and then blame a ceiling. The whole of the coverage, and what would
close each gap, is [`../../reserved/016-what-the-gate-cannot-prove.md`](../../reserved/016-what-the-gate-cannot-prove.md).

**So the shipped four-step `bug` workflow reproduces, fixes, lands and then stops at `review`.**

### Ceilings

The Job's ceilings are the ones that stop a step that will not pass —
`iterations` against its turn ledger, `tokens`, and `wall_clock`
([`PLAN.md`](../../PLAN.md) §14.3). `on_exhausted: needs_human` is the enum's only value and it
means **stop and ask**, never abort: the Job records what it spent and where it reached and is
raised to the inbox. Nothing is discarded and nothing is rolled back.

## Output

```
  JOB          STEP       DID       WHY
  rate-limit   fix        advanced  `fix` passed; it is on `land`
  bad-parse    reproduce  waiting   check run 01JQR… is still RUNNING
  flaky-suite  review     halted    `review` cannot be gated: `review_clean` is settled by a reviewer Job

OK  3 Jobs, 2 moved
```

One row per Job. `did` is one of `idle`, `working`, `waiting`, `advanced`, `retried`,
`finished`, `asked` or `halted`; the first three are the ones where the loop did nothing, and
`--watch` goes round again while any row could still move on its own.

`--json` returns `data.results[]` with `job`, `step`, `did`, `state`, `verdict`, `predicate`,
`evidence[]` and `why`, plus `data.moved`.

## Dependencies

An existing Job, its worktree, its workflow in your guild, and `armada` on `PATH` for the
detached checks.

## Exit codes

`0` the pass answered · `2` `bad_invocation` — unknown Job · `6` `environment` — a Job's worktree
is gone.

**The exit code describes the pass, not the Jobs.** A pass that correctly reports three halted
Jobs exits `0`; read `did` for what happened to them.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`spawn.md`](spawn.md) · [`ls.md`](ls.md) · [`answer.md`](answer.md) ·
[`../manifest/check.md`](../manifest/check.md)
