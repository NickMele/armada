---
id: 034
title: The Job daemon lands the work
status: RESERVED
module: fleet
raised: the orchestrator kept committing on Jobs' behalf, 2026-08-17
---

# 034 — The Job daemon lands the work

**What this is.** The owner's decision, taken after watching the orchestrator commit and merge a
Job's work by hand for the third time in one session: *"Why are you committing on behalf of jobs?
That's it. We need a daemon. Something needs to be monitoring jobs and I don't trust agents."*

Everything below is his design, written down with the decisions it reverses named, the questions it
leaves open surfaced, and one place where I think the shape he sketched can be made safer.

## 1. Why this exists — the measured failure

Over one session the orchestrator hand-landed work three times, and each time the reason was the
same: **a Job had finished something and nothing in Armada would carry it to `main`.**

The worst case is worth stating exactly. Job `worktrees-carry-local-config-plan` implemented a whole
feature — a new `crates/fleet/src/machine.rs` and six modified files, 366 lines, green 5 of 5 in its
own worktree. All of it **uncommitted**. Its workflow's gate is `artifact_exists: PLAN.md`, so
nothing asked it to commit; the single thing standing between those 366 lines and deletion was
[`worktree::holds_uncommitted_work`](../../crates/fleet/src/worktree.rs), the guard inside `armada
fleet reap`. A human read the diff, ran the checks, wrote the commit message and merged it.

That is the defect this design closes, and it is not a defect about committing. **It is that the
last step of every workflow ends in a person.** `land-branch`'s gate is `branch_exists`: a branch, a
clean tree, and then nothing. So every finished Job accumulates against a human who has to read it,
and while that queue is unread the fleet's output is invisible and its worktrees are 2.3 GiB each.

## 2. What it reverses, explicitly

Three standing decisions change. Naming them here is the point: a design that quietly contradicts a
doc leaves two docs disagreeing, which `docs/glossary.md` exists to prevent.

| Was | Now | Why the old reason does not carry |
|---|---|---|
| `ARCHITECTURE.md` §1.9: nothing runs in the background between commands | a daemon runs | The rule existed so **Armada would not assume** a daemon nobody asked for. The owner is asking for one. It is opt-in, per-machine, and off until enabled. |
| `land-branch` SKILL.md: *"Landing is not merging, and it never will be… an agent merging unattended is the one thing that cannot be taken back"* | the daemon merges | **The daemon is not an agent.** It merges on a mechanical condition — every CI check green on a PR — with no model in the loop. The old sentence's actual fear was a *model* deciding a merge was fine; that fear is untouched. |
| a Drone is denied `git push` | still denied — see §4 | Unchanged, deliberately. |

The middle row is the load-bearing one. *"I don't trust agents"* is not an argument for less
automation; it is an argument for the automation to be **a script rather than a judgement**. Every
step below is either a mechanical condition or a report handed to a Drone. Nothing in the daemon
decides whether work is good.

## 3. The landing flow

The owner's sequence, unchanged in substance:

```
Job's land step:  commit → push branch → open PR
                                            ↓
                              daemon watches the PR's checks
                                            ↓
                    ┌──────── all green ────────┐──── any red ────┐
                    ↓                                             ↓
              merge the PR                            report the failing checks,
                    ↓                                 with their logs, to the Drone
              pull main locally                                    ↓
                    ↓                                     Drone fixes and pushes;
              re-run checks on main                       the daemon watches again
                    ↓
              reap the Job's workspace
                    ↓
              tell every running Job that main moved
                                            ↓
                    a Drone may then ask the fleet to rebase its workspace
                                            ↓
                    daemon runs the rebase; conflicts go to the Drone to resolve;
                    a conflict it cannot resolve is reported to the fleet as `blocked`
```

**Re-running checks on `main` after the merge is not redundant.** CI passed on the PR's merge
commit, which is the branch against a `main` that may have moved since. The re-run is the only thing
that catches a semantic conflict between two PRs that each passed alone — the case where both merges
are green and their combination is not.

**The rebase is asked for, never imposed.** The daemon marks that `main` moved; the Drone requests
the rebase on its own next turn. Rewriting a branch under a Drone that is mid-exchange would destroy
work in progress, and the daemon cannot know whether a turn is in flight without asking the same
process table `020` already showed is not a reliable answer. The owner specified it this way and it
is the right instinct.

## 4. Where I would change the shape: the daemon pushes, not the Drone

The sketch says *"push PR"* without saying who pushes. **It should be the daemon, and the Drone
should stay denied `git push`.**

The Drone commits locally, as it does now. The daemon pushes the branch and opens the PR.

- It keeps the outward-facing action in the one component the owner said he trusts. A push is the
  first irreversible thing in this flow — it leaves the machine — and `git push` is currently denied
  to a Drone for exactly that reason.
- It needs no posture change, so no Job gains a capability it did not have. A Drone that could push
  its own branch could push any branch its argv can name, and the deny list is
  [checked by prefix](../../crates/core/src/fleet/drone.rs), not by argument.
- The daemon pushing is also the only way the PR's *body* can be written from the Job's own record —
  the task, the plan, the steps it passed — rather than from a Drone's summary of itself.

The cost is that a Drone cannot see its own PR until the daemon has made one. That is acceptable:
the Drone's contract already ends at *"I have stopped working on this step"* (`032`), and the PR is
the Job's artefact, not the exchange's.

## 5. What has to be built

Nothing here is a small change. Roughly in dependency order:

1. **`armada daemon`** — the process, and `enable`/`disable`/`status` around it. A launchd job on
   macOS. Per-machine, so its switch belongs in `~/.armada/machine.yml` beside `helm.enter`, for the
   reason [`crates/helm/src/machine.rs`](../../crates/helm/src/machine.rs) already gives: whether
   *this* box may act unattended is a fact about the box.
2. **A budget the daemon cannot cross.** See §6 — this gates everything else.
3. **Push and PR** — `gh pr create` from the Job's record. Needs a remote; a repository without one
   must fail legibly at the `land` step rather than at the daemon.
4. **Two gate predicates** — `pr_open` and `pr_merged`, joining the eight in
   [`gate.rs`](../../crates/core/src/fleet/gate.rs). `land`'s gate stops being `branch_exists`.
5. **CI failure → a fixing step**, carrying the failing checks' logs. This is the same shape as the
   local retry that already works: the gate hands back each failing check's id, message and the last
   forty lines of its log. The only new part is that the log comes from `gh` instead of
   `.armada/run/`.
6. **`main` moved → a fact on every running Job**, and a `fleet_rebase` tool a Drone can call.
7. **Conflict reporting** — a rebase that stops goes back to the Drone with the conflicted paths; a
   Drone that cannot resolve it reports `blocked`, which is an existing state and needs no new word.
8. **`armada doctor` reports the daemon.** Not optional: a Job waiting on a daemon that is not
   running is the exact silent stall this session spent eight hours on. If the daemon is off, every
   Job blocked on a PR must say so on the screen, with the keystroke.

## 6. The open question, and it is the owner's

**A daemon that reports CI failures to a Drone resumes that Drone, and resuming a Drone spends
money.** `start_step` calls `start_drone` with `resume_argv`; there is no version of "report the
failures to the drone to fix" that does not launch a `claude` session.

So this daemon spends unattended, which is the one thing the fleet has never done. Per-Job ceilings
are not sufficient: ten Jobs each inside its own ceiling still add up to a weekly window, and this
session put 87% of one through the fleet in a day with a human watching every spawn.

What is needed before the daemon may resume anything:

- a **fleet-wide** budget the daemon refuses to cross, distinct from any Job's;
- a decision about what it does on reaching it — stop and raise one inbox entry, or keep merging
  green PRs (which costs nothing) while refusing to resume Drones (which costs);
- and whether a daemon that has stopped spending should still merge, pull, re-run and reap. I think
  yes: those are free, they are the half that unblocks a human, and a fleet that stops landing
  because it ran out of tokens is a fleet whose finished work is stranded again — which is the
  defect at the top of this document.

The numbers to set it from are on this machine already: `$65.39` for one `feature` Job with its
sub-Jobs, `$148.82` across a day of 41 Jobs, and a seven-day window that reached 87%.

## 7. What this does not do

- **No approval is removed.** A `feature` workflow still has its `approve` step and a human still
  reads the plan. The daemon acts after the work is reviewed, not instead of the review.
- **No agent merges.** The condition is every check green, which is `gh`'s answer and not a model's.
- **Nothing is force-pushed and no history is rewritten** except by the rebase a Drone asked for, on
  that Drone's own branch.
- **`main` is never committed to directly.** Every change reaches it through a PR that passed CI.
