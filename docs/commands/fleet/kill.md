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

Three steps, **in this order**, and the order is the point:

1. **`armada manifest clean`** in the worktree — releases containers, processes, networks,
   volumes and the port block.
2. **Remove the worktree** (unless `--keep-worktree`).
3. **Mark the Job ended** in the index.

Cleaning before removing means resources are released while the config that describes them is
still present. **If the order is ever reversed, or step 2 happens without step 1, nothing is
lost** — ownership is recorded machine-globally, so `armada manifest clean --orphaned` still
reclaims it afterwards ([`../manifest/clean.md`](../manifest/clean.md)). That safety net is the
reason Manifest sits underneath Fleet.

The transcript is **not** deleted. It lives under `~/.claude/projects/` and is the record of
what happened.

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
