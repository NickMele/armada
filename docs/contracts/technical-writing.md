# Technical writing

**Kind:** contract. **Governs:** the shape of every document under `docs/`.
The Design System owns the lexicon and the voice; this owns the structure —
what a page may contain, how a rule is phrased, and when prose becomes a table.

Read before writing or editing any document here.

---

## Structure

**One mode per page: reference (what it is), explanation (why), or procedure
(how). Never mix.** If a section drifts modes, split it.

**Every normative claim uses this shape and nothing else:**

> **Rule.** One declarative sentence, present tense, no hedging.
> Why: one sentence, optional — or a link to the decision.

**No paragraph exceeds three sentences.** No section exceeds 150 words of
prose; anything longer becomes a table.

## Banned

- **Rejected alternatives and decision history.** Also "this refines…", "note
  that…". These live with the decision. Link, do not restate.
- **Bold lead-in paragraphs used as a de facto list.** If it is a list, make it
  one.
- **Adversative essay moves.** "That is what keeps…", "Nothing else would have
  caught it", "and that is accepted".

## Tables

**Cells are fragments, not sentences, and at most fifteen words.** No links
inside a cell — references go in a trailing Notes column, or below the table.

**If a cell needs more, the row is a page.**

## Field and schema sections

**Table only:**

| field | type | required | default | meaning |
|---|---|---|---|---|

Meaning is at most twelve words. No commentary between rows.

## Default output

**Lead with the shortest correct statement.** Detail is a link, not a clause.

## What this binds

**New writing, and any document being edited for another reason.** A document
nobody is touching is left as it is until somebody touches it.

The concept pages are reduced to this contract as they land. Everything else —
`ARCHITECTURE.md`, the contracts carried out of the design workspace, the
practices, the journeys — is reduced when it is next edited, not in a sweep. A
reduction pass over a document nobody is reading spends attention and risks the
loss it is meant to prevent.

**Prose in a data file is governed too.** A `notes` key in
`crates/core-model/domain/`, `crates/config/settings.toml` or
`packages/icons/icons.toml` is a document in a field, and the same rules apply
to it — one mode, no decision history, no date stamps, a table when it grows.

**`docs/instructions/` is not exempt.** Those files are pasted into the desktop
app and the design project, so reducing them means re-pasting both. They are
documents here and the contract governs them.
