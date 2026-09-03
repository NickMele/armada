---
name: epic-as-one-job
description: Run a whole milestone from one approval — write the split, dispatch a wave, assess what came back, plan the next, and stop at a bound the owner set. Load when the owner names a milestone and wants it run rather than discussed. Stands in for the capability of the same name until #215 ships.
---

# A milestone from one approval

**You are the parent Job.** `docs/capabilities/epic-as-one-job.md` says what a
Drone will do when `#215` ships: read the epic, decide the split, dispatch the
rest as Jobs, stand down, come back with every child's outcome. Everything below
that workflow is built and nothing in `fleet` calls it. Until something does, a
session holds the shape by hand, and this is the shape.

**The mechanics are already written and are not repeated here.**
`orchestrate-milestone` owns briefing, reading what comes back, and the merge
bar. `work-issue` is what each child loads. This skill owns the four things a
parent Job has that an ordinary session does not: one approval instead of one
per piece, a plan on disk before anything spends, a bounded loop, and a question
that does not stop the run.

## First, the plan — before the approval, before any spend

Write it to `.armada/epics/<milestone>.md`. That path is ignored, the way a
Drone's own artifact is, because this repository is public and the workspace a
plan quotes is not. `armada clean` does not reach it.

It carries, per wave:

| In the plan | Why |
|---|---|
| Each child: the issue, the brief in one paragraph, and its **write scope** | `#47` landed as *surfaced, never serialised* — a collision is named on an approval card and nothing prevents it. Two children in one file and the second wins silently; `work-issue` has the split that worked |
| What blocks what | Sequence is the whole value of a wave. Getting it wrong wastes a child |
| What each child proves | A wave with no check is a wave you cannot assess |
| What is undecided, and who decides it | Settle these with the owner *now*. Decomposition is decision-dense, and the first ambiguity at 3am is a stalled night |

**Draw it, in the same file.** A Mermaid flowchart: a node per child, a solid
edge where one child needs what another produced, a dashed edge where two are
merely held apart because they would write the same paths. Nothing else in the
plan says what may run at once as fast as one look at it, and `#215` inherits
this — `docs/capabilities/epic-as-one-job.md` carries it as the shape of the
artifact a person approves.

**Only the first wave is a plan. The rest is a guess, and say so.** Building
Throughput by hand on 31 Aug: `#50` landing rewrote the two briefs after it, and
`#47`'s whole premise turned out false once `write_targets` was known to be null
at every gate. A slate written at the start would have paid a Drone to find that
out. Name the later waves; brief them when they are next.

## The approval

**Open the plan in his editor before you ask him anything.** `code <path>` —
`$EDITOR` if it is set, `open` if neither is there — and say the absolute path in
the message as well. A question about a document he has not been handed is a
question he has to go and find the answer to, and the approval is the one moment
in the run where he is reading rather than being reported to.

One `AskUserQuestion`, once the plan is open in front of him, fixing four
things. **One
approval here becomes several agents' worth of spend** — `#51` caps a *Fleet*
dispatch and nothing caps a session's, so the bound is the one he sets and your
own arithmetic.

| The approval fixes | The options to offer him |
|---|---|
| The split | approve as written, or approve with the changes he names |
| The cadence | run wave to wave and report; report and hold for approval each wave; run until a question or a red wave |
| The bound | how many waves before the run stops and reports regardless |
| The merge authority | merge each green child as it lands, or stack branches and leave every merge to him |

**How often he approves is a property of the milestone, not of you.** A low-risk
milestone runs wave to wave. A risky one stops each round. He sets it once, at
the approval, and you do not revisit it mid-run.

## The loop

`plan` -> `dispatch` -> `assess` -> `plan`, until the milestone is done, the
wave bound is reached, or a question is unanswered.

**Dispatch a wave, not the slate.** Every child gets `work-issue`, the brief
items `orchestrate-milestone` lists, the disjoint write scope, and the
instruction to push back rather than build against a brief it believes is wrong.

**Then do nothing but wait.** A parent that keeps working is the deadlock `#50`
removed, one scope up: you start editing what a child is editing, and the wave
comes back onto a tree that moved under it. Hand-landing a child's work also
hides every gap the run existed to find. Waiting is the job.

**A wave is over when every child has stopped, not when every child has
succeeded.** The failed one is what the next plan is most needed for.

**Assess before planning the next wave.** Read the diff, not the report. Take the
corrections — a child that has just read the code is usually right and you are
working from memory. The merge bar in `orchestrate-milestone` is the whole bar,
every time, and the worktree goes back at the merge.

Then write the next wave into the same file, under the last one, saying what the
wave that just ran made untrue. That file is the record of the run.

## Asking, when you do not know

**Neither guessing nor stopping is available.** Guessing spends real agents on
work that is wrong. Stopping until morning idles the whole run over one word.

Ask the way the built tool asks: **two to four labelled options, each saying what
it commits to, and no prose.** `AskUserQuestion`, recommendation first. If a
question can wait for the wave boundary, hold it there and ask everything at once
— a question mid-wave stalls whatever it blocks.

If he is not there, the run holds at that question and every child that does not
depend on it keeps going. A held run is the safe failure. A guessed decision is
not.

## Depth 1

**Children do not dispatch children.** You are the only dispatcher in the run,
the same way `Dispatching::at` answers `None` for a sub-dispatched Job. A child
that thinks it needs a child has found scope you missed — it reports that, and
the next wave is where it goes.

## What you report, and when

At each cadence point, one table and nothing else: every child, what it landed,
green or red, **and who acts next**. Rows he owns lead with a verb. When nothing
needs him, say so in a sentence.

At the bound or the end: what the milestone claims, whether its acceptance test
passes, and what you left undone.

## What this cannot do, and he should know it

| A Job has | You do not |
|---|---|
| Every event recorded, judged and visible on the Board | a plan file and a session transcript |
| A budget cap that stops dispatch (`#51`) | arithmetic and the wave bound he set |
| A gate no child can talk its way past | your own reading of the diff |

**Delete this skill when `#215` ships.** It exists because the workflow does not,
and a hand-run stand-in that outlives its capability is how a milestone gets
marked complete while nothing can reach it.
