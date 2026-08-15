# `armada bridge`

The live screen. Every Job, its state, what it has spent, and who needs you.

> **Status: built — M3.** An earlier draft deferred the ambient view indefinitely;
> [`PLAN.md`](../../PLAN.md) §15.1 records why that was reversed. The frame's layout is frozen by
> `tests/golden/render/bridge.plain` and its `.tty` twin, and by `bridge-filtered` beside them.

Helm is where you talk. The Bridge is what you watch. It redraws in place like `htop` or `k9s`
— no scrollback, no history, just the current state of the fleet.

## Synopsis

```sh
armada bridge [--filter <expr>] [--interval <s>] [--once] [--json]
```

`/bridge` from inside a Helm session opens the same screen.

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--filter` | expression | — | Show only matching Jobs. See [below](#the-filter). |
| `--interval` | seconds | `2` | Redraw cadence. Reads are cheap — no process is interrupted. |
| `--once` | flag | off | Render one frame and exit. For a screenshot, a pipe, or a terminal that cannot hold alt-screen. |
| `--json` | flag | off | Emit one frame as the `--json` envelope and exit. Implies `--once`. |

## How it works

Reads the Job index in `~/.armada/jobs/` and, per Job, the tail of its transcript at
`~/.claude/projects/<slug>/<uuid>.jsonl`. **Every column comes from data Claude Code already
emits** — the turn's `result` event carries `total_cost_usd`, `usage`, `num_turns` and
`duration_api_ms` ([`PHASES.md`](../../PHASES.md) §9.1 F2). The Bridge builds no accounting layer
and estimates nothing.

**Read-only, always.** It never resumes, interrupts, or probes a Drone. Watching something must
not change it — the same rule that governs probe ([`PLAN.md`](../../PLAN.md) §15.2).

**It holds no state.** The Bridge is a renderer over Fleet; closing it loses nothing, and
everything it shows is available from [`../fleet/ls.md`](../fleet/ls.md) as a table.

### The filter

A bare word matches the Job's name, its workflow, its state or its task; `key=value` asks about
one of them. Both are case-insensitive substrings, because `state=running` is what somebody types
at a live screen.

| Expression | Shows |
|---|---|
| `xlsx` | any Job whose name, workflow, state or task contains it |
| `job=rate-limit` | by name |
| `workflow=bug` | by workflow |
| `state=BLOCKED` | by state |
| `task=schema` | by the words the Job was given |
| `needs=you` | only the Jobs waiting on an answer |

**A key nothing knows, or a state word that is not a state, is refused** — `bad_invocation`,
exit 2. Matched against nothing it would show an empty screen instead, and an empty screen is
indistinguishable from an idle fleet.

### Columns

| Column | Source | Notes |
|---|---|---|
| `JOB` | the Job index | The name, not the uuid — names are assigned at spawn and are what you type. |
| `STATE` | Fleet Job state | `QUEUED` `RUNNING` `PAUSED` `STALLED` `BLOCKED` `ABORTED` `DONE` ([`../glossary.md`](../../glossary.md)). |
| `TASK` | the spawn prompt | Truncated. The Bridge is a status view, not a reader. |
| `RUN` | wall clock since spawn | — |
| `SPENT` | `total_cost_usd`, summed over turns | Against the Job's ceiling, so exhaustion is visible before it happens ([`PLAN.md`](../../PLAN.md) §14.3). |
| `NEEDS YOU` | the inbox | The only column that is ever a call to action. |

> **There is no progress column, deliberately.** Nothing emits percent-complete: F2 gives cost,
> tokens, turns and duration, none of which is progress toward a goal an agent has not finished
> defining. A bar computed from turn count would be a confident guess rendered as a measurement,
> which is worse than no column at all.

### Keys

| Key | Does | Verb it calls |
|---|---|---|
| `↵` | Board the selected Job | [`../fleet/board.md`](../fleet/board.md) |
| `d` | **Why does it need me** — the selected Job in full, over the table | [`../fleet/show.md`](../fleet/show.md) |
| `n` | New Job | [`../fleet/spawn.md`](../fleet/spawn.md) |
| `p` | Pause / resume | — · **not built**, and the screen says so rather than swallowing the key |
| `x` | Abort — **then `y`** | [`../fleet/kill.md`](../fleet/kill.md) |
| `a` | Answer the selected Job's question | [`../fleet/answer.md`](../fleet/answer.md) |
| `/` | Filter | — |
| `c` | Drop into Helm | [`helm.md`](helm.md) · **not built** |
| `q`, `esc`, `ctrl-c` | Leave, printing the last frame | — |
| `↑` `↓`, `k` `j` | Move the cursor | — |

Every key maps to a verb that already exists and is reachable from a shell. **The Bridge adds no
capability**, which is what keeps it a rendering choice rather than an architectural one. A key
whose verb does not exist yet says so; it does not grow one here.

### `d` — the detail view

`NEEDS YOU: YES` with no way to find out why was the defect that earned this key: the flag is
raised by an inbox entry, and the frame draws the flag but not the entry. `d` draws
[`../fleet/show.md`](../fleet/show.md)'s payload over the table — the question in its own words,
the whole task, the step, the ceilings, and what the Job is still holding.

**It stays on the screen**, unlike every other key that shows you something. Those hand the
terminal back and run a verb in it; this one answers a question you are already at the Bridge to
ask, and leaving to read the answer would mean coming back to a screen that had moved on. The
pane re-reads on the same cadence as the frame, and `↑`/`↓` move it to the next Job so a fleet is
read one at a time. `esc`, `d` again and `q` all close it; only `ctrl-c` leaves the Bridge.

**`d` is deliberately not on the key line.** That line is seventy-four columns and the shortest
pair worth adding takes it to eighty-one, so a person at a standard terminal would read a wrapped
key line while an agent read a straight one — the same measurement that kept middle dots off it,
and the one thing this whole render is written against. An unnamed key is the cheaper cost, and
this section is where it stops being unnamed.

**`x` asks twice, and anything but `y` declines.** Abort ends a Job, deletes its worktree and
drops its branch. One keypress doing that to whatever row the cursor happened to be on is the
mistake worth one extra character, and a confirmation only one key can refuse is one that gets
answered by accident.

**`n` and `a` need words the screen does not have**, so the Bridge gives the terminal back and
opens the same inline box the interview uses ([`../render.md`](../render.md)). An empty answer
starts nothing.

**Leaving prints the last frame.** The screen is gone and what it was showing is not, which is
the difference between closing a view and losing what you were looking at.

## Output

```
  ARMADA BRIDGE

  STATUS   JOB            TASK                 RUN  SPENT  NEEDS YOU
  RUNNING  rate-limit     add gateway limiter  14m  $2.10  -
  RUNNING  carina-schema  migrate schema        3m  $0.45  -
  STALLED  xlsx-report    generate report      22m  $4.60  -
  BLOCKED  release-merge  merge release         1h  $1.25  YES

RUNNING  4 jobs, 1 need you, $8.40 today

  enter board  n new  p pause  x abort  a answer  / filter  c chat  q quit
```

The live screen is this frame with a caret on the selected row, `LIVE` beside the title, and one
line under the table for the filter box or whatever the last key had to say back.

> **Three departures from an earlier drawing of this page, and each is a rule this repository
> already had.** That drawing put `JOB` first, marked the needs-you column with `●`, and boxed the
> whole thing.
>
> Status is first and always a word in **every** table Armada draws
> ([`../render.md`](../render.md)); a symbol that only appears at a terminal gives the two
> audiences different shapes, which the golden suite asserts of every fixture; and a box drawn in
> text would have to be two different boxes, since the two audiences cannot share one set of
> glyphs. Every column the drawing settled is here — the Job, its state, the task, run time,
> spend and whether it needs you — in Armada's shape rather than in a shape of its own.

**A column no row filled is dropped**, header and all, so `NEEDS YOU` disappears when nothing is
waiting on you. That is the same rule the rest of the CLI follows, and here it means the one
column that is ever a call to action is only ever on the screen when there is one.

`--json` returns one result per Job with the same fields as
[`../fleet/ls.md`](../fleet/ls.md), so a frame and a listing parse identically, plus `running`,
`filter` and `hidden` — what the *frame* is showing, and what it is not.

## Palette

One palette, shared with every coloured surface Armada renders, defined in
[`../render.md`](../render.md).

## Dependencies

Fleet's Job index and the Claude Code transcripts. No network, no daemon, no repository — it
runs anywhere `~/.armada/` is readable.

## Exit codes

`0` clean exit · `2` `bad_invocation` — an unparseable `--filter`, or an `--interval` that is not
a positive number of seconds · `6` `environment` — there is no terminal to take the screen of and
`--once` was not given.

**`--filter` is parsed before the screen is taken**, so a typo is answered on the terminal you are
standing in rather than after it has been blanked.

A key that leaves ends in that verb's exit code: `enter` becomes `claude`'s, `x` becomes
[`kill`](../fleet/kill.md)'s. Armada is not in the middle of it.

Full table and the one rule behind it: [`../reference.md`](../reference.md).

## See also

[`helm.md`](helm.md) · [`../fleet/ls.md`](../fleet/ls.md) · [`inbox.md`](inbox.md) ·
[`../glossary.md`](../../glossary.md)
