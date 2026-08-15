---
id: 001
title: Raised items need identity
status: RESERVED
module: helm
raised: design pass, writing Helm's inbox mechanism
---

# 001 — Raised items need identity

**The complaint this exists to fix.** An agent hands you a table of things to deal with and the
table just sits there. Acknowledging one means typing a sentence — *"I did the second one"* —
and having a small conversation about it. The cost is not the typing; it is that a list of
things you must act on is indistinguishable from a list of things you have already acted on, so
the list stops being trustworthy after the first item.

**The diagnosis: items Helm raises in prose have no identity.** The inbox already has the whole
mechanism for Jobs — `fleet.ask_human` raises an entry, `armada fleet answer <job> "…"` responds
to it, and the Bridge binds `a` to exactly that. What has no id is the *thing inside a sentence*.
"Three things need you" is prose, and prose cannot be acknowledged one row at a time.

So the shape of the answer is not a UI feature. It is that **every item Helm surfaces is an
inbox entry with an id, whether it renders in the Bridge, in a table, or mid-sentence** — and
acknowledgement is one keystroke against that id, not a reply. The Bridge is then a renderer
over the same entries it already renders, and Helm's prose becomes a second view of them rather
than a separate channel.

**Design questions this leaves open**, and they are the reason it is reserved rather than
specified:

- **What acknowledgement means.** *Done*, *not doing it*, and *not yet* are three different
  answers, and collapsing them to a tick loses the one that matters — a dismissed item and a
  deferred item behave differently the next time Helm reports.
- **Whether Helm needs telling.** If you mark a thing done, does the Drone that raised it resume
  on that fact, or is the acknowledgement purely yours? Both are defensible and they are
  different products.
- **Where the keystroke lives.** Inside the Helm session, in the Bridge, or in a notification —
  and a session that is a plain Claude Code conversation has nowhere obvious to put one.

**Not scheduled.** It wants its own design pass, and it is downstream of the inbox and the
Bridge both existing.
