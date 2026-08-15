---
id: 005
title: The inbox records a label, not an identity
status: BUG
module: fleet
raised: real use, 2026-08-15
---

# 005 — The inbox records a label, not an identity

**Found in ordinary use, 2026-08-15.** `armada fleet ls` reported `no Jobs` while `armada fleet
inbox` reported five open entries, all naming `this-test`. Both are telling the truth, and the
gap between them is the defect.

**What is actually wrong.** Jobs are keyed by uuid; **every inbox entry is keyed by the Job's
name**. A name is a label and labels are not unique — there were two Jobs called `this-test` —
so an inbox entry cannot be resolved to the Job that raised it. `armada fleet show this-test`
correctly refused as ambiguous while `armada fleet show c19d0a34` worked, which is the same
fact seen from the other side.

Two consequences, and the second is worse:

1. **Entries outlive their Jobs.** Both Jobs reached `ABORTED`, a terminal state that `ls` does
   not re-observe. The five entries remained "open" against Jobs that no longer exist.
2. **The footer offers an action that cannot work.** `armada fleet answer <job> "…"` on an
   entry whose Job is terminal has nothing to answer, and the reader is told to try anyway.

**This is `001-raised-items-need-identity.md` arriving as a bug rather than a design note.**
That section reserved the problem that *"items Helm raises in prose have no identity"* — and
the inbox turns out to have the same defect one level down: it raises items against a **name**,
which is prose with a narrower grammar. The fix is that an inbox entry carries the Job's
**uuid** and is closed when its Job reaches a terminal state.

**And `ls` should show the uuid.** The user's own conclusion: *"having legible IDs is really
nice, but maybe when we do ls, we should also see the real ID."* A short prefix is enough — the
ambiguity error already prints eight characters and they are what he successfully typed.
