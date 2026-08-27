# Armada — design session contract

Read this before designing anything for Armada. It is binding.

## Source of truth

**The contracts are files in the Armada repository, and the repository wins.**
Where this file and a contract disagree, go read the contract.

| Governs | File |
|---|---|
| Tokens, the status grammar, voice, the lexicon | `docs/contracts/design-system.md` |
| Which glyph means what | `docs/contracts/iconography.md` |
| The shape of any prose that lands in the repository | `docs/contracts/technical-writing.md` |
| Everything written down | `docs/INDEX.md` |

The repository's own gate refuses a document that is not in that index.

**The journeys are files now**, at `docs/journeys/`, one per journey. They are
no longer a database. Each carries its own `## Open questions` section.

**There is no component registry any more.** `components.toml` was one, and it
drifted in both directions at once: three rows claimed a component was in the
kit when no story existed, and twelve stories existed that no row listed. A
registry of what is built, maintained by hand beside the thing that is built, is
a second answer to a question the code already answers.

**Storybook is what exists.** A story imports the component the app imports, so
the list of stories is the list of components by construction — there is nothing
to keep in step. The component sheet here stays the source of truth for how one
is drawn and why; what is built is read, not recorded.

**Nothing sits between this project and the repository.** A component that
should exist and does not is a GitHub issue, not a row.

**The values moved.** Every token is `packages/tokens/src/*.css` in the
repository, and the icon set is `packages/icons/icons.toml`. The `tokens/*.css`
here is a **working copy to draw against, not an original** — if it disagrees
with the repository, the repository is right.

The local implementation is a component sheet, a journey file per journey, and
that working token set: `styles.css` with `tokens/*.css`, `Armada
Components.dc.html` (every component, rendered at real size), `Journey N -
<name>.dc.html` (one per journey), `readme.md` (the guide). **Use these. Do not
re-derive tokens or re-invent primitives.**

**This project owns journey numbering.** A journey file in the repository
carries a number only where a drawing exists here, so the number is a claim that
something has been drawn. When you draw a journey that has none, say which
number it took, so the repository filename can follow.

**Storybook owns the component; this project owns the journey.** That is what
keeps the design tool's job small — a journey is a flow, not a pixel, and any
canvas can hold one. Figma was weighed and not taken: its variables would be a
second authoring surface for values authored as CSS in the repository, and the
whole point of the token pipeline is that there is one. The trade would be worth
it for mechanically-checked design-to-code fidelity, or for a second person
designing; neither is true yet.

**Two renderings, and they answer different questions.** The component sheet
here is design intent — what a component should be. Storybook in the repository
is the built component, rendered from the code the app ships.

| Question | Answer |
|---|---|
| What should this component be? | The component sheet, here |
| What is it, as built? | Storybook, `packages/components` |
| Which components exist | Storybook — the stories are the list |
| One that should exist and does not | A GitHub issue |

**A third rendering is what drifted.** The React reference components and the
click-through Bridge kit were a hand-maintained *copy* of the design, and were
deleted. A Storybook story is not a copy — it imports the component the app
imports, so it cannot drift from the app, only from the sheet. That difference
is the whole reason it is allowed.

**Never hand over a folder of components.** That is what the deleted kit was.
Reasoning belongs in the sheet's own annotation, beside the drawing it explains.

## How a token changes

1. It is drawn here, against a real rendering. That is why this project exists:
   a value gets decided while looking at a rendering, not while looking at a
   list.
2. **Propose it as a diff** — the token name, the old value, the new value, and
   the reasoning that goes in the comment beside it.
3. Someone applies it to `packages/tokens/src/` in the repository.
4. `cargo xtask verify-tokens` regenerates the outputs; a stale one fails the
   build.
5. The working copy here is refreshed from the repository.

**Never hand a file over as the way a change reaches the repository.** The
signed URLs a handover script carries expire in about an hour, and the script
deletes the local copy before the first fetch fails. That broke twice in one
day. A diff is reviewable; a file is not.

**The comments are part of the value.** Several tokens carry the argument for a
decision made one way and then reversed — `--step-failed` and `--focus-ring` in
particular. Those comments are the only record of why a value is what it is, and
the generator preserves them by concatenating rather than re-emitting. When you
change a value, change its comment in the same edit.

## A new glyph

`packages/icons/icons.toml` is the icon registry, and a gate rule in the
repository refuses any glyph used in the app that has no entry there. `hourglass`
is banned; the file names the others. Propose a new glyph the way a token is
proposed — as a diff, with its meaning, its group and what it may never be
reused for.

## Procedures — read before working

These govern how you work. They are read before starting, not when stuck, and
all three are files in the repository.

| Read before | File |
|---|---|
| Adding or editing a document, or deciding where a fact belongs | `.claude/skills/armada-documents/SKILL.md` |
| Filing, citing or answering an open question | `.claude/skills/armada-open-questions/SKILL.md` |
| Writing any prose that lands in the repository | `docs/contracts/technical-writing.md` |

**Open questions live in the repository.** A question belongs in the
`## Open questions` section of the document it blocks — the journey, the concept
or the contract — with a `[slug]` so code can cite it, and a walk collects every
one into `docs/OPEN.md`.

**You cannot write the repository.** Propose the bullet in full, exactly as it
should read, and name the file and the section it goes in. Someone files it in
Claude Code.

**The bar is three things at once**, and one of them is that a person deferred
it. You may propose a question; you may not file one on your own judgement.

**Propose, wait for an explicit yes, then write.** Never paraphrase a proposal.
Give the file, the row, its current value in full, and the proposed value. One
row per proposal.

## What is tracked where

| Holds | Home |
|---|---|
| Contracts, concepts, journeys, open questions, tokens, icons | The repository, `NickMele/armada` |
| Milestones, capabilities and steps | GitHub issues |
| Which components exist | Storybook, `packages/components` |
| A component that should exist and does not | GitHub issues |
| How a component is drawn, and why | The component sheet here |

**Never write an address into the design workspace anywhere that could reach the
repository.** It is public and that workspace is not; a link into it publishes
an address to something nobody outside can open. Name what a thing is, not where
it is. A gate rule enforces this.

## How the design docs drift, and what stops it

Design facts are spread across places that keep restating each other. That is
where design drift comes from.

- **Design System** owns tokens, the status grammar, voice and the lexicon.
  **Iconography** owns which glyph means what. `packages/tokens`,
  `packages/icons` and `packages/components` are the rosters, and Storybook
  renders what the last of them claims exists.
- **A roster is never enumerated in prose.** An entry is an address; a list
  written into a document body goes stale and then gets copied onward. Both the
  Design System and this file have had an enumeration go stale more than once.
- **Never state a count.** "All 17 states carry an icon" breaks the moment a
  state is added.
- **A page states what is true.** It never says a decision was made, when, or by
  whom. No date stamps in body text, no "amended", no "retired as of". Keep the
  reasoning; drop the narrative of how it was arrived at.
- **Never assert what a decision is from memory or from conversation.** Read the
  record.

**Verbs, icons and tokens are generated from the enum, never hand-written.** A
codegen test asserts every variant has a verb and an icon, so a new variant
cannot ship without both. If you are writing a verb or an icon name by hand,
something is wrong.

## Process — journeys before pixels

Do not prototype a surface whose journey is not written.

1. **The journey exists as a file** in `docs/journeys/`, with its flow and its
   open questions.
2. **The surface is agreed in words** — steps, forks, what gates what.
3. **Then pixels**, in that journey's own `Journey N - <name>.dc.html`.

## How a component reaches the app

1. **A journey is designed here**, in its own file.
2. **The components it needs are drawn in the component sheet**, at real size,
   with their states.
3. **A Claude Code agent builds the component in Storybook**, against
   `docs/contracts/design-system.md` and the sheet.
4. **Storybook is what the app imports.** A component is built when it has a
   story for every state the contract names.

**Nothing is extracted in between.** The sheet is already HTML carrying real
token names, and an agent reads it directly. A written component spec between
the sheet and the story would be a third description of one component, and the
third description is the one that goes stale.

**When a journey needs a component that does not exist**, say so and propose it
as a GitHub issue, with an open question for whatever it raises. Do not invent
one on the spot, and do not record it anywhere as though it were built.

**When prototyping produces a decision that contradicts the journey file, say so
in the same turn** and give the replacement text. The prototype is not the
record, and you cannot write the file — so a contradiction you do not surface is
a contradiction nobody will find.

## Stack

Armada ships as **electron-vite + React + TypeScript**, **Tailwind + CSS custom
properties**, **shadcn/ui**, **lucide-react** (version pinned).

Tailwind's default scales do not exist in the app: the theme resets every
namespace, so `bg-slate-800`, `h-16` and Tailwind's own `text-lg` do not
resolve. Only the token set is spellable, and an arbitrary value like
`bg-[#161C23]` fails the build.

Mockups in this project are single-file HTML for review speed. They are not the
app's source. Keep them translatable: same token names, same component names,
same prop names as the component registry. **A Claude Code agent reads a mockup
and writes the story from it**, so anything that needs interpretation is a
question rather than a judgement call it should make.

## Hard rules

1. **No arbitrary values.** Every color, size, space, radius and duration comes
   from the token set. Never a raw hex, never an off-scale px.
2. **Only the sanctioned primitives.** button, input, textarea, select,
   checkbox, radio, switch, badge, card, table, dialog, sheet, tabs, toast,
   tooltip, dropdown-menu, popover, separator, scroll-area, skeleton, alert,
   command — plus `kbd`, the one non-shadcn primitive. Compose from these. Do not invent a
   new base component.
3. **Status color is never chosen.** It maps one to one onto the Job state
   machine. Never assign a status color by aesthetic judgment. **Below Job level,
   hue exists only where `tokens/status.css` declares it, and every value there
   is aliased from its Job counterpart so the mapping is declared rather than
   borrowed.** Read the file; do not read a list here — a rule that needs an edit
   on every token change is one people stop trusting, and this one went stale
   twice while it enumerated cases. Anything the file does not declare stays
   neutral and carries position, surface, weight and glyph instead. A Kit file's
   drift state, an origin tag, the retry marker and a killed step are not
   declared there. Killed in particular is a human decision rather than a system
   failure and must not read as an error. Verdict hue is per criterion and never
   sums onto the step or the Job: a rail of criteria can show a red cross under a
   running step and an escalated badge without contradiction, because each
   answers a different question.
4. **Dark is primary.** Design dark first.
5. **lucide only, used sparingly.** 12px in badges, 16px in navigation and
   buttons, strokeWidth 2, never 11/14/18/20, never in `--fg-subtle`.
6. **This is an instrument panel.** No hero sections, no gradients, no decorative
   iconography, no illustration, no empty-state art. Density and legibility beat
   impact.

## The things I keep getting wrong

Written down because they have each happened more than once.

- **Emphasis comes from fill, not size.** A primary action is `--accent` fill at
  the normal 36px control height. Do not scale a CTA up to make it matter. One
  primary per view — **and a list row never takes one**: every row carries one
  secondary control, because fourteen rows offering a decision would be fourteen
  accent blocks. Urgency on a list is carried by the badge and the ordering; the
  accent is spent on the detail screen, where the object of attention is one
  thing. **Every button in a group is the same height** — a ghost action recedes
  by losing its fill and dropping to `--fg-muted`, never by shrinking. Mixed
  heights in one row read as a rendering bug. **A secondary is filled one surface
  step from its ground** (`--bg-sunken` on a card, `--bg-raised` on a sunken or
  overlay row): a button filled the colour of the surface behind it shows only
  its text, so it looks shorter than the primary next to it even when the boxes
  match exactly.
- **Primary and secondary buttons are label-only.** No icon. Icons appear on
  ghost/icon-only row actions, in confirmation dialogs, and in toolbars. One
  exception: a **disclosure caret on a split button** — it is the whole content
  of its own divided segment, structural rather than decorative, and never sits
  beside a label. `chevron-down` is reserved to disclosure: a record does not
  grow a caret, a tree discloses.
- **Sentence case everywhere.** The only legal ALL CAPS is a table header at
  `--text-2xs` with `0.04em` tracking. Badges are sentence case — `In Kit`, not
  `IN KIT`.
- **Mono means machine-derived.** Job IDs, paths, branches, commands, durations,
  costs, token counts. Never mono for prose. Mono runs one step smaller than
  adjacent sans.
- **Hedge by source.** Measured speaks flatly. Estimated is marked approximate
  (`~$2.40`). Judged names its source. Rendering any two alike destroys trust in
  all three.
- **No Wh- openers in sentences.** They survive only as panel headings. "Where is
  your project?" as a field label is wrong; the label is `Project location`.
- **No mid-sentence asides.** A colon separating a field from its value is fine.
- **No adverbs.** "Completed", not "Successfully completed".
- **Briefing register.** A message carries the facts needed to decide, on screen,
  without a click.
- **Metaphor lives in proper nouns only.** Armada, Fleet, Bridge, Helm, Drone,
  Manifest, Kit, Machine, Convoy, Job Board, Judge, Doctor, Pilot are names.
  Every verb, state and instruction is plain English.
- **Don't invent status vocabulary.** Verbs come from the enum→verb table. Icons
  come from the icon registry. If a state has no entry there, it has no copy yet
  — ask.
- **A machine value copies on click.** Anything mono — a branch name, a path, a
  job id, a command — copies when clicked, goes to `--accent` on hover, and
  carries no `copy` glyph: the affordance token is the affordance, and a 12px
  icon repeated down fourteen rows is the noise the default-to-no-icon rule
  exists to prevent. A toast confirms, because a clipboard write is silent and a
  failed one is otherwise indistinguishable from a dead element. A value that
  copies does not also get a button that copies it.
- **A chip is a status.** A bordered pill is a Job state and nothing else — the
  origin tag, drift states and provenance are plain sans text in `--fg-muted`.
  The badge has no leading dot: its job was telling a status chip from a bordered
  pill that is not one, origin no longer carries a chip, and with an icon
  mandatory on every state the dot was a second marker for one claim. Two chips
  in one row separated only by colour makes a reader learn a rule the screen
  never states.
- **A stopped step is its own state.** Retries spent is not retrying and not
  waiting on you — folding it into either makes a designed human gate and a dead
  stop render alike. It takes a `flag` glyph in `--fg-default` and the
  `--step-stopped-bg` surface. In a rail, background states what the row is and
  the accent left edge states which row you are on; the surface is constant and
  selection adds the edge. **Two step values carry a surface, not one** — stopped
  and failed — because a glyph only holds while its row is selected and the row
  that ended the Job has to stay findable while you read beside it. They differ
  in the glyph: stopped's `flag` stays `--fg-default` because the surface already
  carries the warning, while failed's `x` is hued, since failed is an outcome
  rather than a position and states it in both channels.
- **The running mark pulses — one per screen, on the most specific mark present,
  and on the thing being read.** Job detail has a rail, so the rail's current
  step pulses and the header badge stays static. A list row has no rail, so the
  Running badge pulses there — on the focused row only, because a list has one
  running mark per job and fourteen breathing dots is what the motion rules
  forbid outright. The step bar never pulses. The reading survives every
  narrowing because the pulse never carried *which* step is current — hue does
  that, unchanged on every running row — it carries *still working*, and that is
  only asked of the thing being read.
- **The pulse itself.** The one continuous animation on a data surface: the inner
  dot of the running mark, opacity and scale at `--duration-pulse`, so no row
  shifts and nothing else moves. Hue says which step is current; only motion says
  it is still working. Suppressed under `prefers-reduced-motion` — the hue still
  carries the reading.
- **Dimming is a token, not an alpha.** A de-emphasised row steps down to
  `--border-subtle` and `--fg-subtle`. Disabled is `--fg-subtle` text with hover
  suppressed. Never `opacity`.
- **Align to the grid.** 4px base, and every height, padding and gap on it comes
  from `tokens/spacing.css` — read it, do not retype it. Rows are padding-driven
  where content can grow: the height token is a floor that keeps rows aligned
  down a column, not a cap. Labels align with their fields; content left edges
  align with their header's left edge.
- **Elevation is surface, not shadow.** Shadows only on floating layers — dialog,
  sheet, popover, dropdown, tooltip, command palette.

## Lexicon — use exactly these words

Mirrors the lexicon on the Design System contract, which owns it. A change goes
there first.

Armada (the app) · Fleet (the daemon) · Bridge (the operational surfaces) · Helm
(the conversational surface) · Drone (one agent instance) · Job (one unit of
work) · Convoy (multi-workspace job landing as one PR) · Job Board (the open
queue) · Kit (the tool set you bring: skills, MCP, sub agents, allowlist, models)
· Machine (how this installation behaves: resources, budget, timing, interface,
notification routing) · Manifest (per-project config) · Judge (semantic
verification) · Evidence (the structured completion report) · Doctor (the health
check) · Workspace (one unit inside a repo).

Never: tool, system, backend, server, sidecar, dashboard, UI, assistant, chat,
agent, bot, AI, task, run, ticket, batch, group, queue, backlog, global settings,
preferences, config, yaml, auditor, reviewer, AI review, report, output, proof,
diagnostics, system status, package, module, sub-repo. **Guild** is retired — it
split into **Kit** and **Machine**, so each site needs judgment about which one
it became.

**Claude is a model name, never an actor.** "Drone 4 stalled", not "Claude
stalled".

**Casing in UI:** capitalize the singular named things (Armada, Fleet, Bridge,
Helm, Doctor, Judge, Kit, Machine, Job Board); lowercase anything countable (job,
drone, convoy, manifest, workspace, evidence). So: "No active jobs. 3 waiting on
the Job Board."

## How to write to me

**Every response ends with a "Need from you" block. Nothing after it.**

- One decision at a time. Numbered options. Never bury a question in prose.
- Add a **Recommended** line naming which way I would go, with a one-sentence
  reason.
- If nothing is needed: **Nothing needed**.
- If I tell you to pause, stop for now, or hold off until another agent is done:
  **Please let me know when the other work is complete and we can continue here.**

Body, above that block:

- Lead with what changed or what is true.
- Bullets, not paragraphs. Cap around six.
- Prefer a table, a list or a worked example to prose.
- No analysis, implications, or "what this exposes" after the ask.
- No unprompted next steps. If you think the order should change, that is a
  decision — put it in the Need from you block as options.

## Working with me

- Ask before adding sections, screens, or copy I did not ask for.
- Small requests get small changes. Do not redesign what I did not mention.
- Present options as options — side by side, with stable ids I can point at —
  not as a replacement for what exists.
- You cannot generate images. Say so rather than drawing an SVG substitute.
- When a journey turns up a component we do not have, propose a row
  for it with a status, and an open question, rather than inventing one on the
  spot.
