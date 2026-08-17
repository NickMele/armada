---
name: land-branch
description: Leave the work committed on the Job's branch, with a history somebody can read and merge. Use for the `land` step of the `feature` and `bug` workflows.
---

# Land the branch

The last step. The change is written and a reviewer has read it; what is left is to leave the
branch in the state a person can merge without asking you anything.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/land-branch/SKILL.md` changes it.

## Landing is not merging, and it never will be

Your gate is `branch_exists`: the work is committed on this Job's own branch, and `git status` is
clean. **Nothing in Armada merges to the default branch**, and that is deliberate — an agent
merging unattended is the one thing that cannot be taken back.

So the finished state is: a branch, complete, green, and readable. A person decides the rest.

## Commit everything, and check that you did

```
git status --short        # must be empty
git log --oneline main..HEAD
```

An untracked file is the commonest way a Job "finishes" with half its work invisible — a new
source file, a new test, a document the plan asked for. `git status` being clean is the gate's
own second half, so a stray file is a step that fails for a reason that has nothing to do with
the work.

**Do not commit what the repository ignores.** A gitignored file is ignored for a reason, and a
worktree that commits one is a worktree that leaks it.

## Make the history readable

The commits are what the reviewer already read and what a person merges. Before you finish:

- **Squash the noise.** `fix typo`, `wip`, `oops` are steps in your session, not in the
  repository's history.
- **Write messages that say why.** The diff says what changed; the message is the only place the
  reason survives. Name the failure the change fixes, and the measurement if there was one.
- **Do not rewrite what the reviewer read** beyond tidying. If you changed something substantive
  after the review, that is worth saying through `mcp__armada__fleet_report`.

`git push`, `git remote`, `git branch`, `git checkout` and `git switch` are denied to you. Your
branch is yours and no other branch is your business.

## Say where it is

Report through `mcp__armada__fleet_report`: the branch name, what landed on it, and anything a
person needs to know before merging — a follow-up you did not do, a decision you made that they
might reverse, a check that is green only because something unrelated was fixed.

**A `DONE` Job with reviewed work on a branch is the most actionable thing the fleet produces.**
Leave it so that the person reading your report can merge without opening the diff to find out
what you meant.
