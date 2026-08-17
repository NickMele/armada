---
name: review-diff
description: Review the commits on an Armada Job's branch against the task it claims to do, and write REVIEW.md saying what would block landing it. Use for the `read` step of the `review` workflow, or whenever asked to review a branch a Drone produced.
---

# Review a branch

You are reviewing work **somebody else's session did**, on the branch checked out in your
worktree. You have not seen how it went, and that is the point: a reviewer that shares the
implementer's context shares its blind spots (PLAN.md §14.6).

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/review-diff/SKILL.md` changes it.

## Read the change, not the repository

The commits this branch added are the change. Everything else is the repository as it already
was, and reviewing that spends a budget on code nobody touched.

```
git log --oneline -30        # the commits on this branch
git show <sha>               # each one
git diff <first-sha>~1..HEAD # all of it at once, once you know where it starts
```

## What blocks landing, and what does not

Say the first only. The task you were given is the standard: work that does what it was asked
lands, and work that does something else does not, however well written it is.

| Blocks | Does not |
|---|---|
| It does not do what the task asked | You would have named it differently |
| It breaks something that worked | It is not the structure you would have chosen |
| A test asserts the bug rather than the fix | There is no test for a case nobody asked about |
| An error is swallowed where it mattered | A comment could be longer |

## Write `REVIEW.md` at the root of the worktree

It is the artifact the step is gated on, and it is what a person opens afterwards. Lead with
the answer:

```markdown
# Review — <branch>

**<Nothing blocks landing this.|N things block landing this.>**

## Blocking
- <what is wrong, where, and what it breaks> — `path/to/file.rs:120`

## Noted, not blocking
- <the rest, briefly>
```

**If nothing blocks it, say so in those words and write the file anyway.** The empty review is
the common case and the file is the evidence that somebody looked.

## A blocking finding has to stop the Job, not just be written down

**Writing it in `REVIEW.md` is not enough, because nothing reads the file.** The step you are
running is gated on `artifact_exists: REVIEW.md` — the file existing is what proves a reviewer
looked — and the Job under review advances on *your Job's verdict*, not on your prose. So a
review that says *"one thing blocks landing this"* and then finishes cleanly lands the work it
just refused. That happened: a reviewer named a real defect with file and line numbers, returned
`PASS`, and the Job it was reviewing went straight to `land`.

**So if the `## Blocking` section has anything in it, call
`mcp__armada__fleet_ask_human` with the blocking findings as the question, before you finish.**
That is what holds the Job under review short of landing until somebody answers. One call, with
every blocker in it — not one per finding.

If the `## Blocking` section is empty, finish normally. That is the pass.

## When the finding is genuinely a person's

Something you cannot judge from the diff — the task is ambiguous, the change is right but the
approach commits everyone to something — is the same call, for a different reason: not *this is
wrong* but *this is not mine to decide*. Say which of the two it is in the question, because the
answer that unblocks them is different.
