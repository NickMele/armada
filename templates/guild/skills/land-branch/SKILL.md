---
name: land-branch
description: Leave the work committed on the Job's branch, with a history somebody can read and merge. Use for the `land` step of the `feature` and `bug` workflows.
---

# Land the branch

The last step. The change is written and a reviewer has read it; what is left is to leave the
branch in the state a person can merge without asking you anything.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/land-branch/SKILL.md` changes it.

## Landing is not merging — but something else now merges

Your gate is `branch_exists`: the work is committed on this Job's own branch, and `git status` is
clean. **You do not merge, and you never will** — an *agent* merging unattended is the one thing
that cannot be taken back.

What changed is what happens after you. This section used to end *"a person decides the rest"*, and
that was the defect: every finished Job piled up against somebody who had to read it, and while that
queue went unread the fleet's output was invisible. [`034`](../../../../docs/reserved/034-the-job-daemon-lands-the-work.md)
gives that job to a daemon, which pushes your branch, opens a PR, and merges it **only** when every
CI check is green. That is a mechanical condition rather than a judgement, which is the whole reason
it is allowed to be automatic and you are not.

Two things follow for you:

- **`git push` is still denied to you, and the daemon pushes.** A push is the first irreversible step
  — it leaves the machine — and the PR's body is written from your Job's own record rather than from
  your summary of yourself.
- **CI can fail after you have finished.** If it does, you will be resumed with the failing checks
  and their logs, exactly as a local gate failure resumes you. That is not a new kind of work; it is
  the same fix-what-the-log-says loop, with the log coming from CI.

So the finished state is: a branch, complete, green, readable, and **pushed by something other than
you**.

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
