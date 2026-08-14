# `armada manifest up`

Bring this workspace's services up and ready-checked.

> **Status: not built.** Answers `bad_invocation` today. ([`PHASES.md`](../../PHASES.md) §8.6 depends
> on `check`, not on this; `up` has no milestone blocker other than being unwritten.)

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

`--json` returns one result per component with argv, the ready-check that was waited on, the
wait duration, and the assigned ports.

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
