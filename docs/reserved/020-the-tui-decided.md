---
id: 020
title: The TUI, decided
status: RESERVED
module: helm
raised: a design session after real use, 2026-08-15
---

# The TUI, decided

**What this is.** Nine decisions taken in one session, after the user drove the Bridge for the
first time and found it wanting. Each is his answer to a question with the trade stated, and
each section says whether it has been built.

**Status: eight of the nine are built.** §3 is the one that is not, and it is waiting on
[`021`](021-the-work-hierarchy.md)'s rename rather than on anybody's time. What each of the rest
means, section by section, is in the table below.

| § | Built | What landed |
|---|---|---|
| 1 · the Stop hook ticks the Job | **yes** | every Drone carries `--settings` with a `Stop` hook that waits for its own process to go and then ticks |
| 2 · the chain breaks silently | **yes** | the relay sweeps the **whole fleet**, and Helm's own `Stop` hook sweeps too — two mechanisms, [`PLAN.md`](../PLAN.md) §15.3's shape |
| 3 · one signals listing | **designed, not built** | [`021`](021-the-work-hierarchy.md)'s *"The design, decided 2026-08-17"* settles all eleven decisions — the verb is `arm inbox`, it absorbs `fleet inbox` too, and the screen half is [`035`](035-the-bridge-becomes-a-ratatui-application.md). `021`'s own change list had to be corrected first: the five things do **not** share one store, and `untried` stays out |
| 5 · an action gets a state word | **yes** | `Acting` and `Job.doing` were the state machine; the three surfaces that draw a Job now read them — `fleet ls`, the Bridge's table and its detail pane all say `ABORTING · docker 12s…` |
| 6 · `SILENT` and `STALLED` | **yes** | both are real Job states, told apart by the tick watermark and by whether the Drone said anything |
| 7 · `QUEUED`, not `WAITING` | **yes** | `Status::Queued`, in the enum and in [`glossary.md`](../glossary.md)'s status table |
| 8 · the menu | **yes** | bare `armada` is the front door — five modules, a status word and a fact each, Helm first. [`PLAN.md`](../PLAN.md) §15.1's rationale is kept and marked superseded |
| 9 · Helm opens beside you | **yes** | `↵` hands a Job's worktree to `cmux` and the Bridge keeps drawing; where cmux is absent it is the `board` handoff it always was |

## What §2 actually needed, and what it got

The relay has three ways to be lost — a SIGKILL, a hook that could not run, a crash in between —
and **no amount of care inside one hook fixes any of them.** What does:

1. **Every relay sweeps every Job.** A Drone's hook runs `armada fleet tick` with no handle, so a
   Job whose own relay was lost is picked up by the next Drone *anywhere on the machine* to
   finish an exchange. A pass over an idle fleet is a directory listing, a transcript tail and a
   `ps`, so the sweep costs nothing.
2. **Helm's `Stop` hook sweeps as well.** You cannot talk to the orchestrator without ending a
   turn, so asking Helm anything at all catches the fleet up. This is the same pairing
   [`PLAN.md`](../PLAN.md) §15.3 already uses for the inbox: the monitor is timely, the hook is complete.

**A read verb still does not tick, and that refusal is deliberate.** `armada fleet ls` advancing
a Job behind the reader's back breaks [`PLAN.md`](../PLAN.md) §15.1 — §1 below rejects it by
name. Reporting a Job as `STALLED` is honest; doing the work unasked is not. The repair for the
symptom is that `STALLED` is now a word you can *see*, which it was not before.

## The residual, stated rather than hidden

**One link in the relay is not provable by any test in this repository**, and it is worth
naming: that Claude Code runs a `Stop` hook registered through `--settings`. Proving it needs a
real session, and no test here may start one ([`PHASES.md`](../PHASES.md) §8.5). Everything
either side of it is proved:

| Link | How |
|---|---|
| the spawned Drone's argv carries `--settings <path>` | asserted on what `execve` received, not on what Armada built |
| that path holds a document registering an executable hook | the file is read back and parsed |
| that hook, run for real, ticks the fleet once its Drone has gone | the generated script is executed as a child of a real process group that then exits |
| a tick rescues a Job whose relay was lost | a Drone finishes an exchange, nothing relays, and the sweep advances the step |

The unproved link is the same mechanism `armada helm` has used since M0 — [`PHASES.md`](../PHASES.md) §9.1 F3
measured it — and `armada doctor` holds `--settings` against `claude --help` on every run, which
is what catches it disappearing.

**The sweep costs one thing, and it is paid for**: five exchanges ending in the same second start
five passes over the same records, and two passes gating one step would both `claude --resume`
one session. `~/.armada/tick.lock` serialises them, and a second pass declines rather than
queueing — the pass that holds the lock is walking the same records.

It supersedes nothing and completes two reservations:
[`003`](003-bridge-command-centre.md) — the Bridge as a command centre — and the parts of
[`001`](001-raised-items-need-identity.md) about where a raised item is acknowledged.

## What is built

**Eight of the nine, and all six notes under "Also decided".** The one left is §3, and it is
blocked on a rename rather than on effort.

| Decision | State |
|---|---|
| §1 the Stop hook ticks the Job | **built** |
| §2 the detail pane's `SAID` row | **built** |
| §3 one signals listing, origin a filter | **designed** 2026-08-17 — [`021`](021-the-work-hierarchy.md), plus [`035`](035-the-bridge-becomes-a-ratatui-application.md) for the screen. Not built; [`PLAN.md`](../../PLAN.md) is the plan |
| §4 window usage first, dollars second | **built** |
| §5 `ABORTING` / `REAPING` / `PAUSING` | **built** — the word *and* the rendering |
| §6 counts, never an aggregate word | **built** |
| §7 `QUEUED`, not `WAITING` | **built** |
| §8 the menu, and what bare `armada` becomes | **built** |
| §9 Helm opens beside you, in cmux | **built** |
| the wide layout · `NEEDS YOU` carries the question | **built** |
| `SILENT` and `STALLED` as real states | **built** |
| the id is shown | **built** |
| a new Job is spawned detached | **built** |
| the tagline | **built** |

### What §5 needed beyond the state machine, and what it got

The record already carried the transient — `Acting` and `Job.doing`, written by `kill`, `reap`
and `pause` — and **nothing drew it**, which is the half a reader can see. Three surfaces draw a
Job and all three now prefer the action to the state: `armada fleet ls`, the Bridge's table and
the Bridge's detail pane. The word is composed once, on the payload, precisely because the
failure this section is about is one surface remembering and another forgetting.

**The slow part takes whichever column answers *what is it doing now*** — `DETAIL` in `ls`,
`STEP` on the Bridge — because while somebody is aborting a Job, the abort *is* what it is doing.
Both come back the moment the action settles.

**The reader's clock does the subtraction.** `Doing` records the wall-clock millisecond a stage
was entered, because that is the only honest thing the acting process can write down: it is
blocked inside `manifest clean` and does not know when anybody will look. `JobRow.acting_for_s`
is that subtraction, taken where somebody is looking — the same shape `step` and `on_step_s`
already have.

### What §8 and §9 each turned out to need

**§8 is a verb, not a page.** Bare `armada` was the `--help` root page under the wordmark; it is
now `armada_helm::verbs::menu`, which reads five modules and prints `STATUS · MODULE · DETAIL ·
VERB`. `Topic::Bare` is gone rather than left unreachable — a second front door nothing draws is
one the next reader finds and wires back up.

**The `VERB` column varies with the row's state, and that is what makes the row usable.** A
column that always said `armada helm` would, beside `DOWN`, advertise the one command that
refuses; it says `armada helm enable` there. Same for `armada manifest init` over an unclaimed
directory and `armada init` over an absent guild. The Bridge's `p` key already worked this way.

**§9 needed one line of cmux's CLI, and it is measured rather than guessed.** `cmux <path>` —
a bare path, no subcommand — opens a directory in a new workspace and launches cmux if needed.
Every plausible guess (`cmux open`, `cmux new`, `cmux workspace`) is wrong, and a guessed
subcommand would exit non-zero into a one-line notice: a key that appears bound, appears to run
something, and never opens anything. [`traps.md`](../traps.md) records the measurement. Detection
is `cmux --help`, checked for that literal form, so a cmux whose CLI moves on falls back rather
than failing quietly.

**One departure from what is written below, and the arithmetic forced it.** The wide layout
sheds `WORKFLOW`, then `TURNS`, and then — where this page names only those two — **`TASK`**,
before `NEEDS YOU` truncates. Measured against the Bridge's own fixture, a table carrying both
flexible columns needs eighty-one columns, so at eighty it would overhang the screen rather than
truncate inside it and one of the two had to go. It is `TASK`, because a Job's handle is already
two significant words of its task, and because `NEEDS YOU` is the only column that is ever a call
to action. The trade, stated: an eighty-column Bridge with something waiting on you shows the
question and not the task.

**And one thing this page assumed that the event does not carry.** `window 71%` is measured, but
the percentage rides along only once the window crosses a threshold — Claude Code sends
`utilization` on the `allowed_warning` shape and not on every event. So the line draws the
percentage when it has one and the reset alone when it does not, rather than computing a number
nobody measured. That is the same rule that keeps a progress bar off this screen.

## 1 · The Drone's Stop hook ticks the Job

**The bug.** A Drone runs one exchange under `--print` and exits — correct. Nothing observed
that, so a Job read `RUNNING` with a `GONE` Drone until somebody typed `armada fleet tick`.
The user lost eight hours on a dead Job and had no way to know.

**Decided: the Stop hook ticks it.** Event-driven, no daemon, no polling.
[`PLAN.md`](../PLAN.md) §15.3 already argues it — *"hooks are the spine — an agent can forget to
report progress, but it cannot forget to stop"* — and the exchange ending **is** the event.

**Why not the alternatives.** The Bridge ticking on its redraw cadence breaks [`PLAN.md`](../PLAN.md) §15.1's *"the
Bridge holds no state"*: watching would change what is watched, and a Job would only advance
while somebody had the screen open. Any `fleet` verb ticking first makes a read verb mutate.
A manual key is honest and is the complaint, not the fix.

## 2 · The detail pane shows the last reported line

**The bug.** A Drone answered the user in **prose** instead of calling `fleet.ask_human`. Zero
inbox entries were written, so the Bridge correctly said *"nothing open to answer"* while he sat
waiting. Verified against the stream, not guessed.

**Decided: surface the Drone's own last summary as a `SAID` row.** A summary is what [`PLAN.md`](../PLAN.md) §15.2
permits; a transcript is what it forbids. The Drone brief (`019`) makes the tool call more
likely, but a prompt is guidance and not a guarantee — this is the backstop.

## 3 · One noun, one verb; origin is a filter

**The complaint, in his words:** *"it feels like I'm running in circles."* Five entry points,
five nouns, one file — and `armada report` filing into `armada failures` is the sharpest edge,
because the verb and the view disagree.

**Decided: one listing, with origin as a filter.** `armada report` and `armada task` remain as
ways to **write**; there is one place to **look**. This is what `001` actually asked for — it was
right about storage and the surface never followed.

## 4 · Window usage first, dollars second

**Nearly free, and the finding is the point:** `crates/core/src/fleet/drone.rs` already parses
Claude Code's `rate_limit_event` into a `RateLimit` carrying the `five_hour` window and its
`resetsAt`. **Nothing in Helm has ever read it.** Every exchange has reported his window position
and Armada has discarded it.

**Decided:** `window 71% · resets 2h14m` leads; spend stays. What stops him working outranks what
it cost.

## 5 · Every action with a duration gets a state word

**The bug.** He aborted a Job, pressed `y`, and the screen said nothing for several seconds while
`manifest clean` talked to docker. The abort **succeeded** — a working abort and a hung one looked
identical.

**Decided:** `ABORTING` / `REAPING` / `PAUSING` as the row's status, with the slow part named
beside it (`docker 12s…`). The status word carries it, so no spinner and no bar. `reap`, `tick`
and `pause` all have the same shape.

**Built, in two halves that landed apart.** The state machine came first — `Acting`, and
`Job.doing` written to the record by `kill`, `reap` and `pause`. Nothing drew it, which meant the
bug above was still on the screen: a working abort and a hung one looked identical because both
rows kept saying `RUNNING`. The rendering is now on all three surfaces that draw a Job —
[`fleet ls`](../commands/fleet/ls.md), the Bridge's table and its detail pane — and the word is
composed **once**, on the payload, because the failure mode here is one surface remembering the
precedence and another forgetting it.

**`Job.doing` is on disk because the process doing it is not the process drawing it**, and that
is not incidental — it is the mechanism. The terminal running the abort is blocked inside
`armada manifest clean` and can draw nothing; the only thing that can report `docker 12s…` is
another reader of the same record. The elapsed second is therefore subtracted against *the
reader's* clock rather than written by the actor, which is the same shape `step` and `on_step_s`
already have.

**A Job being aborted is still `RUNNING` on disk**, which is why `Acting` is a fourth enum and
not three more Job states: folding them together would leave a crash mid-abort with a Job
claiming a state no verb ever reached.

## 6 · No aggregate status over several Jobs

His objection: *"what's the point of an aggregate status when multiple jobs are running?"* — and
he is right. A word derived from the worst row describes no Job in particular.

**Decided:** counts, which cannot be wrong — `4 jobs · 2 need you · 1 stalled`. A **single** Job
keeps its status word, because there it means something.

## 7 · `QUEUED`, not `WAITING`, for a step not yet reached

His objection: *"WAITING sounds like it's waiting for something to happen but in fact the
workflow is just not at this step yet."* **Decided: `QUEUED`.**

## 8 · The menu, and what bare `armada` becomes

**Decided: bare `armada` opens a menu of the modules** — Helm, fleet, inbox, manifest, guild —
each with a status word and a one-line fact. Helm is first, because it is who you talk to.

**This changes [`PLAN.md`](../PLAN.md) §15.1**, which says bare `armada` enters Helm. Both cannot be the default, and the
change is deliberate: entering Helm is off by default on a machine anyway (`helm.enter`), so a
front door that lists everything is the more honest default. **[`PLAN.md`](../PLAN.md) §15.1's rationale must be rewritten
rather than deleted.**

**Built.** The rows are `STATUS · MODULE · DETAIL · VERB`, drawn under the wordmark — bare
`armada` is the moment of orientation, and the wordmark moved with it rather than staying on a
help page nothing reaches. [`PLAN.md`](../PLAN.md) §15.1 keeps its argument and carries a note
saying what superseded it, which is what *rewritten rather than deleted* meant.

**Two things the section did not say, decided while building it.**

- **The fact is a fact and the verb is the instruction.** *"no guild yet"* on the row, `armada
  init` in the `VERB` column — not one sentence carrying both. Left together, the fact grows
  until the flexible column truncates it, and a truncated command is not a shorter answer, it is
  the wrong one ([`commands/fleet/board.md`](../commands/fleet/board.md) makes the same argument
  about a resume line).
- **The verb varies with the row's state.** A column that always said `armada helm` would, beside
  `DOWN`, be advertising the one command that refuses; it says `armada helm enable` there. The
  Bridge's `p` key — *pause* over a running Job, *resume* over a paused one — already worked this
  way, and for the same reason.

**No new status words.** Every row uses [`glossary.md`](../glossary.md)'s existing `Status`:
`READY` / `DOWN` for the three that describe setup, and `WAITING` / `RUNNING` / `OK` for the two
that move. `DOWN` is Manifest's own word for *not standing up*, which is what an off switch, an
unclaimed directory and an absent guild each are.

## 9 · Helm opens beside you, not inside the TUI

He asked for a full chat pane — *"a TUI in which I can do everything, including talking to the
Helm"* — then chose the cheaper shape once the cost was laid out.

**Decided: the menu and the panes now; `enter` on Helm opens a cmux workspace and the TUI stays
up.**

**Why, and it is worth keeping.** The transport is not the problem — Armada already drives Claude
Code as a two-way `stream-json` channel, because that is exactly how every Drone runs. The problem
is that **rendering a live conversation is a terminal chat client**: streaming text, tool calls,
scrollback, resize, interrupt. Claude Code is already an excellent one, and Armada would be
rebuilding it to avoid a screen handoff — owning every rough edge to save one keypress.

**Not closed forever.** If the handoff still annoys him after living with the panes, the transport
is already there.

**Built, and one thing it needed that this page could not know.** `↵` on a Job hands its worktree
to `cmux` and the Bridge keeps drawing; where cmux is absent, `↵` is the `armada fleet board`
handoff it has always been, so a machine without a multiplexer loses nothing. The argv is `cmux
<path>` — a bare path with no subcommand — and it is measured rather than guessed, in
[`traps.md`](../traps.md): every plausible name for a subcommand here is wrong, and a wrong one
would exit non-zero into a one-line notice, which is a key that appears bound and never opens
anything.

**The distinction that made this an `Action` rather than a `Departure`.** `board --exec`
*replaces this process*, so there is nothing left to draw with; handing a path to a multiplexer
starts something else and returns. That is the whole mechanism behind *"the panes stay up"*.

## What it costs

| Piece | Size |
|---|---|
| The menu; fleet and inbox panes | small — the selector and both listings exist |
| Wide layout; window usage | small — column-drop exists, `RateLimit` is parsed |
| Manifest and guild panes | medium — new renderers over verbs that already answer |
| Bare `armada` changes meaning | medium — spec, docs, help, [`PLAN.md`](../PLAN.md) §15.1's rationale |
| **Holding `ARCHITECTURE.md` §1.9** | **the real risk** |

**The risk is not effort.** The Bridge may *read* all four modules; it must never become the place
where they read *each other*. One screen touching everything is exactly how that boundary erodes,
and nothing in a test will catch it — `cargo xtask boundaries` checks the crate graph, not a
renderer's habits.

### How the menu holds it, written down rather than intended

`boundaries` cannot see this, so the discipline is three rules in
`crates/helm/src/verbs/menu.rs`'s header, and two of them are asserted:

1. **One function per row, each given only its own module's input.** `helm_row` takes a path,
   `fleet_row` a Fleet `Where`, `guild_row` a `Guild`. None can read another module's data
   because none is handed any — a property of the signatures rather than of anybody's care.
2. **No row's outcome gates another's.** All five are computed unconditionally. The tempting
   shortcut — *"there is no workspace here, so skip the fleet row"* — is Manifest deciding what
   Fleet reports, and it is precisely the erosion above. Asserted against a machine with nothing
   on it and a `Run` that fails every call: all five rows still appear, in order, with a word.
3. **No aggregate.** There is no headline over the five, so there is no field anywhere that would
   have to be computed from two modules at once. `020` refuses an aggregate over several Jobs
   because a word derived from the worst row describes none of them; here that argument happens
   to double as the structural guarantee.

## Also decided

- **The Bridge table grows into a wide terminal**, shedding `WORKFLOW`, then `TURNS`, then
  truncating `NEEDS YOU` — the same priority-drop the key line already does.
- **`NEEDS YOU` carries the question**, not `YES`, so most answers need no second screen.
- **`SILENT` and `STALLED` become real states.** Both were drawn as `RUNNING`, which is how a dead
  Job read as alive.
- **The id is shown.** Generated handles like `now-that` are unreadable; the handle stays for
  typing, the id is what you trust.
- **A new Job is spawned detached** so the screen is never handed back — its arrival in the table
  *is* the progress.
- **The tagline becomes `agents that work while you do not`**, replacing *"one vocabulary for a
  repo's stack, and the agents working in it"*, which described an internal design goal rather
  than what the tool does for you. It lives in `crates/helm/src/render/help.rs`.

## Left open

The detail view's proposed `r retry step` and `b budget` keys were offered and not chosen either
way. Neither exists; both are worth having if a Job ever hits a ceiling and stops.
