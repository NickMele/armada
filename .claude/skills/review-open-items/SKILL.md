---
name: review-open-items
description: Walk the owner through everything this session left waiting on him, one item at a time, each with options to settle it. Load when he says /review-open-items, or asks to go through what is open, what you need from him, or your questions.
---

# Review open items

**One item, one question, one answer, then the next.** He asked for this because
questions stacked in one message get answered partly, or not at all.

On 17 Sep 2026 a reply about Overview's colour ended with three decisions in a
callout: which mock, which colour for Helm, and how to colour the rail. He
answered with one line about something else. All three were still open, and
nothing in the conversation said so.

## What counts as an item

**Anything from this session that cannot move until he decides.** The session
decides which items exist, and the repository confirms each one's state. That is
the same rule `standing` uses for its rows, and the same checks apply.

| An item | Where it comes from |
|---|---|
| A question asked and never answered | Read the conversation back. A callout, a closing question, an `AskUserQuestion` he skipped or answered "Other" to without deciding |
| A question never asked | An assumption made to keep going: invented copy, a picked default, a scope line drawn on his behalf. Say what was assumed |
| A handoff waiting on him | A PR to review, a mock to choose from, a command only he can run |
| A risk he has not seen | Something found along the way that changes what he would decide |

**Not an item:**

| Looks like one | Actually |
|---|---|
| Work with nothing blocking it that is Claude Code's to do | Do it. It is not his to decide |
| The milestone backlog, other sessions' PRs, `docs/OPEN.md` | Not this session's. `armada-open-questions` owns filed questions |
| A choice with one sensible answer | Make it, and say so in the summary at the end |

**If nothing is open, say so in one line and stop.**

## The walk

1. **Gather and check.** Build the list from the conversation. Confirm each
   item's state before asking, because a question already settled by a merge or a
   comment wastes his answer. For an artifact he may have commented on, read
   the comments.
2. **Show the list once.** A short numbered table: the item, and what waits on
   it. Order by what unblocks the most, then by how quick it is to answer. No
   options yet.
3. **Ask the first item alone.** One `AskUserQuestion` call holding **one**
   question, even though the tool takes four. His answer to one often changes
   or removes the next.
4. **Act on the answer before asking the next.** Small and in scope: do it, and
   say in one line what changed. Larger: say what it now commits to and where
   that will happen. Then look at the rest of the list again and drop anything
   the answer settled, saying which.
5. **Repeat until the list is empty or he stops.** "Later" or "skip" is an
   answer. Mark the item deferred and move on without asking again.
6. **Close with one table:** each item and what came of it (decided, done,
   deferred, dropped). Anything deferred that meets the bar in
   `armada-open-questions` is *proposed* for filing there. He decides whether
   it is filed.

## Writing each question

**Load `asking-a-person` first. Its shape is the shape here:** the moment he
would recognise, what happens now, why that is a problem, then the options.
Identifiers belong in what gets written afterwards, not in the question.

| Part | Rule |
|---|---|
| Options | Two to four real ones. Recommended first, labelled "(Recommended)" |
| Each option | What it commits him to, and its cost. That includes the recommended one |
| Leave it | Include it as an option when leaving the item alone is a real choice |
| Visuals | When options are layouts, copy or code, use `preview` so he compares them side by side |

**Say what already exists.** "The Board already does this" can turn a build
into a small wiring job, and that often decides the question.

## What not to do

**Do not batch.** Four questions in one call is exactly the stacked message this
skill replaces.

**Do not re-ask a deferred item** in the same walk.

**Do not narrate the walk** ("Now on to item 3"). The table from step 2 already
says where he is.
