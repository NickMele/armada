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

## The command centre

[`033`](../../reserved/033-the-command-centre-designed.md) is the design; this section is what of
it is built. At 138 columns and above, seven boxes: `ARMADA` (the workspace and both usage
windows), `JOBS`, `INBOX`, `MANIFEST`, `GUILD`, `SYSTEM`, `KEYS`. Under that threshold, four:
`ARMADA`, `JOBS`, `INBOX`, `KEYS` — whole panels drop rather than shedding columns, because
`MANIFEST`/`GUILD`/`SYSTEM` have nothing left to shed once their own minimum content overhangs.

`tab` cycles focus through all five row-bearing and read-only panels; `1`-`5` jump to one
directly. Only `JOBS` has a verb wired to a row today — `enter`, `d`, `a`, `x`, `p`, `r` all act on
the focused Job and are inert while a different panel is focused, because acting on a row the
reader cannot see would be worse than the key doing nothing.

**Two gaps, named rather than hidden**, both because they need App-level state
(`crates/helm/src/app.rs`) the Bridge's read path was built to do without:

- `MANIFEST` draws one row saying it is not wired — `check::status`/`status::run` need
  `App<R, C, F>`, which opens `manifest.db` and reads `MachineConfig`. Wiring it in is a real fix
  (build the App once at `watch()`'s entry, not per redraw) but it reverses a deliberate line in
  `main.rs`'s dispatch, so it is a decision rather than a Drone's to make unilaterally
  (`PLAN.md`'s audit table).
- `INBOX` is reachable by `tab`/`2` but has no cursor of its own yet — no verb acts on one of its
  rows today, so the churn of threading a row count through `core::fleet::bridge::press`'s 88 test
  call sites is not yet earned.

`d`, or `enter` from `INBOX` once it has a cursor, opens the Job detail full screen: identity,
`WORKFLOW` beside `NEEDS YOU`, `TIMELINE · what the gate did` full width, `REPORTS · the Drone's
own words` beside `FACTS`, and its own key line. `TIMELINE` and `REPORTS` are two separate tables
reading two separate fields (`transitions`, `progress`) — 033's own point, restated here: one is a
machine's decision carrying the predicate and exit code that settled it, the other is an agent's
summary, and collapsing them is how a Drone's claim gets read as gate evidence.

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
| `STATE` | Fleet Job state | `QUEUED` `RUNNING` `PAUSED` `STALLED` `BLOCKED` `ABORTED` `DONE` ([`../glossary.md`](../../glossary.md)). |
| `JOB` | the Job index | The name, not the uuid — names are assigned at spawn and are what you type. |
| `ID` | the Job index | **The eight characters `armada fleet ls` prints.** A derived handle like `now-that` is unreadable; the handle stays for typing and this is what a person trusts, and what matches a row against the other listing. |
| `WORKFLOW` | the Job index | *Wide terminals only.* |
| `STEP` | the record, plus how long it has been on it | A step and an elapsed time, never a fraction — *"three of five steps"* would be the banned progress bar written in words. |
| `TASK` | the spawn prompt | Truncated, and *shed before `NEEDS YOU`*: a Job's handle is already two significant words of its task. |
| `RUN` | wall clock since spawn | — |
| `TURNS` | `num_turns`, summed over turns | *Wide terminals only.* A count, never a fraction of the ceiling. |
| `SPENT` | `total_cost_usd`, summed over turns | Against the Job's ceiling, so exhaustion is visible before it happens ([`PLAN.md`](../../PLAN.md) §14.3). |
| `NEEDS YOU` | the inbox | The only column that is ever a call to action, and **it carries the question** — the open entry's own words, so most answers need no second screen. `YES` only where a Job's state wants a person with nothing raised against it. |

**The table grows into a wide terminal.** `WORKFLOW`, `TURNS` and `TASK` are carried when there
is room and shed in that order when there is not — the same priority-drop the key line does, and
for the same reason: a row that wraps stops lining up with its header, and a row that overhangs
loses its right-hand columns to the viewport without saying so.

**Which columns a given width carries depends on the fleet**, not on a threshold written down
here: how wide a table needs to be depends on how long the Jobs happen to be called. Measured
against the frame below, `TASK` returns at 81 columns, `TURNS` at 88 and `WORKFLOW` at 98 —
which is why an eighty-column terminal, the one this page's output is drawn at, carries none of
the three.

> **There is no progress column, deliberately.** Nothing emits percent-complete: F2 gives cost,
> tokens, turns and duration, none of which is progress toward a goal an agent has not finished
> defining. A bar computed from turn count would be a confident guess rendered as a measurement,
> which is worse than no column at all.

### Keys

| Key | Does | Verb it calls |
|---|---|---|
| `↵` | Board the selected Job — **in a cmux workspace where there is one, and the screen stays up** | [`../fleet/board.md`](../fleet/board.md) |
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
| `↑` `↓`, `k` `j` | Move the cursor **on the focused panel** | — |
| `tab` | Next panel, wrapping | — |
| `1`-`5` | Jump to a panel directly — `JOBS`, `INBOX`, `MANIFEST`, `GUILD`, `SYSTEM` | — |

**Movement never sheds; verbs do.** Under the narrow threshold the `KEYS` box keeps every
movement pair and drops verbs from lowest priority first, behind `?` — a dropped verb is one
keypress away; a dropped movement key is a screen you cannot get around.

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

### `↵` opens the Job beside the screen where something can open it

**One key, two destinations, decided by what the machine has** — the decision
[`020`](../../reserved/020-the-tui-decided.md) records as *Helm opens beside you, not inside the
TUI*. Where `cmux` is on `PATH`, `↵` hands it the Job's worktree and **the Bridge keeps
drawing**: `cmux <path>` opens a directory in a new workspace, so something else takes the new
window and this process still owns its own screen. Where nothing is found, `↵` is the
`armada fleet board` handoff described above, unchanged.

**Which it is, is measured once when the Bridge starts** and not per keypress. A multiplexer does
not appear halfway through a session, and a probe behind `↵` would put a subprocess on the hot
path. The probe is `cmux --help`, which starts nothing, and it checks that the bare-path form is
still offered rather than only that the file exists — so a cmux whose CLI has moved on is treated
as absent and the fallback takes over. [`../../traps.md`](../../traps.md) records the
measurement.

**The key line still says `board`, and the word is right either way.** *Step aboard this Job* is
the same intent; only the thing carrying it out differs.

**Why not a chat pane in the Bridge itself.** It was asked for — *"a TUI in which I can do
everything, including talking to the Helm"* — and refused with the cost stated. The transport is
not the problem: Armada already drives Claude Code as a two-way `stream-json` channel, because
that is how every Drone runs. The problem is that rendering a live conversation *is* a terminal
chat client — streaming text, tool calls, scrollback, resize, interrupt — and Claude Code is
already an excellent one. Rebuilding it to avoid a screen handoff means owning every rough edge
to save one keypress. Not closed forever: the transport is already there if the handoff still
annoys after living with the panes.

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

  STATUS   JOB            ID        STEP           RUN  SPENT  NEEDS YOU
  RUNNING  rate-limit     c19d0a34  implement 12m  14m  $2.10  -
  RUNNING  carina-schema  94b1fd2e  plan 3m         3m  $0.45  -
  STALLED  xlsx-report    3d9cc7ba  reproduce      22m  $4.60  -
  BLOCKED  release-merge  7f2ab618  implement 18m   1h  $1.25  the CI timeout i…

4 jobs, 1 need you, 1 stalled, window 71%, resets 2h14m, $8.40 today

  enter board  d detail  n new  p pause  x abort  a answer  ? keys  q quit
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

### The summary line

**Counts over several Jobs, and no word derived from any of them.** Every other summary in this
CLI leads with a verdict about one thing; a fleet has no one thing to be a verdict about, and a
word taken off the worst row — `RUNNING` above four rows of which one is stalled and one wants
an answer — is true of one row and misleading about three. So `4 jobs · 1 need you · 1 stalled`,
which cannot be wrong. **A single Job keeps its status word**, because there it is that Job's
state rather than a summary of anything.

**Window usage leads and spend follows.** `window 71% · resets 2h14m` is the rate-limit window
the fleet is working inside, read off the `rate_limit_event` Claude Code sends on every exchange
([`PHASES.md`](../../PHASES.md) §9.1 F2): what stops you working outranks what it cost.

- **Both halves are measurements, and either can be missing.** The percentage is the service's
  own `utilization`, floored the way Claude Code floors it, and it only rides along once a
  window crosses a threshold. When it is absent the line says the reset and nothing else —
  a percentage nobody measured is the one number this line may not carry.
- **It is not the banned progress bar.** Nothing here is a fraction of a turn count. The service
  is stating how much of a window is gone, which is exactly the kind of number
  [`PLAN.md`](../../PLAN.md) §14.3's ceilings are built out of.
- **It survives a filter**, unlike every other number on the line. The rows are what the filter
  selected; the window is a fact about the account, and `state=RUNNING` does not change how much
  of five hours is left.

`--json` returns one result per Job with the same fields as
[`../fleet/ls.md`](../fleet/ls.md), so a frame and a listing parse identically, plus `running`,
`filter`, `hidden` and `window` — what the *frame* is showing, and what it is not.

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
