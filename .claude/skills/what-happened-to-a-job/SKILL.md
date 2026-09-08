---
name: what-happened-to-a-job
description: Find out what a Job did and why it stopped — one command that prints its transitions, each step's verdict, what its Drone was refused, and how the run ended. Load when a Job escalated, went quiet, failed a gate, or did something nobody can account for.
---

# What happened to a Job

```sh
./scripts/job <job-id>
```

**Read that before you open a single file under `.armada/`.** It is the whole
record of one Job in one screen, and it is assembled from both places the
record lives — Fleet's store over HTTP, and the JSONL files no route serves.

**`docs/practices/running-locally.md` owns the mechanics** — the flags, what it
needs, what it can still answer with Fleet down. This file is what to do with
what it prints.

## Reach for it when

- A Job escalated and the reason is a single word — `blocked_by_policy`,
  `run_ended`, `gate_failure` — that names a category and not an instance.
- A Job went quiet, or is sitting where you did not expect it to sit.
- A step failed a gate and you need what the Judge actually said.
- The owner pastes a Job id and asks what is going on.

**Do not reach for the transcript first.** A transcript is hundreds of rows and
reading one is what this repository exists to escape. `--transcript` exists and
is the last thing to try, not the first.

## The six sections, and what each is for

| Section | What it settles |
|---|---|
| **JOB** | What it is, which step it is on, its branch, whether its worktree is still on disk, and what Fleet says would move it |
| **WHAT HAPPENED, IN ORDER** | Every transition with a time and an actor — whether a person, Helm or Fleet caused each move |
| **EACH STEP** | Per step: the verdict, every Judge criterion with its finding, every Check with its outcome, what the Drone claimed and what it said it had *not* done |
| **WHAT THE DRONE WAS REFUSED** | The tool **and the argument**. Start here |
| **HOW THE RUN ENDED** | Turns, cost, whether anything was submitted, and per Drone how many calls came back failed |
| **THE LAST OF …** | The tail of the Job's own log — what Fleet wrote down while it ran |

## Reading the refusals

**This is the section the command was built for.** A refused call is recorded
as a tool name and a tool-use id; the argument lives on the `called` row that
came a fraction of a second earlier under the same id. Joining them is what
nothing did before, and without it a `blocked_by_policy` escalation says a Bash
call was blocked and never which one.

**`because: nothing was recorded` is the common case and it is not a gap in the
tool.** The harness carries a reason only where it has one, and a call the
allowlist declines is refused silently — so the argument *is* the finding.

**Read the argument as an allowlist question, not a prompt question.** A Drone
that keeps trying the same denied command is not confused; the Manifest does
not declare what it is reaching for. Almost every refusal here has been an argv
the allowlist did not admit.

**A step that ended with refusals and no evidence is the shape to recognise.**
The Drone spent its turns being told no, ran out, and submitted nothing —
`HOW THE RUN ENDED` will say `nothing was submitted` beside a non-zero refusal
count. That is a Manifest defect, not a Drone defect.

## Reading the steps

**`checks: none ran` beside a list of declared Checks means the step never got
that far.** The Checks are the gate; a step that stopped before submitting
never reached them. That is different from a Check that ran and failed, which
prints its outcome and the path to what it printed.

**`checks: Fleet cannot say` is a third thing** — this Job names a workflow
Fleet does not hold, or holds one that no longer declares the step. Not an
ungated step, which prints as one in so many words.

**The Judge's brief is a path, and the paths are worth opening when a verdict
surprises you.** Three records sit beside each other and one without the others
cannot separate a bad Judge from a bad brief: `brief_path` is what the Judge was
asked, the deliverable is what it read, and a Check's `output_path` is what the
Checks printed.

**`and said it had not done` is the most useful paragraph a Drone writes.** It
is where a step that passed every gate admits what it did not verify.

## What it cannot tell you

| | |
|---|---|
| Whether the work is any good | Read the diff. `armada clean` will delete the worktree it is in |
| What a Judge was thinking | Only what it ruled. The brief at `brief_path` is the input, not the reasoning |
| Anything about a Job whose records were cleaned | `armada clean` takes the transcripts and the log. The command says so rather than printing an empty report |

## After you read it

**Say what you found, not what you ran.** The owner asked what happened to a
Job; a list of the commands you used to find out is not the answer. Name the
refused argument, or the criterion that failed, or the Check that never ran.

**A refusal you found is a claim about the Manifest — verify it before filing
one.** `.claude/skills/armada-bug/SKILL.md` holds the rest of that loop, and
its first rule is the one that matters here: read the code that would falsify
the claim before writing it down.
