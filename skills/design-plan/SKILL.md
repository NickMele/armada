---
name: design-plan
description: Produce a design or plan document through draft-and-feedback iterations, the way Armada's Design Plan workflow will run it. Use when asked to design something, write a plan, or work up an approach before building it.
---

# Design Plan

**Two steps, and it loops.** The only instantiated loop in the workflow set —
`draft → present → (approve | request changes | reject)`, with `request changes`
routing back to `draft`.

| Step | Evidence | Mechanical | Judge | Advances on |
|---|---|---|---|---|
| `draft` | document | plan document exists | none | Automatic |
| `present` | document | — | none | **A human, always.** `request_changes` loops to `draft`. Iteration cap 5 |

**No Check tier at all.** The evidence is a document, so every pass is the
owner's to judge. There is nothing an exit code could say about it.

## `request_changes` and `reject` are not two strengths of the same verdict

This is the distinction that matters most here, and a design loop is exactly
where the temptation to conflate them is strongest — a third unsatisfying draft
*feels* like a rejection and is not one.

| Verdict | Means | Effect |
|---|---|---|
| `approve` | Good enough to act on | The Job advances |
| `request_changes` | "Try again, here is what to fix" | Loops back to `draft`. **Never terminates the Job** |
| `reject` | "This whole approach is wrong" | Ends the Job |

## What carries across an iteration

**Only the latest draft, and all of the feedback.** The drafts are large and the
verdicts are a sentence or two, so appending every prior draft breaches the
context cap by construction while appending every verdict costs almost nothing.

**Keeping all the verdicts is the point** — it is what shows the same note went
unaddressed three times, which is the judgement the iteration cap exists to
force. Carrying only the latest draft entire would make each round read as a
fresh note.

Rejected: a rolling summary, which adds a model call per iteration and puts a
lossy step inside the evidence path.

## Running it by hand

Write the document to a file. `docs/design/` for a design, `PLAN.md` for a plan
of work. The document is the evidence — a conversation in which you described the
design is not.

Reaching the iteration cap escalates the way a stuck gate does. It is **not** a
retry limit; nothing failed. Five unsatisfying drafts is a signal that the brief
is wrong, not that the drafter is.
