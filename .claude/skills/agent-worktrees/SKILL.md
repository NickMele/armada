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

## Three things that must survive

Check all three before removing anything. Merged-ness alone is not enough.

| Condition | How to tell | Why |
|---|---|---|
| **An agent is still working in it** | it is running, or its last report said "not committed" | Its edits are on disk and nowhere else |
| **The tree is dirty** | `git status --porcelain` is non-empty | Uncommitted work looks identical to no work from outside |
| **The branch is not in `main`** | `git merge-base --is-ancestor <branch> main` | Commits exist only there |

**The dirty check is the one that catches the dangerous case.** An agent that has
written files but not committed is on a branch with *no commits ahead of `main`* —
so it reads as fully merged, and removing it destroys work that was never
anywhere else.

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
