---
name: standing
description: One table of what is in flight, what each thing waits on, and who moves it next. Load when the owner asks where things stand, what is left, what is blocked, or what he should pick up.
---

# Where things stand

**One table. Nothing else unless he asks.**

He is holding several threads and cannot tell, from a wall of prose, which rows
need him. The table is the answer; the paragraphs around it are what made him
ask in the first place.

## Read the state, do not remember it

Every row comes from something you can check. **Never summarise this session
from memory** — a merged branch you still think is open, or an agent you think
is running, is worse than no table.

| What | How |
|---|---|
| Agents still running | The task notifications you have received, and which have not reported back |
| Unmerged work | `git branch -a`, and `git log --oneline origin/main..<branch>` for anything that looks live |
| Waiting to merge | `gh pr list --state open` |
| Filed and not started | `gh issue list --state open --milestone <current>` |
| Worktrees holding work | `git worktree list`, then `git status --porcelain` in any that looks stale |
| Something he is holding | Read back: a prompt handed to a design agent, a decision asked for and not given |

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
| **Claude Design** | A drawing, or a correction to one |
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

**One sentence under the table, at most**, and only for something the table
cannot hold — a risk he has not seen, or a thing that is about to need him.

## When something is waiting on him

Say what to do, not that it is waiting. *"Hand the prompt to a design agent"*
beats *"blocked on design"*. He is deciding what to pick up next; a row that
does not name the act makes him work it out again.
