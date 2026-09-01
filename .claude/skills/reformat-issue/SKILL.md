---
name: reformat-issue
description: Rewrite one existing GitHub issue so its first two sentences say what a person cannot do, without losing anything an implementer needs. Load when the owner says /reformat-issue with a number or a link, or asks to clean up an issue.
---

# Reformatting one issue

**The rules for what an issue says are in `docs/practices/writing-an-issue.md`.**
Read it first. This skill is the loop for applying it to prose that already
exists, which is a different problem from filling in a blank form.

**The premise: the answer is usually already in the issue.** In the corpus this
was built from, `## Why it matters` carried exactly the sentence the owner wanted
and sat in third position under two layers of provenance. Reformatting is
mostly promotion and reordering. **Rewriting from scratch is how a claim gets
invented**, and `armada-bug` already carries the standing rule:

> One rule above the rest: verify the claim before writing it down. A bug report
> is a claim about the code, and a wrong one is worse than none.

That rule survives the reformat. A reformat that adds a fact is a reformat that
made one up.

## The loop

### 1. Read the whole issue, including every comment

```
gh issue view <n> --json number,title,body,labels,milestone,state,url
gh issue view <n> --comments
```

**Never reformat from the body alone.** This is the failure mode that matters,
and it is proven three times over:

| Issue | Body says | Comments say |
|---|---|---|
| #110 | Two of five `said` rows are Armada's | **Eight** vocabularies render as wire spellings |
| #235 | Two gate holes | **Four** files, in two packages |
| #209 | An `## Open` section with a question in it | The owner settled it on 31 Aug |

A reformat of #235's body alone would preserve a title — *"Two gate holes"* —
that the comments had already made wrong. **A comment is where a decision lands
in this repository.** Fold what it settles into the body and say in the rewrite
that you did; leave the comment where it is.

**Refuse a closed issue.** Nothing reads it, and the edit destroys the record of
what was actually worked.

### 2. Find the consequence, in this order

Look for a sentence that already says what a person cannot do. Stop at the first
hit.

1. **The bolded lead of `## Why it matters`.** Where the section exists it is
   almost always already right — *"A record nobody can reach is the same as no
   record"*, *"It fails silently in the direction that looks like success"*.
2. **A quoted owner sentence.** #246 carries *"I can't find the
   `implement.3.3.log` anywhere. Its not clickable so I can't even open it."*
   That is the lede, and it was in the middle of the page.
3. **`## Definition of done`, inverted.** It states the end state; the negation
   is the consequence. *"A person looking at a failed step can open the log"*
   becomes *"a person looking at a failed step cannot open the log"*.
4. **The title, if it is already consequence-shaped.**

**If none of the four hits, stop and ask.** Do not synthesise one from the
mechanism. Step 5.

### 3. Sort every remaining paragraph into one of four piles

| Pile | What lands there |
|---|---|
| **Promote** | The consequence sentence, and the cost to a person |
| **Move** | Everything an implementer needs, under the heading that owns it |
| **Cut** | Bookkeeping the tracker already holds |
| **Ask** | What the issue does not contain |

**Move, not delete** — this is the whole discipline. File paths, line numbers,
code blocks, the argument against the obvious fix, the history of an abandoned
branch, the design reference, the measurement and its numbers **all survive**.
They move under `## What happened` / `## What is missing`, `## In`, `## Watch
for`, or `## How this was found`. An issue stripped of them sends an agent to
rediscover what somebody already found, at full cost.

Provenance is the largest move and the most common: *"Found by…"*, *"Drawn in
`Journey 6`, frame `6a`"*, *"This was prototyped once, against a tree 373 files
behind today's main"*. All of it goes to `## How this was found`, last.

**Cut, and only these:**

- The title restated as the opening line.
- `**Area:** <name>` — that is a label.
- `Updated 2026-08-31`, `Rewritten 2026-08-31 from the drawing` — edit history.
- Commentary on the filing: *"Filing this small because…"*, *"Recorded here
  rather than filed separately."*
- A `## Related` list of issue numbers with nothing said about them. GitHub
  renders the backlink already. A related issue keeps its line when the sentence
  says what the relationship costs.

Cut nothing else. If a paragraph is not on that list, it moves.

### 4. Rewrite the title only if it names a mechanism

Reword `HealthReport carries launchd intent for Fleet` toward what it lets a
person do. Leave `Drive the app without a mouse` alone — it is already right, and
a rewrite that improves nothing is a notification everyone watching gets.

**Never change what the title denotes.** The subject stays the same subject. Show
the old and new title side by side before writing.

### 5. Ask, rather than invent

**This is the step that keeps the skill honest.** Some fields are recoverable
from the text and some are not, and the unrecoverable one is the one the owner
actually wants.

| Field | Recoverable from the existing issue? |
|---|---|
| The mechanism, file paths, precedent | **Yes.** It is what these issues are made of |
| The wrong fix | **Usually.** `## Watch for`, or a *"Why this is not X"* section |
| Definition of done | **Often.** Derivable from `## In` when absent |
| **Who is hurt, and what they cannot do** | **Often not.** #75, #187, #240 and #250 contain no person anywhere. It cannot be synthesised from a specification of a command palette |
| **Whether it still matters** | **No.** #187 was rewritten once because a decision reversed |
| Priority, milestone, label | **No.** His call, and this skill does not touch them |

When the consequence is not recoverable, ask **one** question, offering the
candidates the text does support, marked as candidates. Use `AskUserQuestion`.
Never write a consequence sentence the issue does not support, and never write
one hedged — a lede that says *"this likely costs"* is the same failure with a
hedge on it.

### 6. Show the diff, then write

Print the proposed title and body. Get a yes. Then:

```
gh issue edit <n> --title "…" --body-file <path>
```

**Save the original to a scratch file first**, and say where. GitHub keeps every
prior body in the issue's edit history, but that history is not reachable from
`gh` and does not survive a transfer, so the local copy is the one you can hand
back within the session.

Do not post the original as a comment. The comment stream is where decisions live
in this repository, and burying them under an archive of text nobody will read
again makes the thing that already works worse.

## What this skill never does

1. **Never edit without having read the comments.** Step 1, and the reason is a
   table of three issues it would have got wrong.
2. **Never drop a file path, a line number, an issue reference or a URL.** Check
   it mechanically: every `` `path` ``, every `#\d+` and every link in the
   original appears in the rewrite, or is named in the summary as deliberately
   cut.
3. **Never resolve a question the issue leaves open.** #294 lists three
   defensible answers and says the code cannot settle it; #209 carries an
   `## Open` section. A reformat that picks one is a decision smuggled in as an
   edit.
4. **Never invent a person, a date, a measurement or a quote.**
5. **Never touch labels, milestone, assignees, project or state.**
6. **Never reformat more than one issue per invocation.** There is no bulk
   migration. An issue is reformatted when somebody picks it up.

## What you tell the owner afterwards

One line: the new title, what was promoted, and anything you had to ask about.
Not a diff summary — he just approved the diff.
