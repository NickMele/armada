# `armada fleet reap`

End every finished Job and release what they hold.

> **Status: built — M3.**

## Synopsis

```sh
armada fleet reap [--dry-run] [--yes] [--json]
armada fleet reap --job <job> [--job <job>]… [--yes] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--dry-run` | flag | off | The plan, and nothing reaped. At every surface. |
| `--yes` | flag | off | Reap without asking. What a pipe passes. |
| `--job <job>` | Job name or uuid | — | Reap exactly these, instead of the default set. Repeatable. |

## What is offered, and what is taken

**A state you might still act on is not garbage.** That one sentence decides every row.

| State | Reap |
|---|---|
| `DONE`, `ABORTED` | **taken** — genuinely finished, and nothing further will happen to them |
| `STALLED`, `BLOCKED` | listed and left — a Job whose Drone died is one you may want to start again |
| `PAUSED` | listed and left — it *means* needs-you, and [`resume.md`](resume.md) is the other thing you might do with it |
| `RUNNING` | **never offered** |
| `QUEUED` | never offered — it has spent nothing, holds nothing, and is about to start |

**Observed rather than recorded**, which is what makes the verb useful at all. A record that
still says `RUNNING` with a dead process group is exactly the Job this exists to find — it holds
a port block nothing can use and nothing else reports — and it observes as `STALLED` or `PAUSED`,
so it is offered. A Job only observes as `RUNNING` while its group is provably still Armada's
([`../../PLAN.md`](../../PLAN.md) §14.1).

## The preview is the feature

A bulk delete that only listed names would be asking you to approve a decision on less
information than the machine already has. What makes the answer possible is the second half of
every row: **what each Job is holding**. The cost of reaping and the cost of *not* reaping are
on the same line.

```
  STATUS  JOB            UUID      STATE    HOLDING                        SPENT
  take    rate-limit     8f2a1c40  DONE     ports 5470-5479, worktree ~/…  $2.10
  keep    this-test      94b1fd2e  PAUSED   ports 5460-5469, branch …      $4.60
  take    this-test      c19d0a34  ABORTED  branch armada/this-test            -

OK  3 jobs, 2 to take, nothing reaped
```

**The uuid is on every row and it is not decoration.** Two Jobs can share a name — the example
above is a real one — and a preview of what is about to be deleted that cannot tell two rows
apart is a preview that cannot be read. It is also what `--job` takes, so the row carries the
handle the next command needs.

`take` and `keep` are **words rather than a tick and a cross**, for the rule every table here
follows: a glyph that only appears at a terminal gives the two audiences different shapes
([`../render.md`](../render.md)).

## Nothing is reaped without an answer

| Given | What happens |
|---|---|
| `--dry-run` | the plan, and nothing else, at every surface |
| `--yes` | the reap happens |
| a terminal, no `--yes` | the plan is printed and the question is put; the default answer is **keep them** |
| no terminal, no `--yes` | the plan, then `bad_invocation` naming `--yes` |

**The last row is the one worth stating.** A destructive bulk action with nobody there to
confirm must refuse rather than proceed: "nobody said no" is not consent. `--json` changes only
who reads the answer, exactly as it does everywhere else — a `--json` reap without `--yes` emits
the plan and reaps nothing.

## How it works

Exactly [`kill.md`](kill.md)'s teardown, per Job, in the same order — the Drone, then `armada
manifest clean` in the worktree, then the worktree, then the record. One implementation, because
the order is the point and two copies would be two answers to what it tolerates.

**One Job that will not clean does not stop the batch.** The failure is carried on that Job's
row and the rest proceed, for the same reason `kill` carries rather than raises: one container
that refuses to stop must not leave four other Jobs holding their worktrees.

**A Job named on the line that is no longer reapable is refused rather than killed** — its Drone
came back to life between the preview and the confirmation, and a reap is not a kill.

## Exit codes

`0` clean · `1` something would not release — the Jobs still ended · `2` `bad_invocation` — a
Job that is working, or a bulk reap with nobody to confirm it.

## See also

[`kill.md`](kill.md) · [`pause.md`](pause.md) · [`ls.md`](ls.md) ·
[`../manifest/clean.md`](../manifest/clean.md) ·
[`../helm/bridge.md`](../helm/bridge.md) — `r` on the live screen
