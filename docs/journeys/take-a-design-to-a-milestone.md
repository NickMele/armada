# Take a design to a milestone

**What it is:** Turning a design nobody has built — a written direction, a set of screens — into a milestone of Jobs, without doing the translation by hand.

Design fidelity: not set. Analysis: partial. UI/UX design: not started.

---

**Trigger:** A design exists and is close enough to build, and the work it implies is more than one Job.

**Concepts touched:** Studio, Job, Job Board, Workflow, Plan, Judge.

**Milestone:** Throughput. The Studio half of it is Studio's.

**Capability:** [`epic-as-one-job.md`](../capabilities/epic-as-one-job.md) covers approving one Job that decomposes. This document covers what happens before that Job exists.

## Flow

1. **Bring the design in.** A written direction and a set of screens, made outside Armada or on a Studio.
2. **Have it read against the code.** Several readers at once, each from one angle: does the model hold together, does the seam survive it, can the screens be built, can the daemon serve them, is the order right.
3. **Answer what they could not.** The readers bring back questions only a person can settle. They arrive together, not one at a time.
4. **Keep the answers.** Each answer is a decision, dated, and it beats the design where the two disagree.
5. **Revise the design.** The readers' findings and the answers go back to whoever drew it.
6. **Read it again.** The second pass is against the decisions, not the first draft: what was fixed, what changed on purpose, what is new.
7. **File the milestone.** One issue per piece of work, each naming the screens it builds and the decisions it obeys, in waves by what blocks what.
8. **Dispatch the first wave.** From here it is `dispatch-a-milestone.md`.

## What a person is doing at each step, and what Armada does

| Step | Who does it now |
|---|---|
| Bring the design in | A person, by pasting links |
| Read it against the code | A person, dispatching readers by hand |
| Answer the questions | A person, in rounds somebody assembled |
| Keep the answers | A person, in an issue body |
| Revise the design | A person, carrying findings to the designer |
| Read it again | A person, dispatching a second round |
| File the milestone | A person, writing each issue |
| Dispatch the first wave | Armada, once the issues exist |

## The decisions are the product

A design session produces two things, and only one of them is the drawing. The other is the set of answers the drawing forced — what supersedes what, and from when.

Everything downstream reads the second one. An agent building a screen needs the design and the answers, and needs to know that the answers win.

> **Rule.** Where a design and a decision disagree, the decision wins, and the record says which decision and when.
> Why: a design is revised in place, and an issue written from it is not.

## Worked example, 21–22 September 2026

The Job redesign — a system design, a canvas of screens, and the milestone they became.

| | |
|---|---|
| Readers, first pass | Five, in parallel: the model, the seam, the screens, the daemon, the order |
| Decisions taken | Twelve, in three rounds |
| Readers, second pass | Three, against the revised canvas: what was fixed, what moved, what Fleet cannot serve |
| Decisions taken | Twelve more, one of which retired four names |
| Filed | One milestone, twenty-two issues, four and a half waves |
| Built by Armada | The last step only |

Two readings found what a single reading would not: that the screens named data the daemon has no field for, and that two boards disagreed about the same minute of the same Job.

## What does not exist yet

Named here so the flow above is not read as a description of something that runs.

| Missing | Consequence for this flow |
|---|---|
| A Studio that can hold a design made elsewhere | Step 1 is a paste into a prompt |
| A Job whose plan fans out readers over one document | Step 2 is dispatched by hand, and the findings are joined by hand |
| Questions from several Drones, asked in one round | Step 3 arrives one Job at a time |
| A home for decisions, and what they supersede | Step 4 is prose somebody maintains |
| A Job that files issues rather than dispatching them | Step 7 is written by hand |
| Anything that notices a design changed after issues were written from it | Step 6 has to be remembered |
