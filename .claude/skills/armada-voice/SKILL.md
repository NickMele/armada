---
name: armada-voice
description: How Armada writes — the lexicon, the prose rules and the status grammar. Use before writing any user-facing string, error, status message, commit message or planning page. The contract itself is in Notion; this points at it.
---

# How Armada writes

**The Design System owns this and it is in Notion, not here.** Read it before
writing product copy, a status message, an error, a commit message or a planning
page — its rules govern internal documentation as well as the product, because
the docs are read constantly and their conventions leak into the product.

Root page, which lists the contracts: https://app.notion.com/p/3c0173a35eb9800a9da2e6b7f1403ab1

## Why this skill is a pointer and not a copy

The lexicon, the retired terms, the six voice principles and the status grammar
all change. A copy of them in this repo is a second source that drifts silently
from the first, and the two would disagree exactly when someone is relying on
one of them. **Every fact has one owner.**

What this skill is for is the trigger: it fires when you are about to write
something a person reads, so that you go and read the contract instead of
writing from memory of it.

## The one thing worth carrying, because it is about this repo

**Never use `§`.** Write "M0 step 4". It is banned along with `¶`, `cf.`,
`ibid.`, `op. cit.`, `viz.` and `q.v.` — `e.g.` and `i.e.` are fine. This is
here because it is the rule most often broken while writing code comments and
commit messages, where nobody thinks to check a contract.

**A bare file path refers to v1**, at tag `v1-final`. v1 is deleted from the
working tree, so cite it as `git show v1-final:<path>`.
