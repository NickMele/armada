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
| `n` | New Job — **write the task here, then `ctrl-d`**; the screen comes back | [`../fleet/spawn.md`](../fleet/spawn.md) |
| `p` | Pause a running Job, **resume a paused one** | [`../fleet/pause.md`](../fleet/pause.md) · [`../fleet/resume.md`](../fleet/resume.md) |
| `x` | Abort — **then `y`** | [`../fleet/kill.md`](../fleet/kill.md) |
| `a` | Answer the selected Job's question | [`../fleet/answer.md`](../fleet/answer.md) |
| `r` | Reap — opens a preview, reaps nothing until you confirm | [`../fleet/reap.md`](../fleet/reap.md) |
| `/` | Filter | — |
| `c` | Drop into Helm | [`helm.md`](helm.md) · **the Bridge does not exec on this key**, and it names `armada helm` and whether entering is on for this machine |
| `?` | Every binding, including the ones the key line could not carry | — |
| `q`, `esc`, `ctrl-c` | Leave, printing the last frame | — |
| `↑` `↓`, `k` `j` | Move the cursor | — |

Every key maps to a verb that already exists and is reachable from a shell. **The Bridge adds no
capability**, which is what keeps it a rendering choice rather than an architectural one. A key
whose verb does not exist yet says so; it does not grow one here.

### A key that fails does not take the screen with it

**Every key used to leave.** The Bridge stopped, said which verb it had chosen, and let the
shell run it — so a `kill` that could not remove a worktree took away the view of four other
Jobs in order to say something about a fifth, and said it into a shell nobody was looking at any
more. It was measured: pressing `x` on a Job whose worktree had been deleted answered *"`armada
manifest clean` could not be found to run — reinstall armada"*, ended the screen, and left the
Job `RUNNING` in the record.

So `x`, `p`, `r` and `c` **run where they were pressed**, and their answer — success or failure
— is the line under the table. The frame is re-read immediately afterwards, so what changed is
on the screen by the time the notice is.

**Two keys end it, and each has a reason the others do not**: `q` is the deliberate exit, and
`↵` *replaces this process* with `claude`, so the tty, the signals and the exit code become the
session's. `a` still needs words the screen does not have, so it gives the terminal back and
opens the same inline box the interview uses ([`../render.md`](../render.md)); an empty answer
starts nothing. A `↵` that could not board is a notice like any other failure — only a
successful board ends the frame.

### `n` — the new Job is written here, and the screen comes back

**Three things were wrong with this key and they were one flow.** It ended the screen to ask for
the task, so the fleet was gone while you typed — *"it took me to a screen that felt like it took
me out of the bridge"*. The box it opened advertised no keys, so the only way out of it was a
chord you had to already know — *"there is no help text … I guessed with control-d"*. And having
created the Job it stopped, in a shell, with the one screen that watches Jobs no longer on.

So the task is written **in the Bridge**, in a box drawn under the table with the fleet still
above it, and the box names its keys: `enter` for a new line, `ctrl-d` to start it, `esc` to
start nothing. They are the interview's three, quoted from the same list
([`../render.md`](../render.md)) — a box that meant `ctrl-d` in one place and something else in
another would be worse than either.

**The screen is given back for the spawn itself and taken again afterwards**, which is the one
part `Pressed::Act` could not do. `armada fleet spawn` wants the terminal twice: for its live
progress table, because classification is one call to a model and is the whole of the wait; and
for the workflow question, when the guess is not confident enough to settle on its own. Both are
stderr widgets ([`PLAN.md`](../../PLAN.md) §3.1.1) and neither can be drawn under an alternate
screen. So the Bridge stands down for those few seconds and comes back with the new Job on the
table, the cursor and the filter exactly where they were, and one line saying what was started.

**A spawn that failed comes back too.** Its report is on stderr with everything else the spawn
said, and the sentence is the notice under the table — the rule every other key already follows.

> **A new Job goes `RUNNING` and stays there.** A Drone does one exchange and exits, which is
> `--print` working correctly; nothing yet advances a workflow to its next step. The Bridge shows
> what is on disk and does not pretend otherwise.

### `d` — the detail view

`NEEDS YOU: YES` with no way to find out why was the defect that earned this key: the flag is
raised by an inbox entry, and the frame draws the flag but not the entry. `d` draws
[`../fleet/show.md`](../fleet/show.md)'s payload over the table — the question in its own words,
the whole task, the step, the ceilings, and what the Job is still holding.

**It stays on the screen**, which is what `x`, `p`, `r` and `c` now do too — it answers a
question you are already at the Bridge to ask, and leaving to read the answer would mean coming
back to a screen that had moved on. The pane re-reads on the same cadence as the frame, and
`↑`/`↓` move it to the next Job so a fleet is read one at a time. `esc`, `d` again and `q` all
close it; only `ctrl-c` leaves the Bridge.

**`d` is second on the key line, and the overflow is what pays for it.** It was left unnamed
while the line had no way to shed anything: eight pairs is eighty-four columns, and the choice
then was a wrapped key line — which a person at a standard terminal would read while an agent
read a straight one — or a silent key. Now what does not fit drops in priority order and `? keys`
says so, so at eighty columns `/ filter` and `r reap` move onto the page `?` opens and the key
that answers `NEEDS YOU: YES` is advertised where the reader is looking. `q quit` never drops.

**`x` asks twice, and anything but `y` declines.** Abort ends a Job, deletes its worktree and
drops its branch. One keypress doing that to whatever row the cursor happened to be on is the
mistake worth one extra character, and a confirmation only one key can refuse is one that gets
answered by accident.

**A key names its Job by uuid, not by name.** A uuid is identity; a name is a label, and nothing
enforces that a label is unique — a finished `rate-limit` and a running one are both ordinary and
both on disk. From a shell that ambiguity is refused ([`../fleet/kill.md`](../fleet/kill.md)),
because only you know which one you meant. On the screen there is nothing to be ambiguous about:
the cursor is on exactly one row, and the key carries that row's uuid.

### The key line is one line, and that is a budget

The line must not wrap: a second row would make the frame taller, which moves everything above
it, and the redraw tests assert the height does not change. Eighty columns leaves seventy-eight
for the line, and nine key/word pairs is eighty-two.

So the line **drops rather than wraps**, in priority order — `enter`, `n`, `p`, `x`, `a`, `/`,
`r` — with `q quit` pinned, because a full-screen program that does not say how to leave is a
trap. When anything was dropped the line says `? keys`, and `?` opens the page that lists all of
them.

**`c chat` is not on the line.** It is still bound and still answers, but space on that line is
the scarcest thing the Bridge has and an unbuilt verb does not get any of it while a built one
goes unadvertised.

**Keys are named, never glyphed.** The drawing above uses `↵`; the line writes `enter`, because
it is read by both audiences and a glyph that folds to ASCII would give them different words for
the same key.

### The reap preview

`r` reads [`../fleet/reap.md`](../fleet/reap.md)'s plan and draws it **over** the table — a
different question deserves the whole screen rather than a pane, and two cursors on one screen
is two questions about which row you are on. Rows toggle with `space`, `enter` reaps exactly
what is ticked, and `esc` leaves everything untouched.

Being safe to open out of curiosity is what makes it get read, so `r` reaps nothing and
confirming an empty selection is treated as a keypress that meant something else.

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

  enter board  n new  p pause  x abort  a answer  / filter  r reap  q quit
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

**A key that acts does not change the exit code**, because it does not end the screen: its
answer is a line under the table, and the Bridge still exits `0` when you leave it. That now
includes `n`, which runs a whole verb and comes back — a spawn that failed is a notice and the
Bridge still exits `0` when you leave it afterwards. The one key that ends in another verb's code
is `↵`, which becomes `claude`'s — Armada is not in the middle of it from the `exec` on.

Full table and the one rule behind it: [`../reference.md`](../reference.md).

## See also

[`helm.md`](helm.md) · [`../fleet/ls.md`](../fleet/ls.md) · [`inbox.md`](inbox.md) ·
[`../fleet/pause.md`](../fleet/pause.md) · [`../fleet/resume.md`](../fleet/resume.md) ·
[`../fleet/reap.md`](../fleet/reap.md) · [`../glossary.md`](../../glossary.md)
