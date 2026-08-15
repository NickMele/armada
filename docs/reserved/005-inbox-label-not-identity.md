---
id: 005
title: The inbox records a label, not an identity
status: FIXED
module: fleet
raised: real use, 2026-08-15
---

# 005 — The inbox records a label, not an identity

> **Fixed.** An inbox entry carries the Job's uuid in `job_uuid` and every verb resolves on
> that; the name stays beside it as a label that is shown and never resolved against. An entry
> is closed when its Job reaches `DONE` or `ABORTED` — **marked and retained, not deleted**,
> because the reason a Job stopped is often the last thing written about it and `inbox --all`
> and `show` both read entries back. `fleet ls` draws an `ID` column. Entries already on disk
> are migrated on the first read that sees one: a name meaning exactly one Job is bound to it,
> and a name meaning none or several is closed `UNRESOLVABLE` rather than guessed at. What
> follows is the diagnosis, kept because the reasoning is the part a later change is most
> likely to get wrong.

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

## What was built, and the two decisions inside it

**A closed entry is kept and marked rather than deleted.** The alternative was tempting — a
`DONE` Job's questions are noise in a list of things to do — and it is wrong for the same
reason the file is append-only in the first place: the inbox is the log of why Jobs stopped,
and *"reached its wall clock ceiling on the explore step"* is frequently the only written
record of an ABORTED Job's last minute. Deleting it makes that disappear at the moment it
becomes history. So `is_open()` is false, `armada fleet inbox` omits it, `--all` still prints
it, and the footer's `armada fleet answer` line is printed only when something is actually
open — an action that can only fail is worse than no action, which is this section's second
consequence stated as behaviour.

**An unresolvable legacy entry is closed, not guessed at and not left open.** Three answers
were available for an entry whose name means two Jobs. *Guess* — newest, or the live one — is
the coin flip `Store::find` already refuses to take, with better odds and the same failure
mode. *Leave it open* reproduces the defect exactly: a row that cannot name its Job is a row
nothing can act on, and a list containing one is a list that stops being trusted. So it is
closed `UNRESOLVABLE` and kept readable. On the machine this was found on that is the right
answer twice over: both Jobs called `this-test` were `ABORTED`, so every one of the entries
would have been closed `ENDED` a moment later anyway.

**The migration runs on the first read that finds a legacy entry**, rather than as a command.
A fix that needs a command run first is a fix most machines never get, and the whole complaint
was about a machine whose state had silently stopped making sense. It is append-only and
idempotent, so it converges after one read and two readers racing on it write the same lines.

## Its relationship to 001

This is deliberately not a second identity scheme.
[`001-raised-items-need-identity.md`](001-raised-items-need-identity.md) is the general form —
*items Helm raises in prose have no identity* — and what it will eventually generalise is this:
an item carries the uuid of the thing that raised it, acknowledgement is recorded against the
item's own id, and an item whose subject has ended stops being open without being destroyed.
001's three open questions are untouched by this; none of them is about what an item points at.
