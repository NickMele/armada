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
| `--job <handle>` | Job name or uuid | all | Only this Job's entries. Resolved through the Job index, so a name meaning two Jobs is refused exactly as [`show.md`](show.md) refuses it. |
| `--all` | flag | off | Include entries already answered or closed. |

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

**An entry names its Job by uuid, and the name beside it is only a label.** Names are handed
out again once the Job holding one is over, so an entry keyed by a name cannot be resolved to
the Job that raised it — which is the defect
[`../../reserved/005-inbox-label-not-identity.md`](../../reserved/005-inbox-label-not-identity.md)
records, found when `armada fleet ls` reported *no Jobs* while this verb reported five open
entries all naming `this-test`.

**An entry is closed when its Job reaches `DONE` or `ABORTED`**, and is then shown only under
`--all`. **Marked and retained rather than deleted**: the reason a Job stopped is often the
last thing written about it. The footer's `armada fleet answer` line is printed only when
something is actually open, because an action that can only fail is worse than no action.

**Entries written before this was fixed are migrated on the first read**, appended to rather
than rewritten. A name that means exactly one Job is bound to it; a name that means none or
several is closed `UNRESOLVABLE` and never guessed at, because guessing is the coin flip
[`show.md`](show.md) already refuses to take.

`--json` returns one result per entry with `uuid`, `job_uuid`, `job`, `kind`, `raised_at`,
`waiting_s`, `body`, `answered` and `closed`. **Resolve on `job_uuid`, never on `job`.**

## Dependencies

`~/.armada/inbox.jsonl`, and `~/.armada/jobs/` while a legacy entry remains to migrate.
Absent means an empty inbox, not an error.

## Exit codes

`0` whenever the file is readable. **An empty inbox is a normal state, not a failure** — use `--json` and check for an empty result set rather than reading the exit code.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`answer.md`](answer.md) · [`ls.md`](ls.md) · [`../helm/inbox.md`](../helm/inbox.md)
