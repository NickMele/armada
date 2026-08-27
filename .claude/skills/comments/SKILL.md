---
name: comments
description: A comment says why in a line or two. Anything longer is a document that has not been filed. Load before writing a comment block, a module doc, or a header.
---

# Comments

**A comment answers *why*, in a line or two.** The code already says what.

**Anything longer is a document living where nobody looks for documents.** It
cannot be indexed, linked to, or found by somebody who does not already know the
file exists — and it is read by whoever opens that file and nobody else.

## The rule

| Length | What it is |
|---|---|
| **A line or two** | A comment. Why this and not the obvious thing |
| **Three to about fifteen** | Still a comment, if it is one argument. If it is three arguments, it is three comments or it is a document |
| **Longer than that** | **A document.** File it, and leave a pointer |

**A pointer is not a loss.** `See docs/practices/rust.md on the dependency
discipline` is findable, linkable and checked by the gate — which a paragraph in
a header is not.

## When a comment is too long, reduce it first

**Do not assume a long comment is a document.** Most are not — they are a
comment somebody wrote at length, and the same fact fits in two lines. Filing
one as a document moves the verbosity somewhere new and calls it filing.

The order is always:

1. **Reduce it.** Cut restatement, throat-clearing, the narrative of arriving at
   the decision, and anything the code already says. Most blocks lose most of
   their lines here and no facts at all.
2. **Then check what survived.** If it is under three lines, it was a verbose
   comment and now it is a comment.
3. **Only if it cannot get there without losing a fact, a measurement or
   reasoning somebody would otherwise re-derive** — file it, and leave a
   pointer.

**The test is not length, it is loss.** A block that shortens without losing
anything was never a document. A block that cannot shorten is one.

## What earns length, and where it goes

The reasoning in this repository is worth keeping. **The question is never
whether to keep it — it is where.**

| The comment carries | Where it belongs |
|---|---|
| A measurement | A spike under `docs/spikes/`, cited from the code |
| A rejected alternative somebody will propose again | The document that owns the decision |
| A rule that binds more than this file | A contract under `docs/contracts/` |
| How a whole module works | A practice doc, with the module doc pointing at it |
| Why *this line* is not the obvious thing | Right here, in a line or two |

## What to write

- **Say why, not what.** `// increment the counter` above `count += 1` is noise.
  `// counts attempts, not retries — a redispatch starts a new one` is a comment.
- **Name the failure, not the theory.** "Round caps close the two-unit
  clearance" beats "be careful with stroke geometry".
- **A measured fact keeps its number and loses its story.** The number is the
  reason; the story of finding it is the spike's.
- **No date stamps, no attribution, no changelog.** The history is in the
  history.
- **Never restate a rule that lives somewhere else.** Point at it. A copy goes
  stale and nothing checks it.

## Where the length actually comes from

Almost always one of three, and each has a home that is not a header.

**A decision with its alternatives.** A comment explaining what was tried and
rejected is a decision record. It belongs with the decision, and the code gets
the sentence that says which way it went.

**A rule and its reasoning.** If it binds this file only, two lines. If it binds
more than this file, it is a contract and the comment is a copy of one.

**A module explaining itself.** A module doc says what the module is for and
what it refuses. When it starts explaining how the whole area works, the area
needs a practice doc and the module needs a pointer.

## What this repository looks like today

Measured, so nobody has to measure it again: **about three lines in ten are
comments**, and single blocks reach seventy consecutive lines. The largest are
module headers in `armada`, `fleet`, `verification` and `store` — several of
them longer than the code beneath them.

Most of it is not a document. It is reasoning worth keeping, written at three
times the length it needs, and the first pass over any of it is reduction rather
than relocation.
