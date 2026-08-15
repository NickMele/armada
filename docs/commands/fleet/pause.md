# `armada fleet pause`

Stop a Job's Drone and keep everything else.

> **Status: built — M3.**

## Synopsis

```sh
armada fleet pause <job> [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name or uuid prefix | — | Which Job. Required. |

## How it works

**A Job is durable and a Drone is not**, and that is what makes pause a verb rather than a
signal ([`../../PLAN.md`](../../PLAN.md) §14.1). Pausing stops the process that is working and
leaves everything the Job *is* exactly where it was:

| | |
|---|---|
| **Stopped** | the Drone's process group — SIGTERM, a grace period, then SIGKILL |
| **Kept** | the worktree, the branch, the port block, the transcript, the inbox |

The spend is settled from the transcript on the way out, because nothing is going to write to
that transcript again until the Job is resumed. [`resume.md`](resume.md) starts a new Drone on
the same session, so the ledger carries on being appended to and the budget is **not** reset —
a Job held for an hour has spent nothing in that hour and has no more rope than it had.

**`SIGSTOP` was the other candidate and is the wrong one.** A stopped process still answers
`ps`, so the observation would go on calling the Job `RUNNING` and the pause would not stick;
and a Claude Code session frozen mid-request holds a connection open for as long as you are
away.

**A Job that has already ended is refused**, and so is one that is already paused — the second
refusal names [`resume.md`](resume.md), which is the verb that would have worked.

Nothing signals a process group Armada cannot prove is its own. A handle from a previous boot,
or one whose pid now has a different start time, names a recycled pid rather than a Drone.

## Output

```
  STATUS  JOB         DETAIL                        TIME
  paused  rate-limit  stopped the Drone, group 771     -

PAUSED  rate-limit, $2.10 spent, worktree kept
```

A Job between turns has no live Drone, and holding it is still something you can ask for; the
detail says `no Drone was running` and the Job is `PAUSED` either way.

## Exit codes

`0` paused · `2` `bad_invocation` — no such Job, a Job that has ended, or one already paused ·
`5` `tool_failed` — the process group was still there after SIGKILL. **The Job is held either
way**: a pause that bailed out because a group would not die would need a second pause to do
the same thing again.

## See also

[`resume.md`](resume.md) · [`kill.md`](kill.md) · [`ls.md`](ls.md) ·
[`../helm/bridge.md`](../helm/bridge.md) — `p` on the live screen
