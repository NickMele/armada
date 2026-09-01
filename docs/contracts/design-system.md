# Design System — UI & Voice

**Kind:** contract. **Governs:** static UI chrome, tokens, and the Voice &
Copy contract — pasted into every design session as the parent contract
that nothing else may contradict.

Read by Claude Design, or any design tool, before generating an Armada
screen.

---

The single contract handed to Claude Design, or any design tool, before
generating an Armada screen. Constraining the input is what makes output
drop into the Electron app without restyling — this replaces per-screen
conversion work.

Paste this document's contents at the top of a design session. Design Job
Board first; it is the densest screen and therefore the real test of the
density and status tokens.

Complete. UI tokens and the Voice & Copy contract are both in force. Two
sibling documents carry what a design session does not need: the [Agent
Copy Contract](agent-copy.md) (text written at runtime by Drones, Judge
and Helm, with its surfaces and their samples in Armada Copy) and the
[Voice Contract — Engineering Requirements](voice-engineering.md).

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
Alerts, Doctor, Manifest. Helm is a sibling conversational surface. The
count is not a contract: a surface earns a place in the rail where a
journey needs one, and the roster lives on Bridge. Active Jobs, Reviews
and the Activity Feed were retired into the Board, which holds every Job
with state as a filter. Alerts stays because an alert is a condition on a
Job rather than a status a Job holds.

**The screen's job:** at a glance, tell one person what is running, what
needs them, and what broke.

This is an instrument panel, not a marketing page. No hero sections, no
gradients, no decorative iconography, no illustration. Density and
legibility win over impact.

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
   `command` (cmdk) backs the command palette. A `kbd` element is the one
   non-shadcn primitive, specified under Keyboard and command palette.
3. **Status colors are never chosen.** They map to the Job state machine
   one to one. Never assign a status color by aesthetic judgment. **Below
   Job level, hue exists only where `tokens/status.css` declares it**,
   and every value there aliases its Job counterpart so the mapping is
   declared rather than inferred. Read the file rather than a list; this
   rule used to enumerate the cases and went stale twice. Anything the
   file does not declare stays neutral. See Below Job level under Tokens.
4. **Dark is primary.** Design dark first. Light exists but is secondary.
5. **Icons: lucide-react only**, used sparingly. A dashboard dense with
   icons reads as noise.

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
--bg-raised      #161C23   cards, table rows, panels
--bg-overlay     #1D242D   dialogs, popovers, dropdowns
--bg-hover       #212A34   row and control hover
--border-subtle  #232B35   table rules, dividers
--border-default #2E3946   card and input edges
--border-strong  #3D4A5A   focus rings, active edges
```

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

Each has a `-bg` variant at ~12% opacity for badge and row-tint fills.

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
new one. The mapping is declared, so a design session reads it instead
of inferring it.

```
--step-advanced    var(--status-completed-success)
--step-running     var(--status-running)
--step-waiting     var(--status-awaiting-review)
--step-failed      var(--status-completed-failed)
--step-failed-bg   var(--status-completed-failed-bg)
--step-stopped-bg  var(--status-escalated-bg)
--verdict-met      var(--status-completed-success)
--verdict-not-met  var(--status-completed-failed)
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

**Refusals sort first, and every criterion row carries its number.** A
card that reorders breaks correspondence with the frozen
`acceptance_criteria[]` order, so a citation to "criterion 4" would no
longer sit fourth on screen. Explicit numbering is what lets both hold:
the rows a person needs are at the top, and the citation still resolves.
See How are criterion verdicts encoded without status hue?

**Everything else below Job level stays neutral.** A Kit file's drift
state, an origin tag and the retry marker carry position, surface,
weight and glyph. Adding a value to `tokens/status.css` is a contract
change, not a design decision.

### Diff

```
--diff-add-bg     #14301F
--diff-add-fg     #6FD196
--diff-del-bg     #351A1D
--diff-del-fg     #E88A8A
--diff-context    #93A1B1
```

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

Weights: 400 body, 500 labels and emphasis, 600 headings. Never 700+.
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

```
--radius-sm  3px    badges, small controls
--radius-md  5px    buttons, inputs, cards
--radius-lg  8px    dialogs, panels
```

No full-round pills except avatars. No shadows on flat surfaces —
elevation comes from `--bg-raised` / `--bg-overlay`, not blur. Shadows
only on floating layers (dialog, popover, dropdown).

---

## Motion

```
--duration-fast   120ms    hover, focus
--duration-base   180ms    panel and dropdown transitions
--duration-pulse  1600ms   the running step mark, and nothing else
--ease            cubic-bezier(0.2, 0, 0, 1)
```

No entrance animations on data. A Job Board that animates rows in on
every poll is unusable. Live-updating values may pulse once on change —
nothing more. Respect `prefers-reduced-motion`.

**One carve-out: the running mark animates continuously.** A hue says
which step is current; only motion says it is still working, and that
is the reading a static rail cannot give — it matters most on the step
that has been running for nine minutes.

**Scope is one animated mark per screen, on the most specific mark
present, and on the thing being read.** Job detail has a rail, so the
rail's current step pulses and the header's Running badge stays static —
the rail names *which* step is working, the badge one line above only
names the Job's state. A list has no rail, so the Running badge pulses
there instead, **on the focused row only**: a list carries one running
mark per Job, and fourteen breathing dots is the thing the first
sentence of this section forbids. The step bar never pulses — its job is
where the work got to, which is a static fact, and the badge sits in a
fixed column on every row so the motion appears in one predictable place
rather than moving with the workflow's length.

Opacity and scale only, at `--duration-pulse`. The ring holds still, so
no row shifts and nothing reflows. The scope narrowed three times — per
rail, then the focused Job, then this — and the reading survived each
time because the pulse never carried *which* step is current. Hue does
that, unchanged on every running row. The pulse carries *still working*,
and that is only asked of the thing being read, which is why it follows
focus rather than status. Nothing else on a data surface animates on a
loop. Under `prefers-reduced-motion` the pulse stops and
`--step-running` carries the reading alone.

---

## Window and layout model

One responsive prototype covers all widths — not separate comps per
breakpoint.

### Window chrome

Frameless, `titleBarStyle: 'hiddenInset'`, macOS traffic lights inset
over the sidebar's top region. Reclaims ~28px of vertical space and lets
the sidebar run to the top edge. Costs a custom drag region: the sidebar
header and any empty area of a top toolbar are draggable; interactive
elements inside them are not.

### Sidebar

Collapsible and resizable, and both states are designed rather than one
being an afterthought.

```
default     200px
drag range  160-320px
collapsed   48px icon rail
persistence width and collapsed state survive app restart
```

**Two levels, rendered structurally.** Bridge is a section label above
its surfaces, listed on Bridge rather than counted here. A separator,
then Helm as a sibling beneath — not one more peer in a flat list. This
is the app/surface-group hierarchy made visible; a flat nav quietly
contradicts it.

**The rail never disappears.** 48px is cheap and losing navigation
entirely is worse than losing 48px, at any width.

**Nav items do not carry escalation or approval counts.** The status bar
already carries both on every surface, by contract. Duplicating them in
the sidebar creates two places to check and two chances to disagree.

### Content area

**Full-width routes. No inspector pane, no modal for Job detail.** Board
and detail are separate destinations.

This follows from what a detail view actually holds: the escalation
payload, the full attempt history including every prior Judge summary
rather than the latest, per-step evidence, and a diff. That is not
inspector content, and a split pane would cramp the thing the page
exists to show.

**If triage speed suffers in practice**, the fix is prev/next navigation
within the detail view — staying in the queue without splitting the
layout. Not an inspector.

### Status bar

Fixed to the bottom, full window width, spanning **beneath** the
sidebar rather than inset to the content area.

Fixed because a healthy state has to say "Fleet running" out loud, and
that guarantee fails the moment the bar can scroll away — it is a
liveness indicator for a daemon that outlives the window. Full width
because the bar is app-level, not Bridge-level: it appears on Helm too,
and running it edge to edge makes that claim visible. Inset it and it
reads as belonging to whatever surface is open.

Token treatment is specified under Component → token mapping.

### Responsive behaviour

**768px hard floor.** Half of a 1536px display, and a normal way to run
something you glance at beside an editor. With the rail at 48px that
leaves 720px of content.

**One breakpoint at ~1100px, with one consequence:**

| | ≥ 1100px | < 1100px |
| --- | --- | --- |
| Sidebar | Expanded, user-resizable | Auto-collapses to the 48px rail |
| Job row | One shape at every width — a stacked row carrying the badge, the headline sentence and the labelled field run beneath | The same row. Nothing reshapes |

The stacked row is the status grammar's own shape: headline sentence on
line one (`Job 12 stalled at step 3`), labelled field run on line two
(`api · 3 pokes · auth/session.rs · 12m · ~$1.80`). The badge stays
leading on line one so status is still the first thing caught.

**No field is dropped at any width.** Every field in the universal row
exists because a decision depends on it — responsive-hiding them
contradicts P2, which requires the facts needed to decide to be on
screen without a click. Narrow changes the row's *shape*, never its
content. Secondary values truncate with a tooltip carrying the full
string; they do not vanish.

**Honest cost:** the stacked row is taller than a table row, so fewer
jobs are visible at once. That is a real loss on a monitoring surface,
and it was accepted deliberately: the Job Board and Alerts disagreeing
about what a job looks like is what retired the two-shape version, and
the row is the most repeated element in the app.

Below 1100 the user may still expand the sidebar manually. It overlays
the content in that case rather than compressing the table further — a
720px table has no width to give back.

**Validation:** if the row cannot carry its whole field set at 720px,
the field set needs revisiting, not the row. No field is dropped, and
the row does not reshape.

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
⌘1–⌘4    Bridge surfaces, in rail order
⌘5       Helm, the digit after the last Bridge surface
⌘\       toggle sidebar
⌘[ ⌘]    back / forward
Esc      close an overlay, or return to the list from a detail route
```

**Contextual — single-key, on the focused row or the open job.** This is
what makes triage fast: move down the queue and act without reaching for
a modifier.

```
j / k / ↓ / ↑  move focus
Enter          open the focused job. Acts on nothing
o              open. The same act, named so the palette can display it
r              review
t              attest
d              redirect
s              restart step        (detail only)
p              pilot               (not built)
c              copy debug info
x              kill                (confirms)
n              new job
/              search the current list
1–5            state filter        (Job Board only, in tab order)
a              approve             (dispatch card only)
v              observe             (detail only)
u              submit for verification  (piloted job only)
e              redispatch
h / l / ← / →  expand and collapse  (detail only)
[ ]            move between chapters  (detail only)
f              open the diff        (detail only)
g              open the stage       (detail only)
```

**This is the map, not a pattern.** It was settled by drawing the Job
Board and the command palette together, which is what forced the two
halves into one artifact — the palette displays a binding beside every
entry, so an unreconciled map is a palette that cannot be drawn.

**Both blocks are transcribed from
`crates/core-model/domain/actions.toml`**, which is the artifact above
and the authority. They stay here because this document is pasted whole
into a design session, where a pointer resolves to nothing; the gate
reads both and fails where they disagree, on the binding, the verb and
the annotations in brackets.

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

**`⌘1`–`⌘4` follow the rail, which is four destinations** — Job Board,
Alerts, Doctor, Manifest — since Active Jobs, Reviews and the Activity
Feed folded into the Board. The digits shift if the rail does; the rule
is rail order, not the numbers.

**`1`–`5` and `⌘1`–`⌘5` are different acts on one row of keys.** One is
modified and one is not, which is the whole separation. It was drawn
that way deliberately rather than by omission.

### Safety rules for single-key actions

These are constraints on the map, not suggestions.

- **Destructive keys are never adjacent to navigation keys.** Kill is
  `x`, never `k`, because `k` sits against `j` and a mistyped navigation
  keystroke must not be able to end a running job.
- **Every destructive action confirms**, even from the keyboard. In the
  confirmation dialog **Cancel holds initial focus**, `Enter` confirms,
  `Esc` cancels. A destructive action is never one keystroke from a
  focused row.
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
           12px leading icon · shortcut right-aligned as kbd
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

**The palette is the discovery surface.** It is how a person learns
forty shortcuts without a cheat sheet, which is why every entry displays
its binding and why no action may exist outside it.

### kbd

The one non-shadcn primitive. Used in palette rows, dropdown-menu items,
and tooltips.

```
surface  --bg-sunken · --border-subtle · --radius-sm
type     --text-2xs mono · --fg-muted
size     20px height · 4px horizontal padding
```

Never `--fg-default` — a shortcut hint is reference material sitting
beside the thing it describes, and rendering it at full contrast makes
it compete with the label.

### Consequences elsewhere in this document

- **Tooltips gain a trailing kbd** where the action has a binding. The
  400ms delay stands.
- **Dropdown-menu items gain a right-aligned kbd.** Item height is
  unchanged by it.
- **The 48px sidebar rail is more usable than it looks**, because the
  ⌘-digit bindings reach every surface without labels.

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
which muddies status colors. Every interactive element transitions on
`--duration-fast`.

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
already separates. Status appears as a badge in its own column, never as
a row-background tint — eight tinted rows in a list is unreadable, and
the tint would fight `--accent-muted` on selection.

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

### Button

| Variant | Rest | Hover | Use |
| --- | --- | --- | --- |
| Primary | `--accent` fill, `--fg-inverse` text | `--accent-hover` | One per view. Approve, Dispatch |
| Secondary | `--bg-sunken`, `--border-strong`, `--fg-default` | `--bg-hover` | Everything ordinary |
| Ghost | transparent, `--fg-muted` | `--bg-hover` • `--fg-default` | Row actions, icon buttons, toolbars |
| Destructive | transparent, `--status-completed-failed` text and border | fill at 12% | Kill only. Never a filled red button |

```
height   36px default · 32px sm (use sm inside table rows)
padding  16px default · 8px sm
type     --text-sm · weight 500 · --radius-md
```

Destructive stays outlined because a solid red button reads as an error
state rather than an action, and `--status-completed-failed` is already
spoken for as a *status*. Kill is deliberate, not alarming.

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
```

Tooltips carry the full value of anything truncated in a row — a path,
a branch name, a full timestamp. They never carry an explanation the
row should have made plain, per the briefing-register rule.

Placement, alignment and collision are under Floating layers.

### Status bar

Present on every surface, so it is a primitive rather than a screen
element.

```
height     32px · --bg-raised · top rule --border-subtle
type       --text-2xs · --fg-muted
separator  · in --fg-subtle
healthy    "Fleet running" in --fg-muted, always stated out loud
escalation count  --status-escalated, shown only when non-zero
approval count    --status-awaiting-review, shown only when non-zero
```

The two counts are the only status color in the bar. Everything else
stays `--fg-muted`, or the bar becomes a second alert surface and the
escalations-interrupt / approvals-queue distinction collapses.

**Fleet's state is one of three, and the bar names which.** A leading
6px dot carries the hue — the one exception to the rule above, on the
same grounds Doctor's pass, warn and fail reuse the Job values rather
than inventing a third set. It is not a glyph, so "the status bar
carries no icons" is unaffected.

```
running       --status-completed-success dot
              "Fleet running" · mono: pid, port, drone count
not running   --status-completed-failed dot
              "Fleet is not running" · mono: no runtime file at its path
              plus what to do, since Bridge cannot start it
unreachable   --status-awaiting-review dot
              "Fleet unreachable" · mono: pid alive on its port, no response for N
              plus how stale the last read is
```

The two failure states differ on the runtime file, which is the fact
that separates them: Fleet writes port, pid and protocol version on
startup and removes them on a clean exit, so a missing file is a Fleet
that is not there and a live pid with no answer is a Fleet that is
wedged. Two different things to do about it, so two sentences rather
than one timeout message.

### Closed

- **~~Icon set for the escalation sub-reasons and the `not_started` axis
  values.~~** **Closed.** Specified in full on
  [Iconography](iconography.md) — every badge state, navigation,
  actions, Doctor, and the rule for anything unlisted. lucide-react
  confirmed against Phosphor, Tabler, Radix and Heroicons; hard rule 5
  stands. The enum→verb test asserts an icon entry in the same pass.
- **~~Window and layout model~~** **Closed.** Specified in full under
  Window and layout model above — frameless `hiddenInset` chrome,
  collapsible/resizable sidebar with Bridge and Helm as two levels,
  full-width routes with no inspector, bottom-fixed full-width status
  bar, 768px floor with a single ~1100px breakpoint. Delivered as one
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
| Toast | Bottom trailing, inset `--space-6`, clear of the status bar, shadowed | The only one that may carry no act |
| Banner | Above the surface, inside it | Persistent. The surface works beneath |
| Full-surface | Replaces the surface | The one placement that takes the screen |

**Blast radius picks the placement, never severity.** Approve-refused is
red-serious and affects one row, so it renders in that row and nowhere else.

**Rows around an inline error are undisturbed.** Same height, same badges, and
the pulse continues.

**Every placement names the failure and the act.** A toast is the one
exception, because it reports something already over.

**A toast clears the status bar rather than covering it.** The bar states
Fleet's liveness out loud, which is the one thing still true while everything
else fails.

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

### Lexicon

- **Armada** the app. Never the tool, the system.
- **Fleet** the daemon. Never the backend, the server, the sidecar.
- **Bridge** the operational surfaces. Never the dashboard, the UI.
- **Helm** the conversational surface and its agent. Never the
  assistant, the chat.
- **Drone** one agent instance. Never the agent, the bot, the AI,
  Claude.
- **Job** one unit of work. Never task, run, ticket.
- **Convoy** a multi-workspace job landing as one PR. Never batch,
  group.
- **Job Board** the open queue. Never the queue, the backlog.
- **Job proposer** the model call that reads a request — a prompt, a
  ticket link — and proposes a Job: its workflow, its scope, and where
  the work is several Jobs, the graph. Lowercase, because it is a call
  rather than a component. Never the classifier, the Job-shape
  classifier, the shape classifier. What it produces is a **Job
  proposal**, and that is what the dispatch gate approves.
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
| Job-shape classifier | **Job proposer** | It stopped classifying a shape when shape became derived, and stopped only stating scope when workflow selection joined it. Neither half of the old name survived, and both halves misled — the second reading hid the fact that nothing chose a Job's workflow at all. A page still calling it a classifier is describing a narrower call than the one that exists |
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
silent, since `stalled` owns silent. `evidence_suspect` renders as
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

**Workspace is not in the row.** It was, and it came out: a row is
scanned, and the workspace is the field a reader already knows — they
opened this board, and every Job on it is theirs. Spending a track on it
costs the one that answers "is this stuck", which is elapsed. It stays on
the detail view, where a reader is asking about one Job rather than
comparing several.

The cost is real and is worth writing down rather than discovering: the
Job list is not scoped by the rail's Manifest picker — that picker sets
what a new Job starts pointed at — so a board holding Jobs from more than
one project cannot say which project a row belongs to. Revisit when a
second project is dispatched against, not before.

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
and shape: `68% quota` against `~$2.40 of $20`. A design session must
size this column for the wider of the two and confirm the row survives
both, rather than fitting it to whichever example appears first in this
document. The same applies to the status bar's trailing segment.
Neither mode is the default — which one renders depends on the machine,
and both are first-class.

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

**Collapsed when healthy, expanded when not.** Detail appears in
proportion to what is wrong. Collapse the detail, never the assertion:
a healthy state says "Fleet running" out loud, because an empty bar
reads the same whether Fleet is healthy, loading or dead, and Fleet
outlives Bridge.

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
which is what the status bar's two counts and the push-alert design
both rest on. It is a product rule about what these events mean, not a
config-tier rule: notification routing is a Machine setting with one
value and no merge, so no Manifest is party to it.

**Status bar**, present on every surface including Helm. "Fleet
running" when idle. "Fleet running · 3 jobs · 68% quota left" when
working on a personal machine, or "Fleet running · 3 jobs · ~$2.40 of
$20" on a work machine. It expands when something is wrong, and
escalation and approval counts appear only when non-zero. Five items is
the ceiling.

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

- **[pilot-exit-bindings]** What are the keys for Close as superseded and
  Override the verdict? Every action owes a verb, an icon and a shortcut, and
  these two have no binding. Both end or overrule a Job's record, so neither
  should take a spare letter by default — the destructive-key rule exists
  because a binding chosen for convenience is one a person reaches by accident.
  Three neighbouring acts took keys in the same pass: Observe `v`, Submit for
  verification `u`, Redispatch `e`.

- **[status-bar-loudness]** How loudly do the status bar's escalation and
  approval counts render, beyond taking their status token?
  The counts are the only status colour in the bar and show only when
  non-zero. The constraint is that escalations interrupt and approvals
  queue — styling an approval count as loudly as a broken drone would undo
  that distinction and turn the bar into a second alert surface.

- **[status-bar-onboarding]** During the hard-gated first-run sequence,
  does the status bar read the same three runtime states as everywhere
  else, or does onboarding get its own reading?
  The bar states a healthy status out loud rather than implying it, because
  an empty bar reads the same whether Fleet is healthy or dead — that is
  settled and gives the three runtime states. But "Fleet is not running" is
  correct and reads as an error on a new user's first screen, and it was
  written for someone who already knows what Fleet is. Fleet is started by
  hand at M1 and becomes reachable partway through onboarding, so the bar
  changes state mid-journey either way; whether the bar is even present
  before the first onboarding step completes is part of the same question,
  since onboarding is not yet a Bridge surface.

Also bearing on this document, and written where each belongs: `[attested-verdict-glyph]` in `iconography.md`; `[verdict-artifact-rows]` in `voice-engineering.md`. A question has one home — answering it in two places is how one of them goes stale.
