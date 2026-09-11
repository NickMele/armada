---
name: agent-worktrees
description: Cutting an agent worktree and giving it back — the cleanup that has to happen when a branch merges, and why a build directory is not the thing to delete. Load before dispatching agents at issues, and before merging their work.
---

# Agent worktrees

**A worktree outlives the agent that asked for it, and nothing reclaims it.**
`armada clean` gives back a *Job's* worktrees under `.armada/worktrees/`. Nothing
gives back an *agent's* under `.claude/worktrees/`, so they accumulate silently
until a disk fills mid-run.

That has happened once, and it cost the session: **74 worktrees, 220 GB, three
agents dead at zero bytes free** with uncommitted work in each. This skill is
what that taught.

## Give it back when the branch merges

**The merge is the moment.** Not "later", not "when disk is low" — a branch that
is in `main` has a worktree that holds nothing that is not also in `main`.

```
git worktree remove --force <path>
git branch -D <branch>
```

Do both. A branch left behind with no worktree is cheap; a worktree left behind
is not.

**Removing the worktree is the fix. Deleting its `target/` is not.** A build
directory rebuilds; a worktree that nobody removes stays forever and takes a new
build directory with it the next time anyone touches it. Clearing `target/` and
leaving the worktree is the move that has to be made twice.

**`Directory not empty` means something is still writing there, and it is
usually you.** After an `isolation: "worktree"` agent finishes, the dispatching
session's working directory moves into its worktree and the session's own
rust-analyzer starts building there. `git worktree remove --force` then
deregisters the tree and leaves a gigabyte behind. Once the three checks below
pass, `rm -rf` what is left. Confirmed 2026-09-11.

## Three things that must survive

Check all three before removing anything. Merged-ness alone is not enough.

| Condition | How to tell | Why |
|---|---|---|
| **An agent is still working in it** | it is running, or its last report said "not committed" | Its edits are on disk and nowhere else |
| **The tree is dirty** | `git status --porcelain` is non-empty | Uncommitted work looks identical to no work from outside |
| **The branch is not in `main`** | `git merge-base --is-ancestor <branch> main`, and where that says no, `gh pr list --head <branch> --state all` | Commits exist only there |

**A PR that merged rebased leaves a branch that fails the ancestor check.**
Confirmed 2026-09-11: two days after its PR merged, `fleet/the-check-record-on-the-wire`
still read as unmerged. An agent reported it as work that would conflict with its
own, and that reached the owner as a rebase still owed. `gh pr list --head`
settled it in one line.

**The dirty check is the one that catches the dangerous case.** An agent that has
written files but not committed is on a branch with *no commits ahead of `main`* —
so it reads as fully merged, and removing it destroys work that was never
anywhere else.

## An agent that cuts no worktree takes somebody else's

**A brief that says to cut one is not evidence that one was cut.** Confirmed
2026-09-09, three times in one session: an agent told to
`git worktree add .claude/worktrees/<name> -b <branch>` ran `git checkout -b`
instead, inside the worktree the dispatching session was already working in. The
dispatching session's `HEAD` moved under it mid-command, and it found out when
`git log` printed somebody else's commits.

**The first time it cost an agent everything it had not committed.** Two actors
in one tree, and the second one's checkout reset the first one's files. That
agent had been told to commit after every step and had not — which is why that
instruction is in every brief, and why it is not sufficient on its own.

Two rules, and the second is the one nobody thinks of:

- **Verify before the first edit, not after.** `git worktree list`, and confirm
  the path you were given is in it and is yours. One command, before anything is
  written.
- **A dispatching session must not assume its own worktree is still its own.**
  Check `git rev-parse --abbrev-ref HEAD` before any command whose meaning
  depends on which branch is checked out — a rebase, a stash, a `checkout --`.
  The session that lost work here had been in that worktree for hours.

**A harness that pins a shell to a worktree makes this worse, not better.** When
the dispatching session moved itself out, every agent still pinned to the old
path had its next command refused, mid-task. Moving is safe only once nothing
else is working there.

## Sweeping when it has already got away

Audit before deleting, and print what will be kept rather than what will go — the
keep list is short and readable, and a mistake in it is visible.

`sed`, `cut` and `sort` have been unavailable in this environment's non-interactive
shell. Prefer a `python3` heredoc over a pipeline for anything that has to parse
`git worktree list`.

## Build size is a separate problem with a separate fix

`[profile.dev]` sets `debug = "line-tables-only"`, which took `target/debug` from
**32 GB to 1.8 GB** on this workspace with every test still passing. Rust links
statically, so without it each of a thousand-odd test binaries carries its own
copy of the full debug info for the whole dependency graph.

**Do not share one `CARGO_TARGET_DIR` between worktrees to save space.** It was
tried: two manifest directories against one target poisoned the incremental cache
and produced phantom link failures on `main` that took a `cargo clean` to clear.
Cargo also locks the directory, so parallel agents would serialise their builds.

## What to say when you cannot clean up

**A blocked agent should stop and report, not free space on its own judgement.**
Two did exactly that when the disk filled, and both were right to: one asked
before deleting a 37 GB `target/`, which was not the problem and would have cost
a cold rebuild for nothing.

Deleting a build directory is recoverable. Deleting a worktree with uncommitted
work is not, and it looks the same from outside.
