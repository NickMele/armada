# Design System — UI & Voice

**Kind:** contract. **Governs:** static UI chrome, tokens, and the Voice &
Copy contract — the parent contract that nothing else may contradict.

Read before building a screen or component in `packages/components`.

---

Constraining what a surface may spell is what makes it drop into the
Electron app without restyling. A value or a word this document does not
sanction is a finding, not a judgement call.

UI tokens and the Voice & Copy contract are both in force. Two sibling
documents carry what building a surface does not need: the [Agent Copy
Contract](agent-copy.md) (text written at runtime by Drones, Judge and
Helm, with its surfaces and their samples in Armada Copy) and the [Voice
Contract — Engineering Requirements](voice-engineering.md).

---

## Stack

- **Component library:** shadcn/ui — you own the component code,
  Tailwind-native
- **Style library:** Tailwind + CSS custom properties as tokens
- **Icons:** lucide-react, version pinned — 12px in badges, 16px in
  navigation and buttons, strokeWidth 2 throughout. See
  [Iconography](iconography.md)
- **App:** electron-vite + React + TypeScript

---

## The product

Armada is a personal AI workflow orchestration system. It dispatches AI
coding agents (Drones) against real Git repositories, monitors them, and
escalates when they misbehave. Single user, local, always open on a
second monitor across a working day.

**Surfaces:** Bridge is the operational surface group — Job Board,
Alerts, Doctor, Manifest. The count is not a contract: a surface earns a
place in the rail where a journey needs one, and the roster lives on
Bridge. Active Jobs, Reviews and the Activity Feed were retired into the
Board, which holds every Job with state as a filter. Alerts stays because
an alert is a condition on a Job rather than a status a Job holds. Helm
is not a surface — see Window and layout model, Content area, below.

**The screen's job:** at a glance, tell one person what is running, what
needs them, and what broke.

This is an instrument panel, not a marketing page. No hero sections, no
decorative iconography, no illustration. Density and legibility win over
impact.

**An instrument panel still has depth.** It sits open all day, and on a quiet
day nothing on it carries status colour, so a flat grey screen reads as a dead
one. Cards are glass lifted off a canvas lit in faint pools of light (see
Depth, under Tokens). Light, glass and shadow carry no information. A gradient
is allowed only as that light, and never where a person reads a state.

---

## Hard rules

1. **No Tailwind arbitrary values.** Never `bg-[#3b82f6]`, `p-[13px]`,
   `text-[15px]`. Every value comes from the token set below.
   Lint-enforced in the app — arbitrary values fail the build.
2. **Only shadcn/ui primitives:** button, input, textarea, select, checkbox,
   radio, switch, badge, card, table, dialog, sheet, tabs, toast, tooltip,
   dropdown-menu, popover, separator, scroll-area, skeleton, alert,
   **command**. Compose from these; do not invent new base components.
   `textarea` is sanctioned because a Job's brief is prose a person writes at
   length, and a single-line input for it is a control that fights its content.
   `command` (cmdk) backs the command palette. A `kbd` element and an `a`
   element are the two non-shadcn primitives — `kbd` is specified under Keyboard
   and command palette, and `a` is what a fact that names something outside
   Armada is drawn as. It takes `--accent`, which this document already gives
   links, and underlines on hover only. **No surface may navigate.** Every
   anchor cancels its own default and hands the address to the process that
   owns the shell; a window that loaded a forge would be a window with no rail
   and no way back, which is the frozen surface Bridge exists to escape.
3. **Status colors are never chosen.** They map to the Job state machine
   one to one. Never assign a status color by aesthetic judgment. **Below
   Job level, hue exists only where `tokens/status.css` declares it**,
   and every value there aliases its Job counterpart so the mapping is
   declared rather than inferred. Read the file rather than a list; this
   rule used to enumerate the cases and went stale twice. Anything the
   file does not declare stays neutral. See Below Job level under Tokens.
   **Outside status and accent, two hue families exist.** `--helm` marks
   Helm's own chrome and nothing else. The three **tool families** in
   `tokens/tools.css` say what a Drone's call does, and they are a
   separate file because they alias no status and must never carry one —
   see Tool families under Tokens. Depth's light and glass never take a
   status hue.
4. **Dark is primary.** Design dark first. Light exists but is secondary.
5. **Icons: lucide-react only**, used sparingly. A dashboard dense with
   icons reads as noise.
6. **React Flow (`@xyflow/react`) is the one sanctioned graph surface**, and
   a [Studio](../concepts/studio.md)'s whiteboard is what it draws. It
   supplies placement, pan, zoom, fit and selection, and nothing that is
   seen: every node inside it is built from the primitives above and the
   tokens below, its controls are `button`, and every value its own
   stylesheet would paint is set to a token. No second graph or canvas
   library, and no node drawn from React Flow's defaults.

---

## Tokens

The token set below is mirrored row by row in `packages/tokens/src/*.css`,
the authority on each value; each token's role, source file, contrast
measurements and revision history are tracked in the Armada Tokens
database. Components that consume a token are related to it from Armada
Components, which also records what is still missing and which journey
first needs it.

Reference as Tailwind classes mapped to CSS variables (`bg-surface-raised`,
`text-fg-muted`, `text-status-running`). Never raw hex.

### Ground

Deep desaturated blue-slate, not near-black. Reads as instrument panel
rather than terminal, and gives status color room to sit without
vibrating.

```
--bg-base        #0F1419   canvas
--bg-sunken      #0B0F13   wells, code blocks, log panes
--bg-raised      #161C23   table rows, and a flat card (see Depth)
--bg-overlay     #1D242D   dialogs, popovers, dropdowns
--bg-hover       #212A34   row and control hover
--border-subtle  #232B35   table rules, dividers
--border-default #2E3946   flat card and input edges
--border-strong  #3D4A5A   focus rings, active edges
```

### Depth

```
--bg-glass          rgb(25 33 45 / 0.86)   card top
--bg-glass-end      rgb(22 30 41 / 0.86)   card bottom
--border-glass      rgb(255 255 255 / 0.07) card edge
--border-highlight  rgb(255 255 255 / 0.05) 1px light along a card's top inner edge
--accent-faint      --accent at 10%        the canvas's pool of light
--shadow-card       1px contact + 28px soft drop
--glass-blur        12px                   backdrop blur under a card
```

**A card** is a vertical gradient from `--bg-glass` to `--bg-glass-end`, with
`--border-glass`, `--shadow-card`, a `--border-highlight` line along its top
inner edge and a `--glass-blur` backdrop blur. The gradient, the highlight and
the blur sit on a layer behind the card's content rather than on the card, so
a tooltip or menu inside a card is never clipped by it. It
replaces `--bg-raised` and `--border-subtle` on every panel that sits directly
on the canvas: the left column's three panels, Overview's cards, Helm's dock,
and every `Card` a surface draws on the canvas — Dispatch, Studios, Reports,
Cleanup and Settings — each at `--radius-lg`. A `Card` inside a sheet, a
dialog, a well or another card is not on the canvas: it stays flat, at
`--radius-md`. A row, a well or an input inside a card stays flat on its
Ground token.

**The canvas** is `--bg-base` under two radial pools of light. `--accent-faint`
sits in a 760 × 480px ellipse centred on the top leading corner.
`--helm-faint` sits in a 640 × 520px ellipse on the bottom trailing corner,
behind Helm's dock, and only while the dock is open.

**The glass top is set by contrast.** It is the lightest ground text sits on,
measured under the accent pool. `--fg-subtle` reads 4.74:1 there, and every
status badge clears 4.5:1 on its own 12% tint, `not_started` included at
4.55:1. A lighter top, `rgb(30 41 55)`, dropped `--fg-subtle` to 4.37:1.

**No blur while a scrim covers the card.** Behind an open Sheet or Dialog the
blur cannot be seen, and redrawing it every frame under the scrim made presses
on the sheet go missing: three full runs of the app's tests in six, and none in
five once it was off. The glass is 86% opaque, so the card paints the same
without it. A Drift or Verify panel inside a Sheet or Dialog is not on the
canvas, so it takes no glass at all and stays on `--bg-raised` and
`--border-subtle`.

### Foreground

```
--fg-default  #E4E9EF   primary text
--fg-muted    #93A1B1   labels, secondary text
--fg-subtle   #7E8CA0   timestamps, metadata, placeholders
--fg-inverse  #0F1419   text on solid accent fills
```

### Accent

One accent, used for interactive affordance only — never for status.

```
--accent        #4A9EDB   primary buttons, links, selected state
--accent-hover  #5FB0E8
--accent-muted  #1C3A52   subtle fills, selected row backgrounds
```

**A status mark never sits on an accent fill.** `--status-running` is
1.28:1 against `--accent`, so a status line drawn on a primary button
cannot be seen. A control that has to carry a status mark first steps
down off the accent — to `--accent-muted` for a press, to the pending
rendering under Button while Fleet has not answered — and the mark reads
against that.

### Helm

```
--helm        #8E95FF   Helm's chip icon, Send's text
--helm-muted  14%       chip and Send fill
--helm-edge   38%       the dock's top edge, Send's border
--helm-faint  6%        the dock's wash, the canvas's pool behind it
```

Helm answers questions about whatever is on screen, so the dock needs to
read as Helm at a glance and not as another panel. **Indigo sits between
`--accent` and `--status-rejected`**, so it appears on Helm's own chrome only,
never on or beside a Job row. `--helm` reads 6.09:1 on `--bg-glass` and 4.82:1
on its own chip.

### Status — derived from the state machine

One token per Job state, and the set is the state machine's — not a
palette. The critical semantic distinction: `rejected` and `killed` are
**deliberate human decisions**, not system failures, and must not read as
errors.

```
--status-not-started        #8C97A6   dormant, queued
--status-running            #4FB8D9   active, in flight
--status-awaiting-review    #E5A93D   needs you, not urgent
--status-escalated          #EE8450   needs you, urgent
--status-completed-success  #4FAF7C   landed
--status-completed-failed   #E97878   system failure
--status-rejected           #B489DA   you declined it
--status-killed             #9BA3AC   you stopped it
```

Each has a `-bg` variant at ~12% opacity for badge fills. A Job Board
row takes the same hue at `--row-tint` (5%) — see Table.

**Contrast pass, 2026-08-20.** Five values were lifted in lightness so
badge text clears 4.5:1 as 12px text on its own 12% tint over
`--bg-raised`. Hue and semantic assignment are unchanged — nothing was
reassigned, only brightened.

| Token | Was | Now | Badge contrast, before → after |
| --- | --- | --- | --- |
| `--fg-subtle` | #5D6B7C | #7E8CA0 | 3.15 → 5.02 on `--bg-raised` |
| `--status-not-started` | #5D6B7C | #8C97A6 | 2.83 → 4.83 |
| `--status-escalated` | #E8763D | #EE8450 | 4.93 → 5.52 |
| `--status-completed-failed` | #DC5B5B | #E97878 | 4.06 → 5.12 |
| `--status-rejected` | #A97BD1 | #B489DA | 4.48 → 5.12 |
| `--status-killed` | #6B7684 | #9BA3AC | 3.27 → 5.52 |

`--fg-subtle` no longer equals `--status-not-started`, which closes the
contrast item Iconography flagged. `running`, `awaiting-review` and
`completed-success` already passed and were left alone. One shortfall
remains: `not_started` badge text on `--bg-overlay` reads 4.38:1, and
badges appear on floating layers rarely enough to accept it. `--accent`
as text on `--accent-muted` is 4.06:1, so selected rows keep
`--fg-default` text (9.68:1) rather than accent text.

**Escalation sub-reasons** all use `--status-escalated`, differentiated
by **label and icon**, never by hue — a column of oranges would be
unreadable. **The trigger list is not enumerated here.** It lives on
Workflow, it has grown twice, and several triggers are not yet in
`core-model`'s enum. What this document owes is the rule, not the
roster; the enum→verb test is what catches a trigger shipping with no
label.

**The approval axis is a status, not a reason.** `awaiting_approval` and
`queued` are statuses of their own, and what remains as `queued`'s reason
names the resource — with ready becoming the null rather than a value.
`queued` and its reasons share `--status-not-started` and differ by
label and icon; `awaiting_approval` left grey for amber. See Job.

**The symptom that forced this is a rendering bug, which is why this
document carries it.** A sub-dispatched Job inherits its parent's
approval. Under a single four-value field, one out of headroom computed
as `pre_approved_queued` and never rendered on the Job Board at all.
Under the current set it enters at `queued` with its reason naming the
resource, so no combination computes to an unrenderable label.

> Long-term intent: generate these token names from `core-model`'s Rust
> enum via the same codegen step that emits the `ipc` TypeScript types,
> so adding a ninth state fails the build until a token exists for it.

### Below Job level

Drawn from the workflow rail, which broke the rule that hue stops at the
Job — a done step, a running step, and a Judge criterion verdict all
wanted it. **Hue below Job level exists only where `tokens/status.css`
declares it.** That file is the list; this section carries the
reasoning, and deliberately does not restate the roster, because an
enumeration here went stale twice.

Every value **aliases** its Job counterpart rather than introducing a
new one. The mapping is declared, so it is read rather than inferred.

```
--step-advanced    var(--status-completed-success)
--step-running     var(--status-running)
--step-waiting     var(--status-awaiting-review)
--step-failed      var(--status-completed-failed)
--step-failed-bg   var(--status-completed-failed-bg)
--step-stopped-bg  var(--status-escalated-bg)
--verdict-met      var(--status-completed-success)
--verdict-not-met  var(--status-completed-failed)
--run-running      var(--status-running)
--run-passed       var(--status-completed-success)
--run-failed       var(--status-completed-failed)
--run-stopped      var(--status-killed)
--phase-live-edge  --status-running 45% into --border-default
--phase-live-bg    --status-running 6% into transparent
--phase-live-rule  --status-running 25% into --border-subtle
```

**Step activity answers where the work is.** `retrying` and
`not_started` take no hue — `--fg-muted` and `--fg-subtle`. A **killed**
step takes none either, and that exclusion is load-bearing: killing is a
human decision rather than a system failure and must not read as an
error. The rail's current row keeps its `--accent-muted` tint and 2px
`--accent` left edge, which is emphasis and not status.

**`failed` is the one value that reports an outcome rather than a
position.** A step whose Check refused takes `--step-failed` with a bare
`x` glyph, following `advanced` taking `check` — the same mark as the
`completed_failed` badge, meaning the same thing one level down. It was
drawn neutral first, on the grounds that a Check result is measured and
measured facts render flatly. That was reversed: where a failed Check
ends the Job with no retry and no triage, that row is the entire reason
a person opened the screen, and making them find it by weight in a rail
of muted rows is the frustration the surface exists to prevent. **The
gate row beneath stays neutral** — the step's state is hued, the Check's
exit code is measured.

**Two step values carry a surface rather than a glyph hue alone:
`stopped` and `failed`.** A step whose retries are spent is its own
activity value — not retrying, and not waiting on you either, since
folding it into `waiting` would render a designed human gate and a dead
stop alike. Both take a surface for the same reason: a glyph only holds
while its row is selected, and the row that ended the Job has to stay
findable while you read the Check output beside it. **In a rail,
background states what the row is and the accent left edge states which
row you are on** — the surface is constant, selection adds the edge. One
of each per rail, because a Job stops or fails in exactly one place.
They differ in the glyph: `stopped`'s `flag` stays `--fg-default`,
because with the surface carrying the warning a hued flag would say it
twice, while `failed`'s `x` is hued, since failed is an outcome and
states it in both channels.

**Criterion verdicts are measured facts and render as flatly as one.** A
criterion is met or it is not. The red does not claim the Job failed: a
Judge refusal is the gate working, and the row's copy names which
criterion and why. **Verdict hue is per criterion and never sums onto
the step or the Job** — that is the rule that lets a red cross sit under
a running step beneath an escalated badge without any of the three
contradicting the others.

**A run started by hand takes Job colours, and is still no verdict.** A
Check or Command run from a Job's run sheet or the Manifest surface hues its
result: the code it expected is `--run-passed`, any other ending
`--run-failed`, one in flight `--run-running`, and one somebody stopped
`--run-stopped` — killed's grey, so it never reads as an error. The hue sits on the result's chip and the
elapsed figure, and it counts for nothing — a run writes no Evidence and no
`check_runs` row, and on Job detail the run sheet keeps its own heading so a
rehearsal never reads as the gate's result.

**Refusals sort first, and every criterion row carries its number.** A
card that reorders breaks correspondence with the frozen
`acceptance_criteria[]` order, so a citation to "criterion 4" would no
longer sit fourth on screen. Explicit numbering is what lets both hold:
the rows a person needs are at the top, and the citation still resolves.
See How are criterion verdicts encoded without status hue?

**The live phase takes running on its edge, with a faint wash.** One row
of a step's timeline is the one the Drone is inside, and on a card that
size a mark alone does not carry it across a desk: `--phase-live-edge`
lifts the card's own border 45% toward running, `--phase-live-bg` washes
it at 6% — below a summary tile's 9%, because a phase card is larger
again and its body is a log — and `--phase-live-rule` carries the same
lift to the line under its header. All three alias `--status-running`,
which is what a running phase is one level down. **The mark leaves that
header**; see Motion.

**Everything else below Job level stays neutral, with one exception.** An
origin tag and the retry marker carry position, surface, weight and glyph.
Drift is the exception: `gone` and `current` render in `--notice-caution`,
because a drifted file is behind rather than broken. Adding a value to
`tokens/status.css` is a contract change, not a design decision.

> **Rule.** A caution notice is the one Alert tone below Job level that
> takes hue, and it aliases `--status-awaiting-review` as
> `--notice-caution`.
> Why: amber already means a person is on this, and a run that lands in
> a Drone's worktree is that.

It carries no glyph, because `triangle-alert` is Doctor's and the
contract has no generic alarm glyph.

### Diff

```
--diff-add-bg     #14301F
--diff-add-fg     #6FD196
--diff-del-bg     #351A1D
--diff-del-fg     #E88A8A
--diff-context    #93A1B1
```

**The diff colours are the patch view's and every other place a count of
lines appears**: a log row's `+2 −2`, the files under a plan task, and
Produced's per-file counts and its size bar. What the numbers *are* still
differs by surface — a task's are the sizes of the Drone's own edits, the
diff's are the patch — and the label beside them is what says which.
Colour says added and removed; it never says which kind of number.

### Tool families

What a Drone's call *does*, in three hues, declared in
`tokens/tools.css`.

```
--tool-look    #8FA3D9   Read, Grep, Glob
--tool-change  #D98BB5   Edit, MultiEdit, Write, NotebookEdit
--tool-run     #C2B48A   Bash
```

**Its own file, and that is the rule rather than the filing.** Every
value in `status.css` aliases a Job status, so the mapping is declared
rather than inferred; these alias nothing, because they answer a
different question — not where the Job stands, but what the call in front
of you reaches for. **A tool colour never carries status, and a status
colour never carries a family.**

**Why it is worth a hue at all.** A Drone's hour is a long stretch of
looking, a burst of changing, then a run, and in a mono column where
every tool name is the same colour that shape is invisible — which is the
reading a person opens the log for.

**They sit near two statuses and are told apart by where they are**,
which is the risk hard rule 3 guards. `--tool-change` reads near
`completed_failed`'s red and `--tool-run` near `awaiting_review`'s amber.
Nothing in a log row is a status: a status is a badge, a mark or a row
surface, and a family is one word at the head of a mono line, so the two
never appear in the same slot.

**A tool the roster does not name takes no colour.** Hue is scarce and an
unclassified tool is not a fourth family — the name draws in the line's
own foreground. Adding one is a decision about what a family means.

---

## Typography

**IBM Plex Sans** for interface. **IBM Plex Mono** for anything
machine-derived — job IDs, file paths, branch names, commands, diffs,
durations, token counts.

That split is a rule, not a style preference: monospace signals *this is
a fact the system reported*, and it makes IDs and paths scannable in a
dense table.

Scale is tighter than web defaults. This is a dashboard.

**Legibility pass, 2026-08-20.** The whole ladder was raised ~15% after
the 11px and 13px steps proved hard to read at desk distance. Ratios and
roles are unchanged — every step moved together, so nothing about
hierarchy or the mono-one-step-smaller rule changes. The heights that
hold the larger text moved with it, and `tokens/spacing.css` is the
authority on each one.

```
--text-2xs   13px / 18px   table metadata, timestamps
--text-xs    14px / 20px   labels, badges, secondary
--text-sm    15px / 22px   BODY DEFAULT — most UI text
--text-base  16px / 24px   emphasis within body
--text-lg    18px / 28px   panel headings
--text-xl    23px / 32px   page titles
--text-2xl   28px / 36px   the rare hero number
```

Weights: 400 body, 500 labels, 600 headings. Never 700+.

**Emphasis on the dark ground is bought with size, tracking and hue, not
weight.** Dark polarity reads slower than light, and raising weight
improved glanceable reading in light mode only; it did not resolve
halation on a dark ground (Beier et al., *How bold can we be?*,
Readability Consortium, 2023). That is also why the 20 Aug legibility
lift raised the whole ladder and touched no weights. Emphasis within
body is one step up the ladder, or `0.012em` tracking at body sizes, or
`--fg-default` against `--fg-muted` — never 500 on a dense surface. 500
stays for labels, which is a role, not emphasis.

**Figures are lining and tabular everywhere**, set once at the root, so
numbers in a column align in the sans and compare by shape. Mono is
therefore never used to line figures up; it keeps its one job, marking
a fact the system reported. `--text-2xl` is the Overview summary's
count, the one hero number.
Mono runs one step smaller than adjacent sans at the same optical size —
14px mono next to 15px sans.

---

## Spacing and shape

4px base grid: `1`=4, `2`=8, `3`=12, `4`=16, `6`=24, `8`=32, `12`=48.
Deliberately tight. Table rows 36px, header rows 32px. Card padding
20px, not 24. Controls 32px (sm) / 36px (default). Section gaps 24px,
not 48. If it feels slightly cramped against normal web instincts, it is
correct — this window holds a job list, a diff, and a graph view at
once.

Every one of those is a token in `tokens/spacing.css`, which is the
authority on the value. Read it rather than retyping a number from here.

**A row height token is a floor, not a cap.** Rows are padding-driven
where content can grow, so the token keeps rows aligned down a column
without truncating one that needs more. Labels align with their fields,
and content left edges align with their header's left edge.

```
--radius-sm  3px    badges, small controls
--radius-md  5px    buttons, inputs, a flat card
--radius-lg  8px    dialogs, panels, a card on the canvas
```

**A card's radius follows its surface.** On the canvas it takes the card
treatment under Depth and `--radius-lg`, the same corner as the panel beside
it. Flat — inside a sheet, a dialog or another card — it is `--radius-md`.

No full-round pills except avatars.

**Two heights of shadow.** A card lifts off the canvas on `--shadow-card`. A
floating layer (dialog, popover, dropdown) lifts off the card on
`--shadow-overlay`, the deeper of the two. Nothing inside a card takes a
shadow: a shadowed row inside a card reads as a second card. The one solid
accent on a view takes `--shadow-primary` (see Button).

---

## Motion

```
--duration-press   90ms     a press answering
--duration-fast    120ms    hover, focus, a body crossfade
--duration-base    180ms    menus, dialogs, toasts, a tab bar travelling
--duration-sheet   220ms    a sheet from the trailing edge
--duration-travel  300ms    a row re-sorting to its new place
--duration-pulse   1600ms   a loop that says still working
--duration-answer  1600ms   how long a control shows Fleet's answer
--duration-hold    900ms    holding a destructive control to commit
--duration-decay   45s      a changed row's mark fading
--ease             cubic-bezier(0.2, 0, 0, 1)
```

**Three clocks, for three audiences.** A *transition* (press to travel,
under 300ms) is for a person watching. A *loop* (pulse) says something
is still working. A *decay* (45s) is for a person returning: a change
away from the focus of attention is missed, and a one-shot flash is only
seen by someone already looking, so a changed row keeps a mark that
fades slowly enough to still be there when they look back. A decay is
neither a loop nor an entrance, so neither rule below governs it.

**No entrance animations on data.** A Job Board that animates rows in on
every poll is unusable. The rule is about data. **Chrome that a person
summoned may enter**: a menu rises from its trigger, a dialog scales from
0.97 with its scrim fading, a sheet travels in from the trailing edge,
and a toast rises, because each fires once, on an act, and says where it
came from. A row re-sorting under a person **travels** to its new place
rather than teleporting, because animation measurably improves tracking
an object across a change (Heer & Robertson, InfoVis 2007); a new row
does not travel, so nothing enters. "Placement resolves before paint"
still holds: a layer never moves after it lands.

**A changed row decays.** When a Job's status changes, its row takes its
new status hue at `--row-tint-recent`, falling to the resting
`--row-tint` across `--duration-decay`, and a mono line in the same hue
says what changed and how long ago, fading with it. It takes no edge:
the left edge is focus and hover's. The decay rides on the resting tint
and ends at it, so there is one tint channel and never two computed on
top of each other. Every changed row decays, with no cap:
`[decay-cap]`.

**Press moves colour, never geometry.** A pressed control walks one step
down its ground ladder at `--duration-press`: `--accent` to
`--accent-muted`, a secondary or ghost to `--bg-sunken`; tonal drops its
hover lift back to its `--accent-muted` rest, because its only caller
sits on `--bg-sunken`. Nothing transforms and nothing reflows. A focused field's ring grows from its
edge and its ground lifts one step at `--duration-fast`.

**Nothing may carry information by motion alone.** Under
`prefers-reduced-motion` every transition duration is zero and each
state still reads from colour and label.

**What animates on a loop is what is still working**, because a hue or a
label can say *which* and only motion says *still*. Three things do: the
running mark, a control waiting on Fleet, and the live phase's top edge.
There was a rule that one thing animates per screen; it was retired on
2026-09-14, #1117.

**One loop per card, and that rule is what the third one bought.** A bar
travelling and a mark breathing in the same header are two things saying
one word, and a reader has to decide which of them they were watching. So
where a card sweeps, every mark inside its header holds still.

**A control waiting on Fleet sweeps a bar along its bottom edge**, from
the press until Fleet answers or refuses. Its label says what it is
doing — *Request changes* reads *Requesting changes…* — and the rest of
its group is disabled. The bar is `--status-running`, 2px, travelling at
`--duration-pulse`; under `prefers-reduced-motion` it holds still at full
width and the label carries the reading. A person who pressed it and has
waited five seconds is told under the group that Fleet is still on it.
The Job itself does not move until Fleet says so — a refused act would
otherwise have to snap back.

**The bar is one line across four states**, so a press, the wait and the
answer read as one object rather than a bar appearing from nothing and
vanishing to nothing:

| State | The 2px line |
|---|---|
| press | opens from the centre to full width at `--duration-press` |
| pending | travels in `--status-running` at `--duration-pulse` |
| accepted | fills the edge in `--status-completed-success`, held for `--duration-answer` |
| refused | retracts to nothing in `--status-escalated`, and nothing else moves |

A refusal is told from an acceptance by the line's colour and direction
before the label is read. The control never moves in any of the four.

**The live phase sweeps a bar along its top edge**, the same line on the
same clock: `--pending-bar` tall, `--status-running`, one segment
travelling at `--duration-pulse`, linear rather than `--ease` for the
control's own reason. It runs while the Drone is in that phase and stops
when the phase does. Under `prefers-reduced-motion` it holds still at
full width, and nothing is lost — `--phase-live-edge` and the wash
already say which phase is live.

**Under the phase name, what the Drone is doing right now.** The call
still in flight — one with no answer yet, which is the one thing the log
beneath cannot show, because its `answered` row does not exist. Between
calls it is the Drone's own last sentence, with no verb: nothing is in
flight to conjugate, and a verb invented for the gap would claim a call
nobody is making. The verb takes `--status-running` and nothing else on
the line does.

**The running mark animates continuously.** A hue says which step is
current; only motion says it is still working, and that is the reading
a static rail cannot give — it matters most on the step that has been
running for nine minutes.

**Every working mark pulses.** A list pulses every running row's Running
badge. Job detail pulses the rail's current step, and the header's Running
badge stays static, because the rail names *which* step is working and the
badge only names the Job's state. A sheet open over Job detail does not stop
the rail, and the sheet's own live mark pulses beside it. A
[Studio](../concepts/studio.md) pulses every node that is still working. The
step bar never pulses — its job is where the work got to, which is a static
fact, and the badge sits in a fixed column on every row so the motion appears
in one predictable place rather than moving with the workflow's length.

**The live phase's header carries no mark at all.** It is the one place a
running mark was dropped rather than made to pulse, and the sweep is why:
a mark says *which*, and on the one card already edged, washed and swept
in running there is no *which* left to say — what a second mark would add
is a second loop. **Marks stay wherever one row among many is the live
one**: the plan's task rows, and the live narration row inside that
phase's own body. The word the mark owed a screen reader is still said,
off `enum-verbs.toml` rather than retyped, because hue, wash and motion
are three channels a reader may have none of.

Opacity and scale only, at `--duration-pulse`. The ring holds still, so
no row shifts and nothing reflows. Hue says *which* state, unchanged on
every running row; the pulse says *still working*, which is true of every
working thing on screen, so the pulse follows status rather than focus.
Under `prefers-reduced-motion` the pulse stops and `--step-running`
carries the reading alone. #1276 carries this to the screens.

---

## Sound

**A sound says which kind of wait, to a person who is not looking.** macOS
gives every notification the same chime, so work that has stopped and a
review that will keep sound identical. Armada gives the two their own tone.

| Tone | Plays when | Notes |
|---|---|---|
| **Blocked** | A notification tells of a Job that has stopped: it entered Escalated, or ran out of retries and awaits repair | G4 → D4, falling, 170ms apart |
| **Waiting** | A notification tells of any other Job that started waiting on a person | One A4 |

Each note is a sine of 200ms, rising to full in 12ms and decaying
exponentially. Both tones sit at one level, because loudness is macOS's
setting and not Armada's.

**A tone is the notification's own sound, never a second channel.** It rides
on the banner, so macOS's permission, Focus and per-app sound settings govern
it exactly as they govern the banner. Bridge never plays audio itself, and a
refused notification is a silent one. A packaged Bridge carries both tones in
its bundle; a development Bridge runs Electron's own, so it installs them into
`~/Library/Sounds`, which macOS searches for the same names.

**One notification, one tone.** A batch that holds a stopped Job plays
Blocked, because the most urgent Job in it decides. Nothing else makes a
sound: a Job that lands, a Check that passes and a press are silent, which
keeps the loudness order in Behaviour rules intact.

## Touch

**A press that lands can be felt, so the eyes can already be elsewhere.** A
person holds Kill and looks at the next row before Fleet has answered. The
trackpad answers the act that finger made, and nothing else.

| Pattern | Plays when |
|---|---|
| **Alignment** | Fleet accepts an act a person pressed |
| **Level change** | Fleet refuses one |

**The tap follows the answer, not the control.** It plays where Fleet's
answer arrives, so an act whose control is already gone — an accepted
Forget leaves no row, and an event can replace a control mid-act — is still
felt.

**Touch only ever answers the person's own press.** Fleet's events never
play one, and neither does hover or focus. A haptic is only felt while a
finger rests on the trackpad, so anything else would be a signal that
mostly fires into nothing.

**Nothing may carry information by touch alone.** The press bar's colour and
direction already say accepted or refused, and a mouse, or macOS's haptic
feedback turned off, loses nothing. Electron cannot reach the trackpad, so
the pattern is performed from outside the renderer, behind one seam that
names the two patterns and not how they are played.

---

## Window and layout model

One responsive prototype covers all widths — not separate comps per
breakpoint.

### Window chrome

Frameless, `titleBarStyle: 'hiddenInset'`, macOS traffic lights inset over
the **title row** — the bar across the window's top carrying the repository
picker, search, Dispatch and Helm's reopen control (#1087 moved all four off
the left column and into this row). Costs a custom drag region: the whole
row is a drag region, and every interactive control inside it opts out by
hand.

### Left column

Bridge/1088 replaced the rail and the status bar with one column of three
rounded panels — **Navigation**, **Stats** and **Fleet** — collapsible and
resizable, and both states are designed rather than one being an afterthought.

```
default     200px
drag range  160-320px
collapsed   48px icon rail — Navigation's own form; Stats and Fleet
            collapse to one centred status dot at the same width
persistence width and collapsed state survive app restart
```

**One panel style, shared by all three.** `--radius-lg` and the card
treatment under Depth, held apart by the column's own 16px gap (`--space-4`)
rather than by margin on each panel. A panel's head is 40px
(`--space-8` + `--space-2`), 12px horizontal padding (`--space-3`), and
collapses to its head only — never to nothing, so a glance still answers
the one question the panel is for. Token treatment for Stats and Fleet's
own content is under Component → token mapping.

**One width, shared by all three.** Navigation, Stats and Fleet resize and
collapse together, on one drag handle at the column's trailing edge — never
one handle per panel, and never three panels each resolving their own width.

**Navigation is one level, Bridge's own.** It lists Bridge's surfaces and
nothing else — Helm left it for the dock (#948), so there is no second tier
beneath it any more.

**The column never disappears.** 48px is cheap and losing Navigation, Stats
and Fleet entirely is worse than losing 48px, at any width — Stats and Fleet
keep their one status dot each at that width, so a glance still says whether
anything needs attention.

**Nav items do not carry escalation or approval counts.** Stats already
carries both, as its own rows. Duplicating them in Navigation creates two
places to check and two chances to disagree.

### Content area

**Full-width routes. No inspector pane, no modal for Job detail.** Board
and detail are separate destinations. **Helm's dock is the one
exception**: at `--layout-breakpoint` and wider it sits beside the content
when open, because it answers questions about whatever is on screen rather
than inspecting one Job, and draws nothing at all when closed — the title
row's own Helm button is the one way back, since the edge strip that used to
sit there at any width is gone. It never resizes the content beneath it.
See Responsive behaviour, below, and [Helm](../concepts/helm.md).

This follows from what a detail view actually holds: the escalation
payload, the full attempt history including every prior Judge summary
rather than the latest, per-step evidence, and a diff. That is not
inspector content, and a split pane would cramp the thing the page
exists to show.

**If triage speed suffers in practice**, the fix is prev/next navigation
within the detail view — staying in the queue without splitting the
layout. Not an inspector.

#### The trail

**Job detail is the one screen that keeps its own head**, because it has to
say which Job rather than repeat what Navigation already says — #1090 ended
the page head every other screen used to spend on its own name. Its head
carries the trail: where this Job was opened from, then the Job itself.

| | |
|---|---|
| Placement | Job detail's own head, leading edge, before the status badge |
| Type | Prior segment `--text-xs`, `--fg-subtle`; current segment (the Job's title) `--text-base`, `--fg-default`, body weight |
| Separator | `chevron-right` at 12px in `--fg-subtle` |
| Segments | Exactly two — where the Job was opened from, then the Job |

> **Rule.** A trail segment is a control, never an `a`.
> Why: the anchor rule is about addresses leaving the shell, and a trail that
> was an anchor would be the one control able to break it.

> **Rule.** A surface reached from Navigation carries no trail.
> Why: Navigation is already the answer to where you are, and a one-segment
> trail repeats the title underneath it.

> **Rule.** The last segment is the current destination and does not act.
> Why: a control that returns you where you already are is a control that does
> nothing.

> **Rule.** The repository is never a trail segment.
> Why: it is a fact about the Job, read in the header's own field run one line
> down, not a place the trail can open.

**It replaces a back control rather than joining one.** A back button names one
step and says nothing about where that step sits; the trail names the whole
path, which is what a route reached from a dock rather than from Navigation
needs.

### Responsive behaviour

**A floor per client.** The desktop window floors at 768px, half of a
1536px display and a normal way to run something you glance at beside an
editor; with the rail at 48px that leaves 720px of content. The touch client
floors at 390px, which leaves 358px between its gutters.

> **Rule.** `--window-floor` is the desktop window's minimum, and the touch
> client never reads it.
> Why: the main process sets the window's `minWidth` from it and
> `packages/shell/src/floor.ts` answers whether the window is at it, and a
> touch client has no window to bound.

**One breakpoint at ~1100px, and one client boundary at the desktop floor:**

| | ≥ 1100px | < 1100px | Touch client |
| --- | --- | --- | --- |
| Left column | Expanded, user-resizable — Navigation, Stats and Fleet together | Auto-collapses to the 48px rail; Stats and Fleet each keep one status dot | A bottom tab bar |
| Job row | One shape at every width — a stacked row carrying the badge, the headline sentence and the labelled field run beneath | The same row. Nothing reshapes | The same row, field run wrapped |
| Helm's dock | Beside the content when open; closed draws nothing, and the title row's Helm button reopens it | An edge strip; open draws it as a sheet over the content instead | Not built |

The third column is a client and not a window width. Nothing between 390px and
768px is drawn, because the desktop window cannot get there and the touch
client is not resized into it.

The stacked row is the status grammar's own shape: headline sentence on
line one (`Job 12 stalled at step 3`), labelled field run on line two
(`api · 3 pokes · auth/session.rs · 12m · ~$1.80`). The badge stays
leading on line one so status is still the first thing caught.

**No field is dropped at any width.** Every field in the universal row exists
because a decision depends on it, and responsive-hiding them contradicts P2,
which requires the facts needed to decide to be on screen without a click.
Narrow changes the row's shape, never its content.

> **Rule.** Above the desktop floor the field run holds one line, and a
> secondary value truncates with a tooltip carrying the full string.
> Why: a fixed shape is what lets the run read down a list, and a pointer can
> always reach what truncation hid.

> **Rule.** Below the desktop floor the field run wraps to as many lines as the
> field set needs, and nothing truncates.
> Why: a touch width has no room to truncate into and no hover to open a
> tooltip, so the row grows taller rather than thinner.

**Honest cost:** the stacked row is taller than a table row, so fewer
jobs are visible at once. That is a real loss on a monitoring surface,
and it was accepted deliberately: the Job Board and Alerts disagreeing
about what a job looks like is what retired the two-shape version, and
the row is the most repeated element in the app.

Below 1100 the user may still expand the sidebar manually. It overlays
the content in that case rather than compressing the table further — a
720px table has no width to give back.

**Validation:** the field set needs revisiting rather than the row where it
cannot carry itself at 720px on one line, or at 358px wrapped. No field is
dropped at either width.

---

## Floating layers

Where a layer opens, which edge it aligns to, and what happens when it does
not fit. Anchored layers open against a trigger; framed layers open against
the window. Flip and alignment apply to the anchored family only.

**Placement is CSS anchor positioning.** The trigger carries an `anchor-name`,
the layer names a preferred area and an ordered list of fallbacks. Bridge's
renderer is one known Chromium, so this costs no positioning library and no
measuring in JavaScript.

### Anchored layers

| Layer | Opens | Preferred alignment |
| --- | --- | --- |
| Dropdown menu | Below the trigger | Trailing edges flush |
| Popover | Below the trigger | Leading edges flush, caller may set trailing |
| Tooltip | Below the element it wraps | Leading edges flush |
| Split-button menu | Below the whole control, not the caret | Leading edges flush |

**Alignment is a preference, not a rule.** The trigger's edge is what the
layer tries first; the window's edge is what overrides it.

### Collision

**A layer that does not fit flips. It never squashes.** Width and height come
from tokens, so a narrow gap is not a reason to reflow what is inside.

**Fallbacks are tried in order and the first that does not overflow wins.**

| Order | Try | Gives up |
| --- | --- | --- |
| 1 | Preferred side, preferred alignment | Nothing |
| 2 | Preferred side, opposite alignment | The trigger's edge |
| 3 | Opposite side, preferred alignment | The side |
| 4 | Opposite side, opposite alignment | Both |

Overflow is measured against the window, never against the layer's parent. A
menu that fits inside its card and runs off the screen has not fitted.

### Framed layers

These have no trigger, so flip and alignment do not apply to them.

| Layer | Opens |
| --- | --- |
| Dialog | Centred in the window on both axes |
| Sheet | Full height, flush to one side edge, trailing by default |
| Toast | Bottom trailing corner, inset `--space-6` |
| Command palette | Horizontally centred, top-anchored |

The palette's offset, and why it is not centred, are under Command palette.

**A sheet's width is a fraction of the ground it takes, not a column width.**
`default` is `--w-sheet`, 480px. `wide`, `widest` and `reading` are read
against the window instead, because what has to fit each is a reading rather
than a value: `wide` (62%) is the activity log's file rail beside a line that
does not wrap, `widest` (76%) is the diff's file rail beside a patch line that
does not wrap, and `reading` (88%) is the run sheet's own — a fixed 240px list
of the Manifest's Setup, Checks and Commands, with everything past it the run's
own output. `--armada-sheet-wide`, `--armada-sheet-widest` and
`--armada-sheet-reading` have no token behind them: `packages/tokens` carries
no width scale for an overlay panel, only the sidebar's own range, which does
not describe this. Reported, not minted here.

### Stacking

**A floating layer takes a stacking token and never a written number.** The
order is the order a person meets them.

| Token | Layer | Why it sits here |
| --- | --- | --- |
| `--z-menu` | Dropdown, popover, split-button menu | Opens over the surface |
| `--z-tooltip` | Tooltip | Explains the thing a menu is over |
| `--z-modal` | Dialog, sheet | Interrupts both |
| `--z-toast` | Toast | Reports on the dialog just dismissed |
| `--z-palette` | Command palette | The way out of anything |

A number meaning "above my sibling" means "under every other layer" the moment
its layer resolves against the window rather than its parent.

### Placement resolves before paint

**Nothing animates into place.** Motion forbids an entrance animation on a
data surface, and a floating layer is one as soon as it carries content.

Hover, focus and dismissal still transition on `--duration-fast`. What is
forbidden is the layer arriving, not what it does once it is there.

---

## Keyboard and command palette

Foundational rather than additive. Both change the component inventory
and the focus model, which is why they are specified before the first
screen instead of retrofitted onto it.

**Principle: every action reachable by mouse is reachable by keyboard,
and nothing is keyboard-only.** The palette is a superset of the UI,
never a substitute for it. A capability that exists only behind a
shortcut is undiscoverable, and a capability that exists only behind a
click is unusable at speed.

### One artifact, three columns

Every action carries a **verb**, an **icon**, and a **shortcut**. The
source is `crates/core-model/domain/actions.toml` and the test is the
gate rule over it, which fails where an entry is missing any of the
three. This is the same discipline already applied to the enum→verb map
and the icon map, extended one column. A new action cannot ship with no
shortcut, and a shortcut cannot exist without a label to display next to
it.

**The glyph is named, never redrawn.** An entry's icon is a key in
`packages/icons/icons.toml`, which stays the authority on what a
silhouette may mean; the gate refuses one that is unregistered or
banned.

**A blank glyph column says why it is blank, and the gate counts the
blanks.** Copy debug info carries none by decision, for the reason the
error treatment gives below. Several acts carry none because no
registered glyph means them and assigning one is a decision for
[Iconography](iconography.md). An entry that leaves the column empty and
says nothing fails — which is the closure working, since the alternative
is the back-fill by hand that this section existed to prevent.

### Two tiers

**Global — modifier-based, work anywhere.**

```
⌘K       command palette
⌘1–⌘9    Bridge surfaces, in rail order
⌘J       Helm, toggles the dock on every surface
⌥⌘C      capture a note onto the open Studio
⌘\       toggle sidebar
⌘[ ⌘]    back / forward
⌘Enter   send the message in the field that has focus
Esc      close an overlay, or return to the list from a detail route
```

**`⌘Enter` is the one Global binding that needs a focused field**, and it is
in this tier because it is modified rather than because it fires from
nowhere. Plain `Enter` in a message box is a new line — a redirect to a drone
and an ask to Helm are both prose — so the key that sends has to carry a
modifier, and a single-key binding would be suppressed inside a field anyway
by the rule below. It is drawn on the Send button only while `⌘` is held, per
`kbd` below.

**Contextual — single-key, on the focused row or the open job.** This is
what makes triage fast: move down the queue and act without reaching for
a modifier.

```
j / k / ↓ / ↑  move focus
Enter          open the focused job. Acts on nothing   (list only)
o              open. The same act, named so the palette can display it   (list only)
r              review             (list only)
t              attest
d              redirect
s              restart step        (detail only)
p              pilot               (not built)
c              copy debug info
b              report this job     (detail only) (confirms)
x              kill                (confirms)
X              kill & redispatch   (detail only) (confirms) (not built)
n              dispatch
/              search the current list
1–6            state filter        (Job Board only, in tab order)
a              approve             (dispatch card only)
v              observe             (detail only)
u              submit for verification  (piloted job only)
e              redispatch as a new job
h / l / ← / →  expand and collapse  (detail only)
[ ]            move between chapters  (detail only)
L              open the log         (detail only)
f              open the diff        (detail only)
o              open the output      (detail only)
g              open the stage       (detail only)
B              raise the cost cap   (detail only) (confirms)
T              raise the turn cap   (detail only) (confirms)
r              open the run sheet   (detail only)
N              add a note           (open studio only)
V              add a link           (open studio only)
S              add a sketch         (open studio only)
```

**This is the map, not a pattern.** It was settled by drawing the Job
Board and the command palette together, which is what forced the two
halves into one artifact — the palette displays a binding beside every
entry, so an unreconciled map is a palette that cannot be drawn.

**Both blocks are transcribed from
`crates/core-model/domain/actions.toml`**, which is the artifact above
and the authority. They stay here because a binding is read beside the
rule that governs it rather than followed to a data file; the gate reads
both and fails where they disagree, on the binding, the verb and the
annotations in brackets.

**Job detail's bindings are in the map before the screen is rebuilt.**
The run tree, the chapters, the produced files and the phase strip each
carry one. The run tree roves on the same `j`/`k` as a list rather than
taking a second pair of keys — moving between steps and moving between
rows are one act — and expanding a step's facts is the same act as
opening a log entry's payload, so both are one binding on the other
axis. #265 builds the screen they belong to.

**`not built` means the binding is registered and nothing answers it.**
The map was settled by drawing, so it holds acts nobody has written, and
the palette displays a binding beside every entry — a row a person
presses and gets nothing from is worse than one that is absent. The
registry's `unbuilt` column names the issue that gives each of them an
act, and the gate holds the annotation and the column to each other. So
the palette can draw these rows disabled, or leave them out, on a fact
rather than on a list of exceptions kept in the app.

Three reversals against what stood before, each with a reason:

- **`a` is deleted.** Nothing on a list approves. Approval is a second
  act from detail, which [Job Board](../concepts/job-board.md) has always
  said and the built row contradicted.
- **`r` is review, and redirect moves to `d`.** Review is on every
  needs-you row and is the most-pressed contextual key in the app;
  redirect is reached from a job that has already gone wrong.
- **`⌘F` is deleted.** `/` already searches the current list, and two
  bindings for one act breaks the one-artifact rule above.

**`x` for kill and every safety rule below are unchanged.** Neither was
in play, and the destructive-key rule is what kept `x` off `k`.

**A Studio's three are shifted, and `open studio` is a place rather than
an object.** Every other contextual scope names what the act acts on —
the focused row, the open job, the dispatch card. These act on the board
a person is looking at, and the node they make lands where they are
looking rather than at the origin. They are shifted because the letters
the design drew are all spoken for: `n` is dispatch and its scope is
`anywhere`, `v` is observe and `s` is restart step, so an unshifted key
would answer twice on one press. See [Studio](../concepts/studio.md).

**`⌘1`–`⌘9` follow the rail** — Overview, Job Board, Studios, Alerts,
Doctor, Manifest, Cleanup, Kit, Settings — since Active Jobs, Reviews and the Activity
Feed folded into the Board and Cleanup joined at the end of it.
The digits shift if the rail does; the rule is rail order, not the
numbers.

**Helm moved from `⌘6` to `⌘J` on 2026-09-13**, when it left the rail
for a dock on every Bridge surface (#948). A digit is a place in the rail,
and Helm no longer has one; `⌘J` toggles the dock instead. It had moved
once before, from `⌘5` on 2026-09-03, when Held worktrees, now Cleanup, took that
digit. The palette displays the binding beside every entry — which is
where a person finds out. A learned key does not move quietly, and this
paragraph is the noise.

**Overview joined the rail first rather than last, on the same day** —
it is where Bridge opens (#921), so it took `⌘1` and pushed every other
surface's digit down by one rather than taking the next free one. The
rule stays rail order; only the arrival was the exception, and it is the
one recorded against `bridge_surfaces` in `actions.toml` rather than
against a surface's own row, because no single surface's binding moved —
the rail's shape did.

**Settings joined the rail last, on 2026-09-14 (#1089), taking `⌘7`** —
the ordinary case, not Overview's exception: it is Fleet's four limits and
this machine's own settings, on the screen a sheet reached from the status
bar used to hold before #1088 removed the bar.

**Studios joined third, on 2026-09-17 (#1287), taking `⌘3`** — the second
arrival to move digits other than its own, after Overview's. The owner placed
it straight after the Job Board, because where a stretch of work is read
belongs beside the work it becomes rather than after the settings; Alerts,
Doctor, Manifest, Cleanup and Settings each moved down one, Settings from `⌘7`
to `⌘8`. The rule is still rail order — only the arrival was the exception.

**Kit joined before Settings, on 2026-09-18 (#1275), taking `⌘8`** — the third
arrival to move a digit other than its own, and the first to join anywhere but
the end: Settings moved to `⌘9`. The owner's decision, and his reason is that
Kit is what he brings and Settings is what this machine is, so the machine reads
last. It is a rail row rather than a view on one repository's Manifest because
Kit is his and not a repository's. Its glyph is `settings`, whose own registry
row has meant Kit since it was written; the Settings surface took
`sliders-horizontal` in the same change, because two rail rows cannot both be a
cog.

**`1`–`5` and `⌘1`–`⌘9` are different acts on one row of keys.** One is
modified and one is not, which is the whole separation. It was drawn
that way deliberately rather than by omission.

### Safety rules for single-key actions

These are constraints on the map, not suggestions.

- **Destructive keys are never adjacent to navigation keys.** Kill is
  `x`, never `k`, because `k` sits against `j` and a mistyped navigation
  keystroke must not be able to end a running job.
- **Every destructive action confirms**, even from the keyboard. A
  confirmation is a dialog or a hold, never nothing. In the
  confirmation dialog **Cancel holds initial focus, so `Enter` cancels**
  — `Enter` fires whatever holds focus, and Cancel is what holds it. The
  kbd is drawn on Cancel, where the key actually fires; confirming is a
  deliberate move to the other control, and `Esc` cancels as well. A
  destructive action is never one keystroke from a focused row, which is
  what the two rules together mean.
  **A dialog that collects a field is the exception**, and it is not a
  weakening of the rule: the field *is* the confirmation, nothing is
  destroyed by pressing it, and the kbd is drawn on the confirm because
  that is where `Enter` fires there. Every dialog that collects a field
  before it confirms works this way: Redirect, Overrule, Report,
  Raise the cost cap, Raise the turn cap, Add task.
  This line read "**Cancel holds initial focus**, `Enter` confirms,
  `Esc` cancels" until 2026-09-02. Both halves were true of something
  and the sentence did not say which won, so the implementation guessed
  — it bound `Enter` on the window and confirmed past the focused
  Cancel, which is exactly the one-keystroke destruction the rule above
  refuses.
- **A hold confirms in place.** Kill on Job detail fills while held and
  commits only once it has been held for `--duration-hold`; releasing,
  leaving the control or cancelling the pointer before then does
  nothing, and it behaves the same from Space or Enter. It sits beside
  the dialog rather than replacing it: `x` and a row's menu still reach
  the dialog. **Under `prefers-reduced-motion` the hold is not offered**
  and the control opens the dialog, because the fill is the only thing
  that shows how long is left.
- **Single-key shortcuts are suppressed whenever a text input holds
  focus.** Typing "axe" into a filter box must not approve, kill, and
  open something.
- **Pilot is exempt.** Once the terminal has focus, every keystroke
  belongs to the terminal. Only `Esc Esc` releases it.

### Focus model

Focused and selected are different states and can coexist. A 1px ring
around a full-width table row is nearly invisible, so the row does
something stronger.

```
focused row   2px --accent left edge bar + --bg-hover
selected row  --accent-muted fill
focused ctrl  2px --accent ring at 2px offset, per the global focus rule
```

Focus is visible at all times during keyboard navigation, not only on
`:focus-visible` heuristics — if the person is driving with `j`/`k`, the
ring is the cursor.

### Command palette

A floating layer, so `--bg-overlay` and a shadow.

```
surface    --bg-overlay · --border-default · --radius-lg · shadow
width      560px · max-height 400px · scroll-area beyond
anchor     top-aligned at 20% of viewport height, never centered
input      --text-base · no border · --fg-default · placeholder --fg-subtle
           bottom rule --border-subtle
row        32px · 12px padding · --text-sm · --fg-default
           16px leading icon · shortcut right-aligned as kbd
active row --bg-hover
section    --text-2xs · --fg-subtle
```

Top-anchored rather than centered because a centered dialog shifts
vertically as the result count changes, and a target that moves while
you type is a target you misclick.

**Contents, in order:** actions available on the current context,
navigation, jobs by id or name, settings.

**The palette obeys the lexicon.** Displayed labels always use the
lexicon term — Kill, Drone, Convoy. The search index may carry aliases
so that "terminate" finds Kill, but the alias never renders. This is
where the lexicon earns its keep: one vocabulary, searchable, with the
shortcut shown beside every entry.

**A matched span is marked by contrast**: `--fg-default` against the rest
of its label at `--fg-muted`, and never by a hue or a fill. Status hue is
never chosen and the accent is reserved to interactive affordance, so a
step within the row's one hue is the channel left. A danger row marks the
match by underline instead, because red has no contrast step. **An
alias hit marks nothing**, because the match was on a word that never
renders and there is nothing on the lexicon term to mark.

**The leading glyph read `12px` here until 2026-09-03.**
[Iconography](iconography.md) is the authority on what a silhouette
means and at what size it holds, and this line was transcribing it
wrongly: every glyph the Navigation section draws is assigned 16px
there, and `hard-drive`'s row records that its interior marks merge into
the band above them below that size. The record was wrong rather than
holding a second opinion, so it is the record that moved. A 32px row
takes 16px comfortably.

**A row that cannot act draws dimmed and says why**, in `--fg-subtle` at
`--text-2xs`, beside its binding. Two kinds reach it: a binding the
registry carries that nothing answers, where `unbuilt` names the issue,
and an act the app cannot reach from where you are standing. Both are
facts rather than a list of exceptions, which is the same reasoning the
`not built` annotation carries under Two tiers.

**One exception: an act on one Job, with no Job focused, is left out.**
Open, Review, Attest, Redirect, Kill, Redispatch and Restart step need a
Job to act on. With no Job open and nothing under the Board's cursor, six
dimmed rows all saying *no job focused* led the palette and told a person
nothing they could act on. They come back when a Job is focused, which
is where their shortcuts are learned. A `not built` row is never left
out. Settled 2026-09-17.

**The palette is the discovery surface.** It is how a person learns
forty shortcuts without a cheat sheet, which is why every entry displays
its binding and why no action may exist outside it.

### kbd

The one non-shadcn primitive. Used in palette rows, dropdown-menu items,
and tooltips.

```
surface  --bg-raised · --border-default · --radius-sm
lip      bottom edge at twice --border-width
type     --text-2xs mono · --fg-muted
size     20px height, border included · 4px horizontal padding
```

Never `--fg-default` — a shortcut hint is reference material sitting
beside the thing it describes, and rendering it at full contrast makes
it compete with the label.

**Raised, and edged like a card rather than ruled like a table.** This
was `--bg-sunken` behind `--border-subtle`, and on any hovered or
focused row the edge was not visible: `--border-subtle` is `#232B35`
and `--bg-hover` is `#212A34`. A key drawn on a row appears only while
that row is focused, so the one ground it was guaranteed to sit on was
the one it disappeared against, leaving a letter loose beside a button.
`--border-subtle` rules a table; an edge is `--border-default`.

The fill follows the same argument. A key is pressed, so it stands off
its surface; `--bg-sunken` is a well you type into, which is the
opposite claim. The doubled bottom edge is the lip, and it is what
makes the box read as a key rather than as a small chip.

### Consequences elsewhere in this document

- **Tooltips gain a trailing kbd** where the action has a binding. The
  400ms delay stands.
- **Dropdown-menu items gain a right-aligned kbd.** Item height is
  unchanged by it.
- **A confirmation's buttons gain a trailing kbd**, on the one control
  `Enter` fires — Cancel on a plain confirmation, the confirm on a
  dialog carrying a field. One per dialog, because two would be the
  ambiguity the safety rule above was rewritten to remove.
- **The 48px sidebar rail is more usable than it looks**, because the
  ⌘-digit bindings reach every surface without labels.
- **A control may draw its own Global binding while `⌘` is held**, and
  never at rest. The sidebar's rows, Helm's reopen button and the Send
  button in a message box each do; the badge mounts on the hold and goes
  on release. This is not the rule under the debug payload — that one
  refuses a kbd standing permanently on a button, which is reference
  material sitting on the thing it describes. A hold is asked for: a
  person pressing `⌘` is looking for what it reaches, and a badge that is
  gone a moment later cannot compete with the label. Every reveal reads
  one listener through `ShortcutRevealProvider`, because a hold watched in
  two places ends in two places. Contextual bindings are never revealed
  this way — holding `⌘` fires none of them.

---

## Component → token mapping

Tokens alone don't determine a screen. Without this section a design
tool infers which token each primitive uses — plausibly, and differently
every session, which is a slower version of the drift this contract
exists to prevent. Below is the binding for the primitives a job list
needs. Anything not listed follows the same logic: surfaces from Ground,
text from Foreground, interaction from Accent, and status **only** from
the status tokens.

**Global.** Focus is a 2px `--accent` ring at 2px offset, no glow. It
was a 1px `--border-strong` ring; once a secondary button took
`--border-strong` as its resting edge, the two were the same colour,
width and position and focus rendered as nothing — a resting edge and a
focus ring must differ on all three. Control focus now matches row
focus, and the accent already carried the keyboard focus edge. Disabled
is `--fg-subtle` text with hover suppressed — never reduced opacity,
which muddies status colors. **Dimming is a token, not an alpha** — a
de-emphasised row steps down to `--border-subtle` and `--fg-subtle`.
Every interactive element transitions on `--duration-fast`.

### Table — the Job Board row

The densest thing in the app and the reason the spacing scale is tight.

```
header row   32px · --bg-base · --text-2xs · --fg-subtle
             uppercase, 0.04em tracking (the one legal ALL CAPS)
             bottom rule --border-default
body row     36px · --bg-raised · 12px horizontal padding
             row rule --border-subtle
hover        --bg-hover
selected     --accent-muted
primary cell --fg-default · --text-sm
secondary    --fg-muted · --text-sm
metadata     --fg-subtle · --text-2xs   (timestamps, elapsed)
mono cell    --text-xs mono              (job id, path, branch, duration, cost)
```

No zebra striping. At 36px rows it reads as noise, and the row rule
already separates. Status appears as a badge in its own column **and** as
a row tint of its own hue at `--row-tint` (5%), so a person scanning the
Board sorts rows by state without reading a badge. This read "never as a
row-background tint" until 2026-09-16: that was true at the badge's 12%,
where eight tinted rows were unreadable. At 5% it is a wash. Hover and
selection **replace** the tint rather than mixing with it, so
`--accent-muted` still owns selection, and the row's left edge stays the
focus and hover edge — status takes no edge of its own.

**An identifier copies on click.** A job id, a drone id, a branch
name — a value whose whole use is being quoted somewhere else — copies
to the clipboard when clicked and goes to `--accent` on hover.

**A path or a command is mono and does not copy by default.** Both have
somewhere to go: a path opens where it lives, a command opens what it
did. A surface with nowhere to open to may fall back to copying, but the
gesture belongs to the destination first, and a file list whose paths
copy instead of opening is the case this rule was narrowed for. It carries no `copy` glyph: the
affordance token is the affordance, and a 12px icon repeated down
fourteen rows is the noise Iconography's default-to-no-icon rule exists
to prevent. A toast confirms, because a clipboard write is silent by
nature and a failed one is otherwise indistinguishable from a dead
element. A value that copies does not also get a button that copies it.

### Badge — status

The one place status tokens are used directly. `{state}` is the enum
variant, and the label comes from the enum→verb table in the Voice &
Copy section, never hand-written.

```
background  --status-{state}-bg   (12% opacity variant)
text        --status-{state}
border      none
height      20px · 6px horizontal padding · --radius-sm
type        --text-2xs · weight 500 · sentence case
icon        required, 12px lucide, strokeWidth 2, leading, inherits text color
```

Icons are how the escalation sub-reasons and the `not_started` axis
values differentiate, since they share one hue each. Every badge state
carries an icon, so the column never reads as ragged and hue is never
the only channel. Full specification on [Iconography](iconography.md).

12px rather than 11px is deliberate: lucide draws on a 24px grid, so
12px is an exact half-scale and a stroke of 2 lands on exactly 1px. 11px
scales to 0.917px and antialiases into fuzz on a dark ground.

**A bordered pill is a Job state and nothing else.** An origin tag and
provenance are plain sans in `--fg-muted`; drift states are plain sans in
`--notice-caution`. Two chips in
one row separated only by colour makes a reader learn a rule the screen
never states.

**The badge carries no leading dot.** Its job was telling a status chip
from a bordered pill that is not one; origin no longer carries a chip,
and with an icon mandatory on every state the dot is a second marker for
one claim.

### Button

| Variant | Rest | Hover | Use |
| --- | --- | --- | --- |
| Primary | `--accent` fill, `--fg-inverse` text, `--shadow-primary` | `--accent-hover` | One per view. Approve, Dispatch |
| Secondary | `--bg-sunken`, `--border-strong`, `--fg-default` | `--bg-hover` | Everything ordinary |
| Ghost | transparent, `--fg-muted` | `--bg-hover` • `--fg-default` | Row actions, icon buttons, toolbars |
| Destructive | transparent, `--status-completed-failed` text and border | fill at 12% | Kill only. Never a filled red button |
| Tonal | `--accent-muted` fill, `--accent-hover` text, no border | 24% `--accent` mixed into `--accent-muted` | Chrome present on every screen, carrying the app's main entry. The title bar's Dispatch. Never the one solid accent of a view |

```
height   36px default · 32px sm (use sm inside table rows)
padding  16px default · 8px sm
type     --text-sm · weight 500 · --radius-md
```

**Emphasis comes from fill, not size.** A primary action is `--accent`
fill at the normal 36px control height; a call to action is never scaled
up to make it matter. **A list row never takes one** — every row carries
one secondary control, because fourteen rows offering a decision would
be fourteen accent blocks. Urgency on a list is carried by the badge and
the ordering, and the accent is spent on the detail screen, where the
object of attention is one thing.

**Every button in a group is the same height.** A ghost action recedes
by losing its fill and dropping to `--fg-muted`, never by shrinking.
Mixed heights in one row read as a rendering bug.

**A secondary is filled one surface step from its ground** —
`--bg-sunken` on a card, `--bg-raised` on a sunken or overlay row. A
button filled the colour of the surface behind it shows only its text,
so it looks shorter than the primary beside it even where the boxes
match exactly.

Destructive stays outlined because a solid red button reads as an error
state rather than an action, and `--status-completed-failed` is already
spoken for as a *status*. Kill is deliberate, not alarming.

**Pending is the pressed control, and only that one.** Every variant
collapses onto one pending rendering — `--bg-sunken`, `--border-strong`,
`--fg-default` text, and the bar under Motion — so the button a person
pressed is the one control in its group that is not greyed out. It stays
focusable and refuses a second press. Disabling alone was the treatment
before, and it drew the pressed control exactly like its siblings.

### Input

```
background  --bg-sunken        (recessed, opposite of raised)
border      --border-default
text        --fg-default · placeholder --fg-subtle
focus       2px --accent ring at 2px offset
invalid     --status-completed-failed border, message below in --text-xs
height      36px · 8px horizontal padding · --radius-md · --text-sm
```

Select, checkbox, radio, and switch inherit the same border, focus, and
height rules. Switch uses `--accent` when on, `--border-strong` when
off.

### Dropdown menu

A floating layer, so it takes `--bg-overlay` and is the one place a
shadow is legal.

```
surface    --bg-overlay · --border-default · --radius-lg · shadow
item       32px · 8px padding · --text-sm · --fg-default
hover      --bg-hover
danger     --status-completed-failed text, --bg-hover on hover
separator  --border-subtle
label      --text-2xs · --fg-subtle
```

Sheet and dialog use the same surface treatment at `--radius-lg`.
Where it opens, which edge it aligns to and what it does when it does not
fit are under Floating layers.

### Tooltip

```
surface  --bg-overlay · --border-subtle · --radius-sm · shadow
type     --text-xs · --fg-default · 8px / 4px padding
timing   400ms delay in, --duration-fast
group    300ms grace: inside it the next tooltip opens on arrival
```

**A tooltip carries one of three things, and never a fourth.** A path,
a branch name or a full timestamp — the value behind an abbreviated
one. What pressing a control does, written at the control. Or what a
word naming an Armada concept *is*, from the one place those sentences
live. It never restates the text it sits on, and it never carries an
explanation the row should have made plain, per the briefing-register
rule. A criterion sentence, a Judge's grounds, a log line and a step's
own name already read as themselves.

**An act names the control it is on, and never the one beside it.** A
reader who presses on the strength of one that describes a neighbour
has learnt that the surface lies, which costs more than every correct
hover on the screen was worth — so this is worse than no tooltip, not
merely less good. The run tree's chevron nearly took *Click to open
details in the right panel*: true of the step name one column over, and
false of the chevron, which opens that step's facts in place.

**The delay is per group, not per tooltip.** Crossing a row of eight
annotated chips is one wait: the first waits out `--tooltip-delay`, and
for `--tooltip-grace` after any one closes the next opens on arrival.
A delay held per instance is eight waits, which is why nobody reads the
second chip.

**A concept sentence has exactly one home.** The three gate tiers are
`phaseSaid`; every other word is `concepts.ts` beside it, keyed by the
word a reader sees. Two explanations of what a Check is would be two
things that can disagree, which is the distinction the gate rests on.

Placement, alignment and collision are under Floating layers.

### Stats panel

The left column's second panel — what used to be the status bar's two
counts, gathered with what Overview's own tiles already read for Drones and
Manifest drift.

```
row     --text-xs, --dot, label in --fg-muted, value right-aligned in --font-mono
value   --fg-default at rest; past zero, amber (--status-awaiting-review)
        for Awaiting approval and Needs review, red (--status-escalated)
        for Escalated
dot     the row's hue, mixed into the ground at --dot-idle while the count
        is zero, full past it
```

**Every row carries a dot in its own hue**, so the panel has colour on a quiet
day, and a full dot still means something is waiting.

| Row | Dot |
| --- | --- |
| Awaiting approval, Needs review | `--status-awaiting-review` |
| Escalated | `--status-escalated` |
| Jobs | `--status-not-started` |
| Drones | `--stat-drones`, dim with nothing running |
| Manifest | `--stat-manifest-current`; `--notice-caution` while behind |

`--dot-idle` is 65%, the lowest mix that keeps every dot at 3:1 against
`--bg-glass`. A dot is a mark and not an icon, so Iconography's text-only rule
for this panel stands.

**Three counts, not two.** Awaiting approval and Needs review are the old
approval count, split by which gate a Job is waiting at; Escalated is the
old escalation count, carrying the louder tone the same way it always did.
All three span every repository Fleet serves, whichever one Navigation has
picked. **No Queued row**: Overview's own Queued panel already lists those
Jobs, and a count here would be a second place to check the same fact.

**The panel's own dot, at the collapsed 48px width, is the worst tone among
its rows** — red past an Escalated count, amber past any other, neutral
otherwise — the same rollup reasoning Doctor's pass/warn/fail uses.

### Overview summary tiles

Overview's four counts, each its own card, two by two.

```
tile     the card treatment under Depth, --radius-lg, and a radial wash of
         the tile's hue at --status-wash from the top leading corner
label    --text-xs, weight 600, in the tile's hue, after a --dot of it
count    --font-mono --text-2xl, --fg-muted; past zero, the tone the
         caller passes
```

| Tile | Hue |
| --- | --- |
| Needs you | `--status-awaiting-review` |
| Running | `--status-running` |
| Queued | `--status-not-started` |
| Recently ended | `--status-completed-success` |

**The label takes the hue at every count, and the count only past zero.** An
amber "0" reads as something waiting. The label reads 4.39:1 or better on its
own wash, `not_started` the lowest.

### Overview empty state

```
card     the card treatment under Depth, centred content
line     "No jobs." --text-base weight 600, then "Propose one." --fg-muted
action   Dispatch, Primary — the view's one solid accent
```

**No icon.** Iconography's empty-state rule holds. **The title row's Dispatch
stays Tonal** while this one is Primary, so the view still carries one solid
accent, and it is the one beside the empty space.

### Helm dock

```
frame    a 1px --helm-edge top edge fading to --border-glass, over the card
         treatment, with --helm-faint washed in from the top trailing corner
chip     --helm-muted fill, --helm icon, beside "Helm"
folded   the sheet the dock folds to below the breakpoint takes the same frame
         and chip, over the sheet's --bg-overlay: --helm reads 4.65:1 on its
         chip there, against the 3:1 a non-text mark takes
composer --bg-sunken, --border-glass
Send     --helm-muted fill, --helm text, --helm-edge border
```

### Instruments

**A mark that draws a measurement is an instrument, not an icon or an
illustration.** Twelve minutes of a Drone doing nothing is a flat line no
figure can show, and a footprint's shape says where the work went before a
path is read. The ban on decorative iconography stands; an instrument is
exempt only while every mark in it is a value from the wire.

| Instrument | Where | Draws |
|---|---|---|
| **Activity** | A running Job | Tool calls per 30s window over the last twelve minutes, one bar per window |
| **Footprint** | A finished Job's footprint | Each file as a column, width by lines changed, split into added over deleted |

```
activity bar      --instrument-activity at 75%; an empty window --border-subtle at 1.5px, never absent
activity axis     1px --border-default along the base
footprint added   --diff-add-fg at 55%
footprint deleted --diff-del-fg at 45%
outside the plan  1.5px --instrument-outside-plan outline, and the words beside the key
labels, key       --font-mono --text-2xs --fg-subtle
```

**An instrument takes the hue of what it measures.** Tool calls are the
running step's work, so activity draws in the running hue, aliased once in
`tokens/status.css`. A file outside every declared plan is drift, which a
badge already draws amber, so its outline is amber too. A file with no line
count is left out of the footprint and counted in its key, because a guessed
width is a wrong measurement.

**An instrument carries its own reading in words.** Each has a label naming
what it measures and an accessible description stating what it shows, such as
*no tool calls for the last 12 minutes*. It never animates in, and a new
reading replaces the drawing rather than moving it.

### Fleet panel

The left column's third panel — what the status bar used to read.

```
state    --dot (6px) + --text-base --fg-default
rows     pid / port / protocol / up, one row each: label --text-xs
         --text-label on the left, value --font-mono --text-xs --text-body
         right-aligned to the panel's edge, where Stats puts its counts —
         Pulse's figure rows, FigureList
detail   --font-mono --text-2xs --fg-subtle, the sentence a state carries
doctor   border-top --border-subtle above it; a dot, "Doctor", the outcome
         (--status-completed-success / --status-awaiting-review /
         --status-escalated) and the modules checked, in --fg-muted
```

**The rows were settled on 2026-09-17**, replacing two `·`-joined mono lines,
at the cost of two lines in a column that was already tall. **A state draws
only the rows it has a value for**, never a label beside a blank: a Fleet
being connected to or not answering has pid and port, and protocol and up
arrive with the connection.

**Fleet's state is one of three, and the panel's dot names which** — the
same three the status bar used to carry, on the same grounds Doctor's pass,
warn and fail reuse the Job values rather than inventing a third set. A
connection reading none of the three — reading the runtime file, connecting,
a refused runtime file, a protocol Bridge does not speak — keeps a neutral
dot and names itself in the label instead. It is not a glyph, so "this panel
carries no icons" is unaffected; see [Iconography](iconography.md).

```
running       --status-completed-success dot
              "Fleet running" · rows: pid, port, protocol, up
not running   --status-escalated dot
              "Fleet is not running" · mono: what the runtime file says —
              no file at its path, a pid held by nothing, or a pid held by
              something else — plus what to do, since Bridge cannot start it
unreachable   --status-awaiting-review dot
              "Fleet unreachable" · rows: pid, port · mono: alive, no
              answer for N, plus how stale the last read is
```

**Drone count moved to Stats.** This panel's rows carry pid, port, protocol
and up only; the running count is Overview's own arithmetic, read once and
shared by both panels.

The two failure states differ on the runtime file, which is the fact
that separates them: Fleet writes port, pid and protocol version on
startup and removes them on a clean exit, so a missing file is a Fleet
that is not there and a live pid with no answer is a Fleet that is
wedged. Two different things to do about it, so two sentences rather
than one timeout message.

**A version gap rides the running state, not a fourth dot.** Where Fleet is
newer than this Bridge — additive only, so nothing drawn is wrong — a
detail line under the rows names both versions, as advice on a healthy
connection rather than a failure notice. See `../practices/protocol.md`,
What Bridge does with the version it reads.

### Closed

- **~~Icon set for the escalation sub-reasons and the `not_started` axis
  values.~~** **Closed.** Specified in full on
  [Iconography](iconography.md) — every badge state, navigation,
  actions, Doctor, and the rule for anything unlisted. lucide-react
  confirmed against Phosphor, Tabler, Radix and Heroicons; hard rule 5
  stands. The enum→verb test asserts an icon entry in the same pass.
- **~~Window and layout model~~** **Closed.** Specified in full under
  Window and layout model above — frameless `hiddenInset` chrome insetting
  the traffic lights over the title row, a collapsible/resizable left column
  carrying Navigation, Stats and Fleet as one unit, full-width routes with no
  inspector but for Helm's dock, no page head and no status bar, and the
  floors and breakpoint under Responsive behaviour. Delivered as one
  responsive prototype rather than per-width comps.

---

## The error treatment

A failed Job is Armada working. An error is Armada failing. Both are red,
and they are told apart by shape.

What an error carries and how it crosses the wire is the [Error
Contract](error-contract.md). This section governs what a person sees.

### One red, told apart by shape

**`--error` aliases `--status-completed-failed` and carries no value of its
own.** One red, and no ninth hue to keep in step when the state machine
moves.

**Shape separates an error from a status, on two channels.** An error is the
only solid fill on a data surface, where every Job status is a 12% tint in a
chip; and an error always carries a code, which a status never does.

**The code is always shown, in mono at `--text-2xs`.** It is what a person
reads back to someone else, and the wire guarantees one on every error.

**No generic alarm glyph.** `triangle-alert` is Doctor's and `octagon-alert`
is `stalled`'s, so an error carries the code and the sentence instead.

```
code chip  solid --error or --degraded fill · --fg-inverse text
           --h-badge · --space-2 horizontal padding · --radius-sm
           --text-2xs mono · weight 500 — the status badge's geometry exactly
edge       leading, --error-edge or --degraded-edge. Never a box: the solid
           fill in this treatment belongs to the chip
surface    the placement's own. Inline adds none; banner and full-surface
           take --bg-raised; toast takes --bg-overlay and a shadow
```

### Two fault classes, and only one is red

| Class | Edge | Headline | Dot | Means |
| --- | --- | --- | --- | --- |
| Fault | `--error-edge` in `--error` | `--error` | none | Armada cannot do the thing |
| Degraded | `--degraded-edge` in `--degraded` | `--fg-default` | `--degraded-dot` | Armada cannot refresh what it shows |

**Unreachable Fleet and dropped events are degraded, not faults.** The fixes
are opposite — restarting Fleet is wrong when the process is alive — so the
dot has to differ.

**The dot is amber rather than red.** Amber already means a person is waited
on rather than something being broken, and stale data is a wait.

**One value, not a ladder.** Placement carries blast radius, so severity picks
nothing but the edge.

### The four placements

| Placement | Where | Rule |
| --- | --- | --- |
| Inline | In the row, or beside the act | Contained to the thing you touched |
| Toast | Bottom trailing, inset `--space-6`, shadowed | The only one that may carry no act |
| Banner | Above the surface, inside it | Persistent. The surface works beneath |
| Full-surface | Replaces the surface | The one placement that takes the screen |

**Blast radius picks the placement, never severity.** Approve-refused is
red-serious and affects one row, so it renders in that row and nowhere else.

**Rows around an inline error are undisturbed.** Same height, same badges, and
the pulse continues.

**Every placement names the failure and the act.** A toast is the one
exception, because it reports something already over.

**A toast used to clear the status bar rather than cover it**, because the
bar spanned the window's bottom edge and a bottom-right toast could
otherwise sit over it. Fleet's liveness statement is now the left column's
own Fleet panel, nowhere near a bottom-right toast. `Toast.css`, the app
shell's own toast region and `ErrorNotice.css` all carried the stale inset
against the deleted bar; all three now sit at the plain inset this document
names, and `--h-status-bar` is gone.

### The debug payload, and what each placement does with it

**Every error carries the payload. The four placements differ only in whether
it is shown, offered or expandable** — which is the placement's blast radius
again, not a second decision.

| Placement | Form |
| --- | --- |
| Inline | Ghost control, copying directly. A row has no room for an expanded view |
| Toast | Its one action. Copies and dismisses in one press, because a toast is often the only sighting |
| Banner | Copy, plus **Details** opening the expanded view. A standing condition gets read, not only quoted |
| Full-surface | Shown rather than offered. Nothing else is on the screen |

**The act is called "Copy debug info" wherever it appears, and it is bound to
`c`.** One verb for one act, from the contextual key map above — the control,
the palette entry and the tooltip all say it. It names the artifact rather than
what somebody is about to do with it, because the decision being taken is
whether to paste a machine record into a public issue.

**The control carries no glyph and no kbd.** No glyph, because nothing in the
error treatment carries one and `triangle-alert` and `octagon-alert` are
spoken for. No kbd, because a binding is discovered in the palette and the
tooltip, which are the two surfaces this document gives one to — a kbd inside
every button that has a binding would put the reference material on the thing
it describes.

**The key runs the control's own function, never its own copy.** A binding that
reimplemented the write would be a second artifact the day either side changed,
and this whole treatment rests on there being one producer.

**The expanded view renders the exact string the control copies.** One producer
formats it, so what was read on screen is what arrives in the issue body — not
two renderings agreeing about field order on the day they were written. What
the artifact holds is the [Error Contract](error-contract.md).

**A clipboard write is silent, so a toast confirms it — and the toast carries
no status dot.** A leading dot carries a Job state and is never chosen, and a
clipboard write is not a Job state.

**The act is not an error's alone.** A Helm session carries the same payload
under the same word, bound to the same key, with one producer and an expanded
view rendering the string the control copies — `../concepts/helm.md`,
*Reporting a bad answer*. It takes the banner's form, because a dock is a
standing surface: the copy acts on one press and a second control opens the
reading beside it. Its safety sentence is its own — nothing in it is a
structured field a type bounds, so the sentence below cannot be reused over
it.

**One sentence about safety, in the expanded view only, stating the mechanism
rather than promising an outcome.** It is bounded to what the mechanism
reaches: structured fields carry primitives and a credential does not compile
into one, while the message and the chain are prose an error wrote and nothing
bounds those. A claim over the whole artifact would be a promise the type
system does not make, and it makes none about the wider context — see the
file-an-issue flow, which is not bounded this way.

### Filing, which is a second act with a review

**Copying stays on the machine. Filing leaves it.** So `Copy debug info` acts on
one press and **File an issue** opens a dialog first, naming every item that
would go, showing its text, and offering a control to take it out. **Send is
never one press from an error.**

**It appears on the full-surface state and in the expanded view, and nowhere
else.** A review needs the artifact legible in full; an inline error has no room
for one and a toast is gone before it would be read.

**Armada makes no scrub claim, and the dialog says what it does not do.** Every
row carries a sentence naming what is unbounded about that item — which is the
read-this mark, made specific — and the row that cannot be removed carries the
payload's own safety sentence rather than a claim written for the dialog. A
promise Armada cannot keep is worse than the work of reading.

**The confirm copies.** Nothing in Armada opens anything in a tracker, and the
dialog states that in those words. What the artifact holds, and what the drawing
asked for that follows from having no transport, is the [Error
Contract](error-contract.md).

---

## Voice & Copy

### Typography of reference — applies to docs, not just UI

Unlike the rest of this section, these rules govern **internal
documentation and planning pages as well as product copy**. They exist
because the docs are read constantly and their conventions leak into the
product.

- **Never use `§`.** Write "M0 step 4," not "M0 §4." The section sign is
  legal-brief and academic-citation typography; it reads as affectation
  in a working document and nobody says it out loud. "Step" is one
  syllable longer and infinitely more readable.
- Same reasoning bans `¶`, `cf.`, `ibid.`, `op. cit.`, `viz.`, and
  `q.v.` Write "see," "compare," or "same source."
- `e.g.` and `i.e.` are fine — they are common enough to have stopped
  reading as Latin.

**Citing v1 code.** A bare file path on any Armada page refers to
**v1**. The convention is that such paths resolve against branch
`v1-archive` / tag `v1-final` and never `main`, because Ground Zero step
1 orphans `main` — but **that step has not run**, so neither the branch
nor the tag exists in the clone. Verified when the v1 port-allocation
extraction found the files present on `main` and cited a commit hash
instead.

So, until it runs: v1 paths resolve against `main`, and **a citation
should carry a commit hash**, because line numbers on a live branch are
not stable. After it runs: the convention applies as written and
existing citations need re-anchoring — a commit hash cited today still
resolves from `v1-archive`, so nothing is lost by citing one now. Line
numbers stay accurate against `v1-final` once it exists, because a tag
is frozen.

Approval prompts, status reasons and escalation messages are what you
read at 11pm deciding whether to kill a drone. These rules apply to
product copy, not to internal documentation.

**Scope split.** This contract governs static UI chrome, which is not
configurable. The Machine-level **Voice** setting tunes runtime-generated
prose (Judge summaries, Helm replies, job summaries) within this
contract. It may adjust length and formality. It may not override the
principles, the lexicon or the status grammar. "Terse" and "explanatory"
are legal Voice values. "Playful" is not.

### Principles

**P1. Metaphor lives in proper nouns only.** Nautical vocabulary is
confined to names: Armada, Fleet, Bridge, Helm, Drone, Manifest, Convoy,
Job Board. Kit and Machine are lexicon terms but carry no metaphor —
they say what they are. Every verb, state, error and instruction is
plain English. Write "Drone 4 stopped reporting 12 minutes ago", not
"Drone 4 has gone dark". Pilot is a proper noun; "take the wheel" is not
a verb Armada uses.

**P2. Briefing register.** A message carries the facts needed to decide,
on screen, without a click. Weak: "Drone 4 stopped reporting. Poke limit
reached." Correct: "Drone 4 stopped reporting 12 minutes ago after 3
pokes. Step 2 of 5, last wrote `auth/session.rs`."

**P3. First person is Helm's alone.** Bridge and Fleet never say "I".
Helm says "I" only for what Helm itself did. Reporting a Fleet event,
Helm uses the same impersonal phrasing Bridge does.

**P4. Hedge by source.** Three source classes, three registers.

- **Measured** speaks flatly. "Tests passed." "`pnpm test` exited 1 on 4
  assertions."
- **Estimated** is marked as approximate. `~$2.40`, never `$2.40`. A
  derived figure is not a measured one, and rendering it with the
  authority of an exit code trains you to act on a number that may be
  wrong.
- **Judged** is visibly a judgment and names its source. "Judge read the
  evidence as not covering the error path."

Render any two of these identically and one bad value teaches distrust
of the other two.

**P5. Event-first, with cause.** The subject of a failure sentence is
the job or step, never the drone. Write "Step 3 did not advance. No
evidence after 3 clarification rounds", not "Drone 4 failed to submit
evidence". Known causes state flatly. Hypothesised causes hedge and name
their source, and only Judge and Helm may produce them.

**P6. Fixed copy is a template; generated copy is a substance
requirement.** Fleet's own strings should be identical every time,
because uniformity is scannability. Generated text is specified by what
it must contain, never by what shape it takes, because a structural
rule produces twenty interchangeable paragraphs. A summary that would
read plausibly under a different job has failed.

### Prose rules

- **Sentence case everywhere.** No title case, no ALL CAPS except table
  headers at `--text-2xs` with `0.04em` tracking. Lexicon proper nouns
  keep their capitals inside sentence case.
- **Name things by what the person controls.** "Approve dispatch", not
  "Submit job payload".
- **No mid-sentence asides.** The rule targets the reflex rather than
  the character, because banning the em dash breeds a colon and banning
  the colon breeds a trailing negation. A colon separating a field from
  its value stays legal: "Step 3 stalled: no evidence after 3 rounds."
- **No adverbs by default.** "Successfully completed" is "Completed".
  "Currently running" is "Running".
- **No Wh- sentence openers.** They survive as panel headings: "Why
  this stalled", "What ran", "What changed".
- **No sentence that survives deletion without loss.** Remove it and
  see whether anything was lost.
- **Errors say what happened and what to do.** Never apologise, never
  be vague.
- **An action keeps its name through the flow.** A button that says
  Kill produces "Killed". The verb table below enforces this.

  An act that moves no Job status is not in that table, because there is
  no status for it to render:

  | Button | Produces |
  | --- | --- |
  | Add task | Added |
  | Drop | Dropped |

  `copy.ts`'s own labels are what enforce these.

### Lexicon

- **Armada** the app. Never the tool, the system.
- **Fleet** the daemon. Never the backend, the server, the sidecar.
- **Bridge** the operational surfaces. Never the dashboard, the UI.
- **Helm** the dock and its agent. Never the assistant, the chat, the
  surface.
- **Drone** one agent instance. Never the agent, the bot, the AI,
  Claude.
- **Job** one unit of work. Never run, ticket.
- **Task** one line of a Job's [plan](../concepts/plan.md). Never step —
  a step is the workflow's own unit, and a Job's plan is its own account
  of the work rather than the workflow that gates it.
- **Convoy** a multi-workspace job landing as one PR. Never batch,
  group.
- **Job Board** the open queue. Never the queue, the backlog.
- **Job proposer** the model call that reads a request — a prompt, a
  ticket link — and proposes a Job: its workflow, what to call it, and
  where the work is several Jobs, the order between them. Scope is not
  among them, and [Job proposer](../concepts/job-proposer.md) owns why.
  Lowercase, because it is a call rather than a component. Never the
  classifier, the Job-shape classifier, the shape classifier. What it
  produces is a **Job proposal**, and that is what the dispatch gate
  approves.
- **Kit** the tool set you bring — Skills, MCP, sub agents, Agent
  files, Plugins, Commands, the allowlist, the models list. Never
  global settings, preferences. Replaces Guild, retired Aug 2026.
- **Machine** how this installation behaves — resources, timing,
  budget, interface, notification routing. Never system settings,
  environment.
- **Manifest** per-project config. Never the config, the yaml.
- **Judge** the semantic verification layer. Never the auditor, the
  reviewer, AI review.
- **Evidence** the structured completion report. Never the report,
  output, proof.
- **Doctor** the health check. Never diagnostics, system status.
- **Workspace** one unit inside a repo. Never package, module,
  sub-repo.
- **Studio** the typed graph of one stretch of work, kept per repository.
  Never canvas, playground.
- **Scout** the read-only agent a Studio sends to read and report back as a
  Finding. Never researcher, explorer.
- **Drone transcript** the record of a Drone's turns. Never the Drone
  log, the Drone output.
- **Judge record** one Judge call and every judge's verdict inside it.
  Never a transcript — a Judge call is one-shot, and the Judge never
  reads the Drone's transcript, which is the isolation that makes its
  verdict worth anything.
- **Check log** what a Check wrote to stdout and stderr. Never the
  Check output, the Check results.

**Claude is a model name, never an actor.** Write "Drone 4 stalled", not
"Claude stalled". The word appears only where a model is selected or
reported.

### Retired terms

These were in use and are not any more. A page still carrying one is
stale, not merely old-fashioned. Search for them when cleaning up a
page.

| Retired | Now | Note |
| --- | --- | --- |
| Guild | **Kit** and **Machine** | A split, not a rename. Tools and the allowlist became Kit; resources, timing, budget, interface and notification routing became Machine. Each site needs judgment about which one it was |
| Armada Server | **Armada API** | Named for the `api` crate |
| Job-shape classifier | **Job proposer** | It stopped classifying a shape when shape became derived, and stopped naming scope at all when scope became the workflow's first step. Neither half of the old name survived, and both halves misled — the second reading hid the fact that nothing chose a Job's workflow at all. A page still calling it a classifier, or saying it proposes scope, is describing a different call from the one that exists |
| Daemon | **Fleet** | Dropped as a redundant second name for the same process |
| Ground Zero | **M0 — Foundations** | Archived with the phase plan |
| Phase 0 through Phase 6, and numbered implementation steps | **Milestones** and their **Steps** | The nine-phase plan and its ~110 Steps live under "Archive — v2 phase plan" and are reference only. Milestone Steps are disposable and discarded when the milestone is met |

**Casing.** Docs capitalise throughout. UI capitalises the singular
named things (Armada, Fleet, Bridge, Helm, Doctor, Judge, Kit, Machine,
Job Board) and lowercases anything countable (job, drone, convoy,
manifest, workspace, evidence, workflow). So: "No active jobs. 3 waiting
on the Job Board."

### Status grammar

**Shape: headline plus fields.** A headline sentence, facts as labelled
fields beneath. Machine-derived fields render in mono, per the
typography rule above.

> **Job 12 stalled at step 3**
> Workspace `api` · 3 pokes · `auth/session.rs` · 12m · ~$1.80

**Verbs are generated from the enum, never written.** `stalled` always
renders "stalled", never "went quiet". For `escalated` and `queued`, the
headline verb is the reason rather than the state, because nobody says
"Job 12 escalated at step 3". This supplies the labels the token
section relies on to differentiate escalation reasons and `queued`'s
reasons by label rather than hue.

**`queued` takes its reason's verb where one is set.** With no reason it
reads queued; with one set, the reason supplies the headline and the
glyph. A Job out of headroom therefore reads "waiting on resources"
rather than falling through unrendered, which is what the old two-axis
field did.

**The map is a database, not a table on this page.** Every vocabulary
the UI renders is one row per variant, grouped by axis, carrying the
verb alongside the glyph and the hue that variant owes. A row with an
empty verb is a variant with no sanctioned copy, which is what the test
fails on. Pages needing the labels embed a filtered view; none of them
restates a verb. The map itself lives in the Armada Enum Verbs database.

**The plain label is what a queue row shows. The raw enum is
recoverable, never primary.** A row in Alerts carries the verb and
nothing else, matching the voice contract. The enum sits set back in
the **detail view header**, so an engineer can grep Fleet's logs with
the exact string without the queue reading like a stack trace. This
matters most for `fan_out` and `evidence_suspect`, whose plain forms are
not guessable back to the enum; the triggers named for their condition
are near-identity and lose little either way.

`silent` takes no verb of its own. It is a sub-kind of `stalled` and
renders as **stalled** — the difference is entirely in the suggested
action on the payload, which is rephrase and redispatch rather than
plain redispatch. A badge that distinguished them would imply the Job
behaves differently, and it does not.

`thrashing` renders as **churning**. The enum name is OS jargon, and
the distinction that has to survive is busy-but-going-nowhere against
silent, since `stalled` owns silent.

`no_report` renders as **went quiet**, and it means the Drone said
nothing back — never that the Drone did nothing. That word belongs to
`stalled`, which is a Drone producing nothing at all; this one was
producing plenty and ignored an instruction to stop and report. The two
badge apart because a person answers them differently, and a reader who
collapses them has erased the trigger. It is not **churning** either:
that is the finding this followed, and the Drone may have been writing
the whole time it was said of.

`drone_killed` renders as **the Drone was ended by hand**, and the last
three words are the verb rather than padding. `interrupted` renders as
**interrupted** and draws a severed plug, a connection lost; this is a
person pulling it out on purpose, from a step they mean to run again.
Drop "by hand" and the two badges say the same thing about opposite
events, and a reader goes looking for a failure somebody caused
deliberately. It is not **stalled** either — that is usually what the
person killed the Drone *for*, and the Job's own escalation reason
keeps it. This one is the step's.

`run_ended` renders as **the Drone's run ended**, and it is
`drone_killed`'s sibling one actor over: there a person took the process
away, here the Drone said its own run was over and Fleet took it at that
word. Both are passive and neither names who acted, because what a step
badge answers is what became of the step. Not **stalled** for the same
reason as above — that is what the *Job* is held for, and one Job carries
both: the step says what happened to it, the Job says what it happened
for.

`scope_refused` renders as **the scope request was refused**, and the
Judge is deliberately not in it — attribution goes in the source field,
which is `evidence_suspect`'s rule below. It is not **blocked by
policy**, which is the badge it is being split away from: that one names
a configuration to go and edit, and a reader who cannot tell them apart
on the word edits a Manifest when what they should do is read what the
Drone was trying to write. It is not **stopped at the gate** either —
that says work was weighed, and here nothing was submitted.

`unheard` renders as **nothing is reading this Drone**, and it is the
one reason in this vocabulary written in the present tense. Every other
one names something that is over; this names a condition still true
while a person reads it — the Drone is working at that moment, and
nobody is on the other end of it. It is not **stalled**, which is the
badge it is split away from: that word is a Drone that stopped
producing, and a person who reads it here redispatches and ends a Drone
that was finishing its step. It is not **went quiet** either, for the
same reason from the other side — `no_report` is a Drone that could
hear an instruction and did not answer, and nothing was ever sent into
this one. The verb is about the reading rather than about the Drone,
which is what puts the fault where it is; "Fleet cannot hear it" was
refused because a failure headline names what happened and not who,
which is `no_worktree`'s rule above.

`evidence_suspect` renders as
**evidence disputed**, with Judge in the source field, which keeps
attribution out of the headline so P5 holds. Avoid "Judge rejected the
evidence", since `rejected` is already a job state.

The enum-to-verb map is one artifact with a test asserting every
variant has an entry, so a new reason cannot ship with no copy. Same
codegen intent already noted for the status tokens.

**The verdict vocabularies are not Job states and sit on their own
axes**, which is why they group separately above rather than joining
the status list. Step verdict is `workflow_status.last_step_verdict`; a
criterion verdict lives per criterion inside the Judge record and reads
differently by verification source, which is the P4 hedging device
working at the smallest scale it has.

**"No objection" rather than "accepted", and the step rather than the
Judge.** The Judge declines to refuse; it never grants. A pass headline
names the step — "Step 3 of 5 verified" — so attribution stays out of
the headline and the reader knows where they are in the workflow. Avoid
"Judge passed", which breaks both rules at once.

**A criterion attested by a person takes neither vocabulary above.**
Source Attestation reads **confirmed** · **withheld**. Affirmative where
the Judge's vocabulary is not, because a person may grant and the Judge
may only decline to refuse — the three registers are measured, hedged
and vouched, and the words have to carry that before the source field
is read. **Withheld**, not failed or refused: a person who looked and
would not put their name to it has done something neither other source
can do.

All of these belong in the same one-artifact-one-test map as the Job
states.

**Icons.** The token section differentiates escalation and `not_started`
values by label and icon. The verb table supplies the labels. The
icons, across every badge state rather than only these, are specified
on [Iconography](iconography.md) along with navigation, actions, and
the rule for anything unlisted. lucide-react only, per hard rule 5.

That document supersedes the ten-row table this section used to carry.
Four entries changed: `stalled` moved off `hourglass`, which read as be
patient for the state that most needs to read as wrong and is now
banned outright; `blocked_by_dependency` moved off `lock`, which claims
a permissions problem that does not exist; `fan_out` moved off
`git-fork`, which loses its nodes at 12px; and the six base states
gained icons they previously lacked.

**Fields: universal in lists, per-state in detail.** The universal row
carries job identity, state, step N of M, elapsed, spend so far,
verification source, actor. The detail view expands per state and shows
only what applies.

**A fact may point somewhere, and it stays in the run.** Job detail's
header names the pull request Fleet opened — `Pull request #4711`, and
`Pull request #4711, merged` once somebody has taken it — beside the
branch it was opened from, not among the acts at the trailing edge.
Going to read something is not one of the things that end a Job. The
number is what is drawn and never the address: a forge address is sixty
characters of which a person reads four, and the whole of it stays on
the link's `title`. A Job with no pull request draws nothing here rather
than an empty field.

**Workspace is not in the row.** It was, and it came out: a row is
scanned, and the workspace is the field a reader already knows — they
opened this board, and every Job on it is theirs. Spending a track on it
costs the one that answers "is this stuck", which is elapsed. It stays on
the detail view, where a reader is asking about one Job rather than
comparing several.

**On All repositories, with more than one served, the row names its
repository.** There the repository is not implicit the way workspace is,
so a reader needs it to tell one Job's row from another's — see
[Job Board](../concepts/job-board.md).

**Spend follows the active billing mode.** Personal-machine mode gates
on the quota % floor, so the row shows quota % remaining, which is
provider-reported and therefore measured. Work-machine mode gates on
the $ cost cap, so the row shows dollars, marked approximate (`~$2.40`)
until v1's figures are validated against actuals. The visible number is
always the number that gates dispatch. A permanently visible figure
that is not the gating figure is its own failure.

**Verification spend takes its own line, in the active mode.** A Judge
call is Job spend and renders like every other spend figure — quota on
a personal machine, dollars on a work machine. A Judge call priced in
dollars on a machine that gates on quota is a permanently visible
non-gating number, which the rule above forbids.

Spend stays in the row rather than moving to the detail view. It lost
the headline at the approval gate on the promise that it is always
visible, and removing it from the row breaks that trade.

**Layout consequence — the spend column is sized for both modes, not
one.** The two billing modes produce strings of very different width
and shape: `68% quota` against `~$2.40 of $20`. The column is sized for
the wider of the two and the row survives both, rather than fitting
whichever example appears first in this document. Neither mode is the
default — which one renders depends on the machine, and both are
first-class.

**Spend and quota render on the row and on `JobDetail` only.** Bridge/1088
removed both from the shell chrome along with the status bar — the left
column's Stats and Fleet panels carry neither, and nothing in the shell
states a running total any more.

**Repetition.** A second stall at step 3 reads "stalled at step 3, 2nd
time", and the detail view surfaces the prior attempt. Presentation
only for now. Recurrence changing behaviour is a separate decision.

### Register by surface

**Approval gates** stay descriptive: what the job is, which workspaces,
which workflow. They go consequence-forward on blast radius alone,
meaning a Convoy, auto-merge on, a job touching root `armada.yml`, or a
pre-approved batch. Cost never triggers it.

**Push alerts** carry facts rather than a ping. "Drone 4 stalled on
step 2 of 5, `auth/session.rs`, 12 min." Kill and Redirect are not
notification actions. When the line is cut, identity and verb survive,
then location, then elapsed.

**Empty states** point at available work. "No active jobs. 3 waiting on
the Job Board." An empty screen is where you have the least
information, so the one line goes to orientation.

**Helm** answers, then may add a single observation, only after
actually looking, always flagged as its own inference. No
throat-clearing openers.

**Confirmations** appear on everything destructive and state what
happens and what survives. "Kill the drone on job 12? Step 3 of 5, 14
minutes in. Evidence carries forward if you redispatch." Action buttons
name the action. Pilot has no confirmations, since your hands are
already on the terminal.

**Not configurable.** Bridge confirmations always appear. The Kit →
Manifest "destructive-op list" setting governs Drone-initiated
operations only, not your own clicks.

### Behaviour rules that shape copy

**A healthy state is stated, never implied.** "Fleet running" renders in
the Fleet panel even when nothing is wrong, because an empty panel reads
the same whether Fleet is healthy, loading or dead, and Fleet outlives
Bridge. An unhealthy state adds a sentence naming what to do about it; a
healthy one does not, because there is nothing to do.

**Escalations interrupt, approvals queue.** Escalations cost money in
real time. Approvals cost latency. Push inherits this, so escalations
reach your phone and approvals never do.

**Routing config is bounded by this rule.** The loudness order is
silent < in-app < OS notification < push. Routing may move an event
type *down* that order, never up, and **an approval may never be
promoted to push.**

**This one is a contract rather than a preference, and it is the only
"you may not" left in configuration.** It holds because the two event
classes mean different things. An escalation means work has stopped and
nothing progresses until a person looks. An approval means work is
waiting to start and will keep. If approvals could reach push, the
distinction collapses and the escalation signal stops being trusted —
which is what the Stats panel's counts and the push-alert design both
rest on. It is a product rule about what these events mean, not a
config-tier rule: notification routing is a Machine setting with one
value and no merge, so no Manifest is party to it.

**Stats and Fleet**, present in the left column on every surface. Stats
reads Awaiting approval, Needs review and Escalated — amber past zero for
the first two, red for the third — plus Jobs, Drones and Manifest drift.
Fleet reads "Fleet running" when idle, and pid, port, protocol and up as
rows under it;
neither panel carries spend or quota, which left the shell chrome
entirely with the status bar Bridge/1088 replaced.

**Two separate fields, not one.** These do different jobs and a reader
should not have to guess which.

- **Verification source** is the P4 hedging device and nothing else.
  Closed vocabulary of three: **Check**, **Judge**, **Attestation**.
  Only a human may set Attestation — never a Drone, never Helm, never a
  Judge — and a Job carrying an attested criterion must not render
  identically to one where everything was mechanically verified. It
  answers how far to trust a result. **Attestation names the record
  rather than a verifier**, because there is no third verifier: Check
  and Judge are things Armada runs, and this is what a person leaves
  behind.
- **Actor** is audit attribution, and the field the three-way
  separation depends on. Vocabulary: **human**, **Helm**, **Drone**,
  **Fleet**. It answers who did this.

They are orthogonal, and events may carry one, both or neither. A
manual change during Pilot is actor=human with no verification source.
An allowlist denial is actor=Fleet with no verification source, since
Fleet blocking an operation is not a verification result. A failed gate
is verification source=Check with actor=Drone.

---

## Open questions

- **[decay-cap]** Does a Board with many changed rows cap how many carry a
  decay mark? Today every row whose status changes takes its mark for 45s, so
  thirty Jobs moving within a minute tint thirty rows at once and the mark stops
  pointing at anything. What decides it is seeing a busy Board: whether the
  marks still read at that density, and if not, whether a cap keeps the most
  recent few or the most urgent. Change detection and the ticker are in
  `packages/screens/src/recent.ts`.

- **[pilot-exit-bindings]** What are the keys for Close as superseded and
  Override the verdict? Every action owes a verb, an icon and a shortcut, and
  these two have no binding. Both end or overrule a Job's record, so neither
  should take a spare letter by default — the destructive-key rule exists
  because a binding chosen for convenience is one a person reaches by accident.
  Three neighbouring acts took keys in the same pass: Observe `v`, Submit for
  verification `u`, Redispatch `e`.

- **[status-bar-onboarding]** During the hard-gated first-run sequence,
  does the Fleet panel read the same three runtime states as everywhere
  else, or does onboarding get its own reading?
  The panel states a healthy status out loud rather than implying it,
  because an empty panel reads the same whether Fleet is healthy or dead —
  that is settled and gives the three runtime states. But "Fleet is not
  running" is correct and reads as an error on a new user's first screen,
  and it was written for someone who already knows what Fleet is. Fleet is
  started by hand at M1 and becomes reachable partway through onboarding,
  so the panel changes state mid-journey either way; whether the panel is
  even present before the first onboarding step completes is part of the
  same question, since onboarding is not yet a Bridge surface. The slug
  predates Bridge/1088's move from the status bar to this panel and is
  unchanged, since a citation resolves by name.

Also bearing on this document, and written where each belongs: `[attested-verdict-glyph]` in `iconography.md`; `[verdict-artifact-rows]` in `voice-engineering.md`. A question has one home — answering it in two places is how one of them goes stale.
