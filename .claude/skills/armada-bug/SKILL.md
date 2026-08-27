---
name: armada-bug
description: Report something Armada got wrong, and decide there and then whether it is fixed now or filed for later. Load when the owner reports a defect, pastes an error, or shares a screenshot of something not working.
---

# Filing a bug

**Something is wrong and the owner just said so.** This is the loop from his
sentence to either a fix in flight or an issue that will still make sense in
three weeks.

**One rule above the rest: verify the claim before writing it down.** A bug
report is a claim about the code, and a wrong one is worse than none — it sends
an agent to fix something that is not broken, and it survives in the issue
tracker as a fact. **This has already happened.** Three "gaps" filed in one
session turned out to be operations already declared in `crates/ipc/operations.toml`
and simply not routed, and a claim that a workflow's four steps were all ungated
was a generalisation from reading one of them.

## The loop

### 1. Reproduce it, or say you could not

Run the thing. Read the code the report names. **What the owner saw is
evidence; what the code does is the finding.** He reports symptoms — a truncated
label, a screen that will not load, a Drone that sits still — and the symptom
almost never names its own cause.

Where a symptom has more than one plausible cause, **isolate before writing.** A
Drone that says `Not logged in` was not a credential problem: the Keychain read
fine under the Drone's exact environment, and the cause was one missing
variable, found by bisecting the environment rather than by reasoning about it.

Where you genuinely cannot reproduce, say so in the report in those words. An
unreproduced bug is still worth filing; an unreproduced bug filed as though it
were confirmed is not.

### 2. Sort it before you write it

Four things wear the same clothes, and only one of them is a bug.

| What it is | How you can tell | Where it goes |
|---|---|---|
| **A bug** | The code does something other than what it says it does | An issue, `bug` |
| **Unbuilt** | It was specified and never built. `operations.toml` declares forty operations and serves a subset — one with no route is *not yet built* rather than wrong | An issue naming the existing declaration, `step` |
| **A stale document** | The code is right and the prose is behind it | Fix the prose. Do not file it |
| **My error** | The claim is wrong | Say so plainly, correct it, file nothing |

### 3. Write it so it survives

The owner will read this weeks from now, and so will an agent with none of
today's context. Follow the shape the repository's issues already use:

- **`## What happened`** — the symptom in his words, quoted, and what the code
  actually does. Include the exact error text and the commands that show it.
- **`## Why it matters`** — the consequence, not the severity. "A person who
  cannot approve from the detail view has to go back to the list" beats "high
  priority".
- **`## In`** — what has to change, and **what already exists that it builds
  on.** Name the file and the line. An issue that makes the reader rediscover
  what you already found wastes the work.
- **`## Watch for`** — the wrong fix. Every bug has one, and it is usually the
  first thing that comes to mind.
- **`## Definition of done`** — one sentence, checkable.

**Write down the reasoning, not just the conclusion.** The best issues in this
repository say *why* the obvious fix is wrong — that is what stops it being
tried again.

### 4. Ask: now, or later

**This is his decision and it is never assumed.** Use `AskUserQuestion` with
concrete options, a recommendation first, and enough context that he is not
guessing at cost.

What to weigh, and to say out loud:

- **Is anything holding the files?** A running agent owning `crates/api` means a
  second one in the same route table is a merge conflict, not parallelism. Say
  which files are contested rather than dispatching into them.
- **Does it block him right now?** A bug he is looking at beats one he is not.
- **How big is it really?** "One line plus a test" and "a schema migration" are
  different answers to the same question, and he cannot tell them apart from the
  symptom.

Offer, at minimum: **fix it now**, **file it**, and **file it and fix it** —
because an issue is the record even when the fix is immediate, and a fix with no
issue leaves nothing for the changelog or the milestone to point at.

### 5. Do what he chose

**Filed:** `gh issue create` with a milestone and a label. `bug` for a defect,
`step` for something unbuilt. Milestone by when it has to be true — `M1 — Dogfood`
for anything the current milestone's claim depends on, and a later one otherwise.
Give him the URL.

**Fixed now:** dispatch with the file ownership stated, because agents collide.
Give the agent the reproduction and the reasoning, not just the symptom — an
agent handed a symptom rediscovers the cause you already found, at full cost.

## What not to do

**Do not file a bug the owner did not report**, unless you found it while
verifying one he did. His standing rule is that a person defers the thing; an
agent may propose and may not file on its own judgement. Finding a second defect
while chasing the first is exactly the case where you propose it in the same
breath and let him decide.

**Do not file the same bug twice.** Search first — `gh issue list --search` —
and say when something is already tracked.

**Do not soften it.** "The workflow rail does not match its design" is the
finding. "The workflow rail could be improved" is not.

**Do not fix a symptom you have not explained.** A change that makes the report
go away without a cause you can name is a change that will come back.
