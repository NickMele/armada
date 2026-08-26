---
name: armada-open-questions
description: How an Armada open question is filed, cited from code, and answered. Load before filing one, before answering one, and before editing a document that carries one.
---

# Open questions

A question is a decision somebody deferred. It lives in the document that
blocks on it, carries a slug so code can cite it, and is collected into
`docs/OPEN.md` by `cargo xtask verify-docs`.

**The rule that outranks every other one here:** never assert what a decision
is from memory, from a summary, or from what the conversation says. Read the
record. A summary collapses a suggestion and a reaction into one phrase, and a
confidently wrong claim about a settled decision costs more than the check
saves. Say "I need to look" instead.

## Where a question lives

**In the `## Open questions` section at the foot of the document that blocks on
it.** Not in a central list — a central list names a page, never the sentence,
and that gap is where drift lives: the decision lands somewhere on the right
page while the stale sentence three sections down survives.

**A question has one home.** Where it bears on a second document, that document
names it in a sentence rather than repeating the bullet. Two copies of one
question are two identities for it, and the walk will collect both.

## The shape

```markdown
## Open questions

- **[slug-in-kebab-case]** One sentence saying what must be decided. Then what
  someone needs to know to decide it — facts, constraints, measurements,
  positions already taken — so it can be answered without re-deriving them.
```

**The slug is the citation handle.** Code that takes a shortcut because a
question is open names the slug in a comment, and `cargo xtask verify-docs`
fails when a slug is cited that no question asks. That is what makes answering
a question surface everything that was waiting on it.

**Write down what decides it at filing time**, before anyone knows the answer.
That is the point. Written at closing time, it is written by somebody with an
interest in the list being short.

## The bar for filing

**All three must hold.**

- It came up in the course of doing something else, and cannot be settled now.
- It will change what gets built or written. Something is waiting on it.
- **A person deferred it.** An agent may propose a question; it may not file one
  on its own judgement.

**Do not file when:**

| Looks like a question | Actually |
|---|---|
| Settled this session, page not yet updated | Write the page |
| You are unsure and could find out by reading | Research |
| A thing to do rather than a call to make | A GitHub issue |
| Nobody is blocked and nobody asked | Nothing |

**Filing is not free.** Every question is something a person must later read,
judge and close, and enough of them stop the queue being readable at all —
which is the failure it exists to prevent. Two were once filed in one session
that were already answered in documents that had been read, one of them written
that same day.

**Answer it before proposing it.** Read the concept page, the contract that
governs the subject, and `docs/OPEN.md`. A subagent asking a question is not a
finding: it saw one file and no context.

## Answering one

1. The decision is made by a person.
2. **Write the fact into every document it changes.** State what is true — never
   that a decision was made, when, or by whom. No date stamps in body text.
3. **Delete the bullet.** That breaks every citation of its slug, and the gate
   then names each thing that was waiting on it. Go and resolve them.
4. Run `cargo xtask verify-docs`. A stale `docs/OPEN.md` fails the gate.

**Do the write-back in the same sitting as the decision.** Deferred, it rots.
That is how documents arrive at contradicting settled decisions.

**Answer a question partly by narrowing its text**, not by deleting it. A
question with half an answer is still open, and the half that is settled belongs
in the document as a fact.

## When a decision overturns an earlier one

The document states the current decision, once. It does not carry the history
of arriving at it.

- **Keep reasoning a reader would otherwise re-litigate.** Drop the narrative.
- **Keep a rejected alternative only where somebody will propose it again**, and
  say why it was rejected in one sentence.
- **Where a page now contradicts another page, that is the finding.** Record the
  disagreement explicitly rather than reconciling it on your own judgement — one
  such contradiction sat across two concept documents about a settled decision
  and was only visible once both were files.
