# `armada manifest check`

Lint, format, test. Scoped, scheduled and locked.

> **Status: shipped, `--detach` and `--status` included.** They were the last thing blocking
> M4 — a loop could run a check to completion but could not start one and poll it
> ([`PHASES.md`](../../PHASES.md) §8.6).

This is the objective gate the whole system leans on. A Fleet verdict is only `PASS` if it
carries evidence an external command produced, and this is that command
([`PLAN.md`](../../PLAN.md) §14.3).

## Synopsis

```sh
armada manifest check [<selector>] [--component <name>] [--all-files] [--fix]
                      [--wait] [--concurrency <n>] [--detach] [--dry-run] [--json]
armada manifest check --files <path>… [--fix] [--json]
armada manifest check --status [<run-id>] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<selector>` | `<component>:<check>`, a component, or a check name | the working diff | Which checks to run. One selector, or several paths — never both, so nothing is guessed. |
| `--component <name>` | component name | — | Every check on one component. |
| `--files <path>…` | paths | — | Run only the checks these paths belong to. Exists for names a shell would mangle as positionals. |
| `--all-files` | flag | off | Scope from each component's `match:` globs rather than from the working diff. |
| `--fix` | flag | off | Run `fix:` instead of `cmd:`. Checks with no `fix:` are skipped. |
| `--wait` | flag | off | Queue for the run lease instead of failing fast when another run holds it. |
| `--concurrency <n>` | positive integer | the machine's | This run's CPU budget, overriding the machine's. |
| `--detach` | flag | off | Start the run in its own session and return its id. |
| `--status [<run-id>]` | run id | the most recent run | What a run decided, or is still deciding. Reads only. |
| `--dry-run` | flag | off | Print the schedule and each argv without running anything. |

**There is no `--scope <lens>`.** A run's default scope is the working diff, and `--all-files`
is what widens it — a lens naming both would be a second way to say one thing.

**Lines that say two things at once are refused, not resolved by precedence.** `--status` is a
read verb and takes no scope, so `--status --fix`, `--status <selector>` and the rest answer
`bad_invocation` naming both halves; so do `--detach --status` and `--detach --dry-run`. A
caller who typed one of those meant one of two very different things, and picking one silently
is how an agent comes to believe it repaired a repository it only read.

> **`-C <path>` is reserved and not built.** A verb takes its workspace from where you are
> standing, and `cd` is the interface until something needs otherwise
> ([`config.md`](config.md)).

## How it works

1. **Resolves the check set** from the selector, over the working diff unless `--all-files`
   widens it to each component's `match:` globs.
2. **Schedules them** — checks with no ordering constraint run concurrently; `needs:` on a
   check takes both component names and other check ids ([`PLAN.md`](../../PLAN.md) §4.1).
3. **Takes a lock.** One check run at a time per workspace ([`PLAN.md`](../../PLAN.md) §3.2.1). Two
   concurrent runs would fight over the same ports and containers, and the second would report
   failures caused by the first.
4. **Runs each check**, optionally inside a container via `in:`.
5. **Aggregates verdicts.** A run's verdict is the worst of its checks. `RUNNING` and `WAITING`
   are progress, not verdicts ([`PLAN.md`](../../PLAN.md) §3.1) — a caller must not treat a
   still-running check as a pass.
6. **Records the dispatch record** — argv, what it waited on, who held what — which is the
   evidence [`explain.md`](explain.md) later reads. It is written at dispatch because it is
   point-in-time and unrecoverable afterwards.

## Detaching, and asking afterwards

**A loop runs inside a Drone's turn, and a thirty-minute check cannot block it.** That is the
whole reason `--detach` exists: something must be able to start a check, walk away, and later
ask whether it finished and what it decided.

**It is not a new mechanism.** Fleet's Drones are long-lived `setsid`'d process groups recorded
as owned, and `armada fleet ls` answers about them from what they wrote to disk rather than
from anything they reported. `--detach` is that shape against a run directory
([`PHASES.md`](../../PHASES.md) §8.6).

```sh
run=$(armada manifest check --detach --json | jq -r .data.run_id)
armada manifest check --status "$run" --json     # RUNNING, and which checks are in flight
armada manifest check --status "$run" --json     # …later: the verdict, and the exit code
```

**Everything that can fail synchronously still does.** The selection, the working diff, the
port block and every argv are resolved in the caller's terminal before anything detaches, so a
`--detach` that reports a run id reports a run that started. What detaches is the part that
takes time.

**`--status` reads the run directory. It never asks a process how it is doing.** The record is
rewritten whenever a check changes phase, so a poll reads what the run has decided so far
rather than a summary somebody remembered to send. Three answers, in the order they are ruled
out:

| the record | the group | answer |
|---|---|---|
| has a verdict | either | the verdict, exactly as the run reported it |
| has none | still there | `RUNNING`, with the rows settled so far |
| has none | gone | `DEAD`, class `aborted` — it stopped without deciding |

**The record wins over the probe, and the order matters.** A run that has written its verdict
has finished deciding whether or not its process has got as far as exiting; re-deciding on the
strength of a `ps` would let a detached run answer differently from the attached one that
produced the same rows. Liveness is asked exactly once, for the middle row, and it is
`reap::pgid_is_ours` over the pgid, the boot id and the process's start time — the same three
facts, and the same function, that decide whether `clean` may signal a service's group.

**The detached child takes the run lease, not the invocation that started it.** The parent has
already exited, and a lease held by a process that is gone is one the cold-heartbeat path
reclaims. So a detached run is protected exactly as an attached one is: a second `check`
fails fast, and `clean` refuses while it is in flight and says which run is holding it.

**The group is recorded as owned**, so an orphan — Armada died, the run did not — is reclaimed
by the same pass that reclaims an orphaned service or Drone ([`clean.md`](clean.md)).

## Output

```
lint     PASS     2.1s
format   PASS     0.4s
test     FAILED  18.7s   3 failed, 214 passed
```

`--json` returns one result per check with verdict, argv, duration, and the failure signature.
**The exit code is what Fleet consumes** — not the text.

## Dependencies

| On | Why |
|---|---|
| `armada.yml` | The `checks:` block. |
| [`init.md`](init.md) | Checks that need services need ports claimed. |
| A container runtime | Only for checks using `in:`. |

## Exit codes

`0` every check passed · `1` `tool_failed` — at least one check failed · `2` `bad_invocation` — unknown selector, no such run, or two flags that cannot both be meant · `4` `timeout` · `5` `aborted` — including a detached run that stopped without deciding · `6` `environment` — the lock could not be acquired, or the runtime is unavailable.

**`--detach` exits `0` because it carries no error, and that is not a pass.** It reports
`RUNNING`, which is progress and never a verdict ([`PLAN.md`](../../PLAN.md) §3.1). A gate reads
the exit code of `--status` once the run has one — the exit code of the *start* says only that
the run started.

**The exit code is what Fleet consumes**, not the text.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`explain.md`](explain.md) · [`commands.md`](commands.md) · [`up.md`](up.md)
