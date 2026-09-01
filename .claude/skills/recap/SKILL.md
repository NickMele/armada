---
name: recap
description: One table of what is in flight, what each thing waits on, and who moves it next. Load when the owner asks where things stand, what is left, or what is blocked.
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
| **Owner** | **You** or **Me**. Never blank, never both |

**Owner is you where the next move is a decision, a handoff, or something only
he can run.** Owner is me where the next move is work, and I should say what I
will do rather than asking whether to.

**A row that says `Nothing` under Blocked on and `Me` under Owner is a row I
should be working**, not reporting. Check that before writing it down.

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
