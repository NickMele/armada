---
name: write-design
description: Turn what exploration found into a design document under docs/design/ — the decision, the alternatives, and what each costs. Use for the `articulate` step of the `design` workflow.
---

# Write the design

The step before you explored. You write down what should be built and why, and a person reads it
and decides. **The `design` workflow ends at a person on purpose** — no command can tell you an
approach is right, so this document is the whole output.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/write-design/SKILL.md` changes it.

## Your gate is `artifact_exists: docs/design/*.md`

Any markdown file under `docs/design/` advances the step, so **the gate proves you wrote
something and nothing more.** The reader is the judgement. Name the file after the decision —
`docs/design/how-jobs-carry-local-config.md`, not `docs/design/notes.md` — and **commit it**.

## A design is a decision, not a survey

Open with the thing being decided, in one sentence, and then the answer. A document that walks
through five options and stops has moved the work nowhere: the reader has to do the deciding
*and* read five options first.

Then, for the answer you picked:

- **What it costs.** Every real design costs something. A design with no cost section is one
  where the cost was not found yet.
- **What it rules out.** The alternatives, briefly, and the specific reason each loses. *"Slower"*
  is not a reason; *"it needs a daemon, and `ARCHITECTURE.md` says nothing runs between
  commands"* is.
- **What would make this wrong.** The assumption that, if false, sinks it. Name it so a reader
  can check that one thing rather than re-derive the whole argument.

## Quote the constraint you are working inside

A design that contradicts a decision already recorded is a design that gets re-argued. Find the
constraint and cite it by name — the architecture document, an earlier design, the sentence in a
module's own comment that says why it is the way it is.

**If your design breaks one of those, say so explicitly and argue it.** That is allowed and
sometimes right. What is not allowed is breaking it silently, because then the disagreement is
discovered by whoever hits it next.

## Write it so it survives you

This document is read weeks later by somebody who was not in the exploration. Assume no shared
context: name files by path, quote the code you are describing rather than paraphrasing it, and
spell out the failure that motivated the work.

**If a measurement drove the decision, put the measurement in.** *"It took 182 seconds"* is
checkable; *"it was slow"* is an opinion the reader has to take on trust.

## Then stop

You are not implementing this. The workflow ends at a person, and the next thing that happens is
somebody reading what you wrote and deciding. A design step that starts writing the code has
spent the budget on work nobody approved.
