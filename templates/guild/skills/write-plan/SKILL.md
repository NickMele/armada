---
name: write-plan
description: Turn what exploration found into PLAN.md — what code makes the task real, in what order, and what proves each piece. Use for the `write-plan` step of the `plan` workflow.
---

# Write the plan

The step before you explored. You decide what to build and in what order, and a person reads it
and says yes or no. Everything after that is written against what you commit to here.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/write-plan/SKILL.md` changes it.

## Your gate is `artifact_exists: PLAN.md`

Writing the file is what advances the step. **That means the gate cannot tell a good plan from a
bad one** — it proves somebody wrote something, and the approval step after it is where the
judgement actually happens. Write for that reader.

Put it at `PLAN.md` in your worktree root, and **commit it**. A plan that exists only as an
uncommitted file is a plan the next Job cannot read.

## Open with what already exists

The most useful thing a plan can do is separate *what is already true* from *what is new*,
because that is what the reader cannot check without doing your exploration again.

A table earns its place here:

| Piece | State | Detail |
|---|---|---|
| the thing | done / partial / **missing** | what you actually found, with a path |

**Say `partial` when it is partial, and say what half is missing.** A plan that lists a
half-built helper as `done` sends the implement step looking for behaviour that is a stub.

## Then the order, and what proves each piece

Steps in the order they have to happen, each with the test that shows it works. **Every new test
must be shown to fail before the change makes it pass** — say so per step, because a test written
after the code often only asserts what the code already does.

Name the files you expect to touch. If that list is wrong later, that is information; if there
was no list, nothing is.

## Verify your own assumptions before you write them down

The commonest defect in a plan is a signature quoted from memory. **Open the function you are
about to call and read it.** A plan that says `check::status(run, now, place)` about a function
that is private and generic over a trait sends the implement step into a cascading change nobody
costed.

If you find an assumption from the exploration step was wrong, say so in the plan, in those
words. A correction written down is worth more than a plan that quietly routes around it.

## Cost, honestly

If the work is bigger than it first looked, say that, and say which part grew. *"This is real,
newly-discovered scope inside what was called small"* is a sentence a reader needs; discovering
it at `implement` instead is a Job that runs out of budget.

## Open questions

End with what you could not settle, and resolve what you can before finishing. A question with
`file:line` evidence attached is a decision the reader can make in a minute; a vague one is a
round trip.

Nothing else in the workflow will ask these questions again — the implement step reads your plan
as settled.
