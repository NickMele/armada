---
id: 001
title: Raised items need identity
status: BUILT
module: helm
raised: design pass, writing Helm's inbox mechanism
---

# 001 — Raised items need identity

> **Built, in the half that matters: there is one id space.** Every item Helm surfaces — a
> recorded failure, a filed report, a written task and now an inbox entry — has an
> eight-character id, resolves through one `resolve`, and answers to one `show`. `armada fleet
> inbox` draws an `ID` column and `armada fleet answer <id> "…"` acknowledges the row rather
> than the Job that owns it. The two questions this left open are answered below and in
> [`armada_core::failure::Origin::Raised`]'s doc comment, which is where a reader who is about
> to change this will be standing.
>
> **The acknowledgement vocabulary is deferred**, deliberately and with a reason — see
> *"What is not built"*. What follows is the diagnosis, kept because the reasoning is the part a
> later change is most likely to get wrong.

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

## What had already arrived from elsewhere

By the time this was built, most of it existed — from four directions, none of them this
document:

| Source | Verb | Reserved item |
|---|---|---|
| Armada's own failures | `armada failures` | [`010`](010-armada-records-its-own-failures.md) |
| What the user noticed | `armada report` | [`014`](014-report-what-you-know-went-wrong.md) |
| What the user intends | `armada task` / `tasks` | [`002`](002-tasks.md) |
| What he has never run | `armada untried` | [`017`](017-what-you-have-not-tried-yet.md) |

Failures, reports and tasks already shared one store, one id space, one `show`, one interactive
listing and one promotion into a Job, told apart by an `Origin` column.
[`005`](005-inbox-label-not-identity.md) fixed the inbox to carry a Job's uuid rather than its
name — the same identity problem one level down. **So this was never a design pass. It was a
unification, and the job was to finish it without building a fifth store.**

## What was built

**One id space over four origins.** `Origin` gained a fourth variant, `Raised`, and an inbox
entry projects into the entry store's shape at read time — `armada failures show <an inbox id>`
answers, and an ambiguous prefix is refused across every item on the machine rather than within
one file.

**The inbox is not moved into `failures.jsonl`, and that is the decision rather than a
shortcut.** Helm's Stop hook greps `~/.armada/inbox.jsonl` for unread entries and its monitor
runs `tail -F` on that exact path (`PLAN.md` §15.3). Merging the stores would break the
mechanism that makes a raised item reach anybody at all, in exchange for tidier bytes. **So the
writing stays where it is and the reading is unified** — one `read`, one `resolve`, one `show`.
One id space is the claim `001` makes; one file is not.

**There is no fourth listing.** A raised item resolves and shows, and appears in neither `armada
failures` nor `armada tasks`, because it already has a listing: `armada fleet inbox`. Drawing
one row in two tables is how two tables start disagreeing about it.

**`armada fleet inbox` draws an `ID` column, and `armada fleet answer` takes one.** This was the
complaint printed: an entry has carried its own `uuid` in `--json` since it was first written
and the table never drew it, so the only way to name a row was *"the second one"*. `answer`
resolves an open entry's id first and a Job second — naming the Job answers whichever of its
questions is oldest, which is right for a Job that asked once and a guess for one that asked
twice.

**Two verbs refuse a raised item rather than doing something plausible.** `fix` / `start` would
spawn a second Job for a question a Drone is already stopped in front of; `clear` would append a
line to a file that has never held that id. Both refuse and name `armada fleet answer`, because
clearing a row and unblocking an agent are different acts and doing the second when the first
was asked for is worse than saying no.

**And one bug the work found.** `armada fleet inbox` chose its colour by matching the kind
against `"blocked"` while the field holds `BLOCKED`, so the arm had never matched and the one
kind that cannot proceed without you painted the same orange as one that merely asked a
question. The table had no golden fixture at all until now.

## The two open questions, answered

**Whether Helm needs telling — both, and the origin decides which.** An observed failure, a
filed report and a written task have nobody waiting on them: nothing is blocked, so
acknowledgement is purely yours. A raised item has a Drone stopped in front of it holding a
worktree, a session and a budget — **acknowledging it and resuming it are the same act**, which
is what `armada fleet answer` has always done. It looks like a coin flip only while *item* is
one word for four things. This is also why a raised item's acknowledgement carries a body and
the other three do not: you cannot resume an agent with a tick.

**Where the keystroke lives — not in the Helm session.** Helm renders ids; the acknowledging
happens where there is already a keystroke: `armada fleet inbox`'s table, `armada failures`' and
`armada tasks`' selector, and the Bridge when it is built
([`003`](003-bridge-command-centre.md)). A Helm session is a plain Claude Code conversation;
Armada owns no terminal inside it and binds no keys, and building one there would be building
the Bridge there. **What text can do is print an id, and that is the whole complaint.** *"I did
the second one"* is a sentence because *the second one* was the only handle the reader was
given; `armada fleet answer 4f2a "…"` is a line you can copy. The id is the interface between
Helm's prose and every surface that does have a keystroke.

Both answers live in [`armada_core::failure::Origin::Raised`]'s doc comment as well as here,
because the next person to change this will be reading the type and not this file.

## What is not built

**The acknowledgement vocabulary.** This section's first open question was *what acknowledgement
means*:

> *Done*, *not doing it*, and *not yet* are three different answers, and collapsing them to a
> tick loses the one that matters — a dismissed item and a deferred item behave differently the
> next time Helm reports.

`State` has three words — `OPEN`, `FIXING`, `CLEARED` — and they are not those three.
`FIXING` is *not yet*, which is the one the warning is about and it survives. **`CLEARED` is the
collapse**, and the interactive listing admits it in as many words: the option reads *"clear it;
it is done, or it is not happening"*. Splitting it wants `Line::Cleared` to carry why, a
migration for every line already written without one, a flag on `clear`, two options where the
selector draws one, and the vocabulary settled in `docs/glossary.md`.

**Deferred on purpose, and the order is the argument.** A partial `001` that makes one id space
is worth more than a complete one that makes two, because the vocabulary is a change to what a
word means and the id space is a change to what can be named — and nothing can be marked *done*
rather than *dropped* until it can be named at all. The id space is now built, so the
vocabulary is an ordinary follow-on rather than an architectural one.

**And the Bridge.** [`003`](003-bridge-command-centre.md) is where the keystroke this section
imagined actually goes. It is unchanged by this work except that it now has one id space to
render instead of two.
