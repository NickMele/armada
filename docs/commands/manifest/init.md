# `armada manifest init`

Make a workspace ready to work in. Idempotent.

> **Status: shipped.**

## Synopsis

```sh
armada manifest init [--dry-run] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--dry-run` | flag | off | Report what would be claimed and run. Changes nothing. |

Takes no selector: init is whole-workspace by definition.

> **`-C <path>` is reserved and not built.** A verb takes its workspace from where you are
> standing, and `cd` is the interface until something needs otherwise
> ([`config.md`](config.md)).

## How it works

1. **Resolves the workspace** — walks up for `armada.yml`, then derives the two identities in
   [`PLAN.md`](../../PLAN.md) §2.2. A workspace's identity is stable across renames and moves, which
   is what lets ownership survive them.
2. **Reaps first.** Before claiming anything, releases resources whose owning workspace no
   longer exists ([`PLAN.md`](../../PLAN.md) §2.3.1). This is why a machine does not accumulate
   orphans: cleanup happens on the way in, not only on the way out.
3. **Claims a port block** — a contiguous range recorded against this workspace in
   `~/.armada/manifest.db`. Parallel workspaces cannot collide because the claim is
   machine-global, not per-directory.
4. **Runs `setup:`** for each component, in dependency order. `setup:` must be idempotent —
   `init` is expected to be re-run and re-running it must be free ([`PLAN.md`](../../PLAN.md) §4.1).
5. **Writes `.armada/`** into the workspace: resolved config and the port assignments. It holds
   **nothing reclaimable** ([`PLAN.md`](../../PLAN.md) §4.2) — delete it and nothing leaks, because
   ownership lives in the machine-global store.

## Output

```
workspace  api (a3f2c1)
ports      41200–41209
setup      pnpm install ✓ 4.2s · prisma generate ✓ 1.1s
```

`--json` returns one result per component with the `setup:` argv actually executed, its
duration, and the port assignments. **The argv is in the payload deliberately** — it is where
the bugs are, and a caller that cannot see it cannot diagnose them.

## Dependencies

| On | Why |
|---|---|
| `armada.yml` | Required. Without one there is no workspace. Author it with [`config.md`](config.md). |
| `~/.armada/manifest.db` | Created on first use. |
| Whatever `setup:` invokes | The repo's own toolchain. Armada does not install it. |

## Exit codes

`0` ready · `1` `tool_failed` — a `setup:` command failed · `3` `bad_config` — no `armada.yml`, or it is invalid · `6` `environment` — the store is unreadable or the port pool is exhausted.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`clean.md`](clean.md) · [`status.md`](status.md) · [`up.md`](up.md) · [`config.md`](config.md)
