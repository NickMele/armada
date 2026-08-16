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
looks at the two things that can be looked at and says what is actually true. `STALLED` and
`SILENT` are the two that could only ever come from here — both are conditions a busy Drone
cannot report about itself ([`../../PLAN.md`](../../PLAN.md) §14.3):

| Word | Means |
|---|---|
| `STALLED` | its Drone is gone and nothing has ticked the Job |
| `SILENT` | its Drone ended an exchange with neither a verdict nor a question, so what it decided is only in a transcript nothing reads |

**Both were drawn as `RUNNING` until [`020`](../../reserved/020-the-tui-decided.md) §6**, which
is how a dead Job read as alive for eight hours: a Drone that had exited got the same word as a
Drone that was working. What tells them apart from rest is the tick watermark on the record —
whether anything has gated the exchange that just ended.

**A Job resting between exchanges is `RUNNING` for as long as the relay takes**, which is a
second or so ([`tick.md`](tick.md)). If it is longer than that, the relay was lost, and the row
now says so.

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
  STALLED  xlsx-report    3d9cc7ba  bug       Drone gone, not ticked $4.60   22m
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

### An action with a duration takes the status column while it runs

```
  STATUS    JOB         ID        WORKFLOW  DETAIL       SPENT  TIME
  ABORTING  rate-limit  c19d0a34  feature   docker 12s…  $2.10   14m
```

**A working abort and a hung one used to be the same screen.** Aborting a Job talks to docker,
and for the several seconds that took, the row said `RUNNING` and nothing else moved. That is
the bug [`020`](../../reserved/020-the-tui-decided.md) was written around, and the answer is a
word: `ABORTING`, `REAPING` or `PAUSING` in the status column, with the slow part named in
`DETAIL`.

**The Job is still `RUNNING` on disk, and that is deliberate.** `Acting` is a fourth enum rather
than three more Job states ([`../../glossary.md`](../../glossary.md)) — folding them together
would leave a crash mid-abort with a Job claiming a state no verb ever reached. The row lays one
over the other; the record keeps them apart.

**The one that reports it is never the one doing it.** The terminal running the abort is blocked
inside `armada manifest clean`, so it cannot draw anything — which is why the transient is
written to the Job record, and why a *second* reader of that record is what makes
`ABORTING · docker 12s…` appear at all. Run this in another window during an abort and that is
what you see.

**No spinner and no bar.** Both halves here are measurements — a stage name somebody wrote down
and a subtraction against your clock — and a fraction of an abort would be a guess drawn as a
measurement ([`../../PHASES.md`](../../PHASES.md) §9.1). The status word is what says something
is running; that is the whole reason a bar is refused. An action that has not reached anything
slow yet gets the word and no stage, for the same reason a Job with no measured boundary gets no
`0s`.

`--json` returns one result per Job with `uuid`, `name`, `workflow`, `state`, `detail`, `task`,
`runtime_s`, `cost_usd`, `tokens`, `turns`, `budget_remaining` and `needs_attention` — plus
`acting` and `acting_for_s` while somebody is acting on it, and neither field when nobody is.

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
