---
name: armada-voice
description: How Armada writes — the lexicon, the prose rules and the status grammar. Use before writing any user-facing string, error, status message, commit message or planning page. The contract is docs/contracts/design-system.md; this says when to read it.
---

# How Armada writes

**The Design System owns this, at `docs/contracts/design-system.md`.** Read it
before writing product copy, a status message, an error, a commit message or a
planning document — its rules govern internal documentation as well as the
product, because the docs are read constantly and their conventions leak into
the product.

## Why this skill is a trigger and not a copy

The lexicon, the retired terms, the voice principles and the status grammar all
change. A copy of them here is a second source that drifts silently from the
first, and the two would disagree exactly when someone is relying on one of
them. **Every fact has one owner.**

What this skill is for is the trigger: it fires when you are about to write
something a person reads, so that you open the contract instead of writing from
memory of it. That mattered more when the contract was somewhere you could not
grep. Now that it is a file, the trigger is still the point — reading it is
cheap, and writing from memory is what produced the drift.

## The one thing worth carrying, because it is about this repo

**Never use `§`.** Write "M0 step 4". It is banned along with `¶`, `cf.`,
`ibid.`, `op. cit.`, `viz.` and `q.v.` — `e.g.` and `i.e.` are fine. This is
here because it is the rule most often broken while writing code comments and
commit messages, where nobody thinks to check a contract.

**A bare file path refers to v1**, at tag `v1-final`. v1 is deleted from the
working tree, so cite it as `git show v1-final:<path>`.
