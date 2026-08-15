# `armada fleet kill`

End a Job and release everything it owns.

> **Status: built — M3.**

## Synopsis

```sh
armada fleet kill <job> [--keep-branch] [--keep-worktree] [--json]
armada fleet kill --all-finished [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name | — | Which Job. Required unless `--all-finished`. |
| `--keep-branch` | flag | off | Do not delete the branch. Use when the work is worth keeping. |
| `--keep-worktree` | flag | off | Release resources but leave the directory. Implies `--keep-branch`. |
| `--all-finished` | flag | off | Kill every Job whose workflow has terminated, instead of one. |

**A Job or `--all-finished`, never both and never neither.** Naming one Job and asking for all
of them are two different requests, and picking one for you could kill four Jobs you did not
name.

## How it works

Four steps, **in this order**, and the order is the point:

1. **Stop the Drone** — SIGTERM, a grace period, then SIGKILL, and only if the recorded process
   group is provably still the one Armada started. It goes first because it is still working:
   a live Drone mid-`docker compose up` would otherwise race the teardown of the very
   resources it is creating, and lose.
2. **`armada manifest clean`** in the worktree — releases containers, processes, networks,
   volumes and the port block.
3. **Remove the worktree** (unless `--keep-worktree`).
4. **Mark the Job ended** in the index.

Cleaning before removing means resources are released while the config that describes them is
still present. **If the order is ever reversed, or step 2 happens without step 1, nothing is
lost** — ownership is recorded machine-globally, so `armada manifest clean --orphaned` still
reclaims it afterwards ([`../manifest/clean.md`](../manifest/clean.md)). That safety net is the
reason Manifest sits underneath Fleet.

The transcript is **not** deleted. It lives under `~/.claude/projects/` and is the record of
what happened.

### The Job is marked ended whatever happened above

**Nothing in steps 1–3 is raised.** A resource that will not release, a `git worktree remove`
that refuses, an `armada manifest clean` that will not even start — each is reported on that
Job's row and the kill carries on. A kill that bailed out would need a second kill to do the
same thing again, and by then it has already stopped the Drone.

**A worktree that is already gone is not asked to clean itself**, and is not a failure. There is
no directory to resolve an `armada.yml` in and nothing to release from inside one; what the Job
owned is recorded machine-globally, so [`../manifest/clean.md`](../manifest/clean.md) `--all`
reclaims it. The row says `GONE`.

> **This was raised once, and the symptom is why it is stated here.** `ENOENT` on a spawn has
> two meanings — the program is missing, or the working directory is — and a kernel answers both
> with the same errno. Reporting both as a missing binary told somebody to *reinstall Armada*
> because a worktree had been deleted, ended the screen they were watching, and left the Job
> `RUNNING` in the record with nothing left that could end it.

### A name that means two Jobs is refused

A uuid is identity; **a name is a label, and nothing enforces that a label is unique**. Armada
only refuses to reuse the name of a Job that is still *live*, so a finished `rate-limit` and a
running one are both ordinary and both on disk. Resolution used to take the older of them and
say nothing about the choice, which is a `kill` aimed at a Job you did not mean.

```
error: `this-test` names 2 Jobs: c19d0a34 ABORTED at explore, 2026-08-15;
       94b1fd2e RUNNING at plan, 2026-08-15
  next:  name one by uuid: `c19d0a34`
```

**A live Job does not win the tie**, and that is deliberate. Preferring it would be the same
coin flip with better odds: the person who typed a name that means two things knows which one
they meant and Armada does not, and a kill is not undoable. The uuid — or any unique prefix of
it — always works, and every candidate is named with the state, the step and the day that tell
them apart.

The live screen is unaffected: **every key on [`../helm/bridge.md`](../helm/bridge.md) carries
the uuid of the row the cursor is on**, so `x` never has a name to be ambiguous about.

## Output

```
  STATUS   JOB         DETAIL                                        TIME
  cleaned  rate-limit  3 containers, ports 5470-5479                    -
  removed  rate-limit  worktree ~/.armada/workspaces/api/rate-limit     -
  removed  rate-limit  branch armada/rate-limit                         -

CLEAN  1 job, transcripts kept
```

**A Job's branch is namespaced `armada/<name>`**, which is what makes deleting it safe: a Job
given a bare `rate-limit` could delete a branch a person was working on.

**A worktree that is already gone reports `gone`, not a failure.** A Job whose directory
somebody deleted by hand is exactly the Job the durable record exists for
([`../../PLAN.md`](../../PLAN.md) §14.1).

`--json` returns the clean results plus the worktree and branch disposition.

## Dependencies

The Job index, and whatever [`../manifest/clean.md`](../manifest/clean.md) needs.

## Exit codes

`0` killed · `1` `tool_failed` — something would not release; **the Job is still marked ended**, and `armada manifest clean --orphaned` reclaims the remainder · `2` `bad_invocation` — unknown Job.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`../manifest/clean.md`](../manifest/clean.md) · [`ls.md`](ls.md)
