---
name: armada-components
description: How to build or change an Armada component — where it lives, what proves it, and what to do when the contract does not say. Load before touching packages/components.
---

# Changing a component

**Storybook is the source of truth for what a component is.** Its geometry, its
states and its props live beside its code in `packages/components`. A component
is agreed in its story, against a real rendering, and iterated there — there is
no second drawing of it to reconcile against.

## Read before writing a line

| For | Read |
|---|---|
| The spec | `docs/contracts/design-system.md` — `## Hard rules` and `## Component → token mapping` |
| The rules broken most | `docs/contracts/iconography.md` — default to no icon, the two sizes, the contrast floor |
| The tokens | `packages/tokens/src/*.css`. Read the comments; several carry the argument for a value |
| The glyphs | `packages/icons/icons.toml` |
| The states a status can hold | `crates/core-model/domain/enum-verbs.toml` |

**Where the contract and a built component disagree, the contract wins** — and
the disagreement is a finding worth reporting, not a coin toss. Four were found
in one night: badge type step, badge padding, table cell padding, table header
colour.

## Where a component lives

```
packages/components/src/primitives/<PascalName>/
    <PascalName>.tsx
    <PascalName>.stories.tsx
    <PascalName>.css
```

**A component owns its own stylesheet and nothing else touches it.** Parallel
authorship of one shared stylesheet is how two agents overwrite each other.
Register it with one `@import` line appended to `src/index.css` — that file is
append-only.

**The directory is PascalCase; the story title is the human name.** Drop
non-alphanumerics, capitalise each word, concatenate: `Job row (stacked)`
becomes `JobRowStacked`, with `title: "Compositions/Job row (stacked)"`. The
exact name lives in the title so nothing is lost, and a name maps to a path with
no lookup table.

## Stories

**One story per state the contract names.** Not a kitchen sink, and not states
you invented.

**Where the contract names a state you cannot render, write the story and let it
fail visibly.** A missing thing that renders as an error is a finding; a missing
thing that is silently absent is a gap nobody sees.

## Stories are the tests, and a `play` is the assertion

**Every story is already a test.** `pnpm test` mounts each one in a browser, so
a story that throws fails without anyone writing anything.

**A `play` function is where a story states what a person should see.** A
handful per surface, on the states the surface exists to express — not one per
story.

**Assert on roles and text, never on class names or internals.** `getByRole`,
the accessible name, the text a person reads. A test that names markup fails on
every refactor and proves nothing about what was drawn; one that reads roles
lets the markup be rebuilt underneath it.

**Arithmetic is not tested here.** A pure function belongs in its own package's
unit tests, where a hundred cases cost what one costs — `packages/screens` is
the worked example. A `play` that computes rather than reads is a unit test
paying a browser's price.

## The hard rules, and what enforces them

- **No arbitrary values.** No raw hex, no off-scale px, no Tailwind `-[…]`.
  Gate rule twelve reads this package and names the file and line.
- **Tailwind's default scales do not exist.** The theme resets every namespace,
  so `bg-slate-800`, `h-16` and Tailwind's own `text-lg` do not resolve.
- **Only glyphs in the registry.** `hourglass` is banned. Never
  `import * as Icons from "lucide-react"` — a rule refuses that too.
- **Status colour is never chosen.** It maps onto the Job state machine.
- **Dark is primary.**

## When the contract does not say

**Report it. Do not invent a token, and do not work around its absence.**

Three agents building primitives in one night each independently reached for
`border-width: thin` because no hairline token existed and the lint refused
`1px`. `thin` is browser-defined rather than 1px, so all three shipped something
subtly wrong rather than one of them saying the token was missing.

| What you found | What to do |
|---|---|
| A value with no token | Name it in your report. Do not decide it in a story |
| A state with no glyph or no token | Write the story, let it fail, report the variant |
| The contract and a built component disagree | Follow the contract, report the disagreement |
| A component no story exists for | Propose it as an issue. Do not invent one on the spot |

**A second vocabulary is a defect, not a shortcut.** A verb, a glyph or a status
token is imported from where it is generated, never retyped. `lib/job-states.js`
drifted three times before it was deleted.

## Verify

From `packages/components`:

- `pnpm test` must pass. **It runs the stories.** Every one is mounted in a real
  headless browser, so a story that throws is a failing test, and a story with a
  `play` function has its assertions run against what it drew. Needs Playwright's
  Chromium shell — `armada run browsers` from the root, once per machine.
- `pnpm exec vitest` watches instead, which is the loop to work in.
- `pnpm build-storybook` must succeed.
- `pnpm exec storybook dev -p 6006 --no-open --ci` to look at it. **Pass `--ci`**
  — without it a port conflict opens an interactive prompt and the command hangs.

From the repository root:

- `cargo xtask verify-foundations`. It reports on the whole repository, so run
  it on `main` too and own the lines your change added — never the colour.

## What a change owes

A component changing is a change to what the app renders, so say what moved and
what it was measured against. Where a change came from a disagreement between
the contract and what was built, **say which won and why** — the next person
will find the same disagreement.
