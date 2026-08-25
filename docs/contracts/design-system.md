# Design System — UI & Voice

**Kind:** contract. **Governs:** static UI chrome, tokens, and the Voice &
Copy contract — the parent contract, pasted into every design session, that
nothing else may contradict.

The single contract handed to Claude Design (or any design tool) before
generating an Armada screen. Constraining the input is what makes output
drop into the Electron app without restyling — this replaces per-screen
conversion work. Paste this file's contents at the top of a design session;
design Job Board first, since it is the densest screen and the real test of
the density and status tokens.

UI tokens and the Voice & Copy contract are both in force. Two sibling
documents carry what a design session does not need: the Agent Copy
Contract (text written at runtime by Drones, Judge and Helm, with its
surfaces and samples in Armada Copy) and [Voice Contract — Engineering
Requirements](voice-engineering.md).

---

## Stack

- **Component library:** shadcn/ui — you own the component code, Tailwind-native
- **Style library:** Tailwind + CSS custom properties as tokens
- **Icons:** lucide-react, version pinned — 12px in badges, 16px in
  navigation and buttons, strokeWidth 2 throughout. See
  [Iconography](iconography.md)
- **App:** electron-vite + React + TypeScript

---

## The product

Armada dispatches AI coding agents (Drones) against real Git repositories,
monitors them, and escalates when they misbehave. Single user, local,
always open on a second monitor across a working day.

**Surfaces:** Bridge is the operational surface group — Job Board, Active
Jobs, Alerts, Reviews, Activity Feed, Doctor, Manifest. Helm is a sibling
conversational surface; the roster lives on Bridge.

**The screen's job:** at a glance, tell one person what is running, what
needs them, and what broke. This is an instrument panel, not a marketing
page — no hero sections, no gradients, no decorative iconography, no
illustration. Density and legibility win over impact.

---

## Hard rules

1. **No Tailwind arbitrary values.** Never `bg-[#3b82f6]`, `p-[13px]`,
   `text-[15px]`. Every value comes from the token set below. Lint-enforced
   — arbitrary values fail the build.
2. **Only shadcn/ui primitives:** button, input, select, checkbox, radio,
   switch, badge, card, table, dialog, sheet, tabs, toast, tooltip,
   dropdown-menu, popover, separator, scroll-area, skeleton, alert,
   **command**. Compose from these; do not invent new base components.
   `command` (cmdk) backs the command palette. `kbd` is the one non-shadcn
   primitive — see Keyboard and command palette.
3. **Status colors are never chosen.** They map to the Job state machine
   one to one, never by aesthetic judgment. **Below Job level, hue exists
   only where `tokens/status.css` declares it**, and every value there
   aliases its Job counterpart so the mapping is declared rather than
   inferred — read the file rather than a list here, since an enumeration
   went stale twice. Anything the file does not declare stays neutral.
4. **Dark is primary.** Design dark first. Light exists but is secondary.
5. **Icons: lucide-react only**, used sparingly. A dashboard dense with
   icons reads as noise.

---

## Tokens

Mirrored row by row in the Armada Tokens database, with each token's role,
source file, contrast measurements and revision history. Reference as
Tailwind classes mapped to CSS variables (`bg-surface-raised`,
`text-fg-muted`, `text-status-running`). Never raw hex.

### Ground

Deep desaturated blue-slate, not near-black — reads as instrument panel
rather than terminal, and gives status color room to sit without vibrating.

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

One token per Job state, and the set is the state machine's, not a
palette. `rejected` and `killed` are **deliberate human decisions**, not
system failures, and must not read as errors.

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
`--bg-raised`; hue and semantic assignment are unchanged, only brightened.

| Token | Was | Now | Badge contrast, before → after |
| --- | --- | --- | --- |
| `--fg-subtle` | #5D6B7C | #7E8CA0 | 3.15 → 5.02 on `--bg-raised` |
| `--status-not-started` | #5D6B7C | #8C97A6 | 2.83 → 4.83 |
| `--status-escalated` | #E8763D | #EE8450 | 4.93 → 5.52 |
| `--status-completed-failed` | #DC5B5B | #E97878 | 4.06 → 5.12 |
| `--status-rejected` | #A97BD1 | #B489DA | 4.48 → 5.12 |
| `--status-killed` | #6B7684 | #9BA3AC | 3.27 → 5.52 |

`--fg-subtle` no longer equals `--status-not-started`, closing the
contrast item [Iconography](iconography.md) flagged. One shortfall
remains: `not_started` badge text on `--bg-overlay` reads 4.38:1, accepted
since badges rarely appear on floating layers. `--accent` as text on
`--accent-muted` is 4.06:1, so selected rows keep `--fg-default` text
(9.68:1) instead.

**Escalation sub-reasons** all use `--status-escalated`, differentiated by
**label and icon**, never hue — a column of oranges would be unreadable.
The trigger list lives on Workflow, not here; this document owes the rule,
not the roster.

**The approval axis is a status, not a reason.** `awaiting_approval` and
`queued` are statuses of their own; `queued`'s reason names the resource,
with ready as the null. They share `--status-not-started`, differ by label
and icon; `awaiting_approval` left grey for amber. This closes a real bug:
a sub-dispatched Job's out-of-headroom case used to compute to an
unrenderable `pre_approved_queued` under the old four-value field; now it
enters at `queued` with its reason naming the resource.

### Below Job level

Drawn from the workflow rail, which broke the rule that hue stops at the
Job — a done step, a running step, a Judge criterion verdict all wanted
it. **Hue below Job level exists only where `tokens/status.css` declares
it**; this section carries the reasoning and deliberately not the roster,
since an enumeration here went stale twice. Every value **aliases** its
Job counterpart rather than introducing a new one.

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

`retrying` and `not_started` take no hue — `--fg-muted` and `--fg-subtle`.
A **killed** step takes none either: killing is a human decision, not a
system failure. The current row keeps its `--accent-muted` tint and 2px
`--accent` left edge — emphasis, not status.

**`failed` reports an outcome, not a position.** A step whose Check
refused takes `--step-failed` with a bare `x`, the same mark as the
`completed_failed` badge one level down. It was drawn neutral first, since
a Check result is measured — reversed, because a failed Check with no
retry and no triage is the entire reason a person opened the screen, and
burying it in a muted rail is the frustration the surface exists to
prevent. The gate row beneath stays neutral: the step's state is hued,
the Check's exit code is measured.

**`stopped` and `failed` alone carry a surface, not just a glyph hue.** A
step whose retries are spent is distinct from `waiting` (a designed human
gate) and from a dead stop; both need a surface since a glyph only holds
while its row is selected, and the row that ended the Job has to stay
findable. They differ in glyph: `stopped`'s `flag` stays `--fg-default`
since the surface already carries the warning; `failed`'s `x` is hued,
since failure is an outcome and states it in both channels.

**Criterion verdicts are measured facts and render as flatly as one.** A
criterion is met or it is not; the red does not claim the Job failed — a
Judge refusal is the gate working. **Verdict hue is per criterion and
never sums onto the step or the Job**, which is what lets a red cross sit
under a running step beneath an escalated badge without contradiction.

**Refusals sort first, and every criterion row carries its number** — a
reordering card would break correspondence with the frozen
`acceptance_criteria[]` order. This is an open matter: see the open
question on how criterion verdicts are encoded without status hue.

**Everything else below Job level stays neutral.** A Kit file's drift
state, an origin tag and the retry marker carry position, surface, weight
and glyph. Adding a value to `tokens/status.css` is a contract change, not
a design decision.

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
durations, token counts. Monospace signals *this is a fact the system
reported*, and makes IDs and paths scannable in a dense table.

Scale is tighter than web defaults. **Legibility pass, 2026-08-20:** the
whole ladder was raised ~15% after the 11px and 13px steps proved hard to
read at desk distance. Ratios and roles are unchanged; `tokens/spacing.css`
is the authority on the heights that hold the larger text.

```
--text-2xs   13px / 18px   table metadata, timestamps
--text-xs    14px / 20px   labels, badges, secondary
--text-sm    15px / 22px   BODY DEFAULT — most UI text
--text-base  16px / 24px   emphasis within body
--text-lg    18px / 28px   panel headings
--text-xl    23px / 32px   page titles
--text-2xl   28px / 36px   the rare hero number
```

Weights: 400 body, 500 labels and emphasis, 600 headings. Never 700+. Mono
runs one step smaller than adjacent sans at the same optical size — 14px
mono next to 15px sans.

---

## Spacing and shape

4px base grid: `1`=4, `2`=8, `3`=12, `4`=16, `6`=24, `8`=32, `12`=48.
Deliberately tight: table rows 36px, header rows 32px, card padding 20px
(not 24), controls 32px (sm) / 36px (default), section gaps 24px (not 48).
This window holds a job list, a diff, and a graph view at once. Every
value is a token in `tokens/spacing.css`, the authority on it.

```
--radius-sm  3px    badges, small controls
--radius-md  5px    buttons, inputs, cards
--radius-lg  8px    dialogs, panels
```

No full-round pills except avatars. No shadows on flat surfaces —
elevation comes from `--bg-raised` / `--bg-overlay`, not blur. Shadows only
on floating layers (dialog, popover, dropdown).

---

## Motion

```
--duration-fast   120ms    hover, focus
--duration-base   180ms    panel and dropdown transitions
--duration-pulse  1600ms   the running step mark, and nothing else
--ease            cubic-bezier(0.2, 0, 0, 1)
```

No entrance animations on data — a Job Board that animates rows in on
every poll is unusable. Live-updating values may pulse once on change,
nothing more. Respect `prefers-reduced-motion`.

**One carve-out: the running mark animates continuously**, since only
motion says a step is still working. **Scope is one animated mark per
screen, on the most specific mark present.** Job detail has a rail, so
the rail's current step pulses and the header's Running badge stays
static. A list has no rail, so the Running badge pulses instead, on the
**focused row only** — fourteen breathing dots is exactly what this rule
forbids. The step bar never pulses.

Opacity and scale only, at `--duration-pulse`; the ring holds still, so
nothing reflows. The pulse never carries *which* step is current, only
*still working*, so it follows focus rather than status. Under
`prefers-reduced-motion` the pulse stops and `--step-running` carries the
reading alone.

---

## Window and layout model

One responsive prototype covers all widths.

**Window chrome.** Frameless, `titleBarStyle: 'hiddenInset'`, macOS traffic
lights inset over the sidebar's top region — reclaims ~28px of vertical
space. Costs a custom drag region: the sidebar header and any empty area
of a top toolbar are draggable; interactive elements inside them are not.

**Sidebar.** Collapsible and resizable, both states designed rather than
one being an afterthought.

```
default     200px
drag range  160-320px
collapsed   48px icon rail
persistence width and collapsed state survive app restart
```

Two levels, rendered structurally: Bridge is a section label above its
surfaces, a separator, then Helm as a sibling beneath. **The rail never
disappears** — 48px is cheap and losing navigation entirely is worse.
**Nav items do not carry escalation or approval counts** — the status bar
already carries both.

**Content area.** Full-width routes. No inspector pane, no modal for Job
detail — a detail view holds the escalation payload, the full attempt
history, per-step evidence and a diff, which a split pane would cramp.

**Status bar.** Fixed to the bottom, full window width, spanning
**beneath** the sidebar rather than inset to the content area. Fixed
because a healthy state has to say "Fleet running" out loud, and that
guarantee fails the moment the bar can scroll away. Full width because it
is app-level, not Bridge-level: it appears on Helm too.

**Responsive behaviour.** 768px hard floor — half of a 1536px display.
With the rail at 48px that leaves 720px of content. One breakpoint at
~1100px:

| | ≥ 1100px | < 1100px |
| --- | --- | --- |
| Sidebar | Expanded, user-resizable | Auto-collapses to the 48px rail |
| Job row | One shape at every width — a stacked row carrying the badge, the headline sentence and the labelled field run beneath | The same row. Nothing reshapes |

The stacked row is the status grammar's own shape: headline sentence on
line one (`Job 12 stalled at step 3`), labelled field run on line two
(`api · 3 pokes · auth/session.rs · 12m · ~$1.80`), badge leading. **No
field is dropped at any width** — every field exists because a decision
depends on it, so responsive-hiding would contradict P2. Narrow changes
the row's *shape*, never its content. **Honest cost:** the stacked row is
taller, so fewer jobs are visible — accepted deliberately, since Job Board
and Alerts disagreeing about what a job looks like is what retired the
two-shape version. Below 1100 the sidebar may still expand manually; it
overlays rather than compresses the table, a 720px table having no width
to give back.

---

## Keyboard and command palette

Foundational rather than additive — specified before the first screen.

**Principle: every action reachable by mouse is reachable by keyboard, and
nothing is keyboard-only.** The palette is a superset of the UI, never a
substitute. **One artifact, three columns:** every action carries a
**verb**, an **icon**, and a **shortcut**, generated from one source with a
test asserting no entry is missing any of the three.

**Global — modifier-based, work anywhere:**

```
⌘K       command palette
⌘ digit  Bridge surfaces, in sidebar order
⌘ last   Helm, the digit after the last Bridge surface
⌘\       toggle sidebar
⌘F       filter the current list
⌘[ ⌘]    back / forward
Esc      close overlay, or return to the list from a detail route
```

**Contextual — single-key, act on the focused row** (the pattern, not the
complete map):

```
j / k    move focus down / up
Enter    open the focused job
a        approve
r        redirect
x        kill        (confirms)
/        focus the filter input
```

**Safety rules, not suggestions:**

- **Destructive keys are never adjacent to navigation keys.** Kill is `x`,
  never `k`, since `k` sits against `j` and a mistyped navigation keystroke
  must not be able to end a running job.
- **Every destructive action confirms**, even from the keyboard. Cancel
  holds initial focus, `Enter` confirms, `Esc` cancels.
- **Single-key shortcuts are suppressed whenever a text input holds
  focus.**
- **Pilot is exempt.** Once the terminal has focus, every keystroke
  belongs to it. Only `Esc Esc` releases it.

**Focus model.** Focused and selected are different states and coexist:

```
focused row   2px --accent left edge bar + --bg-hover
selected row  --accent-muted fill
focused ctrl  2px --accent ring at 2px offset, per the global focus rule
```

Focus is visible at all times during keyboard navigation, not only on
`:focus-visible` — if driving with `j`/`k`, the ring is the cursor.

**Command palette.** A floating layer:

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

Top-anchored rather than centered because a centered dialog shifts as the
result count changes, and a target that moves while you type is a target
you misclick. Contents, in order: actions in context, navigation, jobs by
id or name, settings. **The palette obeys the lexicon** — displayed labels
always use the lexicon term (Kill, Drone, Convoy); the search index may
carry aliases ("terminate" finds Kill) but the alias never renders. It is
the discovery surface for forty shortcuts, which is why every entry
displays its binding and no action may exist outside it.

**`kbd`**, the one non-shadcn primitive, in palette rows, dropdown-menu
items, and tooltips:

```
surface  --bg-sunken · --border-subtle · --radius-sm
type     --text-2xs mono · --fg-muted
size     20px height · 4px horizontal padding
```

Never `--fg-default`, which would compete with the label. Tooltips and
dropdown-menu items gain a trailing/right-aligned kbd where bound.

---

## Component → token mapping

Without this section a design tool infers which token each primitive
uses, differently every session. Anything not listed follows the same
logic: surfaces from Ground, text from Foreground, interaction from
Accent, status **only** from the status tokens.

**Global.** Focus is a 2px `--accent` ring at 2px offset, no glow — it was
a 1px `--border-strong` ring until a secondary button took
`--border-strong` as its resting edge, making the two identical and
rendering focus as nothing. Disabled is `--fg-subtle` text with hover
suppressed, never reduced opacity, which muddies status colors. Every
interactive element transitions on `--duration-fast`.

**Table — the Job Board row**, the densest thing in the app:

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

No zebra striping — at 36px rows it reads as noise, and the row rule
already separates. Status is a badge in its own column, never a
row-background tint. **A machine value copies on click** — any mono value
copies to clipboard and goes `--accent` on hover, with no `copy` glyph
(the affordance token is the affordance). A toast confirms, since a
clipboard write is silent by nature. A value that copies does not also
get a button that copies it.

**Badge — status**, the one place status tokens are used directly.
`{state}` is the enum variant; the label comes from the enum→verb table,
never hand-written:

```
background  --status-{state}-bg   (12% opacity variant)
text        --status-{state}
border      none
height      20px · 6px horizontal padding · --radius-sm
type        --text-2xs · weight 500 · sentence case
icon        required, 12px lucide, strokeWidth 2, leading, inherits text color
```

Every badge state carries an icon, so hue is never the only channel —
full specification on [Iconography](iconography.md). 12px rather than
11px: lucide draws on a 24px grid, so 12px is an exact half-scale and a
stroke of 2 lands on exactly 1px; 11px scales to 0.917px and antialiases
into fuzz.

**Button:**

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

Destructive stays outlined: a solid red button reads as an error state
rather than an action, and `--status-completed-failed` is already spoken
for as a status. Kill is deliberate, not alarming.

**Input** (select, checkbox, radio, switch inherit border/focus/height;
switch uses `--accent` when on, `--border-strong` when off):

```
background  --bg-sunken        (recessed, opposite of raised)
border      --border-default
text        --fg-default · placeholder --fg-subtle
focus       2px --accent ring at 2px offset
invalid     --status-completed-failed border, message below in --text-xs
height      36px · 8px horizontal padding · --radius-md · --text-sm
```

**Dropdown menu**, a floating layer (`--bg-overlay`, one place a shadow is
legal; sheet and dialog share this treatment at `--radius-lg`):

```
surface    --bg-overlay · --border-default · --radius-lg · shadow
item       32px · 8px padding · --text-sm · --fg-default
hover      --bg-hover
danger     --status-completed-failed text, --bg-hover on hover
separator  --border-subtle
label      --text-2xs · --fg-subtle
```

**Tooltip:**

```
surface  --bg-overlay · --border-subtle · --radius-sm · shadow
type     --text-xs · --fg-default · 8px / 4px padding
timing   400ms delay in, --duration-fast
```

Carries the full value of anything truncated in a row, never an
explanation the row should have made plain, per the briefing-register
rule.

**Status bar**, present on every surface, a primitive rather than a
screen element:

```
height     32px · --bg-raised · top rule --border-subtle
type       --text-2xs · --fg-muted
separator  · in --fg-subtle
healthy    "Fleet running" in --fg-muted, always stated out loud
escalation count  --status-escalated, shown only when non-zero
approval count    --status-awaiting-review, shown only when non-zero
```

The two counts are the only status color in the bar; everything else
stays `--fg-muted`. **Fleet's state is one of three, and the bar names
which** — a leading 6px dot carries the hue, the one exception to the
rule above, on the same grounds Doctor's pass/warn/fail reuse the Job
values. It is not a glyph, so "the status bar carries no icons" is
unaffected.

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

The two failure states differ on the runtime file: Fleet writes port, pid
and protocol version on startup and removes them on a clean exit, so a
missing file means Fleet is not there and a live pid with no answer means
Fleet is wedged.

---

## Voice & Copy

### Typography of reference — applies to docs, not just UI

Unlike the rest of this section, these rules govern **internal
documentation and planning pages as well as product copy**, since docs
leak into the product.

- **Never use `§`.** Write "M0 step 4," not "M0 §4." Same reasoning bans
  `¶`, `cf.`, `ibid.`, `op. cit.`, `viz.`, `q.v.` — write "see," "compare,"
  or "same source." `e.g.` and `i.e.` are fine.

**Citing v1 code.** A bare file path on any Armada document refers to
**v1**, resolving against branch `v1-archive` / tag `v1-final` — but
Ground Zero step 1, which orphans `main`, has not run yet, so neither
exists in the clone. Until it runs, a v1 citation should carry a commit
hash rather than a line number; after it runs, a hash cited today still
resolves from `v1-archive`.

Approval prompts, status reasons and escalation messages are what you
read at 11pm deciding whether to kill a drone — the rules below apply to
product copy, not internal documentation.

**Scope split.** This contract governs static UI chrome, not
configurable. The Machine-level **Voice** setting tunes runtime-generated
prose (Judge summaries, Helm replies, job summaries) within this
contract — it may adjust length and formality, never override principles,
lexicon or status grammar. "Terse" and "explanatory" are legal Voice
values; "playful" is not.

### Principles

**P1. Metaphor lives in proper nouns only.** Nautical vocabulary is
confined to names: Armada, Fleet, Bridge, Helm, Drone, Manifest, Convoy,
Job Board. Every verb, state, error and instruction is plain English.
Write "Drone 4 stopped reporting 12 minutes ago", not "Drone 4 has gone
dark". Pilot is a proper noun; "take the wheel" is not a verb Armada uses.

**P2. Briefing register.** A message carries the facts needed to decide,
on screen, without a click: "Drone 4 stopped reporting 12 minutes ago
after 3 pokes. Step 2 of 5, last wrote `auth/session.rs`" — not "Drone 4
stopped reporting. Poke limit reached."

**P3. First person is Helm's alone.** Bridge and Fleet never say "I".
Helm says "I" only for what Helm itself did; reporting a Fleet event, it
uses Bridge's impersonal phrasing.

**P4. Hedge by source.** Three source classes, three registers:
**Measured** speaks flatly ("Tests passed."). **Estimated** is marked
approximate (`~$2.40`, never `$2.40` — a derived figure is not a measured
one). **Judged** is visibly a judgment and names its source ("Judge read
the evidence as not covering the error path."). Render any two identically
and one bad value teaches distrust of the other two.

**P5. Event-first, with cause.** The subject of a failure sentence is the
job or step, never the drone: "Step 3 did not advance. No evidence after
3 clarification rounds", not "Drone 4 failed to submit evidence". Known
causes state flatly; hypothesised causes hedge and name their source, and
only Judge and Helm may produce them.

**P6. Fixed copy is a template; generated copy is a substance
requirement.** Fleet's own strings are identical every time, since
uniformity is scannability. Generated text is specified by what it must
contain, never by what shape it takes — a structural rule produces twenty
interchangeable paragraphs. A summary that would read plausibly under a
different job has failed.

### Prose rules

- **Sentence case everywhere.** No title case, no ALL CAPS except table
  headers at `--text-2xs` with `0.04em` tracking. Lexicon proper nouns
  keep their capitals inside sentence case.
- **Name things by what the person controls.** "Approve dispatch", not
  "Submit job payload".
- **No mid-sentence asides.** Targets the reflex, not the character —
  banning the em dash breeds a colon and banning the colon breeds a
  trailing negation. A colon separating a field from its value stays
  legal: "Step 3 stalled: no evidence after 3 rounds."
- **No adverbs by default.** "Successfully completed" is "Completed".
  "Currently running" is "Running".
- **No Wh- sentence openers.** They survive as panel headings: "Why this
  stalled", "What ran", "What changed".
- **No sentence that survives deletion without loss.**
- **Errors say what happened and what to do.** Never apologise, never be
  vague.
- **An action keeps its name through the flow.** A button that says Kill
  produces "Killed". The verb table enforces this.

### Lexicon

- **Armada** the app. Never the tool, the system.
- **Fleet** the daemon. Never the backend, the server, the sidecar.
- **Bridge** the operational surfaces. Never the dashboard, the UI.
- **Helm** the conversational surface and its agent. Never the assistant,
  the chat.
- **Drone** one agent instance. Never the agent, the bot, the AI, Claude.
- **Job** one unit of work. Never task, run, ticket.
- **Convoy** a multi-workspace job landing as one PR. Never batch, group.
- **Job Board** the open queue. Never the queue, the backlog.
- **Job proposer** the model call that reads a request and proposes a
  Job: its workflow, its scope, and where the work is several Jobs, the
  graph. Lowercase, a call rather than a component. Never the classifier,
  the Job-shape classifier, the shape classifier. What it produces is a
  **Job proposal**, which the dispatch gate approves.
- **Kit** the tool set you bring — Skills, MCP, sub agents, Agent files,
  Plugins, Commands, the allowlist, the models list. Never global
  settings, preferences. Replaces Guild, retired Aug 2026.
- **Machine** how this installation behaves — resources, timing, budget,
  interface, notification routing. Never system settings, environment.
- **Manifest** per-project config. Never the config, the yaml.
- **Judge** the semantic verification layer. Never the auditor, the
  reviewer, AI review.
- **Evidence** the structured completion report. Never the report,
  output, proof.
- **Doctor** the health check. Never diagnostics, system status.
- **Workspace** one unit inside a repo. Never package, module, sub-repo.
- **Drone transcript** the record of a Drone's turns. Never the Drone
  log, the Drone output.
- **Judge record** one Judge call and every judge's verdict inside it.
  Never a transcript — a Judge call is one-shot, and the Judge never reads
  the Drone's transcript, which is the isolation that makes its verdict
  worth anything.
- **Check log** what a Check wrote to stdout and stderr. Never the Check
  output, the Check results.

**Claude is a model name, never an actor.** Write "Drone 4 stalled", not
"Claude stalled". The word appears only where a model is selected or
reported.

### Retired terms

A document still carrying one of these is stale, not merely old-fashioned.

| Retired | Now | Note |
| --- | --- | --- |
| Guild | **Kit** and **Machine** | A split, not a rename |
| Armada Server | **Armada API** | Named for the `api` crate |
| Job-shape classifier | **Job proposer** | Neither half of the old name survived |
| Daemon | **Fleet** | A redundant second name for the same process |
| Ground Zero | **M0 — Foundations** | Archived with the phase plan |
| Phase 0–6, numbered steps | **Milestones** and their **Steps** | Reference only; Steps are disposable |

**Casing.** Docs capitalise throughout. UI capitalises the singular named
things (Armada, Fleet, Bridge, Helm, Doctor, Judge, Kit, Machine, Job
Board) and lowercases anything countable (job, drone, convoy, manifest,
workspace, evidence, workflow): "No active jobs. 3 waiting on the Job
Board."

### Status grammar

**Shape: headline plus fields.** A headline sentence, facts as labelled
fields beneath, machine-derived fields in mono:

> **Job 12 stalled at step 3**
> Workspace `api` · 3 pokes · `auth/session.rs` · 12m · ~$1.80

**Verbs are generated from the enum, never written.** `stalled` always
renders "stalled". For `escalated` and `queued`, the headline verb is the
reason rather than the state — nobody says "Job 12 escalated at step 3".
**`queued` takes its reason's verb where one is set**; with none it reads
queued. A Job out of headroom reads "waiting on resources" rather than
falling through unrendered, which is what the old two-axis field did.

**The map is a database, not a table restated here.** Every vocabulary the
UI renders is one row per variant, grouped by axis, carrying the verb
alongside the glyph and hue it owes. A row with an empty verb is a variant
with no sanctioned copy, which is what the test fails on.

**The plain label is what a queue row shows; the raw enum is recoverable,
never primary.** A row in Alerts carries the verb and nothing else. The
enum sits back in the **detail view header**, so an engineer can grep
Fleet's logs with the exact string without the queue reading like a stack
trace — matters most for `fan_out` and `evidence_suspect`, whose plain
forms are not guessable back to the enum.

`silent` takes no verb of its own — a sub-kind of `stalled`, rendering as
**stalled**; the difference is only in the suggested action. `thrashing`
renders as **churning** (busy-but-going-nowhere, since `stalled` owns
silent). `evidence_suspect` renders as **evidence disputed**, with Judge
in the source field, keeping attribution out of the headline per P5.

**Verdict vocabularies are not Job states and sit on their own axes.**
Step verdict is `workflow_status.last_step_verdict`; a criterion verdict
lives per criterion inside the Judge record, reading differently by
verification source — P4's hedging device at its smallest scale. **"No
objection" rather than "accepted", and the step rather than the Judge** —
the Judge declines to refuse, it never grants: "Step 3 of 5 verified",
not "Judge passed". **A criterion attested by a person takes neither
vocabulary above** — Source Attestation reads **confirmed** ·
**withheld**, since a person may grant and the Judge may only decline to
refuse. **Withheld**, not failed or refused: a person who looked and
would not put their name to it has done something neither other source
can do.

**Icons.** The token section differentiates escalation and `not_started`
values by label and icon; the verb table supplies the labels, and the
icons — across every badge state — are specified on
[Iconography](iconography.md). That document supersedes the ten-row table
this section used to carry: `stalled` moved off `hourglass` (read as be
patient for the state that most needs to read as wrong, now banned
outright); `blocked_by_dependency` moved off `lock` (claims a permissions
problem that does not exist); `fan_out` moved off `git-fork` (loses its
nodes at 12px); the six base states gained icons they previously lacked.

**Fields: universal in lists, per-state in detail.** The universal row
carries job identity, workspace (or "convoy, 3"), state, step N of M,
elapsed, spend so far, verification source, actor. **Spend follows the
active billing mode.** Personal-machine mode gates on
quota %, provider-reported and measured. Work-machine mode gates on $
cost, marked approximate (`~$2.40`) until v1's figures are validated
against actuals — the visible number is always the gating number. A Judge
call's spend takes its own line in the same active mode, since priced in
dollars on a quota-gated machine it would be a permanently visible
non-gating figure. Spend stays in the row rather than the detail view,
since it lost the headline at the approval gate on the promise that it is
always visible. **The spend column is sized for both modes**, since `68%
quota` and `~$2.40 of $20` are very different widths — a design session
must size for the wider and confirm the row survives both, and the same
applies to the status bar's trailing segment.

**Repetition.** A second stall at step 3 reads "stalled at step 3, 2nd
time", and the detail view surfaces the prior attempt. Presentation only
for now — recurrence changing behaviour is a separate decision.

### Register by surface

**Approval gates** stay descriptive: what the job is, which workspaces,
which workflow, going consequence-forward on blast radius alone — a
Convoy, auto-merge on, a job touching root `armada.yml`, or a pre-approved
batch. Cost never triggers it.

**Push alerts** carry facts rather than a ping: "Drone 4 stalled on step 2
of 5, `auth/session.rs`, 12 min." Kill and Redirect are not notification
actions.

**Empty states** point at available work: "No active jobs. 3 waiting on
the Job Board."

**Helm** answers, then may add a single observation, only after actually
looking, always flagged as its own inference. No throat-clearing openers.

**Confirmations** appear on everything destructive: "Kill the drone on
job 12? Step 3 of 5, 14 minutes in. Evidence carries forward if you
redispatch." Pilot has no confirmations. **Not configurable** — the Kit →
Manifest "destructive-op list" setting governs Drone-initiated
operations only, not your own clicks.

### Behaviour rules that shape copy

**Collapsed when healthy, expanded when not.** Collapse the detail, never
the assertion — a healthy state says "Fleet running" out loud.

**Escalations interrupt, approvals queue.** Escalations cost money in
real time; approvals cost latency, so escalations reach your phone and
approvals never do. **Routing config is bounded by this rule:** the
loudness order is silent < in-app < OS notification < push. Routing may
move an event type *down* that order, never up, and an approval may never
be promoted to push — a product rule about what these events mean, not a
config-tier rule (notification routing is a Machine setting with one
value and no merge). It holds because if approvals could reach push, the
escalation signal stops being trusted.

**Status bar**, present on every surface including Helm: "Fleet running"
when idle; "Fleet running · 3 jobs · 68% quota left" (personal machine) or
"Fleet running · 3 jobs · ~$2.40 of $20" (work machine) when working.
Escalation and approval counts appear only when non-zero. Five items is
the ceiling.

**Two separate fields, not one.** **Verification source** is the P4
hedging device — closed vocabulary of three: **Check**, **Judge**,
**Attestation**. Only a human may set Attestation, never a Drone, Helm,
or Judge; a Job carrying an attested criterion must not render
identically to one mechanically verified. **Actor** is audit attribution:
**human**, **Helm**, **Drone**, **Fleet**. They are orthogonal — a manual
change during Pilot is actor=human with no verification source; an
allowlist denial is actor=Fleet with no verification source, since Fleet
blocking an operation is not a verification result; a failed gate is
verification source=Check with actor=Drone.

Open items for this document are tracked in Notion's Open Items database
and are not reproduced here.
