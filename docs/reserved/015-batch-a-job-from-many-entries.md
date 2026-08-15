---
id: 015
title: One Job from many entries
status: RESERVED
module: helm
raised: real use, 2026-08-15
---

# One Job from many entries

**The ask.** Select several entries in `armada failures` — a `b` keystroke, multi-select — and
dispatch them as **one** Job rather than one Job each.

**Where it came from is the argument for it.** The orchestrator running this build was
dispatching one subagent per reported bug, within a minute of each report. Four bugs became four
agents, each re-reading the same repository from scratch, and roughly twenty agents cost about
3.8M tokens in a single session. Batching related fixes into one agent was the single largest
saving available. **The product should make the same move cheap for the user that it made for
the orchestrator**, rather than leaving it as a discipline somebody has to remember.

## Why this is small

Every piece already exists.

| Piece | Where |
|---|---|
| Multi-select with a mandatory preview | `armada fleet reap` — rows toggle, enter confirms, esc cancels and touches nothing |
| Promotion of one entry into a Job | `armada failures fix` |
| An entry with an id that can be acted on one at a time | [`001`](001-raised-items-need-identity.md), [`010`](010-armada-records-its-own-failures.md) |
| The interactive listing | `armada failures` at a TTY, on the shared `ask/select.rs` |

So the work is a selection mode over entries that already toggle, and a `fix` that accepts a set
rather than one id. **No new mechanism, and specifically no second selector** — `select.rs`
exists because a one-off was rejected once already.

## The design questions this leaves open

- **What makes a batch coherent.** Four unrelated bugs in one Job is one agent thrashing across
  four contexts, which is the failure mode batching is supposed to fix, arriving from the other
  direction. Entries sharing a `where`, a module, or a root cause are a batch; entries sharing
  only a timestamp are not. Whether Armada should *suggest* the grouping or only accept one is
  the open question, and suggesting it means classification, which costs a model call.
- **What the Job's task text becomes.** One entry's message is a task. Five need synthesising
  into something an agent can act on, and doing that well is itself a model call — which may
  make it the orchestrator's job rather than the CLI's.
- **What happens to the entries when the Job ends.** All closed, or each closed only if the Job's
  verdict names it? A batch that half-succeeds and closes everything is worse than no batching,
  because the survivors become invisible.
- **Whether this belongs to `failures` alone.** Reports ([`armada report`](010-armada-records-its-own-failures.md)),
  tasks ([`002`](002-tasks.md)) and raised items ([`001`](001-raised-items-need-identity.md)) are
  the same record from different directions. If batching is a property of *entries*, it should
  work across all of them, and building it only into `failures` would be the third parallel list
  `001` argues against.

**Not scheduled.** It is downstream of `armada report` and of `001`'s identity scheme settling,
and the grouping question above wants a real backlog to test against — the user's log currently
holds a dozen entries, most of which were a broken shell loop rather than defects.
