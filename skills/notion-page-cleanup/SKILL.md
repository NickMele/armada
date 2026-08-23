---
name: notion-page-cleanup
description: Clean up an Armada Notion documentation or concept page — the order, what gets removed, what becomes a database, and how open questions get recorded rather than silently reconciled. Use before editing any Armada Notion page.
---

# Cleaning an Armada page

Load this before starting. The Page Cleanup Procedure in Armada Docs is the
authority; this is the working form of it.

It exists because the same five failures recur: prose restating a decision that
has since moved, changelog entries read as current state, counts that go stale
and then get copied, tables that should be databases, and the same fact stated on
three pages that will drift apart.

## The one rule that matters most

**Never assert what a decision is from memory, from a summary, or from what the
conversation says.** Query Armada Questions and read the `Resolution` field.

Say "I need to check" rather than stating a resolution you have not read.

## The order, and why it is this order

**1. Stale sweep, first.** A stale reference may be attached to a section a later
step would otherwise preserve, so find them before deciding what to keep. Sweep
for archived structure (Ground Zero, Phase 0 through 6, numbered implementation
steps), retired terminology (see `armada-voice`), counts stated in prose, self-
links and broken links, and places the page disagrees with itself two sections
apart.

**2. Verify before fixing.** Split findings into two piles. *Settled elsewhere* —
applying these is enforcement, not a new decision, so fix them. *Genuinely open*
— **never silently reconcile one.** Create a row in Armada Questions with what
needs deciding spelled out, not just the question, relate it to the page, and
link the inline site to that row so the page says "this is open" rather than
stating something false.

**3. Reconcile changelog into decisions.** Sections reading "Resolved Aug 2026",
"Amended", "Corrected" should read as the current decision, stated once. Keep
reasoning a reader would otherwise re-litigate. Drop the narrative of how it was
arrived at unless the alternative is one someone will propose again. Delete
completed "pages this contradicts" tables outright.

**4. Cut fluff, keep information.** The test is not length — it is whether
removing the sentence loses a fact. Never lose a measurement, a number, or
reasoning that would have to be re-derived.

**5. Extract.** Prefer an inline database to a static table. A row opens into a
page, other pages can link to a single row, properties filter and roll up, and
views travel. **The test:** does something outside this page want to point at an
individual row? Leave it as prose when nothing links to it and it has a code
counterpart — a Notion database mirroring code becomes a second source of truth
that drifts the moment a migration lands.

**6. Deduplicate across pages.** Fetch every page this one links to and read it.
This is the step most likely to be skipped and the one that pays most. The linked
page almost always holds strictly more, so the copy can only drift downward.

**7. Table of contents, last.** Use the native `<table_of_contents/>` block.

## Standing rules

- **Propose, wait for an explicit yes, then write.** Never write to Notion
  without one.
- **One decision at a time.** Numbered options, a recommendation, a one-line
  reason.
- **Fetch before writing** to verify page identity. Pulling an id from a query
  result without re-checking the title is how resolutions get misfiled.
- **Re-fetch after writing.** A silent no-op on a whitespace mismatch is the
  known failure mode of a content update.
- **Never state a count in prose.** Carry the rule that generates the list.
- **Concept pages own decisions.** Step bodies link rather than restate.
- **Record disagreements explicitly** rather than reconciling them.

## Mechanics worth knowing

- `notion-fetch` with a bare UUID is reliable; copy-link URLs with slugs
  sometimes need stripping.
- Creating a page in a database needs `parent: {"data_source_id": "..."}`.
  Omitting it creates an orphan.
- Relation properties accept bare page IDs as an array.
- Databases cannot be created inside a toggle block.
- Additive schema changes only: add new select options, retag rows, then drop the
  old ones.
- A table rendered as a Notion table block will not match a markdown string, so
  `update_content` cannot edit one row of it. Replace the content or rewrite the
  whole block.

## Queue order

Clean the high-degree nodes first — the pages many others link to, where drift
costs most. A page nothing links to can wait.
