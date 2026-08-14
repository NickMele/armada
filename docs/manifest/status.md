# `armada manifest status`

What is running, what is mine, what is stale.

> **Status: shipped** — as `char status` today; renames in M1 ([`PHASES.md`](../PHASES.md) §8.3).

## Synopsis

```sh
armada manifest status [-C <path>] [--all] [--stale] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `-C <path>` | directory | cwd | Report on this workspace. |
| `--all` | flag | off | Every workspace on the machine, not just this one. |
| `--stale` | flag | off | Only workspaces whose directory no longer exists. The input to `clean --all`. |

## How it works

Reads the machine-global store and reconciles it against reality: for each recorded resource it
checks whether the thing still exists. That reconciliation is the point — a store that is
trusted blindly drifts, and a filesystem scan alone cannot attribute what it finds.

Three states per resource: **live** (recorded and present), **stale** (recorded and gone), and
**unowned** (present, and not recorded by any workspace).

## Output

```
workspace  api (a3f2c1)   ~/work/api
ports      41200–41209
live       postgres:5432 · redis:6379 · web pid 41822
stale      —
```

`--json` returns one result per resource with `kind`, `id`, `state`, and the owning workspace.
Read-only in every mode.

## Dependencies

`~/.armada/manifest.db`, plus a container runtime to reconcile container-backed resources.
Works without `armada.yml`.

## Exit codes

`0` whenever the store could be read — status reports, it does not judge, so a stale resource is *not* a non-zero exit · `6` `environment` — the store is unreadable.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`clean.md`](clean.md) · [`explain.md`](explain.md)
