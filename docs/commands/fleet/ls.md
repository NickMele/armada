# `armada fleet ls`

What is running, how long, what it has spent, and who needs you.

> **Status: built — M3.**

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

Reads the Job index in `~/.armada/jobs/`, and for each live Job reads its transcript at
`~/.armada/jobs/<uuid>.stream.jsonl` and asks the process table whether its Drone is still
running.

**What it reports is an observation, not the record.** A Drone runs detached and updates nothing
when its turn ends, so the state on disk is what a verb last wrote — and this is the verb that
looks at the two things that can be looked at and says what is actually true. `STALLED` is the
one that could only ever come from here: a Job is stalled when its Drone produced no transcript
activity, which is the one condition a busy Drone cannot report about itself
([`../../PLAN.md`](../../PLAN.md) §14.3).

**It writes none of it back.** A read verb that persisted would make `armada fleet ls | head` a
change to the fleet; [`kill.md`](kill.md) and [`answer.md`](answer.md) are the verbs that
settle what they saw.

**Every column comes from data Claude Code already emits** — the turn's `result` event carries
`total_cost_usd`, `usage`, `num_turns` and `duration_api_ms` ([`PHASES.md`](../../PHASES.md) §9.1 F2).
Fleet builds no accounting layer and estimates nothing.

Read-only. Never resumes or interrupts a Job.

## Output

```
  STATUS   JOB            ID        WORKFLOW  DETAIL                 SPENT  TIME
  RUNNING  rate-limit     c19d0a34  feature   implement, check gre…  $2.10   14m
  STALLED  xlsx-report    3d9cc7ba  bug       no output for 6m       $4.60   22m
  BLOCKED  release-merge  7f2ab618  feature   wants CI timeout rai…  $1.25    1h
  QUEUED   nightly-flake  e52eaad5  bug       -                          -     -

RUNNING  4 jobs, 1 need you, $8.40 today
```

**`ID` is the Job's uuid, cut to eight characters.** A name is a handle rather than a key —
it is handed out again once the Job holding it is over — so two Jobs can be called
`this-test`, and then `armada fleet show this-test` refuses as ambiguous. Eight characters is
what that refusal prints and what you type instead, so the table shows them rather than making
you run a second command to learn them
([`../../reserved/005-inbox-label-not-identity.md`](../../reserved/005-inbox-label-not-identity.md)).
The cost is ten columns of `DETAIL`; the truncated half is recoverable from
[`show.md`](show.md), and a Job you cannot name is not recoverable from anywhere.

**Status first and always a word**, like every other table Armada draws
([`../render.md`](../render.md)). The layout is frozen by
`tests/golden/render/fleet-ls.plain` and its `.tty` twin — the fixture is the specification
and the renderer follows it.

**A Job that has not run yet gets a placeholder in both number columns**, not `$0.00` and
`0s`: a zero reads as a measurement, and nothing has been measured.

`--json` returns one result per Job with `uuid`, `name`, `workflow`, `state`, `detail`, `task`,
`runtime_s`, `cost_usd`, `tokens`, `turns`, `budget_remaining` and `needs_attention`.

Beside `results` it carries `needs_you`, `spent_usd` and — when one of the Jobs has seen it and
it has not reset yet — `window`, holding `used_percent` and `resets_in_s`.

**`task` and `window` are carried and not drawn here.** The table's `DETAIL` answers *what is it
doing now*; `task` is the words the Job was given, and `window` is the rate-limit window the
account is inside rather than a fact about any row. [`../helm/bridge.md`](../helm/bridge.md)
draws both. The Bridge is a renderer over this listing, so those fields travel with the listing
rather than sending the Bridge back to `~/.armada/jobs/` for a second read — which is also what
keeps the Bridge out of a Drone's transcript ([`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.9).

## Dependencies

The Job index and the transcripts. No network. Does not need the repository the Jobs
branched from.

## Exit codes

`0` whenever the index is readable — `ls` reports rather than judges · `6` `environment` — the index is unreadable.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`inbox.md`](inbox.md) · [`spawn.md`](spawn.md) · [`board.md`](board.md)
