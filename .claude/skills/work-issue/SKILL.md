---
name: work-issue
description: How to work one GitHub issue the way a Job works a workflow — worktree, plan, implement, test, commit, merge. Load before starting any issue, and before dispatching agents at several.
---

# Working one issue

**This is the `bug` workflow, run by hand.** Armada dispatches a Drone into its
own worktree, gates each step, and holds the work at `awaiting_review` before it
lands. When Fleet is not the one dispatching, that shape still applies, and this
skill is it.

`milestone-step` owns how to read an issue, what to check it against, and how to
close it. **This skill owns where the work happens and how it lands** — the two
things that skill does not say, and the two that went wrong.

## Why this exists

On 2026-08-28 ten issues were worked in one working tree on `main`. At the end
none of it could be committed per issue: `gate.rs` carried two, `work_product.rs`
carried two, `routes.rs` carried three, and splitting by hunk would have produced
commits that did not compile. A Job-proposer change that had been sitting
uncommitted since before the session was swept into somebody else's commit by a
single `git add crates/fleet`.

**Nothing was lost and the tests were green.** The cost was entirely in the
history — which is to say, entirely in what the next person can reconstruct.

A Drone never has this problem, because Fleet decides where it works and it
works on one thing. The rule is not "be careful"; it is that the isolation is
not the agent's to arrange.

## The loop

### 1. Worktree, before reading the issue

**Branch and worktree first, every time, including for a change you are sure is
one line.** The judgement about how big a change is comes after you have read the
code, and by then the tree is already dirty.

Use `EnterWorktree` where it is available. Worktrees live under
`.claude/worktrees/`, named for the issue. `.armada/worktrees/` is Fleet's and is
never touched by hand — a Drone is working in there.

**Never work on `main`.** Not for a doc fix, not for a comment.

**One issue per worktree.** Two issues in one tree is the defect above, arriving
early.

### 2. Plan

`milestone-step` steps 1, 2 and 2.5 are the plan: read the issue in full, read
what it disagrees with, read the registry before minting anything. Do not repeat
them here; load that skill.

One addition, from tonight. **Check the source the issue points you at before
trusting it.** Two Jobs failed against #118 because it said to port from
`crates/core-model/domain/workflow-samples/`, and those samples disagree with
the parser in three ways — `config/src/judge.rs` had said so in its own module
docs and nothing had acted on it. An issue is a claim like any other.

### 3. Implement

`milestone-step` step 3. One issue. Finish it, and stop.

### 4. Test

`milestone-step` step 4, and it is not optional because the change looks small.

The Checks this repository actually gates on are in `armada.yml`, and running
them yourself is running what Fleet would run:

| | |
|---|---|
| Build | `cargo build --workspace --locked` |
| Tests | `cargo nextest run --workspace --exclude acceptance` |
| The milestone's own claim | `cargo test -p acceptance` |
| Bridge | `pnpm -C apps/desktop typecheck`, `build`, and `pnpm -C packages/components build-storybook` |
| The gate | `cargo xtask verify-foundations` — **red on purpose**, so read what each line names rather than chasing green |

**Verify it yourself rather than on a report.** An agent's claim of green has
been wrong here.

### 5. Commit

Read `.claude/skills/commit-message/SKILL.md`. Say what the diff cannot.

**Commit at each step, not at the end.** A worktree that has been running for an
hour with nothing committed is the tree this skill exists to prevent, one scope
smaller.

**`git add <path>` takes what is under that path, including files you did not
write.** Stage by name, or read `git status` first and know every entry. That is
how someone else's uncommitted work ended up inside a commit about something
else.

### 6. Merge, and let it be reviewed

The commit lands on the branch. **Whether it merges is not the agent's call** —
which is exactly `human_always` on `handoff`, and the reason six of the seven
shipped workflows now stop before landing.

Open a PR or hand back the branch, and say what you would want looked at
closely. Then `milestone-step` steps 5, 6 and 7: close the issue with what
contradicted the plan, give every open item an owner, report.

## Dispatching several agents at once

**Write scope is reserved by hand, because #47 is not built.** Nothing stops two
agents editing the same file, and the second one wins silently.

Before launching, write down each agent's scope and check the sets are disjoint.
The split that worked was by crate boundary and by side of the seam:

| agent | scope |
|---|---|
| one | `crates/config`, `crates/adapters` |
| two | `crates/ipc`, `crates/api`, `crates/fleet` |
| three | `apps/desktop`, `packages` |

Then say so in the prompt — *"another agent is working in X in parallel; do not
touch it; if your change needs one, stop and report it rather than making it."*
Every agent given that sentence obeyed it.

**Two issues that both land in `JobDetail.tsx` do not run in parallel.** They run
in sequence, and the one that decides the arrangement runs first.

**Pass the owner's rules down.** Report bottom line first, be brief, no
unnecessary caveats, tables over paragraphs for anything comparative, label every
finding and table row with who acts on it, and surface any question as a single
`**QUESTION:**` line at the end rather than burying it in prose.

## What this skill does not do

**It does not replace dispatching a Job.** When Fleet can run the work, run the
work — hand-landing what a Job should have done hides every gap in the fleet, and
that is how a milestone gets marked complete while nothing can reach it. Use this
when Fleet cannot: for changes to Fleet itself, when a Drone has failed at
something twice, or when the owner says to.
