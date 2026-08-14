# `armada doctor`

Report what this machine is missing or has drifted on. Read-only.

> **Status: not built — M2.** ([`PHASES.md`](PHASES.md) §8.4)

## Synopsis

```sh
armada doctor [--fix] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--fix` | flag | off | Repair what is safely repairable: pull a behind guild, recreate a missing directory, re-register a dropped plugin. Never touches anything that could lose work. |

## How it works

Four groups of checks, in order. Each is reported independently; one failure does not stop the
rest.

1. **Tooling** — `git`, `claude`, container runtime: present, and version.
2. **Layout** — `~/.armada/` exists with its four subdirectories and `machine.yml`.
3. **Guild drift** — is `~/.armada/guild/` behind, ahead of, or diverged from its remote, and by
   how many commits. This is the check that earns the command: two machines silently diverging
   is the guild's main failure mode ([`PHASES.md`](PHASES.md) §11).
4. **Projection** — for the current workspace, whether the guild content Claude Code is
   actually reading matches what the guild says it should be.

## Output

One line per check: `ok`, `warn`, or `fail`, with the specific delta rather than a verdict.

```
ok    tooling      git 2.51 · claude 2.x · docker
ok    layout       ~/.armada complete
warn  guild        3 commits behind origin — run `armada guild pull`
ok    projection   in sync
```

`--json` returns one result per check with `status`, `detail`, and `remedy` — the exact command
that would fix it.

## Dependencies

Reads `~/.armada/`. Needs network only for the guild-drift check, which degrades to `warn:
offline` without it rather than failing.

## Exit codes

`0` all ok · `1` `tool_failed` — at least one check reported `fail` · `6` `environment` — `~/.armada/` is missing entirely; run [`init.md`](init.md).

**`warn` alone does not fail**, so `doctor` is safe to run in a shell prompt.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`init.md`](init.md) · [`guild/pull.md`](guild/pull.md)
