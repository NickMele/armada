---
id: 017
title: What you have not tried yet
status: BUILT
module: cross-cutting
raised: in use, the day fifteen features landed
---

# 017 — What you have not tried yet

> **BUILT** — `armada coverage`, counted on every run in `~/.armada/coverage.jsonl`.

**The complaint this exists to fix.** *"I need a task list of what CLI needs me
to test. (I wish we could observe this as I use it.)"* Fifteen features landed in
a day, most of them exercised once or not at all, and nothing on the machine
could say which. Answering it meant reading `~/.armada/recent.jsonl` by hand —
and that buffer keeps ten runs (`armada_core::recent::KEEP`), so it can say what
you *just* did and can never say what you have **never** done.

**So it is observed as you use it**, which is the parenthesis in the ask. Every
run tallies itself at the end of the entrypoint, on the same path the ring buffer
is written from and under the same rule: silent, and never able to change what
the run answered.

#### It is the fourth direction on one question

| | What it holds | Whose |
|---|---|---|
| [`010`](010-armada-records-its-own-failures.md) `armada failures` | what Armada noticed went wrong | Armada's |
| [`014`](014-report-what-you-know-went-wrong.md) `armada report` | what you noticed went wrong | yours |
| [`002`](002-tasks.md) `armada task` | what you intend to do | yours |
| **this** `armada coverage` | what you have not got to yet | the machine's |

#### Why it is a counter and not a fourth list of raised items

[`001`](001-raised-items-need-identity.md) argues against a parallel list of
things needing attention, and the first three above obey it by sharing one store,
one id space and one promotion path. **This one is deliberately not in that
store**, and the reason is that a verb you have not run is not a raised item:

- **Its identity is its own name.** There is nothing to fingerprint and no id to
  act on one at a time, which is the whole of what `001` says an item needs.
- **There is nothing to acknowledge.** A failure can be *done*, *not doing it* or
  *not yet*; an untried verb has exactly one state and it ends the moment you
  type the verb.
- **The row deletes itself.** Nothing else in the store does, because nothing
  else in the store is derived — a failure and a task are records of an event,
  and this is a diff against a roster.

Giving it ids and states would have been *inventing* the parallel list rather
than avoiding one. It is machine state, kept beside `recent.jsonl` for the same
reason and on the same side of `PLAN.md` §13.1's line: what describes you syncs,
what describes this machine does not.

**What it does offer is the crossing.** At a terminal, picking a row writes
`armada task "try armada <verb>"` — the store that already has ids. It is offered
and never automatic, on the ask's own instruction: *a backlog that fills itself
is one nobody trusts.*

#### The roster is derived, never retyped

`armada_helm::args::every_verb()` builds it from the constants the parser and the
help pages already read, and a test already fails when a verb ships without a
page. A second list here is the mistake [`009`](009-smaller-things-raised-in-use.md)
item 5 records — two lists, one of them the one nobody edits.

**Sub-verbs count against their parent.** `armada tasks start` is not a page
`--help` reaches, so it counts as `tasks`; a row that could never be satisfied is
a row that lies. That is the one thing this deliberately cannot see: it reports
coverage of the pages, not of every flag on them.

#### Not telemetry

`~/.armada/coverage.jsonl` never leaves the machine and never syncs — only
`guild/` does. It holds a verb name **off Armada's own roster** and three
numbers, so nothing a person typed can reach it: a repository's declared command
and a typo alike are counted as nothing at all, which is why it needs no
redaction of its own.
