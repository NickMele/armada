# `armada fleet answer`

Give a waiting Job your decision and let it continue.

> **Status: built — M3.**

## Synopsis

```sh
armada fleet answer <job> "<answer>" [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name | — | Which Job. Required. |
| `"<answer>"` | string | — | What to tell it. Required. |

## How it works

1. Marks the Job's open inbox entry answered.
2. **Resumes the Job** with `--resume <uuid>` and your answer as the next turn, in its own
   worktree with its context intact ([`PHASES.md`](../../PHASES.md) §9.1 F1). The resumed
   Drone is detached exactly as a fresh one is: an answer *starts* a turn and does not wait for
   one, so a Job you answered before lunch is working while you are out.
3. The Job continues its workflow from where it stopped. **Its budget is not reset** — an
   answer is a continuation, not a new run, and resetting the ceiling here would make budgets
   unenforceable for any Job that asks a question. The resumed session appends its own `result`
   to the same transcript, so continuing costs what it costs and the sum keeps counting.

**A Job that has already reached a ceiling is refused rather than resumed.** `on_exhausted:
needs_human` means a person decides what happens next; silently continuing past a ceiling is how
a budget stops being one. The refusal names the ceiling and points at
[`board.md`](board.md).

## Output

```
  STATUS    JOB            DETAIL                TIME
  answered  nightly-flake  yes, raise it to 90s     -

RUNNING  nightly-flake, 7 iterations remaining
```

**The summary line is where a reader sees that the budget was not reset.** An answer is a
continuation, not a new run.

`--json` returns `job`, `uuid`, `entry`, `answer`, `state`, `budget_remaining` and the resumed
Drone's `pgid`. **Not what the turn spent** — it has not finished one.

## Dependencies

An existing Job with an open inbox entry, and `claude` to resume it.

## Exit codes

`0` resumed · `1` `tool_failed` — the resume failed · `2` `bad_invocation` — unknown Job, it has no open entry, or it has already reached a ceiling · `6` `environment` — `claude` is not on PATH.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`inbox.md`](inbox.md) · [`board.md`](board.md)
