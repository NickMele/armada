---
name: investigation
description: Find something out — a research spike, a "why is this happening", an unknown that has to be resolved before work can be planned. Use when the task is a question rather than a change.
---

# Investigation / Research Spike

**Four steps, and it loops:** frame → investigate → conclude → present, with the
same three-value human verdict as Design Plan.

**No `WorkflowDef` sample exists yet.** This workflow is designed and not
instantiated, so what follows is its prose design. Where running it by hand
teaches something the design does not say, that is worth writing down — this is
the workflow most likely to gain a sample from having been used.

| Step | What it is |
|---|---|
| `frame` | State the question, and what would count as an answer |
| `investigate` | Open-ended. Read, measure, try things |
| `conclude` | Say what you found, including "nothing is wrong" |
| `present` | The owner reads it. `approve` / `request changes` / `reject` |

## What makes this different from Bug

**"Nothing's wrong" is a valid successful outcome.** There is no artifact whose
absence means failure — no failing test to pin down, no diff to produce. A Bug
that finds nothing has failed; an Investigation that finds nothing has succeeded
and saved everyone the work.

**No mechanical check at all**, for the same reason. And because there is no
"done" signal to fall back on, **the thrashing tripwire carries the whole safety
burden here** rather than backing up a check. An investigation that has been
running for hours with nothing written down is the failure mode, and nothing else
will catch it.

## Scope is wider by nature

An investigation that cannot read outside the files it started with is not an
investigation. Expect to range further than a coding workflow would, and say
where you went.

## Running it by hand

**Frame it before you start.** Write down the question and what would count as an
answer. An investigation with no stated answer-shape runs until someone gets
bored, which is the thing the iteration cap is trying to prevent from the other
end.

**Record a negative result as a result.** "No" is an answer, and it is usually the
more valuable one — it changes the design, where a workaround would have hidden
the change. Write it down with the evidence beside it.

**M0's spikes are worked examples.** `docs/spikes/003` through `006` each frame a
question, measure it against the real thing, and record what was found including
what was not measured. `005` is the useful one to read for how to record a
correction: its first conclusion was wrong, and the record says so in place
rather than being quietly edited.
