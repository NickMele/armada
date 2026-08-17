---
id: 033
title: The command centre, designed
status: RESERVED
module: helm
raised: a design session over a rendered mock-up, 2026-08-16
---

# 033 — The command centre, designed

**What this is.** The design [`003`](003-bridge-command-centre.md) reserved, taken with the owner
over a rendered mock-up he annotated section by section. `003` is the ask and the constraint; this
is the answer, and every decision below is his with the trade stated.

**Why it is written down here.** It was taken over an HTML artifact under `.lavish/`, which is
gitignored — so until this file existed the whole session lived in one untracked file on one
machine. A design that exists only where it was drawn is a design that gets re-argued.

## The instruction

> *"The bridge should be like the bridge on a spaceship. I should have a window into everything."*
> *"We could be utilizing the vertical space so much better. We are restricting ourselves and it's
> limiting the information the bridge can show."*

He named the panels he wanted: manifest checks and leased workspaces; active Jobs with status,
step and iterations; an inbox holding what needs him, failures and reports he can start; guild
stats and quick actions; system health; a command legend; and Claude's own usage — drones running,
the five-hour window and the weekly one, each with a percentage and a time to reset.

## The screen

```text
┌─ ARMADA ───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ armada c24a68b6  ~/Development/armada                  5h ▇▇▇▇▇▇▇▇▇▇▇▇▇▇▁▁▁▁▁▁ 71% resets 2h14m   7d ▇▇▇▇▇▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁ 24% resets 4d│
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ JOBS (4) ───────────────────────────────────────────────────────────────┐ ┌─ INBOX (5) ───────────────────────────────────────────────┐
│ JOB            ID        STATUS   STEP              ITER   SPENT   TIME  │ │ FROM           ORIGIN   DETAIL                     WAITING│
│ rate-limit     c19d0a34  RUNNING  implement 3/4 12m 7/20   $2.10   14m   │ │ release-merge  ASKED    the CI timeout is 30s; ra… 18m    │
│▸carina-schema  94b1fd2e  RUNNING  plan 2/4 3m       2/20   $0.45   3m    │ │ xlsx-report    ASKED    its Drone stopped without… 22m    │
│ xlsx-report    3d9cc7ba  STALLED  reproduce 1/4     4/20   $4.60   22m   │ │ 599657a8       FAILURE  config scan proposed all…  3h     │
│ release-merge  7f2ab618  BLOCKED  implement 3/4 18m 19/20  $1.25   1h    │ │ a41b0c72       FAILURE  arm help → unknown comm…   6h     │
│                                                                          │ │ 7c05e3d1       REPORT   fleet answer rejects the…  9h     │
│  DONE today 3   aborted 1   spent $8.40                                  │ │  /asked   /failure   /report   filter by origin           │
└──────────────────────────────────────────────────────────────────────────┘ └───────────────────────────────────────────────────────────┘

┌─ MANIFEST ─────────────────────────────────────────────┐ ┌─ GUILD ────────────────────────┐ ┌─ SYSTEM ─────────────────────────────────┐
│ CHECK              STATUS   DETAIL       TIME          │ │ skills       14                │ │ drones       2 live                      │
│ armada:test        RUNNING  cargo test   2m45s         │ │ workflows    5                 │ │ docker       up 29.7.2                   │
│ armada:lint        PASS     -            46s           │ │ subagents    1                 │ │ volumes      171 · 12.0 GB               │
│                                                        │ │ remote       in step           │ │ disk         176 GB free                 │
│ WORKSPACE          STATUS   PORTS                      │ │                                │ │ stale pgid   18 · reap                   │
│ armada             LEASED   none declared              │ │  u upgrade  s sync  g ls       │ │                                          │
│ orders             LEASED   5460–5469                  │ └────────────────────────────────┘ └──────────────────────────────────────────┘
└────────────────────────────────────────────────────────┘                                                                                

┌─ KEYS ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ↑↓←→ or hjkl move   tab next panel   1-5 jump to panel   enter act on the focused row   / filter                                       │
│ d detail   n new job   a answer   p pause   x abort   r reap   t tick   ? all keys   q quit                                            │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## At 96 columns

```text
┌─ ARMADA ─────────────────────────────────────────────────────────────────────────────────────┐
│ armada   5h 71% 2h14m   7d 24% 4d                                                            │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ JOBS (4) ───────────────────────────────────────────────────────────────────────────────────┐
│ JOB             STATUS   STEP                SPENT   TIME                                    │
│ rate-limit      RUNNING  implement 3/4 12m   $2.10   14m                                     │
│ release-merge   BLOCKED  implement 3/4 18m   $1.25   1h                                      │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ INBOX (5) ──────────────────────────────────────────────────────────────────────────────────┐
│ FROM            ORIGIN   DETAIL                                                              │
│ release-merge   ASKED    the CI timeout is 30s; raise it                                     │
│ 599657a8        FAILURE  config scan proposed all five                                       │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌─ KEYS ───────────────────────────────────────────────────────────────────────────────────────┐
│ ↑↓←→ or hjkl move   tab next panel   enter act   ? all keys   q quit                         │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

## What shipped differently — corrected 2026-08-17

**The two mock-ups above are kept as drawn, because they are what was agreed.** This section is what
the code does instead, and why. A mock-up quietly edited to match its implementation stops being a
record of a decision.

**The rendering is now a fixture rather than a picture.** `tests/golden/render/bridge-wide.{plain,tty}`
is this screen at 138 columns and `bridge.{plain,tty}` is it at 80, both compared byte for byte on
every run, with a test asserting no line exceeds the terminal. Those files are the authority on the
shape; read them rather than these drawings. They exist because `033` first shipped its panels into
the live `paint()` path only — `armada bridge --once` went on printing the pre-`033` single table at
every width, and no fixture could see either shape. Closed by `75f697c`.

Three things in the JOBS panel differ from the drawing, and each is a decision taken since:

| Drawn | Ships as | Why |
|---|---|---|
| `ITER` column, `7/20` | `TURNS`, a count | `budget.iterations` was **deleted**. It counted model turns, which is not a thing worth bounding — a Job is bounded by cost, wall clock, and consecutive failed gates at one step (`max_attempts`). `7/20` is also a fraction of a ceiling that no longer exists. |
| `implement 3/4 12m` | `implement 12m` | A step and an elapsed time, never a fraction. `3/4` is the progress bar PHASES.md §9.1 F2 bans, drawn in words: nothing emits percent-complete, and a fraction over a step count implies the remaining steps are equal in size. |
| four unrelated Jobs, flat | sub-Jobs nested under the step they satisfy | The fleet is a tree and the drawing showed it as a list. `job-drives-the-drone-plan` is not a peer of `job-drives-the-drone`, it **is** its `plan` step — and three Jobs with three sub-Jobs read as six, which made the fleet look twice as busy as it was. Landed in `6f5ae58`. |

So the JOBS panel a reader meets today looks like this — one `feature` Job with the sub-Jobs its own
gates started, `#2` marking a `review` step that failed once and was tried again:

```text
┌─ JOBS (4) ───────────────────────────────────────────────────────────────────┐
  JOB                         STATUS   ID        STEP      RUN  SPENT  NEEDS YOU
  command-centre              PAUSED   bafe1e02  review     2h  $2.50  -
  └ command-centre-plan       DONE     28e048ca  approve    1h  $4.52  -
  └ command-centre-review     ABORTED  bd8762f3  read      15m  $2.28  -
  └ command-centre-review #2  RUNNING  0e5047b0  read       5m  $1.10  -
 4 jobs   $10.40 today
└──────────────────────────────────────────────────────────────────────────────┘
```

**The glyph carries the attempt and not the parent's step name, and that is a width decision.** `armada
fleet ls`'s table came to exactly eighty columns before nesting, so `└ review: ` took it to
eighty-seven — wider than the terminal, which no listing may be. The glyph alone leaves four columns
of slack and the attempt spends three of them, because it is the half that cannot be inferred: two
rows carry one name when a review is retried. The step is in `--json` as `parent.step`, in `armada
fleet show`, and in `fleet ls`'s own `WORKFLOW` column.

**The `$ today` figure changed meaning too.** It was the sum of every row, which counted each sub-Job
twice — once on itself and once inside the parent that had rolled its spend up. On this machine that
read `$180.56` against a real `$148.82`. It is now the sum of each Job's own share.

**MANIFEST is still the one panel drawn from a placeholder** at the time of writing, because it needs
`App` — `manifest.db` opened, `MachineConfig` read, a boot id probed — and the Bridge's dispatch
deliberately routes around `app::build` so a redraw stays a directory read, a transcript tail and a
`ps`. The owner decided on 2026-08-17 to build `App` once at `watch()` entry, which keeps that
per-frame promise; the work is in flight.

## The Job detail, full screen

`d` on any Job, or `enter` from the inbox.

```text
┌─ JOB · release-merge ──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ 7f2ab618-4c2e-4f19-9d31-6a0b2c74e155   workflow bug   branch armada/release-merge   started 1h04m ago                                  │
│ BLOCKED  its Drone is waiting on you at implement, attempt 2 of 3                                                                      │
│ "raise the CI timeout so the integration suite stops flaking on cold starts"                                                           │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ WORKFLOW ─────────────────────────────────────────────────────────────────┐ ┌─ NEEDS YOU ─────────────────────────────────────────────┐
│ STEP          STATUS     GATE                          TIME                │ │ ASKED  4m ago                                           │
│ reproduce     PASS       failing_test_exists · exit 1  6m12s               │ │ the CI timeout is 30s and the                           │
│ implement     BLOCKED    check_passes · waiting on you 18m04s              │ │ integration suite needs 90s on a                        │
│ review        QUEUED     review_clean · a reviewer Job -                   │ │ cold runner. Raise it, or split                         │
│ land          QUEUED     branch_exists                 -                   │ │ the suite?                                              │
│                                                                            │ │                                                         │
│  step 2 of 4   ends at branch   so this Job can reach DONE on its own      │ │ a answer     b board it                                 │
└────────────────────────────────────────────────────────────────────────────┘ └─────────────────────────────────────────────────────────┘

┌─ TIMELINE · what the gate did ─────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ STEP          EVENT       EVIDENCE                                WHEN                                                                 │
│ implement     ASKED       human_approves · entry a058890c         4m                                                                   │
│ implement     RETRIED     check_passes · api:e2e exited 1         22m                                                                  │
│ implement     ENTERED     attempt 2                               24m                                                                  │
│ reproduce     COMPLETED   failing_test_exists · exit 1            58m                                                                  │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

┌─ REPORTS · the Drone's own words ──────────────────────────────────────────┐ ┌─ FACTS ─────────────────────────────────────────────────┐
│ STEP          THE DRONE SAID                                               │ │ FACT       STATUS    DETAIL                             │
│ implement     Raised the timeout to 90s; the flake persists on…            │ │ drone      RUNNING   pgid 41822                         │
│ implement     Reproduced the cold-start failure locally.                   │ │ worktree   HELD      ~/.armada/workspaces/…             │
│ reproduce     Wrote a failing test at tests/e2e/cold_start.rs.             │ │ branch     HELD      armada/release-merge               │
└────────────────────────────────────────────────────────────────────────────┘ │ cost       SPENT     $1.25 of $5.00                     │
                                                                               │ exchanges  SPENT     19 of 20                           │
                                                                               │ tokens     -         4.1M, 99.6% cache                  │
                                                                               └─────────────────────────────────────────────────────────┘

┌─ KEYS ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ↑↓ move   enter board   a answer   t tick   r retry step   $ raise budget   p pause   x abort   esc back to the fleet                  │
└────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## The decisions, and why each is what it is

### One inbox, not three panels

`NEEDS YOU` and `FAILURES` began as separate panels and he merged them: *"let's turn this into an
inbox section."* That is [`021`](021-the-work-hierarchy.md) arriving on the screen rather than a
layout preference — a question a Drone asked, a failure Armada noticed and a report he filed are
one family sharing one store and one id space. Three panels would have been three names for one
thing, which is the defect [`glossary.md`](../glossary.md) exists to prevent, drawn. `ORIGIN` is a
column and a filter.

### A step count in the list; elapsed time in the detail

[`bridge.md`](../commands/helm/bridge.md) bans a step fraction outright — *"never a fraction —
'three of five steps' would be the banned progress bar written in words."* He split it: *"your
point about the steps is accurate for the detail pane of a job. But when viewing a list of jobs I
should be able to see a quick summary."*

**That is a sharper line than the spec drew.** A count of steps is a fact — a workflow declares
four and the Job is on the third. What [`PHASES.md`](../PHASES.md) §9.1 F2 actually bans is a count
*implying time remaining*, because four steps are not four equal quarters. So the list shows
`implement 3/4` and the detail shows the step and how long it has been on it. **`bridge.md`'s
sentence has to be amended rather than quietly contradicted.**

`ITER 19/20` stays for the same reason: a count against a declared ceiling, and a Job at 19 of 20
is genuinely about to stop.

### Both usage windows, in the header

Measured before drawing: today's Drone transcripts carry `rateLimitType: "five_hour"` forty times
and `"seven_day"` six, each with a measured `utilization` and a `resetsAt`. So the weekly panel is
buildable from data already arriving — but **Armada keeps only one of them**, because each
`rate_limit_event` overwrites the last. That is the smallest piece of work here and the one with
the highest value: a Job dying at 03:55 with `status: "rejected"` cost a whole overnight run.

Usage leads and spend follows, which [020](020-the-tui-decided.md) already decided: what stops
you working outranks what it cost.

### Movement never sheds; verbs do

The first narrow draft kept `d detail` and `a answer` and dropped the movement keys. He caught it:
*"can we still have the ability to navigate different panels?"* **A verb you cannot see is one
keypress away behind `?`; a movement key you cannot see is a screen you cannot get around.** So the
narrow legend leads with movement.

### Arrows and `hjkl`, not `wasd`

He asked for *"arrow keys or wasd"*. `wasd` collides with two keys the Bridge already has — `a` is
*answer*, `d` is *detail* — and the alternatives were renaming two of the most-used verbs or making
movement work only when no row is selected, which is an invisible mode. Arrows plus `hjkl` take no
letter a verb owns. `/` opening the filter is the other reason movement wants non-letter keys:
while it is open, every letter is a letter.

### The focus marker occupies the leading space

`▸` sits *in* the row's leading space rather than being added to it, so focus never shifts a
column. The first attempt added it and pushed the row one character right; the line-width check
passed because the trailing pad had been trimmed to compensate. **Line width cannot see column
drift**, and a check that verifies a property adjacent to the one that matters is the shape of
several failures this month.

### The detail view names its two tables

His complaint about `fleet show`: *"maybe the headers will help but I don't know what the two tables
are. They both look like workflows."* They are `TIMELINE · what the gate did` and
`REPORTS · the Drone's own words`.

**That distinction is not cosmetic.** One is a machine's decision carrying the predicate and exit
code that settled it; the other is an agent's summary. Conflating them is how an agent's claim gets
read as evidence, which is the failure the whole gate design exists to prevent — and
[`032`](032-the-job-drives-the-drone.md) is the same confusion found in the tool schema.

### Three more of his notes, applied to the detail view

- *"Why is ASKED a UUID?"* — the timeline names it `human_approves · entry a058890c`, so the id
  appears as what it is rather than under a header that does not say.
- *"There should be a clear action."* — every blocked state carries the keystroke that clears it.
  `$ raise budget` is on the key line because *reached its iterations ceiling* was the message with
  no way out.
- *"51 of 20 turns"* — `exchanges 19 of 20` counts what the ceiling counts, and cost is the ceiling
  that now matters ([`029`](029-a-job-of-its-own-done.md)'s neighbour, shipped separately).

### Column order

`NAME → STATUS → DETAIL → TIME` throughout, which he decided separately: *"humans read from left to
right, and it reads better to state Fact → Status → Details."* One rule, no per-table judgement.

## What it costs

| Piece | Size | Why |
|---|---|---|
| The panel frame — titles, borders, `hjoin`, shedding | medium | A layout engine; the Bridge draws one table today |
| Header with both usage windows | **small** | `RateLimit` is parsed already; it must keep both instead of the latest |
| The inbox panel | small | One listing over a store that already exists |
| `MANIFEST`, `GUILD`, `SYSTEM` panels | medium | Renderers over verbs that already answer |
| The Job detail screen | medium | Five panels over data the record holds; the two tables exist and are unnamed |
| Focus, `1-5`, `?` | medium | The keymap becomes real rather than one line |
| **Holding `ARCHITECTURE.md` §1.9** | **the real risk** | Seven panels reading four modules |

## The risk, stated once more because it is the one that matters

`ARCHITECTURE.md` §1.9 is *"four modules, and nothing points upward"*: Helm may depend on Fleet,
Guild and Manifest; Fleet on Guild and Manifest; Guild and Manifest on nothing, and they may not
name each other. **Manifest has to keep knowing nothing about agents** — that is what makes it
usable by hand, by CI, and by four parallel agents at once.

The Bridge lives in Helm, so it may *read* all four. The failure is a panel quietly passing a Job id
into a Manifest call: that breaks the rule without any crate depending on anything new, so
`cargo xtask boundaries` — which checks the crate graph — sees nothing wrong. Seven panels make it
likelier than four, and no test catches it. Whoever builds this owes an answer for how they held it.
