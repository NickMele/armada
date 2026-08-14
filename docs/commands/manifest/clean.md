# `armada manifest clean`

Release everything this workspace owns.

> **Status: shipped.**

**The property no other tool has:** clean works **after the directory is gone**. Ownership is
recorded machine-globally, so reclaiming is a query rather than a memory. `docker compose down`
needs the file; this does not.

## Synopsis

```sh
armada manifest clean [-C <path>] [--workspace <id>] [--all] [--dry-run] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `-C <path>` | directory | cwd | Clean this workspace. |
| `--workspace <id>` | workspace id | — | Clean by id, for a workspace whose directory no longer exists. Get ids from [`status.md`](status.md). |
| `--all` | flag | off | Clean every workspace on this machine whose directory is gone. Does not touch live workspaces. |
| `--dry-run` | flag | off | List what would be released. Changes nothing. |

## How it works

Releases, in this order, everything stamped with this workspace:

1. **Processes** — by process group, so no orphans survive. This is why process groups are
   load-bearing and why Armada is POSIX-only.
2. **Containers, networks, volumes, images** — found by label, not by name.
3. **The port block** — returned to the pool.
4. **Leases** — any machine-wide claims released.

Every resource carries the workspace stamp that `init` applied, so each step is a query against
the store rather than an inference from the filesystem.

`--all` is the garbage collector: it releases only workspaces whose directories no longer
exist, which is exactly the set nothing else on the machine can attribute.

## Output

```
released  3 containers · 1 network · ports 41200–41209 · 2 processes
```

`--json` returns one result per resource with its kind, identifier, and whether release
succeeded. **A resource that fails to release is reported and does not abort the rest** — a
half-clean that stops on the first error is worse than one that continues and tells you.

## Dependencies

| On | Why |
|---|---|
| `~/.armada/manifest.db` | The ownership record. Without it nothing is attributable. |
| A container runtime | Only if the workspace owns containers. |
| `armada.yml` | **Not required.** Clean works from the store, which is the whole point. |

## Exit codes

`0` everything released · `1` `tool_failed` — a resource would not release · `6` `environment` — the store is unreadable.

Release failure of one resource never aborts the rest.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`status.md`](status.md) · [`init.md`](init.md) · [`down.md`](down.md)
