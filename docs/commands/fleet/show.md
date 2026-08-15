# `armada fleet show`

One Job in full — and **why it needs you**.

> **Status: built — M3.** The layout is frozen by `tests/golden/render/fleet-show.plain` and its
> `.tty` twin, with `fleet-show-gone` beside them.

## The defect it closes

A Bridge row said this, and there was nowhere to go from it:

```
  STATUS  JOB        TASK                                                     RUN  SPENT  NEEDS YOU
  PAUSED  this-test  this is a test, please don't do any code work. I want …   8h  $0.03  YES
```

`NEEDS YOU: YES` with no way to find out why is the defect. The answer already existed — the
inbox entry that raised the flag holds it ([`inbox.md`](inbox.md),
[`PLAN.md`](../../PLAN.md) §15.3) — and no view printed it. A row cannot: `TASK` is truncated
to a column, and [`ls.md`](ls.md)'s `DETAIL` is already the *fold* of two different facts, the
open entry's body when there is one and the step name when there is not. Neither half can be
recovered from the fold.

**This is what the row was cut from.**

## Synopsis

```sh
armada fleet show <job> [--json]
```

`d` on the selected row in [`../helm/bridge.md`](../helm/bridge.md) draws the same payload
without leaving the screen.

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name | — | Which Job. Required. |

## How it works

One read of the Job record, one of the inbox, and the same observation
[`ls.md`](ls.md) makes — the transcript and the process table. It is **read-only**: it does not
persist what it observed and does not raise a second inbox entry for it, because watching
something must not change it ([`PLAN.md`](../../PLAN.md) §15.2). That is what lets the Bridge
re-read it every interval.

**Nothing here is explained twice.** Every state word is Fleet's own, every entry body is the
inbox's own, and the step is the record's. This verb gathers; it does not rephrase. Two
components wording one state differently is a defect that only surfaces when the two are read
side by side.

### What it shows that a row cannot

| Block | Answers |
|---|---|
| The identity row | The row you came from — same Job, same shape |
| `ASKED` | **Why it needs you.** Every entry this Job raised, each in its own words, with the id `answer` takes |
| `TASK` | What it was asked to do, whole and unwrapped by any column |
| `RECORDED` / `ALIVE` `GONE` `NEVER` / `HELD` | Three facts that only disagree when something is wrong |
| `SPENT` / `LEFT` | Turns, tokens and wall clock **against their ceilings** |
| `REPORTED` | The Drone's own `fleet.report` notes, newest first |

**The three separated facts are the point of the third block.** A Job whose record says
`RUNNING`, whose Drone's process group is no longer Armada's, and whose port block is still
claimed with nothing listening on it reads as healthy in every other view — because every other
view folds the three into one state word. Here they are three rows.

> **The task and the question are prose; everything else is a column.** They are the two values a
> column would have to truncate, and a truncated answer to *why does this need me* is not a
> shorter answer — it is the wrong one.

> **There is no progress column and no percentage**, here least of all. A detail view is exactly
> where one would look like a measurement. Nothing emits percent-complete
> ([`PHASES.md`](../../PHASES.md) §9.1 F2), so what is honest — turns, tokens and wall clock
> against their ceilings — is drawn as the numbers it is.

**It never prints a transcript.** The activity block is the Drone's own notes. The orchestrator
reads summaries and never raw transcripts ([`PLAN.md`](../../PLAN.md) §15.2), and a detail view
is the surface where that constraint would erode first.

**Entries raised before this Job existed are left out.** A handle is reusable once a Job is over,
and a fresh Job opening with its namesake's week-old question is the worst possible answer to
"why does this need me".

## Output

```
  STATUS   JOB            WORKFLOW  STEP                        SPENT  TIME
  BLOCKED  release-merge  feature   implement, attempt 2 of 15  $1.25    1h

  STATUS    ASKED     TIME
  ANSWERED  e30b91aa   47m
    should the 4.2 tag be signed with the release key?
    you said: yes, and push the tag once check is green
  BLOCKED   e4f1a2c9    9m
    the release branch carries two migrations that both rename the orders index.
    Squash them into one, or revert 0042 and re-cut it? Both are safe; the
    second loses the rename.

  TASK
  merge the release branch and cut 4.2, resolving the migration conflict in
  orders before the tag goes out

  STATUS    FACT      DETAIL
  RECORDED  state     RUNNING, as a verb last wrote it
  ALIVE     drone     process group 48122
  SPENT     budget    4 of 15 turns, 119k of 400k tokens, $1.25
  LEFT      budget    11 turns, 280k tokens, 25m
  SINCE     started   2026-08-09T14:02:11Z
  HELD      ports     5470-5479
  HELD      worktree  ~/.armada/workspaces/orders/release-merge
  HELD      branch    armada/release-merge
  FROM      repo      orders

  STATUS    STEP       DETAIL                                           TIME
  REPORTED  implement  rebased onto main, two migration conflicts left    6m
  REPORTED  plan       approach agreed: squash the two migrations        41m

BLOCKED  release-merge, feature, needs you, 1 open, $1.25
```

**A Job that needs you and has raised nothing has no `ASKED` block**, and that is the answer
rather than an omission: `PAUSED` and `BLOCKED` need a person by definition
([`../glossary.md`](../../glossary.md)), so the reason is the state word in the first column.

`--json` returns the whole payload — the identity, both states, `drone_alive`, the step and
attempt, the untruncated task, `budget` and `budget_remaining`, the paths and port block, the
entries as [`inbox.md`](inbox.md)'s own rows, and the notes.

## Palette

One palette, shared with every coloured surface Armada renders, defined in
[`../render.md`](../render.md).

## Dependencies

An existing Job. No network, no repository, no daemon — it runs anywhere `~/.armada/` is
readable.

## Exit codes

`0` reported · `2` `bad_invocation` — unknown Job, or no Job named.

**A Job that is `BLOCKED`, `STALLED` or out of budget still exits 0.** The Job's state is not the
command's, and reporting a stuck Job successfully is what this verb is for.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`ls.md`](ls.md) · [`inbox.md`](inbox.md) · [`answer.md`](answer.md) ·
[`board.md`](board.md) · [`../helm/bridge.md`](../helm/bridge.md)
