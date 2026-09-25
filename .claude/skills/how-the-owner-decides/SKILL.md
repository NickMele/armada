---
name: how-the-owner-decides
description: The patterns behind the owner's answers, each cited to a decision he actually made — load before choosing between two defensible options, before writing the options for a question, and before deciding something small on his behalf.
---

# How the owner decides

**This makes defaults better. It does not answer his questions for him.**
A decision that changes what a person can do is his, however confident these
patterns make you. What they are for is the hundred small choices underneath
one — the wording, the placement, which of two correct things to build — and
for writing options he recognises instead of options he has to translate.

Every rule below is cited to a decision in `.claude/decisions/`, which holds
the question, what he chose, and the cost he took. **Read the cited file before
leaning on a rule.** A pattern with one instance behind it is a guess.

## The rules

> **Describe what a thing does; do not name a category for it.**
> `2026-09-22-no-kind-names.md` — four kinds of Job, all retired: *"They don't
> need names. Jobs are too fluid to have names for everything."* A screen says
> *three pull requests, landing in order*.

A taxonomy is a promise the world breaks. Where you find yourself minting a
word for a shape, draw the shape instead.

> **The app shows facts. An explanation is something he chooses to see.**
> `2026-09-23-facts-on-screen-guides-behind-a-mark.md` — *"I hate it, I just
> want the app to show facts. Any hints or guides should be something I choose
> to see."* An explanation goes behind a `?` beside the thing it explains, and
> into a catalogue he can read later.

This is the rule most often broken by agents writing screens, because prose
explaining a design feels like care. It reads as noise. A sentence that reports
what happened stays; a sentence that teaches the reader what a thing is moves
behind the mark. If you cannot tell which one you have written, ask whether it
would still be true on a Job that had never run.

> **Teach the way a game teaches, and never gamify.**
> `2026-09-23-teach-like-a-game-never-gamify.md` — a card the first time you
> meet a piece, once, with the first one offering to turn them all off; motion
> that shows how pieces relate. No points, no streaks, no badges.

The two rules above and this one are one position: the screen reports, the
teaching is offered once and then waits to be asked. What he rejects is not
being taught — it is being told things he did not ask for, twice.

> **Remove the constraint rather than document it.**
> `2026-09-22-helm-overlays.md` — three screens worked around the dock's 380px
> and the proposal was a contract rule. He made the dock overlay instead, so
> the fact stopped existing.

When the third workaround for one fact appears, the fact is the defect.

> **A screen never says a thing it does not know, and two facts never share one
> sentence.**
> `2026-09-22-no-verdict-recorded.md` — a criterion nothing had ruled on read
> *not covered*, which is what a test with no spec reads, so a Job that merged
> fine reported nothing met.

Honest and loud beats tidy and wrong. Where a reading is absent, say it is
absent in its own words.

> **Pay for the record being right.**
> `2026-09-22-judge-and-check-sign-the-record.md` — told that giving a Judge and
> a Check their own names costs a store migration and that Bridge derives the
> distinction for free, he stood by storing them.

The expensive option wins when the question is what is true. It loses when the
question is only what is convenient — see the next rule.

> **Match the ceremony to the stakes.**
> `2026-09-22-armada-is-pre-alpha.md` — *"Absolutely no one is using armada so
> we don't need to be this precise."* Land green work; skip
> backwards-compatibility ceremony.

Both of the last two rules are alive at once. Rigour about what is recorded,
speed about what is shipped.

> **He will trade a true picture for a readable screen.**
> `2026-09-25-the-plans-graph-lives-on-plan.md` — told that moving the plan's
> graph off Workflow kills the second edge he had asked for two days earlier, he
> moved it anyway. `2026-09-22-helm-overlays.md` went the same way.

This is not the opposite of *pay for the record being right*. Both are alive:
rigour about what is **stored**, restraint about what is **drawn**. Where a
drawing is complete and crowded, he takes the crowding out and accepts the gap.
Where a record is convenient and lossy, he pays for the loss.

> **Give the person the control; do not infer it from a rule.**
> `2026-09-22-the-rail-is-a-choice.md` — the left column collapsed only because
> Helm needed room, and his answer was that he should be able to collapse it
> himself, at any width.

Where a rule and a control both answer a question, he takes the control and the
rule stays only where there is no room for a choice.

> **Cut what no longer earns its place, including something he asked for.**
> `2026-09-23-hand-entry-goes.md` and
> `2026-09-23-take-out-what-else-is-running.md` — both were kept on his earlier
> word, and both went once something better covered the ground.

An earlier ruling of his is a given until he reverses it. Yours is not: if you
argued for keeping something and he cuts it, the record says he cut it.

> **Do not answer a question against a thing that is already being replaced.**
> Asked how a narrow screen should behave, he sent it to the design agent
> instead: *"You're talking about the old designs the app is currently using
> and that's not what we want."*

Before asking, check that the thing the question is about is not already on its
way out. That is a question that wastes an answer.

> **When a promise is the point, make it true rather than shrinking it.**
> `#1571` — Pulse said *taken again whenever this Job moves*, which was honest,
> because no poll existed. The design had promised ten seconds. He took the
> poll.

The honest smaller sentence is right while the gap is unbuilt and wrong as a
destination.

> **The built thing is the reference, not the drawing of it.**
> `2026-09-23-built-screens-are-the-reference.md` — where a board and a decision
> disagree, the decision wins; where a board and the built screen disagree, the
> screen is what he is looking at.

## Writing options for him

`asking-a-person` owns the shape. What this file adds is what his answers say
about the options themselves:

| | |
|---|---|
| He picks the recommended option most of the time | So the recommendation is doing the work. Make it the one you would defend, not the safe one |
| He rejects the frame outright when it is wrong | Twice in one session: the dock, and the rail. Leave room for that — an option list is not a cage |
| He answers with a sentence when no option fits | *"I should be able to collapse it to a rail at any point or at the breakpoint."* Read the sentence, not the nearest option |
| He takes a stated cost and holds to it | So state the cost plainly. A cost he was not told is a decision he did not make |
| He asks to see two options built before choosing between them | 25 Sep, on what a guide is: *"I would like to see both options before deciding."* Where an option is a **shape he would look at** — a layout, a piece of copy, how a thing reads — offer to build both behind a switch in the mock rather than only describing them. Where the options differ in **cost or consequence** rather than appearance, he answers from the description |

## Keeping this honest

**Write the decision down when he makes it**, in `.claude/decisions/`, one file:
the question as he met it, what he chose, the cost he took, and where it landed.
The register is the evidence; this file is only the reading of it.

**A rule that gets contradicted is deleted, not hedged.** If he decides the
other way, the decision file records both and the rule here goes or narrows.
Twelve rules from four days is a thin basis, and this file will read as more certain
than it is.
