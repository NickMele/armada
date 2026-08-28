---
name: milestone-step
description: How to work one Armada milestone step end to end — read it, check it against the registry, build it, verify it yourself, close it with what contradicted the plan. Load before starting any step.
---

# Working a milestone step

Armada is built one step at a time. This is the loop, and it exists because the
loop is identical every time — repeating it into each new session by hand is
exactly the problem Armada is being built to solve.

**Until Fleet can dispatch a Job, this skill is the workflow.**

## Where you are

Steps and capabilities are **GitHub issues** in `NickMele/armada`. A step's state
and `git log` together say where work stopped; that is the whole resume protocol.

| | |
|---|---|
| Steps | Issues labelled `step`, titled `M1 7 — …`, grouped by milestone |
| Capabilities | Issues labelled `capability`. A step serves up to five, named as `#N` |
| Progress | Computed by `cargo xtask verify-roadmap`. **Nobody types a status** |
| The model | `crates/core-model/domain/*.toml` — the authority on statuses, transitions, fields |
| Concepts and contracts | `docs/concepts/`, `docs/contracts/` |
| Every open question | `docs/OPEN.md`, generated |

## The loop

### 1. Read the step, and read what it disagrees with

**Fetch it. Do not work from a summary or from what the conversation says it
is.** Read the issue in full, every time, including one you think you know.

Then read its capabilities, the concept pages it touches, and — this is the part
that gets skipped — **the registry files in `crates/core-model/domain/`.**

**Most steps were written before the registry landed in the repository.** Two in
a row have now contradicted it: one named five Job statuses where the file
declares twelve, and one said its acceptance criteria lived on a capability that
carries a single sentence. **Where a step and a checked-in registry disagree, the
registry wins** — its own header says a page that disagrees with it is stale, not
right. Report the disagreement; do not edit the issue to match what you built.

### 2. Plan, and name what is missing

Turn each "How" bullet into something you can do. Where a bullet needs a decision
the step does not make, **that is a gap in the plan and it needs a person.** Say
what is missing and ask.

Do not pick a reasonable default and continue. Ask with a real prompt carrying
concrete options and a recommendation.

### 2.5 Read the registry before minting anything

**The decision is usually already written down.** `crates/core-model/domain/`,
`crates/ipc/operations.toml` and `packages/icons/icons.toml` are older than any
session, and they carry reasoning as well as values.

In one night, three agents each drafted something new and then found it
decided: a new escalation trigger, where `judge.md` already said a Judge refusal
*is* a gate failure; a trigger's level, which the registry had typed; and a
glyph rule, where the icon's own reservation sanctioned exactly the use being
avoided. **Each of them abandoned the draft and said so, which is the right
outcome — but the reading should come first.**

A row carries `notes` and `why` for this reason. Read them before adding a
sibling, because several rows exist specifically to be told apart from each
other and the notes are where that is stated.

**Minting a second answer is how a vocabulary splits**, and the split is not
visible until two things that mean the same render differently.

### 3. Build

One step. Finish it, report, stop. The order is deliberate and several steps
exist to constrain the ones after them.

- **A test made to pass is worse than a test failing.** The acceptance test is
  the milestone's own claim — read `docs/practices/acceptance-tests.md` before
  touching it. If you want to stub something so a test passes, stop and report
  instead.
- **A negative result is a result.** On a spike, "no" is an answer. Write it down
  with the evidence.
- **Time is injected, never read.** A model that calls the clock cannot be
  tested and cannot be replayed.
- **A second vocabulary is a defect, not a shortcut.** A mapping that already
  exists somewhere is imported, never copied. The one that drifted three times
  was deleted; the next one will drift too.

Commit messages say **why**. The diff already says what. Read
`.claude/skills/commit-message/SKILL.md`.

### 4. Verify it yourself

**Never close a step on an agent's report.** Run the commands and read the
output. A report claiming green has been wrong here, and a report claiming a
faithful conversion has been wrong here twice.

| Check | Command |
|---|---|
| Tests | `cargo nextest run --workspace --exclude acceptance`, then `cargo test -p acceptance` — the acceptance crate is excluded from the first because it is the milestone's claim rather than a unit test, and is worth reading on its own |
| The gate | `cargo xtask verify-foundations` — read every line it names |
| Dependencies | `cargo tree -p core-model` shows only itself |

Then walk the step's own "How" bullets one at a time and say, for each, what
satisfies it. A bullet you skipped is reported as skipped.

**If the step added work no rule watches, add the gate rule.** The gate going
green means *every subject a rule names has landed* — it is not a claim about
the milestone, because the gate only knows what somebody wrote a rule for. This
has already happened once.

### 5. Close the issue with what contradicted the plan

**A step is not done until its issue is closed**, and closing it is not a tick.
The comment carries, in this order:

1. The commit it landed in.
2. What was built, and what you decided that the plan did not decide for you.
3. **What contradicts the plan.** The plan was written before any code existed
   and it is wrong somewhere. This is the part somebody will need.

Then close the capabilities the step completes, if it completed them.

### 6. Give every open item an owner

**An open item that is not attached to something is lost.** This is the rule
broken most often, because filing feels like finishing.

| What you found | Where it goes |
|---|---|
| A gap a later step should close | A comment on **that step's issue**, saying it was carried in |
| An undecided design question | A `## Open questions` bullet in the document that blocks — **and only after a person defers it.** Read `.claude/skills/armada-open-questions/SKILL.md` |
| Work nothing owns | A new issue, labelled and milestoned |
| Something parked | An issue labelled `idea`, no milestone |

**No question is filed on an agent's judgement.** Propose it and wait for an
explicit yes. Before proposing, answer it: read the concept, the contract and
`docs/OPEN.md`. Two questions once filed in one session were already answered in
documents that had been read, one written that same day.

### 7. Report

Four things, and the last two matter most:

1. What changed.
2. What you decided that the plan did not decide for you.
3. **What you built that nothing reads yet.** A finding that reaches a database
   and not a person is not finished, and a field on the wire that nothing draws
   is not either. Say where the value is *read* — and where the answer is "a
   test", say that rather than counting it done.
   `docs/practices/half-built.md` is the defect and why it recurs.
4. **What you found that contradicts the plan.**

Bottom line first. Under 150 words. Tables for anything comparative. Say plainly
when nothing needs the owner.

## Retired vocabulary, because it is still in the issues

**M0 — Foundations replaced Ground Zero.** A reference to "Phase N" becomes the
milestone that owns the capability the sentence is about — not "Milestone N".
Where a phase is named to say *when*, use the milestone; where it says *why now*,
rewrite the sentence.

Never write `§`. "M1 step 4", not "M1 §4".
