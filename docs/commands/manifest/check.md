# `armada manifest check`

Lint, format, test. Scoped, scheduled and locked.

> **Status: shipped, except `--detach` and `--status`,** which answer `bad_invocation` and
> are **the blocker for M4** — the workflow loop cannot close without them
> ([`PHASES.md`](../../PHASES.md) §8.6).

This is the objective gate the whole system leans on. A Fleet verdict is only `PASS` if it
carries evidence an external command produced, and this is that command
([`PLAN.md`](../../PLAN.md) §14.3).

## Synopsis

```sh
armada manifest check [<selector>] [--scope <lens>] [-C <path>] [--dry-run] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<selector>` | check ids or components | all | Which checks to run. |
| `--scope <lens>` | `changed` \| `all` | `all` | `changed` restricts to what the working tree touched. [`PLAN.md`](../../PLAN.md) §3.3. |
| `--dry-run` | flag | off | Print the schedule and each argv without running anything. |

## How it works

1. **Resolves the check set** from the selector and scope lens.
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

`0` every check passed · `1` `tool_failed` — at least one check failed · `2` `bad_invocation` — unknown selector · `4` `timeout` · `5` `aborted` · `6` `environment` — the lock could not be acquired, or the runtime is unavailable.

**The exit code is what Fleet consumes**, not the text.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`explain.md`](explain.md) · [`commands.md`](commands.md) · [`up.md`](up.md)
