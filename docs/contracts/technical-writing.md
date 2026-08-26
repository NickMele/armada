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

## Open questions

- **[writing-rules-retrofit]** Do the documents written before this contract
  get reduced to it, or does it bind new writing only?
  What decides it: several documents already here break these rules —
  `ARCHITECTURE.md` uses adversative moves, the contracts carried out of the
  design workspace carry rejected alternatives inline, and `SECURITY.md` leads
  with explanation rather than the shortest correct statement. A contract
  nothing already satisfies is either a plan or a dead letter, and which one it
  is has not been decided.
