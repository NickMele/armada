---
name: standing
description: One table of what this session started and has not finished — what each thing waits on, and who moves it next. Load when the owner asks where things stand, what is left over, what is blocked, or what he should pick up.
---

# Where things stand

**One table, of this session's leftovers. Nothing else unless he asks.**

He is asking what he is still holding from the work you two just did. A row that
was already in the repository when the session opened is not an answer to that —
it is a second question he did not ask, and it buries the rows that are his.

## Only what this session touched

**The session decides which rows exist. The repository decides what each one
says.** Those are two different steps and both are required.

Walk what actually happened in this conversation — what was built, filed,
dispatched, asked or deferred — and take the candidates from there. Then check
the state of each one, because a branch you still think is open or an agent you
think is running is worse than no table.

| A row this session produced | Confirm it with |
|---|---|
| A branch or PR from this session | `gh pr view <n> --json state,mergeable`, `git log --oneline main..<branch>` |
| An agent dispatched in this session | The task notifications received, and which have not reported back |
| An issue filed in this session | `gh issue view <n> --json state` |
| A worktree cut in this session | `git worktree list`, then `git status --porcelain` in it |
| A decision asked for and not given | Read the conversation back — an `AskUserQuestion` he answered with a deferral, or never answered |

## What is not a row

**Anything that was already there.** Other people's open PRs, other agents'
branches and worktrees, the milestone backlog, a stale branch this session did
not create. All real, none of it what he asked.

**Anything that landed.** A merged PR, a closed issue, a worktree given back.
The table is what is left, not a changelog.

**A pre-existing issue this session merely read.** Finding that an old issue
already covers something is a sentence, not a row — the row would imply the
session put it there.

The exception is a thing this session *changed the state of*: an issue it filed
and left open, a branch it found and rebased, a worktree it inherited and could
not clean. Say which, in Blocked on, so he knows why it is his.

## The table

Four columns, and the last two are the point.

| Column | What goes in it |
|---|---|
| **What** | The thing, named as he would name it. An issue number where one exists |
| **State** | Merged, running, built and unmerged, filed, drawn — a state, not a narrative |
| **Blocked on** | The specific thing, or **Nothing**. Never "in progress" |
| **Owner** | Who moves it. Never blank, never two |

**Name the actor, not the pronoun.** "You" and "Me" read fine in a sentence and
badly in a column he is scanning — by the fourth row he is working out which of
us "me" was.

| Owner | Means |
|---|---|
| **Human** | A decision, a handoff, or something only he can run |
| **Claude Code** | Work in this repository — me, or an agent I dispatch |
| **Fleet** | A Job dispatched through Armada itself — the answer whenever the work could be one |

Where a row is with something already running, say so — **Claude Code
(running)**, **Fleet (running)** — because that is a different thing from work
nobody has started.

**Fleet is the owner more often than it gets written down.** Hand-landing what a
Job could have done hides every gap in the fleet, so a row that reads Claude Code
where Fleet could have taken it is worth a second look before it goes in.

**A row that says `Nothing` under Blocked on and `Claude Code` under Owner is a
row I should be working**, not reporting. Check that before writing it down.

## What not to do

**No status narration.** He does not need to know that an agent is thinking.

**No rows for finished work**, unless he asked what landed. A recap of what is
in flight is not a changelog.

**Do not pad it.** Five real rows beat fifteen with the last ten reading
"filed, not started" — those belong to a milestone, not a recap. If the honest
answer is two rows, it is two rows.

**If the session left nothing, say so in one line and stop.** No table, no
consolation rows swept in from the repository to fill it. "Everything from this
session is merged and cleaned up" is a complete answer, and it is the one he is
hoping for.

**One sentence under the table, at most**, and only for something the table
cannot hold — a risk he has not seen, or a thing that is about to need him.

## When something is waiting on him

Say what to do, not that it is waiting. *"Hand the prompt to a design agent"*
beats *"blocked on design"*. He is deciding what to pick up next; a row that
does not name the act makes him work it out again.
