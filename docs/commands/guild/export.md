# `armada guild export`

Write the whole guild to one portable file.

> **Status: built — M2.**

The escape hatch for a machine that will never hold your git credentials, and the thing you
reach for when a remote is not worth setting up.

## Synopsis

```sh
armada guild export [--out <path>] [--include-secrets] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--out <path>` | file path | `./guild.tar.zst` | Where to write. |
| `--include-secrets` | flag | **off** | Include `machine.yml`. Off by default and prints a warning when on — the whole point of `machine.yml` is that it does not travel. |

## How it works

Archives `~/.armada/guild/` — content only. The git history is **not** included: a bundle is a
snapshot, and a machine restoring from one starts a fresh history. Use a remote if you want
history to travel.

Everything under `~/.armada/` that is not `guild/` is excluded by construction, because it
describes this machine ([`PLAN.md`](../../PLAN.md) §13.1).

## Output

```
exported  ./guild.tar.zst  ·  19 skills · 12 hooks · 4 workflows · 3 fragments  ·  412 KB
secrets   excluded
```

`--json` returns the path, byte size, and a manifest of what was included.

## Dependencies

An initialised guild. No network, no git remote.

## Exit codes

`0` written · `2` `bad_invocation` — no guild exists · `6` `environment` — the output path is not writable.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`import.md`](import.md) · [`push.md`](push.md)
