# Iconography

**Kind:** spec. **Governs:** every badge state, plus navigation, actions and
Doctor — lucide-react only, holding the shield-\*/file-\* reservation rule.

Every icon in the app is named in `packages/icons/icons.toml`; this document
carries the rules that put a glyph there and the reasoning behind each one.
Hand this document to a design tool alongside `packages/icons/icons.toml` and
the parent [Design System](design-system.md) document, and no icon requires a
judgment call.

Each glyph is a table in `packages/icons/icons.toml`, keyed by its
lucide-react name, carrying its meaning, group, size, status and reservation.
This document carries the reasoning behind each choice and the rules that
govern anything not yet in that file; `icons.toml` is the lookup that answers
what a glyph already means and where it may not be reused.

Resolves the Iconography open item. Supersedes the partial 10-icon table
previously carried in the parent [Design System](design-system.md) document's
Voice & Copy section.

---

## Library — lucide-react stays

Hard rule 5 stands. No amendment needed. The alternatives were evaluated
against this product, not in the abstract.

| Library | Verdict | Reason |
| --- | --- | --- |
| **lucide-react** | **Keep** | Full concept coverage, ISC, per-icon ESM modules, and already inside every shadcn/ui primitive |
| Phosphor | Reject, with regret | Genuinely better small-size drawing — 16px design size and a Bold weight. Loses on the shadcn coupling below, not on merit |
| Tabler | Reject | Broader set, same 24px grid and same small-size behaviour. A lateral move with a migration cost and no gain |
| Radix Icons | Reject | Drawn on a 15px grid and crisp because of it, but ~300 icons with no git, process, terminal or health coverage. A chrome set, not a domain set |
| Heroicons | Reject | The 16px micro set is optically correct, but there is no git family at all. Fails the coverage test outright |

**The deciding argument is coupling, not aesthetics.** Hard rule 2 mandates
shadcn/ui, and shadcn primitives import lucide directly — dialog close,
select and dropdown chevrons, checkbox check, sheet close. lucide is in the
bundle whatever else is chosen. Adding Phosphor means two icon languages
sharing one dense table: Phosphor's rounded terminals next to lucide's
Feather-derived ones, in the same 32px row. That is precisely the
mixed-weight failure the brief warns about, arrived at by a different route.

**Coverage was checked, not assumed.** Every concept this app needs exists in
lucide: `git-branch`, `git-commit-horizontal`, `git-pull-request`,
`terminal`, `cpu`, `shield-check`, `unplug`, `split`, `waypoints`,
`stethoscope`, `file-diff`. No gap was found.

**The 11px legibility problem is real but is not a library problem.** It is a
selection problem and a rendering problem, both solved below. Any library
mushes if you pick a document-with-a-question-mark and render it at 11px on a
24px grid.

**What would change this answer:** dropping shadcn/ui. If Armada ever owns
its primitives outright, Phosphor with per-icon CSR imports and Bold at
≤12px is the better set on optical grounds and this decision should be
revisited.

---

## Rendering — two sizes, one stroke, no exceptions

```
badge icons       12px · strokeWidth 2 · leading · 4px gap
nav and buttons   16px · strokeWidth 2
stroke width      2, always. Never tuned per size
sizes             12 and 16 only. Never 11, 14, 18, 20
```

**12px, not 11px.** lucide draws on a 24px grid, so 12px is an exact 0.5
scale: a stroke of 2 lands at exactly 1px and every coordinate falls on a
whole or half pixel. 11px scales to 0.917px — a sub-pixel stroke that
antialiases into grey fuzz on a dark ground, which is exactly the symptom the
brief describes. The icon reads *slightly* larger than the 11px badge label,
which is optically correct anyway, since a glyph needs to exceed cap height
to hold equal weight.

**One stroke value.** Tuning `strokeWidth` per size is how a set starts
looking mixed-weight. 2 everywhere is lint-enforceable and holds the set
together. Never use `absoluteStrokeWidth` — a 2px stroke inside a 12px box is
a blob.

**Contrast floor.** Icons never render in `--fg-subtle`, with two exceptions:
`circle-minus` and `circle-dashed`, below. `--fg-subtle` was `#5D6B7C` on
`--bg-raised`, ~3.2:1, which a 1px stroke does not survive, so `--fg-muted`
was the minimum for chrome and for badges alike. The 20 Aug legibility lift
raised `--fg-subtle` to `#7E8CA0`, 4.58:1 on `--bg-overlay`
(`packages/tokens/src/colors.css`), which clears the floor the earlier value
could not. Both are drawn against it because neither may carry a status hue.
`circle-minus` is `gate_undecided`'s mark: it reads as unjudged rather than
judged, and a verdict hue would say the wrong thing — see Judge criterion
verdicts below. `circle-dashed` is `step_state.not_started`'s mark alone — a
dormant step, never a loading state — where a hue would claim an activity
that has not begun; `--fg-muted` was rejected for the same row because it is
`retrying`'s colour, and `packages/tokens/src/status.css` keeps `not_started`
one step dimmer than `retrying` on purpose. Full argument in
`packages/icons/icons.toml`.

> **Flag for the parent document.** The grey badges are the weakest in the
> set — `--status-not-started` measures 4.39:1 as badge text on its own tint
> over `--bg-overlay`, the only status under 4.5:1, and `superseded` shares
> that by aliasing it. Badges on floating layers only. Every state that asks
> for a person has since left grey for amber, so the badges that mean *act*
> are not the weak ones; the remaining greys are dormant states a reader is
> meant to pass over, which makes the shortfall survivable rather than
> fixed. If a glyph disappears there under real use, the fix is raising
> `--status-not-started`, not changing the icon. It is no longer the same
> value as `--fg-subtle` — the two separated at the 20 Aug legibility lift.

**Version pinning.** lucide renamed a large batch of icons (`alert-triangle`
→ `triangle-alert`, and similar). Names in `packages/icons/icons.toml` are
lucide-react ≥ 0.400. Pin the version in `package.json`; do not float.

---

## Every badge carries an icon

**Accepted, and the brief's reasoning is right, but the deciding argument is
a stronger one.** Column consistency is real — a Job Board column where some
badges have icons and some do not reads as an oversight rather than a
system. But that argument is aesthetic and could be argued the other way.

The argument that settles it is redundant encoding. Badges are differentiated
by **hue alone**, at 11px on 12%-opacity fills, and the palette carries fewer
hues than there are statuses — several statuses share one and are told apart
by glyph alone. That is a single-channel encoding of the most important
distinction in the app, and it fails outright under deuteranopia, on a
miscalibrated second monitor, and in a screenshot pasted into a ticket. Every
status earns an icon on those grounds before consistency is considered.

This does not contradict *used sparingly*. The badges are one column
carrying a fixed 16px prefix, not scattered decoration. Sparingly is
enforced everywhere else in this document, including several places where
the answer is no icon at all.

**Label-only was considered and rejected.** The labels are indeed
distinguishable as text, but that is an argument that the icon is not doing
the *primary* job — which is true and fine. Its job is being the second
channel.

---

## Status badge icons

The glyph and the reasoning for each status is in `packages/icons/icons.toml`,
group `Job state`. That file does not carry labels either — the verb belongs
to the Armada Enum Verbs database and is not restated here. The icon agrees
with the verb, not with the enum name, which is why the two are chosen
separately.

### The statuses — hue is primary, icon is redundancy

Ordered as Job orders them, by who is acting and in what mode. Where statuses
share a hue they are the same claim at different points in a Job's life, and
the glyph carries which point. The full enum → icon → hue mapping is
`packages/icons/icons.toml`, group `Job state`.

**Amber means a person is on it; grey means nobody is.** Every status where a
person is waited on or working renders amber — `awaiting_approval`,
`awaiting_review`, `awaiting_attestation` and `piloted` — which is one claim
at different points in a Job's life. Every dormant status renders grey. Rule
4 below is what holds each group apart: human figure, eye, stamp and
terminal box in amber; clock and lidded box in grey.

### `queued`'s reasons — icon differentiates

`queued` renders grey whatever its reason, and a reader is meant to move past
a grey row. Where a reason is present its glyph replaces `clock` on the
badge; where the reason is none, `clock` stands. The vocabulary belongs to
Job and is not restated here. Categorically different outlines: diagonal
chain, fringed square. The mapping is `packages/icons/icons.toml`, group
`Queued reason`.

### `escalated`'s reasons — the same handover, one status over

`escalated` has its own verb and glyph, and a reason's replaces both where one
is set. That is `queued`'s construction above, and it is not a second mark for
one meaning: the status says the Job stopped and is asking, and a reason says
what it stopped on.

**It exists because not every surface is served a reason.** The held-worktrees
list is handed a Job's status and nothing else about why it stopped
(`packages/protocol/src/holding.ts`), so *an escalated Job renders its reason*
was a rule that surface could not obey — it drew a blank where a status goes,
and a blank cannot be told from a finished Job. The glyph is `megaphone`, in
`packages/icons/icons.toml`, reserved to this status and held clear of `bell`
there: an alert is a condition on a Job rather than a status a Job holds, so
the two populations do not share an outline.

### The escalation reasons — one orange, icon differentiates

The hardest constraint here. Categorically different outlines: octagon,
closed loop, page, shield, Y-split, broken plug, ascender-to-a-line. None
depends on interior detail surviving 12px. The mapping is
`packages/icons/icons.toml`, group `Escalation reason`.

**The status's own cone and `awaiting_repair`'s spanner share that hue and
differ from all of them**, which is rule 4 read across the whole orange set
rather than within the reason list alone.

**A reason may draw its verb alone**, and several do — the heading says what
the glyphs must do where there is one, not that every reason has one. Where
none of the outlines claims the right thing, the row ships wordless and the
glyph is filed rather than borrowed: `[no-report-glyph]` and
`[drone-killed-glyph]` below are the worked cases, and both reach the same
answer `[doctor-warn-glyph]` did.

**Why the `loop_cap` row was nearly missed.** `loop_cap` was added to the
escalation enum on 2026-08-21 and existed for several hours with no glyph,
because the decision landed on the Workflow and Job pages and nothing pulled
it through to here. That is what rule 7 below exists to catch, and it is
worth noting that the rule caught it only because a person asked — the
codegen test it describes is not built yet. `loop_cap` obviously wants a
loop, but *closed loop* is already `refresh-cw` for churning, in the same hue
group and on the reserved list. Those two states are the most semantically
adjacent pair in the set — both mean "went round and round" — so sharing an
outline would put the collision exactly where it does the most damage. The
distinction that matters is that churning is a failure and `loop_cap` is
not: nothing went wrong, the loop simply did not converge. A ceiling says
that; a loop does not.

**Cross-group collisions are permitted.** `circle-dot` and `clock` are both
circular, but they sit in different hue groups where colour already
separates them. The rule is strict *within* a shared hue and relaxed across
hues — otherwise the set runs out of distinct outlines for no benefit.

---

## Changes from the earlier proposal

Both problems in the brief are accepted as stated.

- **The hourglass/clock collision is real, and the deeper fault is worse
  than the collision.** `hourglass` reads *be patient* for a state that
  means a drone went silent and something is wrong. It is removed, and
  `hourglass` is now banned from Armada entirely — nothing may use it.
  `clock` keeps `queued` and becomes unambiguous by that removal.
- **A per-column split is not defensible.** Resolved to every badge, on
  redundant-encoding grounds above.
- Two further changes were made beyond the two flagged: `lock` → `link` and
  `git-fork` → `split`, each justified in `packages/icons/icons.toml`.

---

## Step activity — the rail

The marks a rail row can take, distinct from the Job badge states. Six of
these are `job_steps.state` values; `failed` is not — a refusal lands in
`last_verdict` as `failed(<reason>)`, and the activity/verdict split is why:
a step retrying after a refusal is `running` in activity and `failed` in
verdict at the same moment, so one column cannot say both. A step may carry
the Job glyph that means the same thing one level down — a step and a Job
answer the same question at different scales, so a second silhouette for one
meaning is the collision this document exists to prevent, and a rail row
must show the same mark as the badge above it when both state the same
claim. What a step may not do is reuse a badge glyph in a sense the badge
does not carry.

The common rail values borrow their glyph from the Job badge one level down
— `advanced` (check), `running` (circle-dot), `waiting` (clock), `retrying`
(rotate-cw, no hue). The full mapping is `packages/icons/icons.toml`, under
`[conventions.step_activity_borrowing]`.

Two values carry more than a borrowed glyph:

- **`stopped`** takes `flag`, reserved to this state alone (see
  `packages/icons/icons.toml`). It marks a position rather than a verdict —
  the verdict sits on the criterion rows beneath it and the reason on the
  badge above — so it stays `--fg-default` on a `--step-stopped-bg` row
  rather than taking a hue that would say the warning twice. `octagon-x` was
  rejected because the octagon belongs to `stalled`; a bare attempt count
  was rejected for reading like a not-started row.
- **`failed`** takes the same bare `x` as the `completed_failed` badge,
  meaning the same thing one level down, hued in `--step-failed` on a
  `--step-failed-bg` row. It was drawn without a hue first, on the grounds
  that a measured Check result should render flatly — reversed, because a
  failed Check with no retry and no triage is the entire reason a person
  opened the screen, and a muted rail buries it. `stopped` and `failed`
  differ in treatment though not in kind: `stopped`'s glyph stays neutral
  because its surface already carries the warning, while `failed`'s `x` is
  hued, since failure is an outcome and states it in both channels. The gate
  row beneath either stays neutral — the step's state is hued, the Check's
  exit code is measured.

**An ungated step says so in words.** A step carrying no Check is ordinary
rather than exceptional, so the gate row beneath one reads `no check on this
step` in sans, in `--fg-subtle`. An empty slot where a gate row would sit
reads as a gate that failed to render rather than one that is absent.

### Judge criterion verdicts

`met` and `not_met` take `circle-check` and `circle-x`; `gate_undecided`
takes `circle-minus` — see `packages/icons/icons.toml`, group
`Step and Verdict`.

**The Judge owns `circle-*`, decided 2026-08-21.** Three families, one per
verification source: `shield-*` for Checks, `file-*` for evidence artifacts,
`circle-*` for Judge verdicts. A verdict on a criterion is neither a gate nor
an evidence artifact, so borrowing either family would misstate what
produced it — the silhouette carries hedge-by-source before the label is
read.

**Knock-on, resolved in the same turn:** `circle-minus` was carrying Check's
`not reached`. It moves to `shield-minus`, which completes the check family
rather than damaging it. Bare `check` and `x` are out of criterion rows
entirely — `check` already means `advanced` in step activity, so a criterion
and a finished step were reading alike.

**A third outcome, `gate_undecided`, takes `circle-minus` back.** `met` and
`not_met` are both judged; `gate_undecided` is what the Judge reports when it
could not read the artifact at all, which is not a criterion judged badly —
there is no judgment to render. `circle-minus` returns to the family it left
on 2026-08-21, but the meaning is new rather than restored: `shield-minus`
keeps Check's `not_reached`, and this is a distinct state in a distinct
family that happens to share its outline. `core-model`'s escalation module
states why the rendering differs from a verdict: `gate_undecided` is the one
escalation trigger that is not overrulable, because the machine is saying it
could not read the artifact, so there is nothing ruled to disagree with —
`Recourse::RerunGate` is what answers it, not Override. It renders in
`--fg-subtle`, not `--verdict-met` or `--verdict-not-met`, because the
criterion went unjudged rather than judged and coloured.

**Reserved:** nothing outside a Judge verdict may use `circle-check`,
`circle-x` or `circle-minus`.

**A criterion attested by a person carries no glyph here.** Source
Attestation reads `confirmed` and `withheld`, and every verdict family is
reserved to one of the other two sources. Which glyph it takes, or whether
it deliberately takes none, is an open question.

---

## Navigation

16px, `--fg-muted` at rest, `--fg-default` when active. Never
status-coloured. The full mapping (Job Board, Alerts, Doctor, Manifest,
Helm, Worktrees) is `packages/icons/icons.toml`, group
`Navigation` — `eye` and `file-cog` are shared assignments, carried under
their own primary groups with a `Navigation` usage entry.

**`hard-drive` is Worktrees' and names the destination, never the act.** A
glyph drawing deletion would promise the bulk reclaim that surface exists to
replace. Its two interior dots survive because a rail draws at 16px and rule
3's interior-detail floor is the 12px badge; they are the reason its row is
not `12 and 16px`.

**No nautical iconography.** A ship's wheel or compass for Helm is the exact
failure P1 legislates against — metaphor confined to proper nouns. Helm is a
name; a wheel is decoration, and decorative iconography is banned outright by
the parent document.

---

## Actions

16px. Text buttons carry no icon — primary and secondary buttons are
label-only. Icons appear on **ghost and icon-only row actions**, in
confirmation dialogs, and in toolbars. Per the voice contract an action
keeps its name through the flow, so the glyph must survive both the button
and the resulting past-tense state. The full mapping (Approve, Reject,
Dispatch, Kill, Redirect, Redispatch, Pilot, Freeze dispatch) is
`packages/icons/icons.toml` — several of these glyphs are shared with a Job
badge state (`check`, `ban`, `power`, `terminal`) under group `Job state`;
the rest are under group `Action`.

---

## Everywhere else

### Doctor — health grid

**No glyphs.** Amended 2026-08-21 from the design of Check System Health. A
result is the word, in the status colour, at `--text-2xs` in mono.

```
pass   — no glyph —   --status-completed-success
warn   — no glyph —   --status-awaiting-review
fail   — no glyph —   --status-completed-failed
```

Three reasons, in order of weight. **The glyphs were already taken:**
`circle-check` and `circle-x` are reserved to Judge criterion verdicts, and
two surfaces cannot own one family. **A column of words scans better than a
column of glyphs** — ten rows reading `pass` are read in one movement, and
rule 1 below is to default to no icon. **The word survives greyscale**,
which colour alone does not. Dropping the glyph resolves the collision
without inventing a fourth family or taking anything back from the Judge,
and it makes the circle-wrap argument moot: a lowercase word cannot be
mistaken for a Job badge.

**Colour is still shared with Job states.** Three dedicated Doctor tokens
were rejected and stay rejected — a health grid rendering green, amber and
red in values nobody else uses makes a problem harder to spot, not easier.

**`triangle-alert` stays reserved to Doctor.** It was released on the
reasoning that Doctor draws no glyphs and so had no use for it; that release is
withdrawn. The glyph means *a check warns*, and Doctor is the only surface that
can say that. It remains available for a generic warning in a toast, which is
what `octagon-alert` was kept out of them to protect — but the reservation
means no other surface may adopt it as its own mark.

This leaves one thing unsettled, and it is written below rather than assumed:
the three reasons for a wordless health grid were argued against `circle-check`
and `circle-x`, which belong to the Judge. None of them is an argument against
`triangle-alert`, which nobody else owns.

### DAG / graph view

`waypoints` at 16px is the view toggle — see `packages/icons/icons.toml`,
group `Graph`. Nodes inside the graph reuse the badge icons at 12px — a
graph node and a Job Board row showing the same job must show the same
glyph.

### Convoy

`layers` at 12px, at the approval gate and in the detail header only —
never in a Job Board row. Convoy is blast-radius information that matters
when deciding, and the row already says "convoy, 3" in text. See
`packages/icons/icons.toml`.

### Chrome

The full chrome mapping — expand/collapse, sort, filter, search, copy an id
or path, open PR externally, dismiss dialog — is `packages/icons/icons.toml`,
group `Chrome`.

### Git and config — detail views only

Never in a Job Board row. Precedes a mono value at 12px, in `--fg-muted`.
The full mapping (repository, workspace, branch, commit, pull request,
Manifest) is `packages/icons/icons.toml`, group `Git and config`.

### Where the answer is no icon

Stated explicitly, because each is somewhere an icon would plausibly be
reached for.

- **Verification source and actor.** Closed vocabularies of three and four
  words. "Check", "Judge", "Attestation", "human", "Helm", "Drone", "Fleet"
  are already shorter and more precise as text than any glyph, and P4
  depends on the reader distinguishing them exactly. An icon here trades
  precision for width. This is the source field only — a criterion verdict
  does carry a glyph, one family per source.
- **Diff views.** The diff tokens and the `+`/`-` gutter do the whole job.
  No icon.
- **Empty states.** No large centred icon, no illustration. The parent
  document gives empty states one line pointing at available work; a grey
  ghost glyph above it adds nothing and reads as a consumer app.
- **Spend, quota, elapsed, step N of M.** Numbers in mono. No gauge, no
  coin, no timer.
- **Status bar.** Text only. The escalation and approval counts are the
  only colour in the bar, and an icon beside them would make it a second
  alert surface.

---

## Reserved glyphs

One meaning each. These may not be reused for anything else, ever.

```
refresh-cw     churning only. Refresh controls use rotate-cw
octagon-alert  stalled only. Generic warnings use triangle-alert
file-*         evidence only
shield-*       gates and checks only
circle-*       Judge criterion verdicts only. circle-check, circle-x and
               circle-minus may not be reused — not for Doctor results, not
               as generic success/failure marks, and circle-minus never for
               disabled, absent or removed
human figure   human required, or actor=human
eye            review
terminal       Pilot only. The action and the piloted status, which are one
               concept at two points in a flow
hourglass      BANNED. Nothing may use it
flag           stopped step only. A step whose retries are spent
megaphone      job_status.escalated only. Never an alert or a notification —
               bell is Alerts', and an alert is a condition on a Job rather
               than a status a Job holds. Never an escalation reason
wrench         job_status.awaiting_repair only. Never settings, never a
               Manifest, and never an action — repairing is what a person does
               off the badge, not a button Armada draws
hard-drive     the Worktrees surface only. Never the act of reclaiming, and
               never a delete or a sweep control
chevron-down   disclosure only. The caret segment of a split button, and the
               one exception to "primary and secondary buttons are label-only"
               — it is the whole content of its own divided segment, structural
               rather than decorative, and never sits beside a label
triangle-alert RELEASED 2026-08-21. Was reserved to Doctor; Doctor now draws
               no glyphs, so this is free for generic warnings and toasts
```

---

## The rule for anything not listed

**Listed means: has a table in `packages/icons/icons.toml`.** A glyph with
no table has not been decided, whatever it looks like in a mockup.

1. **Default to no icon.** If the label alone is unambiguous, ship the
   label. Most new things need nothing.
2. If an icon is needed it comes from **lucide-react**, at **12 or 16px**,
   **strokeWidth 2**, inheriting text colour. No second library, no emoji,
   no illustration, no custom SVG. **One exception exists, and only one** —
   see Brand mark below.
3. Choose on **outline, not detail**. At 12px only the silhouette survives.
   If the meaning lives inside the shape, the icon is wrong.
4. The outline must differ from every other icon **sharing its hue**.
   Across hue groups, collisions are fine.
5. Never reuse a reserved glyph above.
6. Never render an icon in `--fg-subtle`, except `circle-minus` and
   `circle-dashed` (`step_state.not_started` only) — see Contrast floor
   above — and never let an icon carry colour independently of its badge.
7. **A new enum variant must add a table here.** The codegen test asserting
   every variant has a verb asserts it has an icon in the same pass, so a
   new reason cannot ship iconless.

---

## Brand mark — the one custom glyph

**Amendment to hard rule 2, added 2026-08-24.** Rule 2 says no custom SVG. It
now carries exactly one exception, and it is named here so that the next
person to read the rule does not delete the component on sight.

`armada-mark` is the Countersign identity mark. **It is not an icon and does
not compete with lucide.** An icon names a state or an action inside the
app; this names the app. That distinction is the whole basis of the
exception and also its limit — the moment the mark is used to mean *home*,
*app*, or a nav destination, it has become an icon and rule 2 applies again.

Its construction — the 24-unit grid, the butt caps and miter joins instead
of lucide's round, the filled element, the size floor, the reserved status —
is recorded in full in `packages/icons/icons.toml` under `armada-mark`, so it
is not restated here. Full specification of the mark itself — construction,
clear space, the size floor, colour, and six misuses drawn rather than
described — is a separate Armada Identity document, not migrated here.

Its entry in `packages/icons/icons.toml` is Proposed, under the Brand group
added 2026-08-24. Brand is a category of one and should stay that way — a
second row in it means rule 2 has quietly stopped holding.

---

## Amendments to the parent Design System document

Both amendments are applied. Verified against [Design
System](design-system.md) on 20 Aug 2026.

- **Hard rule 5: no change.** lucide-react stands.
- **Badge spec: applied.** The parent document's Badge specification now
  reads `icon required, 12px lucide, strokeWidth 2, leading, inherits text
  color`, carrying both the size change and the optional-to-required change.
- **Voice & Copy icon table: applied.** The 10-row table is gone. That
  section now points here and records the four entries that changed.

---

## Open questions

Nothing Doctor-shaped. `[doctor-warn-glyph]` was answered by the Doctor drawing
on 2026-08-31: **`warn` draws no glyph either.** The choice was a glyph on every
row or none — `pass` and `fail` have none available, because the Judge holds
`circle-check` and `circle-x` — and a grid where one row of three carries a mark
is the inconsistency the wordless rule was avoiding.

**The reservation stands.** `triangle-alert` means a check warns, Doctor is the
only surface that can say that, and no other surface may adopt it as its own
mark. A reservation with no current user is not a spare glyph; it is the reason
`octagon-alert` was kept out of generic warnings.

Step-level and criterion-verdict glyphs are settled as of 2026-08-21 — see
Step activity above. `packages/icons/icons.toml` currently lists
`circle-check` as Proposed and `circle-x`/`shield-minus` as Specified; its
status vocabulary (Specified, Proposed, Retired, Banned) has no analogue to
"Decided", so the file and the settled-as-of-2026-08-21 claim do not fully
line up — worth a person's attention rather than something this document
should paper over.

- **[nav-icon-active-fill]** Does the active nav surface change icon fill,
  or only colour?
  Navigation icons render at 16px, `--fg-muted` at rest and `--fg-default`
  when active, never status-coloured. lucide icons are stroke-only outlines
  with no matched filled variant, so "fill" would mean a background shape
  behind the icon rather than a swapped glyph — the realistic option, and it
  matters most in the collapsed 48px rail, where the icon is the whole nav
  item and a background is the only affordance available. Expanded, the
  label carries the affordance too, so the answer may differ by sidebar
  state.

- **[kit-file-icons]** Do the three Kit-file states — in Kit, drifted, not
  in Kit — get icons, or stay label-only?
  These are not Job states, so they sit outside the 16-badge table. Under
  the rule for anything unlisted, the default is no icon, and three
  unambiguous text labels may not need one — the 16-badge table only needed
  icons because several escalation reasons shared one hue. Ship label-only
  unless a real machine's worth of files in the column proves it hard to
  scan; that cannot be judged without one.

- **[attested-verdict-glyph]** Which glyph, if any, carries a criterion
  verdict from source Attestation (`confirmed` / `withheld`)?
  One glyph family per verification source is the settled rule —
  `shield-*` for Checks, `file-*` for evidence, `circle-*` for Judge — and
  all three are taken and reserved, so none can be widened without breaking
  the rule that gives them their meaning. The human-figure family means a
  person is required, not what a person concluded, and `user-check` already
  carries `awaiting_approval` in the badge set. The alternative is that
  Attestation draws no glyph at all, the same way Doctor's health grid
  dropped its glyphs for words — which would make Attestation the one
  verdict source that reads as wordless, arguably the right way to mark a
  verdict a machine did not produce.

- **[no-report-glyph]** Which glyph, if any, carries the escalation reason
  `no_report` — the step told to stop and report that never answered?
  It ships drawing its verb alone, **went quiet**, beside the escalation
  reasons that already draw one. What was looked at and refused:
  `octagon-alert` is reserved to `stalled` and is the loudest shape in the
  set; `refresh-cw` is `churning`'s and is the badge this trigger is being
  split away from; `bell` is escalations-as-interrupt and its own note
  keeps it clear of `octagon-alert`; `message-square` is Navigation's,
  under a note banning metaphor outside proper nouns. The constraint that
  makes this hard is that the trigger's whole meaning is an *absence of a
  reply from something still working*, and a silhouette says presence more
  easily than it says a missing answer — which is why nothing was
  borrowed. `[conventions.step_activity_borrowing]` does not reach it
  either: that convention lends the Job set's glyphs to step *activity*,
  and this is an escalation reason. What decides it is whether a person
  scanning the Board can tell this row from a `churning` row on the word
  alone; if they can, the answer is that it stays wordless, the same way
  Doctor's health grid settled `[doctor-warn-glyph]`.

- **[scope-refused-glyph]** Which glyph, if any, carries the escalation
  reason `scope_refused` — the step whose Drone asked to write outside
  the task's scope and was told no? It ships drawing its verb alone,
  **the scope request was refused**, beside the escalation reasons that
  already draw one. What was looked at and refused: `ban` is
  `rejected`'s and means a person declined a whole Job, so a step
  wearing it would say the Job was rejected when it is running work
  somebody still wants; `circle-x` is `gate_failure`'s and says work was
  weighed and found short, which is the distinction this trigger exists
  to hold; `circle-minus` is `gate_undecided`'s and means the Judge
  could not decide, where here it decided; `octagon-alert` is reserved
  to `stalled` by its own note; and the `shield-*` family is reserved to
  gates and Checks, which a request is neither. What makes it hard is
  that every candidate says either "a person declined this" or "a
  machine could not answer", and this row means **a machine read a
  request and would not grant it** — a decision about a request rather
  than about work, which nothing else in the badge set is. Neither open
  question above reaches it: `[no-report-glyph]` is an absence of a
  reply and `[drone-killed-glyph]` is a person having already acted.
  What decides it is whether a person scanning the Board can tell this
  row from a `stopped at the gate` row on the words alone; if they can,
  it stays wordless, the same way the two above were left.

- **[drone-killed-glyph]** Which glyph, if any, carries the escalation
  reason `drone_killed` — the step whose Drone a person ended? It ships
  drawing its verb alone, **the Drone was ended by hand**, beside the ten
  escalation reasons that already draw one. What was looked at and
  refused: `unplug` is `interrupted`'s, and the distinction against it is
  the entire reason this trigger exists — a process that died and a
  process a person pulled the plug on are opposite events wanting
  opposite responses, so sharing the severed-plug silhouette would undo
  the split at the one place a reader actually looks. `octagon-alert` is
  reserved to `stalled` by its own note. `user-check` is the only human
  silhouette in the badge set and is `awaiting_approval`'s in the Job
  state group; the human-figure family means a person is *required*,
  which is the opposite of a person having already acted.
  `[conventions.step_activity_borrowing]` does not reach it — that lends
  the Job set's glyphs to step *activity*, and this is an escalation
  reason. What makes it hard is that every candidate says either "the
  connection broke" or "a person is needed", and this row means "a person
  already decided". What decides it is whether a person scanning the
  Board can tell this row from an `interrupted` row on the words alone;
  if they can, it stays wordless, the same way `[no-report-glyph]` above
  and `[doctor-warn-glyph]` were left.
