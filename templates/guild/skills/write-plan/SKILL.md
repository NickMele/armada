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

**Say which questions the implement step may answer and which it may not**, and put them where an
approver cannot miss them. *"Nothing else will ask these again"* used to be the whole of this section
and it is not enough — not because a Drone ignores them, but because **the person approving the plan
may never reach them.**

Measured here: a plan's fourth open question asked whether a change should extend to a second call
site, stated its leaning and its cost, and said *this needs the owner's confirmation, not an
assumption*. It was the right question, asked in the right place. The approver read questions one
through three, said *"approved, implement it as written"*, and never saw the fourth. The implement
step then did what the plan said — correctly — and the decision was never actually taken by anybody.

So a plan's open questions are the part most likely to need an answer and the part most likely to be
skimmed. Two things follow. **Lead with them if any of them blocks work**, rather than ending with
them. And mark each one, in these words:

- **`SETTLED BY IMPLEMENT`** — a detail you could not check but whoever writes the code can. A
  signature, a column width, whether a helper already exists.
- **`NEEDS THE OWNER`** — a decision about *what is being built* rather than how. A different
  approach, a new config key, anything that widens what was asked for. The implement step must stop
  and use `mcp__armada__fleet_ask_human` rather than decide it.

An unmarked question reads as the first, so mark the second explicitly.

## If you find yourself implementing, commit it

Your gate is `artifact_exists: PLAN.md` and your workflow ends at a person, so **nothing downstream
of you will commit anything you write.** A plan step that writes code leaves it as uncommitted
changes in a worktree, where the only thing standing between it and deletion is the guard inside
`armada fleet reap`.

That is not hypothetical. A plan Job here implemented its whole feature — a new module and six
modified files, 366 lines, green — and left every line uncommitted. It survived because that guard
held.

Sometimes implementing while planning is the right call: it is faster than the round trip and the
plan comes out better for having been tested against real code. When you do it:

- **Commit it**, in coherent pieces, with messages that say why.
- **Say so in the plan**, at the top, so the reader knows they are reading a description of code that
  exists rather than a proposal.
- **Run the workspace's checks** with `mcp__armada__fleet_check` before you finish, because you have
  now produced work that a gate will judge.
