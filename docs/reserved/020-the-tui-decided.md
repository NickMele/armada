---
id: 020
title: The TUI, decided
status: RESERVED
module: helm
raised: a design session after real use, 2026-08-15
---

# The TUI, decided

**What this is.** Nine decisions taken in one session, after the user drove the Bridge for the
first time and found it wanting. Each is his answer to a question with the trade stated.

**Status: the functional half of §1, §2, §5, §6 and §7 is built.** What that means, section by
section, is in the table below; everything not named there is still the brief for building it.

| § | Built | What landed |
|---|---|---|
| 1 · the Stop hook ticks the Job | **yes** | every Drone carries `--settings` with a `Stop` hook that waits for its own process to go and then ticks |
| 2 · the chain breaks silently | **yes** | the relay sweeps the **whole fleet**, and Helm's own `Stop` hook sweeps too — two mechanisms, [`PLAN.md`](../PLAN.md) §15.3's shape |
| 5 · an action gets a state word | state machine only | `Acting` and `Job.doing`, written by `kill`, `reap` and `pause`; the rendering is not built |
| 6 · `SILENT` and `STALLED` | **yes** | both are real Job states, told apart by the tick watermark and by whether the Drone said anything |
| 7 · `QUEUED`, not `WAITING` | **yes** | `Status::Queued`, in the enum and in [`glossary.md`](../glossary.md)'s status table |

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
