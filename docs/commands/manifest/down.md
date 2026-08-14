# `armada manifest down`

Stop this workspace's services. Keep the port block.

> **Status: built.**

The distinction from [`clean.md`](clean.md) is the whole reason both exist: `down` is
**pause**, `clean` is **release**. `down` keeps the port block so the next `up` gets the same
ports, which keeps URLs, bookmarks and `.env` files valid across a restart.

## Synopsis

```sh
armada manifest down [<selector>] [-C <path>] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<selector>` | component names | all | Which components to stop. |
| `-C <path>` | directory | cwd | Operate on this workspace. |

## How it works

1. Stops components in **reverse dependency order** — dependents before dependencies, so
   nothing is torn out from under a live consumer.
2. Signals process groups rather than processes, so children do not survive their parent.
3. Leaves the port block claimed and the ownership records for durable resources — volumes,
   images — intact.

## Output

```
web       stopped
redis     stopped
postgres  stopped
ports     41200–41209 kept
```

`--json` returns one result per component plus the retained port block, in the same
`data` shape [`up.md`](up.md) answers in.

**A stop is confirmed, not merely signalled.** `down` reports `DOWN` for a `command` service
only once its process group is gone — SIGTERM, a grace period, then SIGKILL, an unconditional
escalation rather than a retry, because a leader that ignores SIGTERM immunises its whole
group and ignores the second one too. A group that survives SIGKILL fails the row.

**A named volume survives `down`.** It is the workspace's data, and `clean` is what releases
it.

**A compose component is a project, not a service** ([`up.md`](up.md)), so `down` on one stops
every service in that file. `driver: command` is the granular driver today.

## Dependencies

The ownership store, and a container runtime for container-backed components.

## Exit codes

`0` stopped · `1` `tool_failed` — a component would not stop · `2` `bad_invocation` — unknown selector · `6` `environment` — the container runtime is unavailable.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`up.md`](up.md) · [`clean.md`](clean.md)
