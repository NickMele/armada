---
name: app-walkthrough
description: File issues while the owner walks Bridge and says what is wrong with it — a bug, a rough edge or something missing — one at a time, worked out with him before anything is filed. Load when he says he is going to use the app and bring you what he finds.
---

# Filing from a walkthrough

**He is using the app and telling you what is wrong with it.** You are the one
who knows the code. Each thing he says becomes an issue an agent can build from,
and nothing is filed until the two of you agree what it is.

This skill is the loop. It does not restate the rules it leans on:

| For | Read |
|---|---|
| Verifying a claim before it is written down, and sorting a bug from a stale document | `armada-bug` |
| The shape of the body and the title | `docs/practices/writing-an-issue.md` |
| Wording a question so he can answer it | `asking-a-person` |

## Before the first report

**Know what is running.** Which build Bridge is on, and whether Fleet is up.
Either one being behind `main` makes a report about a merged fix look like a new
bug. `git log -1 origin/main` and the Fleet's health say which.

**Load the open issues once.** `gh issue list --state open --limit 300 --json
number,title,labels,milestone` in a scratch file, so the duplicate check for
each report is a grep and not a round trip.

## Each report

### 1. Find it in the code before you say anything back

He names a surface and a symptom. Find the component that draws it and the data
behind it, and read enough to say **why** it does what he saw. `armada-bug`
step 1 governs: what he saw is evidence, what the code does is the finding.

Where it helps to see it, look. `armada-local` says how to look at Bridge
without stranding it on his screen.

### 2. Search for it

Grep the scratch list, then `gh issue list --search "<words>" --state all`.
**A closed issue that matches is the most useful thing you can find** — it is
either a regression or a fix that has not reached his build.

| Found | Say |
|---|---|
| Open, and it covers this | The link, and whether his report adds a fact worth a comment he would post |
| Closed, fix merged | Whether his build has it. If it does, it is a regression and gets filed as one, citing the old issue |
| Nothing | Carry on |

### 3. Sort it

`armada-bug` step 2 sorts a defect from unbuilt work, a stale document and your
own error. A walkthrough brings two more kinds:

| What it is | How you can tell | Label |
|---|---|---|
| **A rough edge** | The code does what it says, and what it says is wrong for a person using it | `surface` |
| **Missing** | Nothing does it, and nothing declared it | `idea` until he places it, or `step` in a milestone he names |

**A missing feature has a Fleet half and a Bridge half.** File both together, or
say why one does not exist. A wire change alone leaves him with `curl`.

### 4. Ask what the code cannot answer

**Talk before filing.** Put the questions to him before a draft exists, with
`AskUserQuestion`, in the words `asking-a-person` gives. Ask only what is his:

- **What he expected to see.** The code tells you what it does, never what he
  wanted.
- **Whether it is one issue or several.** A report of a screen can be three
  defects with three fixes.
- **Where it belongs.** The milestone, when it is not obvious from what is open.

Do not ask what you can read. *"Which component is this?"* is yours to find.

### 5. Ask now or later

`armada-bug` step 4, every time: **fix it now**, **file it**, **file it and fix
it**, with what is holding the files and how big it really is. He chose to be
asked on every report, not only the small ones.

A fix started mid-walkthrough runs in the background. The walkthrough keeps
moving.

### 6. Show the draft, then file

Print the title, labels, milestone and body. Get a yes. Then:

```
gh issue create --title "…" --label <label> --milestone "<title>" --body-file <scratch path>
```

`surface` belongs to no milestone, so drop `--milestone` for it. Give him the full link, so it opens from the terminal.

## Across the session

**Keep a table** of every report — the link, or why nothing was filed. When he
says he is done, that table is the reply, with an **Owner** and a **Next action**
column.

**One report at a time.** When he brings several in one message, say which you
are taking first and hold the others. Two drafts in flight is how a fact from
one ends up in the other.

**Do not file what he did not report.** Something you find while chasing his
report is proposed in the same breath, and filed only on his yes.
