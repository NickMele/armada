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
   [`gate.rs`](../../crates/core/src/fleet/gate.rs). `land`'s gate stops being `branch_exists`, and
   **which of the two it becomes is the repository's `fleet.land.merge` answer** (§6.4): `auto` lands
   on `pr_merged`, `never` lands on `pr_open`.
5. **The audit trail** (§6.5) — a field on the Job carrying every daemon act about it, surfaced by
   `armada fleet show` so the Bridge's detail pane can draw it without becoming a second source, plus
   `~/.armada/daemon.jsonl` for what is about no Job. Written from the first action, not added later:
   a trail that starts once the mechanism works has nothing in it for the failure somebody will
   actually need it for.
6. **CI failure → a fixing step**, carrying the failing checks' logs. This is the same shape as the
   local retry that already works: the gate hands back each failing check's id, message and the last
   forty lines of its log. The only new part is that the log comes from `gh` instead of
   `.armada/run/`.
7. **`main` moved → a fact on every running Job**, and a `fleet_rebase` tool a Drone can call.
8. **Conflict reporting** — a rebase that stops goes back to the Drone with the conflicted paths; a
   Drone that cannot resolve it reports `blocked`, which is an existing state and needs no new word.
9. **`armada doctor` reports the daemon.** Not optional: a Job waiting on a daemon that is not
   running is the exact silent stall this session spent eight hours on. If the daemon is off, every
   Job blocked on a PR must say so on the screen, with the keystroke.

## 6. What bounds the daemon — decided, and it is not one number

**A daemon that reports CI failures to a Drone resumes that Drone, and resuming spends.**
`start_step` calls `start_drone` with `resume_argv`; there is no version of *"report the failures to
the drone to fix"* that does not launch a `claude` session. So this daemon spends unattended, which
the fleet has never done, and per-Job ceilings do not bound it: ten Jobs each inside its own ceiling
still sum to a window.

**Decided: a fleet-wide limit, and on reaching it the daemon keeps merging and stops resuming.**
Merging, pulling, re-running and reaping cost nothing and are the half that unblocks a person, so
they continue; only the spending half stops, and one inbox entry says so. A fleet that stopped
*landing* because it ran out of tokens would strand finished work, which is the defect at the top of
this document.

### 6.1 One limit, several ways of expressing it — because machines are billed differently

**This is the owner's correction, and it invalidates the dollar figures this section first carried.**
His words: *"the way Claude bills, I'm not paying like that. I pay for a plan that gives me a percent
of usage on a rolling 5 hour window. But on my work machine I am limited by $ spend. So two machines
and two different ways of thinking. It needs to be easily configurable with different approaches."*

So a dollar total is a *proxy* on a plan machine and the *actual* constraint on a metered one. The
limit therefore has more than one spelling, and it lives in `~/.armada/machine.yml` — which is
already the right file for exactly this reason: it never syncs, so two machines hold two different
answers without either overwriting the other.

Three spellings, any of which may be set, and **whichever trips first wins**:

| Spelling | For | What it reads |
|---|---|---|
| `stop_on_warning: true` | a plan machine | [`RateLimit::status`](../../crates/core/src/fleet/drone.rs) — `allowed`, `allowed_warning`, `rejected` |
| `stop_at_percent: N` | a plan machine, tighter | `RateLimit::utilization`, floored the way Claude Code floors it |
| `daily_usd: N` | a metered machine | the same spend the fleet already totals |

### 6.2 Why `stop_on_warning` is the default and not a percentage

**`utilization` is `None` in the ordinary case.** It rides along only once a threshold is crossed —
the `allowed_warning` shape — so a window well inside its limits reports its reset and no percentage
at all. That has a hard consequence for a ceiling: **a percentage cannot be enforced below the point
at which the service begins reporting the percentage.** `stop_at_percent: 40` is not a 40% ceiling on
a machine that says nothing until 80; it is an 80% ceiling wearing a smaller number.

`status` is always present, so `stop_on_warning` is the one guard that always evaluates. It is also
the honest one: the service is saying *you are near your limit*, which is a better trigger than any
figure Armada could guess on its behalf.

`stop_at_percent` therefore has a documented rule: it can only ever be **stricter than** the warning,
never looser, and the config must refuse a value it cannot honour rather than accept a number that
does nothing.

### 6.3 The reading is a turn old, and the daemon may overshoot by one

A window's utilisation reaches Armada through a **Drone's own transcript** — each turn reports the
window it passed through — so the fleet only learns where the window stands *after* an exchange. Two
things follow, and neither is a defect to fix:

- **The daemon can overshoot by one exchange.** It resumes on the last reading it has, and that
  resume is what produces the next one. One exchange of overshoot is the floor for any design that
  does not poll the service directly, which is not a thing Armada does.
- **An idle fleet has a stale reading.** If nothing has run for hours the last window fact may be
  from a window that has since reset. `RateLimit::resets_at` is what tells it so, and a reading whose
  window has reset must be treated as *unknown* rather than as the number it last held.

Unknown must mean **resume once and re-read**, not *refuse*: a daemon that will not act on a stale
reading is a daemon that never starts after an idle night, which is the stall this whole design
exists to end.

## 6.4 Auto-merge is a per-repository policy, not a machine one

**The owner's requirement: *"auto merging PRs should be a configuration per manifest. At work I can
not auto merge."*** So the daemon's most consequential action is switchable off, per repository.

**It goes in `armada.yml`, and that is consistent with sending `carry:` the other way.** The test is
the same one [`crates/helm/src/machine.rs`](../../crates/helm/src/machine.rs) applies: is the fact
about *this machine* or about *the thing itself*? A gitignored local path exists on one checkout, so
`carry` is machine-scoped. *"This repository's pull requests may not be merged without a human"* is
true of the repository on every machine and in every clone — it is the organisation's rule, not this
box's — so it belongs with the repository.

**It is a sibling section, not a key inside `manifest:`.** `armada.yml` already carries exactly one
top-level section named for a module, which is the same discipline `machine.yml` follows: one section
per module, and no module reads another's. A `fleet:` section beside `manifest:` is that pattern
rather than an exception to it, and **Manifest never reads it** — which is what keeps
`ARCHITECTURE.md` §1.9's rule intact. Manifest does not learn about agents; the file it shares gains
a neighbour.

The schema has `additionalProperties: false`, so it must be extended to permit the section. That is a
smaller claim than adding a key *inside* `manifest:` would have been, and it is the change the frozen
config contract has to absorb either way once a second module has repository-scoped config.

```yaml
fleet:
  land:
    merge: auto    # auto | never — `never` opens the PR and stops there
```

`never` is not a degraded mode. The daemon still pushes, opens the PR, watches its checks and reports
failures; it simply does not merge, and the Job's `land` step is satisfied by `pr_open` rather than
`pr_merged`. A repository where a human merges still gets everything else this design is for.

**`never` must be the default when the section is absent.** A repository that has said nothing has
not consented to unattended merging, and the failure modes are not symmetrical: a daemon that does
not merge leaves a PR for a person, and a daemon that merges when it should not have cannot be
undone.

## 6.5 The audit trail, and it is on the Job

**The owner's requirement: an audit trail for the daemon, and *"I must be able to see what the daemon
has done in the job detail view of the bridge."*** Those are one requirement, and the second decides
where the first lives.

The Bridge is **a renderer over Fleet and never a second source** — `commands/helm/bridge.md`, and
the rule that stopped its JOBS panel reading transcripts directly. Its Job detail pane draws
`ShowData`, which is `armada fleet show`'s payload. So anything the pane must show has to reach it
through that verb, off the Job's own record. A daemon log the Bridge read directly would be exactly
the second source that rule forbids.

So **every daemon act about a Job is recorded on that Job's record**, and `fleet show` carries them:

| Recorded | Example |
|---|---|
| what it did | `pushed`, `opened`, `checks-green`, `merged`, `pulled`, `re-ran`, `reaped`, `marked-main-moved`, `reported-failure`, `refused-to-merge` |
| what it acted on | the branch, the PR number, the run id |
| when | wall clock, so the pane can age it |
| what came back | the PR's check names and conclusions; the exit status of the re-run |

**Not `transitions`, and not `progress`.** `transitions` records state changes and a push is not one;
`progress` is the Drone's own note channel and mixing another writer into it would make *"who said
this"* unanswerable. A separate field keeps the two voices distinct, which is the same argument
`020` made for `SILENT` being its own word.

**A machine-level log as well, for what is about no Job**: the daemon starting, stopping, reaching its
limit, failing to reach `gh`. `~/.armada/daemon.jsonl`, appended and folded on read, which is the
shape `failures.jsonl` and `inbox.jsonl` already use — so it needs no new mechanism and `armada
doctor` can read it to answer *is the daemon running and what did it last do*.

**The trail is written before the action where the action is irreversible.** A merge recorded only on
success is a trail with nothing in it for the one case somebody will need it for: a merge that
happened and then something went wrong. Record the intent, act, record the outcome.

## 7. The bootstrap, stated

Whoever builds this cannot use it to land it. The first version reaches `main` by hand, and that is
expected rather than an exception to §1.

## 8. What this does not do

- **No approval is removed.** A `feature` workflow still has its `approve` step and a human still
  reads the plan. The daemon acts after the work is reviewed, not instead of the review.
- **No agent merges.** The condition is every check green, which is `gh`'s answer and not a model's.
- **Nothing is force-pushed and no history is rewritten** except by the rebase a Drone asked for, on
  that Drone's own branch.
- **`main` is never committed to directly.** Every change reaches it through a PR that passed CI.
