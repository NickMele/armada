# `armada fleet inbox`

What the fleet needs from you.

> **Status: built — M3.**

The CLI view of the same file the orchestrator watches
([`../helm/inbox.md`](../helm/inbox.md)).

## Synopsis

```sh
armada fleet inbox [--job <name>] [--all] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--job <name>` | Job name | all | Only this Job's entries. |
| `--all` | flag | off | Include entries already answered. |

## How it works

Reads `~/.armada/inbox.jsonl`, an append-only file written from two directions:

| Writer | When |
|---|---|
| A Drone's MCP call | It has a question only you can answer. |
| `Stop` / `Notification` hooks | It went idle, got stuck, or is asking for permission. |

**Hooks are the spine.** An agent can forget to report progress, but it cannot forget to stop —
which is what makes "needs my attention" reliable rather than best-effort.

Append-only means it survives every kind of crash, which is the same reasoning that put the
ownership store on disk rather than in a process. Reading does not mark entries answered;
[`answer.md`](answer.md) does that.

## Output

```
  STATUS       JOB            DETAIL                              TIME
  needs_human  nightly-flake  Wants the CI timeout raised to 90s    9m

OK  1 open, armada fleet answer <job> "…"
```

**Every entry has an id**, which is the half [`../../PLAN.md`](../../PLAN.md) §15.3.1 says is
missing from anything raised in prose: an item you cannot name is an item you cannot
acknowledge one row at a time.

`--json` returns one result per entry with `uuid`, `job`, `kind`, `raised_at`, `waiting_s`,
`body` and `answered`.

## Dependencies

`~/.armada/inbox.jsonl`. Absent means an empty inbox, not an error.

## Exit codes

`0` whenever the file is readable. **An empty inbox is a normal state, not a failure** — use `--json` and check for an empty result set rather than reading the exit code.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`answer.md`](answer.md) · [`ls.md`](ls.md) · [`../helm/inbox.md`](../helm/inbox.md)
