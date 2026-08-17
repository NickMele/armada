---
name: explore-codebase
description: Find out how a repository actually does the thing you are about to change, before proposing anything. Use for the `explore` step of the `design` workflow and the `research` step of the `plan` workflow.
---

# Explore before you propose

You are the first step of a workflow whose later steps commit to something. What you learn is
what those steps are built on, and a plan written from a guess is a plan that fails at
`implement` when the guess meets the code.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/explore-codebase/SKILL.md` changes it.

## Your gate is `always`, which means nobody checks this

`explore` and `research` advance when your exchange ends cleanly. There is no predicate, no
artifact, no exit code — the next step simply begins with whatever you leave behind. That makes
this the one step where the quality of the work is entirely on you, and it is why the failure
mode here is *finishing early* rather than failing.

**So the standard is: could the next step be written from what you found, by somebody who has
not read the code?** If not, keep going.

## Read the thing that already does this

Almost nothing is new. Whatever you are about to change, the repository probably already does
something adjacent, and that is the shape to match.

```
git log --oneline -30                    # what has been happening
git log -S'<a distinctive symbol>'       # when this behaviour arrived, and why
rg -n '<the noun the task names>'        # where the concept lives
```

**Read the tests before the implementation.** A test says what the code is *for*; the
implementation only says what it does. A test that pins a strange decision is the fastest
route to why it was made.

**Read the commit that introduced the thing you are about to change.** Its message is often the
only place the reason was ever written down, and changing code whose reason you have not read is
how a fix becomes a regression.

## What to write down

Report through `mcp__armada__fleet_report` as you go — one or two sentences at a boundary, not a
note per file. The next step reads your Job's record, not your transcript.

What the next step needs from you:

- **Where the change goes**, by path, and what else touches it.
- **What already does something similar**, so the new work can match it rather than invent.
- **What would break.** The tests that cover this, the callers you found, the invariant a
  neighbouring comment is protecting.
- **What you could not establish.** An honest gap is usable; a confident guess is not. Say
  *"nothing tells me which of these two paths is live"* rather than picking one silently.

## Do not start the work

You are exploring. The step after you decides what to do, and a design or plan written around
code you have already changed is a plan for a repository that no longer exists.

If the task turns out to be trivial and you can see the whole change, that is worth saying — but
say it, do not do it.
