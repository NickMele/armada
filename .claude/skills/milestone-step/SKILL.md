---
name: milestone-step
description: Work one step of an Armada milestone from Notion — read it, plan it, build it, verify it against its own definition of done, record it, and hand it over. Use when asked to do the next step, a numbered step of M0 or M1, or to pick up where a previous session stopped.
---

# Working a milestone step

Armada is built one step at a time, and the steps live in Notion. This is the
loop. It exists because the loop is identical every time and repeating it into
each new session by hand is exactly the problem Armada is being built to solve.

**Until Fleet can dispatch a Job, this skill is the workflow.**

## Where you are

Everything is in the **Armada Steps** database, filtered to a milestone. A step's
`Status` and the repository's `git log` together say where work stopped — that is
the whole resume protocol. Read them before anything else.

| | |
|---|---|
| Root page | Armada |
| Steps | Armada Steps, ordered by `Order` within a Milestone |
| Concepts | Armada Concepts — Job, Drone, Fleet, Bridge, Kit, Manifest, Workflow, Judge |
| Decisions | Armada Questions. `Status: Open` is not settled, whatever it looks like |
| Contracts | Armada Docs — the Design System, the Page Cleanup Procedure, the practices |

## The loop

### 1. Read the step

**Fetch it. Do not work from a summary, from this file, or from what the
conversation says it is.** Read the step's own page, in full, every time —
including on a step you think you already know.

Then read what it touches: its `Concept` relations, and for any decision it
depends on, the row in Armada Questions. **Never assert a decision from memory.**
Query and read the `Resolution` field. A conversation summary can collapse a
suggestion and a reaction into one phrase, and a confidently wrong claim about a
settled decision costs more than the check saves.

### 2. Plan, and name what is missing

Walk the step's "How" bullets and turn each into something you can do. Where a
bullet needs a decision the step does not make, **that is a gap in the plan and
it needs a person.** Say what is missing and ask. Do not pick a reasonable
default and continue — the plan was audited for silently-filled gaps, and finding
more of them is the point.

Ask with a real prompt carrying concrete options and a recommendation, not a
question buried in prose.

### 3. Build

One step. Finish it, report, stop. Do not start the next one — the order is
deliberate and several steps exist to constrain the ones after them.

Two standing rules while building:

- **Green can be a build failure.** In M0 the acceptance test must fail for the
  whole milestone. If you find yourself wanting to stub something so a test
  passes, stop and report instead.
- **A negative result is a result.** On a spike, "no" is an answer. Write it down
  with the evidence — the design changes, and the change is worth more than a
  workaround.

Commit messages say **why**, not what. The diff already says what.

### 4. Verify against the step's own words

Not against your impression of it. Walk the "How" bullets one at a time and say,
for each, what satisfies it. Then the definition of done, if the step states one.
A bullet you skipped is reported as skipped.

**If the step added work to the milestone, add the gate rule that makes it
visible.** `verify-foundations` going green means *every subject a rule names has
landed* — it is not a claim about the milestone, because the gate only knows what
someone wrote a rule for. A step that ships work no rule watches has quietly
narrowed the gate's coverage. This has already happened once.

**A definition of done must be satisfiable by doing the step.** Where a step's
own stated DoD names something it does not control — the gate's overall colour,
another step's behaviour — that clause is out of scope regardless of whether it
is true, and the question to ask is *does this belong to this step*, not *is this
accurate*. Asking accuracy first is how a stale clause gets edited on the
authority of whatever made it stale.

### 5. Record

Two places, and they are different audiences.

**The repository** gets the artifact — a spike record under `docs/spikes/`, a
design under `docs/`, the code itself. Raw evidence lands beside its write-up.

**The step page** gets a summary of **180 words or fewer**, appended under a
`## Done <date>` heading, saying what was built, what you decided that the plan
did not decide for you, what contradicts the plan, and **where the artifacts
live**. Then set `Status` to `Done`.

The 180 words is a real limit. The step page is read by someone deciding whether
to trust the work, not someone redoing it.

### 6. Give every open item an owner

**An open item that is not attached to something is lost.** This is the rule that
gets broken most often, because filing feels like finishing.

| What you found | Where it goes |
|---|---|
| A gap a later step should close | Appended to **that step's page**, under a heading saying it was carried in |
| An undecided design question | **Propose it and wait for an explicit yes.** Then a row in **Armada Questions**, with `Home` set, and `Blocks Capability` or `Blocks Milestone` wherever true |
| Work nothing owns | A new **Step**, in the milestone, ordered so nothing is renumbered |
| Stale vocabulary | Fixed under the **Page Cleanup Procedure**, not find-and-replaced |

Where a mapping is genuinely ambiguous, leave the field empty **deliberately** and
say why. Naming a milestone to fill a blank pre-answers the question the row
exists to ask.

**No question is filed without the owner's explicit yes.** This is the Page
Cleanup Procedure's standing rule — *propose, wait for an explicit yes, then
write* — and it applies hardest here, because filing feels like diligence and is
often the opposite.

Before proposing one, answer it. Read the concept page, the step that owns the
subject, and the doc that governs it. **Two questions filed in one session were
already answered in documents that had been read, and one in a document written
earlier the same day.** A subagent asking a question is not a finding: it saw one
file and no context, and checking costs a minute. A row that could have been
closed makes the genuinely open ones harder to see.

### 7. Report

Three things, in this order, and the third matters most:

1. What changed.
2. What you decided that the plan did not decide for you.
3. **What you found that contradicts the plan.** It was written before any code
   existed and it will be wrong somewhere.

Bottom line first. Under 150 words. Tables for anything comparative. Say plainly
when nothing needs the owner — do not make them infer it from the absence of
their name.

## Retired vocabulary, because it is still in the pages

The nine-phase plan was replaced by milestones. **M0 — Foundations replaced
Ground Zero.** A reference to "Phase N" does not become "Milestone N" — it
becomes the milestone that owns the capability the sentence is about. Where a
phase is named to say *when*, use the milestone. Where it is named to say *why
now*, rewrite the sentence: milestones carry a condition that makes them urgent
rather than a position in a sequence.

Never write `§`. "M0 step 4", not "M0 §4".
