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

**Then fetch, and fast-forward onto what `main` is now.** `EnterWorktree` cuts
from the local `origin/main` ref, which is only as fresh as the last fetch.
Confirmed 14 Sep 2026: the #1001 worktree came up at a commit from before #999
merged, missing the code the step was built on. `git fetch origin main && git
merge --ff-only origin/main` before reading anything.

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

**Run what your change can affect, once.** The Checks this repository gates on
are in `armada.yml`, and each one's `when:` list names the files that make it
apply. Confirmed 13 Sep 2026: a session ran every row of the old table on every
branch and again after every rebase, beside another session's test run, and the
owner's machine was unusable.

| You changed | Run |
|---|---|
| A crate under `crates/` | `cargo nextest run -p <crate>` for it and each crate that depends on it (`cargo tree -i <crate> -e normal --depth 1`) |
| Any Rust | `cargo fmt --all --check`, and `cargo build --workspace --all-targets 2>&1 \| grep -c '^warning'` once — **the same count as `main`**, whatever the exit code |
| What a milestone's claim reads | `cargo test -p acceptance` |
| `apps/` or `packages/` | `armada check typecheck`, and `pnpm exec vitest run <files>` for the tests and stories you touched. **Every story, `screens`' included, runs from `-C packages/components`**; `packages/screens` has only its `.test.ts` and `.test.tsx` projects, and a story filter there finds no files. `bridge_build` and `storybook` only where their `when:` matches |
| `docs/`, or `crates/ipc/operations.toml` | `cargo xtask verify-docs` |
| Anything | `cargo xtask verify-foundations` once, before the PR — **no worse than the baseline you took off `main`.** Read what each line names; never chase a colour |

**One heavy run at a time, across every session on the machine.** Never start a
build or a test suite while another is running, your own background runs
included.

**Warnings and Format were missing from this table, and a merge paid for it.**
Confirmed 12 Sep 2026: #843 ran every row above, merged green, and left `main`
with ten unformatted hunks and an unused import. `format` is a Check in
`armada.yml`, so the next Job cut from `main` would have failed on work that
was not its own.

**Verify it yourself rather than on a report.** An agent's claim of green has
been wrong here.

**A filtered Check that prints nothing did not pass.** `armada check` reads
`armada.yml` from the working directory. Confirmed 14 Sep 2026 on #1117: a
typecheck run from `packages/components` could not read the file, and the
`grep` for `passed|failed` around it printed nothing at all. Run Checks from
the repository root, and read a silence as nothing having run.

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

**A rebase re-opens the gate, and step 4's run does not survive one.** The gate
reads the merged tree, and a branch and `main` can each sit under a limit that
the two together cross. Confirmed 2026-09-12: #730 passed `verify-foundations`
at 898 lines in `packages/screens/src/JobDetail.tsx`; `main` grew the same file
by seven while the branch was open, and the rebase landed it at 905 — over the
900-line rule. It merged on its own green measurement and left `main` red, with
nothing to raise it until somebody ran the gate by hand. **After `git rebase`,
rerun `verify-foundations` before the force-push when the commits you crossed
touched a file you changed or a rule under `xtask/`; otherwise rerun nothing.**
Where a file is near a threshold, leave headroom rather than sitting on it.

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

**Tell every agent to wait for its own runs in the foreground.** An agent is
woken only by a message, never by its background build finishing. Confirmed
14 Sep 2026: the Fleet agent for #1105 ended its turn three times with nextest or
`cargo build` still running, and sat idle with nothing committed until it was
watched and woken by hand each time. Put *"run heavy commands in the foreground
and wait; never end your turn while a run is in the background"* in the brief.

## Give the worktree back

**The merge is the moment.** A branch that is in `main` has a worktree holding
nothing `main` does not, and nothing reclaims it on its own — `armada clean`
gives back a *Job's* worktrees, not an *agent's*.

```
git worktree remove --force <path>
git branch -D <branch>
```

**Whoever merges does this**, in the same breath as the merge. Left undone it is
invisible until a disk fills: seventy-four worktrees and 220 GB, three agents
killed mid-run at zero bytes free, each with uncommitted work.

**Never remove one without checking it is clean.** An agent that has written
files and not committed sits on a branch with no commits ahead of `main` — so it
reads as merged, and removing it destroys the only copy. `git status --porcelain`
is the check that catches it.

`agent-worktrees` has the rest: what to keep, how to sweep safely when it has
already got away, and why sharing one build directory between worktrees was tried
and is not the answer.

## What this skill does not do

**It does not replace dispatching a Job.** When Fleet can run the work, run the
work — hand-landing what a Job should have done hides every gap in the fleet, and
that is how a milestone gets marked complete while nothing can reach it. Use this
when Fleet cannot: for changes to Fleet itself, when a Drone has failed at
something twice, or when the owner says to.
