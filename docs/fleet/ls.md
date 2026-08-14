# `armada fleet ls`

What is running, how long, what it has spent, and who needs you.

> **Status: not built — M3.**

## Synopsis

```sh
armada fleet ls [--all] [--needs-attention] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--all` | flag | off | Include finished and killed Jobs, not just live ones. |
| `--needs-attention` | flag | off | Only Jobs waiting on you. The same set [`inbox.md`](inbox.md) reports, in table form. |

## How it works

Reads the Job index in `~/.armada/jobs/`, and for each Job reads the tail of its
transcript at `~/.claude/projects/<slug>/<uuid>.jsonl`.

**Every column comes from data Claude Code already emits** — the turn's `result` event carries
`total_cost_usd`, `usage`, `num_turns` and `duration_api_ms` ([`PHASES.md`](../PHASES.md) §9.1 F2).
Fleet builds no accounting layer and estimates nothing.

Read-only. Never resumes or interrupts a Job.

## Output

```
NAME           WORKFLOW   STATE     RUN    SPENT   NEEDS YOU
rate-limit     feature    RUNNING   14m    $2.10   —
nightly-flake  bug        BLOCKED    9m    $1.35   ● timeout call
```

`--json` returns one result per Job with `uuid`, `name`, `workflow`, `state`, `runtime_s`,
`cost_usd`, `tokens`, `turns`, `budget_remaining` and `needs_attention`.

## Dependencies

The Job index and the transcripts. No network. Does not need the repository the Jobs
branched from.

## Exit codes

`0` whenever the index is readable — `ls` reports rather than judges · `6` `environment` — the index is unreadable.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`inbox.md`](inbox.md) · [`spawn.md`](spawn.md) · [`board.md`](board.md)
