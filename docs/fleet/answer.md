# `armada fleet answer`

Give a waiting Job your decision and let it continue.

> **Status: not built — M3.**

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
   worktree with its context intact ([`PHASES.md`](../PHASES.md) §9.1 F1).
3. The Job continues its workflow from where it stopped. **Its budget is not reset** — an
   answer is a continuation, not a new run, and resetting the ceiling here would make budgets
   unenforceable for any Job that asks a question.

## Output

```
answered  nightly-flake — resumed, 7 iterations remaining
```

`--json` returns the Job record with the answered entry and remaining budget.

## Dependencies

An existing Job with an open inbox entry, and `claude` to resume it.

## Exit codes

`0` resumed · `1` `tool_failed` — the resume failed · `2` `bad_invocation` — unknown Job, or it has no open entry.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`inbox.md`](inbox.md) · [`board.md`](board.md)
