---
name: armada-documents
description: How Armada's written record is organised and how a document is written or cleaned. Load before adding a document, before editing one, and before deciding where a fact belongs.
---

# Armada's written record

Everything an agent reads while writing code is a file in this repository. The
roadmap is GitHub issues.

| Holds | Home |
|---|---|
| Contracts, practices, concepts, journeys, spikes, open questions | `docs/` |
| Registries code reads — tokens, icons, settings, the domain model | `packages/`, `crates/` |
| What is being built and in what order | GitHub issues and milestones |
| A component as built, and how it is agreed | Storybook, `packages/components` |

**The test when it is not obvious:** does an agent need this while writing code?
If yes it is a file here, because a subagent cannot follow a link out.

## The four kinds, and what each owns

| Kind | Owns | Never restates |
|---|---|---|
| Contract | A binding rule, and the reasoning behind it | Another contract's rule |
| Concept | What the thing is, and the decisions governing it | Another concept's decisions |
| Practice | How to work inside one area of the code | A rule a contract owns |
| Journey | How a thing is used or shown | The rule it is applying |

**Restatement between concept documents is where drift starts.** Nothing checks
a copy, and a copy can only go stale.

`docs/contracts/technical-writing.md` governs the shape of every one of them —
one mode per page, three sentences per paragraph, a table past 150 words. Read
it before writing.

## Adding a document

1. **It goes in `docs/`, in the subdirectory matching its kind.**
2. **Add it to `docs/INDEX.md`.** Gate rule fifteen refuses a document that is
   not indexed, and an index entry naming a document that does not exist.
3. **Name paths exactly.** Gate rule eighteen refuses a repository path that
   resolves in neither this tree nor `v1-final`. A pointer to a directory where
   you meant a file passes the rule and fails the reader — write
   `crates/core-model/domain/job-statuses.toml`, not `crates/core-model/`.
4. **Never write an address into the design workspace.** This repository is
   public and that workspace is not; a link into it publishes an address to
   something nobody outside can open. Gate rule sixteen enforces it.

## Cleaning one

The same five failures recur: prose restating a decision that has since moved,
changelog entries read as current state, counts that go stale and then get
copied, tables that should be a data file, and one fact stated on three pages
that will drift apart.

### 1. Stale sweep, first

A stale reference may hang off a section a later step would otherwise preserve.

- **Archived structure.** M0 — Foundations replaced Ground Zero. A reference to
  a numbered phase becomes the milestone that owns the capability the sentence
  is about, not "Milestone N".
- **Retired terminology.** The lexicon is `docs/contracts/design-system.md`.
  Guild split into Kit and Machine — a split, not a rename, so each site needs
  judgment about which one it became.
- **Counts stated in prose.** Every one goes stale and is then copied onward.
  Carry the rule that generates the list.
- **Self-links, dead paths, and internal contradictions** two sections apart.

### 2. Verify before fixing

**Settled elsewhere** — applying it is enforcement, not a new decision. Fix it.

**Genuinely open** — never silently reconcile it. File it under
`armada-open-questions`, and link the inline site to it, so the document says
this is open rather than stating something false.

### 3. Reconcile changelog into the decision

A section reading "Resolved", "Amended" or "Corrected" becomes the current
decision, stated once.

- Keep reasoning a reader would otherwise re-derive. Drop the narrative of
  arriving at it.
- Delete a completed "pages this contradicts" table outright.
- Where a date-stamped note marks a real distinction, keep the distinction and
  drop the date.

### 4. Cut fluff, keep information

**The test is not length — it is whether removing the sentence loses a fact.**
Never lose a measurement, a number, or reasoning that would have to be
re-derived.

**Convert faithfully, then reduce as a separate pass.** A document once lost a
quarter of itself during a conversion and the loss was invisible for hours,
because one diff was doing two jobs.

### 5. Extract what code reads

**A set code reads is a data file with a check over it** — `packages/tokens`,
`packages/icons`, `crates/config/settings.toml`, `crates/core-model/domain/`.
A set nobody's code reads stays a table in the document.

**Prose inside a data file is still a document.** A `notes` key is governed by
the writing contract exactly as a paragraph is.

### 6. Deduplicate across documents

**The step most likely to be skipped and the one that pays most.** Read every
document this one links to.

- Where this one restates what the linked one owns, delete the copy and leave a
  pointer. The linked document almost always holds strictly more.
- Check both directions.
- Where a fact belongs to neither, the document owning the *rule* keeps it.

## Standing rules

- **A document states what is true.** Never that a decision was made, when, or
  by whom.
- **Never state a count in prose.**
- **Prefer a table, a list or a worked example.** Prose is where a fact hides.
- **Record disagreements explicitly** rather than reconciling them.
- **Clean the high-degree documents first** — the ones many others link to,
  where drift costs most. One nothing links to can wait.
