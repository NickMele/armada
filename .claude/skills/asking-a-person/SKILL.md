---
name: asking-a-person
description: How to put a decision to the person you are working for — describe the moment they would recognise, never the machinery. Load before asking any question, and before writing the options.
---

# Asking a person

**Describe the moment, not the machinery.** A person decides on what somebody
can and cannot do. Name the types, the files and the error codes and they have
to reconstruct the situation before they can even start deciding.

## The rule, and what it cost to learn it

On 3 Sep 2026 a question was put to the owner in these words:

> A redirect typed at a running Job with no Drone on it — queued, between steps,
> waiting on a slot. `drone.md` promises the note holds; `redirect_drone`
> answers 409, and `redirect_waiting` is written only by `request_changes` at
> `awaiting_review`.

He answered: **"I don't know what you're asking me. Please rephrase and clarify,
you're speaking in technical jargon. Tell me the actual problem."**

The same question, rewritten and called *perfect*:

> You're watching a Job. Some of the time there's an agent actively working on
> it — and then you can type it a note, which reaches it mid-work. But a Job
> spends real stretches with **nobody on it**: waiting its turn to start, or in
> the gap between one step finishing and the next beginning.
>
> Right now, if you type a note during one of those gaps, Armada refuses it.
> Nothing to deliver it to.
>
> That's awkward, because the gap is often exactly when you notice something —
> you watch a Job sitting in the queue and think *when this starts, don't let it
> touch the config file.*

Nothing was left out. Every fact in the first version survives in the second,
said as a thing that happens to somebody.

## The shape

| In this order | |
|---|---|
| The moment | Where a person is standing when they meet this |
| What happens now | The behaviour, in what they would see |
| Why that is wrong | What it costs them |
| The options | What each one commits them to |

**The concrete instance is what makes it land.** *"Don't let it touch the config
file"* is invented, and it is why the second version reads as a real situation
rather than a category. Invent one: a real thing they might type, a real file an
agent might touch, a real moment they would recognise.

**Say what already exists.** Half of a decision is how much it costs, and
*"Armada already does exactly this when you send work back at a review"* is the
sentence that turns a build into a wiring job.

## The options carry the same rule

Each option says what it commits the person to, in their terms. Not which module
changes — which is a fact for the issue you file afterwards, where an
implementer needs it.

**Name the cost of each, including the recommended one.** An option with no cost
stated reads as a trick.

## Where the identifiers go

**Out of the question entirely, and into what you write next.** An issue's `## In`
section is exactly where `redirect_drone` answering 409 belongs, and the same
sentence that was wrong in a question is right there — see
`docs/practices/writing-an-issue.md`, which is the same rule for a different
reader: consequence first, mechanism under a heading.

## It is not only questions

**Anything written for a person to read takes the same shape**, and an artifact
most of all — a page is read once, alone, without you there to translate.

Asked what a new screen was for, the answer that worked was not *it lists held
worktrees with their `HeldReason`*:

> Every Job gets its own copy of the repository to work in — that's how two
> agents don't trample each other. When the Job's over, that copy is just disk
> sitting there. It's how this machine once ended up with 74 of them and 220 GB,
> and three agents killed mid-run at zero bytes free.

Then the mechanism, once the reader knows why they should care. **The number is
doing work there**: 74 and 220 GB are what makes *disk sitting there* a problem
rather than a phrase. Reach for the real measurement, the real incident, the real
sentence somebody typed — a page that argues from a category persuades nobody who
did not already agree.

**In an artifact this is structural, not a flourish.** Lead a section with the
situation, put the identifiers in the table underneath, and let a reader stop
after the first paragraph having understood what the page is about.

## What this is not

**It is not writing less.** The rewritten question is longer than the one it
replaced. It is not simplifying the decision either — the options were unchanged.
What moved is who does the translating.
