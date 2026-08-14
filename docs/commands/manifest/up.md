# `armada manifest up`

Bring this workspace's services up and ready-checked.

> **Status: built.**

## Synopsis

```sh
armada manifest up [<selector>] [-C <path>] [--dry-run] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<selector>` | component names | all | Which components to start. Selector grammar in [`PLAN.md`](../../PLAN.md) §3.2. |
| `-C <path>` | directory | cwd | Operate on this workspace. |
| `--dry-run` | flag | off | Report the argv that would run and the ready-checks that would be waited on. |

## How it works

1. **Resolves the dependency order** from `needs:` and starts components in it.
2. **Starts each component** through its driver — compose, or a raw process
   ([`PLAN.md`](../../PLAN.md) §6).
3. **Records ownership immediately**, before waiting. A container that starts and then fails
   its ready-check is still owned, and therefore still reclaimable by [`clean.md`](clean.md).
   Recording after the wait would leak exactly the resources most likely to be broken.
4. **Waits on the ready-check**, not on the process existing. "Started" is not "ready", and
   a caller that cannot tell the difference races.

`up` is not `init`. It assumes the port block is already claimed and setup already ran.

## Output

One line per component with its readiness and how long it took.

```
postgres  ready  1.9s
redis     ready  0.3s
web       ready  4.1s  http://localhost:41200
```

`--json` returns one result per component with `argv`, the ready-check that was waited on in
`reason`, the wait duration, the assigned ports, and `owns[]` — the ids of everything Armada
now holds for it, in the `<kind>:<reference>` grammar ([`PLAN.md`](../../PLAN.md) §3.1).

**Services are started one at a time.** `needs:` already forces a dependency to be *ready*
before its dependent starts, so concurrency would only overlap two independent ready-checks —
and a second concurrent scheduler is a second place a deadlock can hide.

**A service whose dependency did not come up is `SKIPPED`**, naming the one that stopped it,
rather than started into a failure two levels from its own logs.

## Dependencies

| On | Why |
|---|---|
| [`init.md`](init.md) | The port block must be claimed first. `up` will not claim one. |
| A container runtime | For compose-backed components. |
| `armada.yml` | The `components:` block. |

## Exit codes

`0` all ready · `1` `tool_failed` — a component failed to start or failed its ready-check · `2` `bad_invocation` — unknown selector · `4` `timeout` — a ready-check did not complete · `6` `environment` — the container runtime is unavailable.

**A partial failure still leaves what started recorded and owned** — [`clean.md`](clean.md) or [`down.md`](down.md) reclaims it, and nothing is stranded.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`down.md`](down.md) · [`init.md`](init.md) · [`status.md`](status.md)
