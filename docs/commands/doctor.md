# `armada doctor`

Report what this machine is missing or has drifted on. Read-only.

> **Status: built — M2**, less the projection group, which needs a projector to
> compare against and lands with one, and `--fix`, which is refused by name.
> ([`PHASES.md`](../PHASES.md) §8.4)

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
   is the guild's main failure mode ([`PHASES.md`](../PHASES.md) §11).
4. **Projection** — for the current workspace, whether the guild content Claude Code is
   actually reading matches what the guild says it should be.

## Output

One row per check, with the specific delta rather than a verdict, and a `→` line naming the
command that fixes each problem. Frozen byte for byte by `tests/golden/render/doctor.plain`.

```
  STATUS   CHECK        DETAIL                      TIME
  ok       git          2.51.0                         -
  ok       claude       2.0.14                         -
  missing  docker       compose driver unavailable     -
  stale    guild        3 commits behind origin        -
  partial  guild        voice.md still as imported     -
  ok       manifest.db  2 workspaces, 0 orphans        -

NEEDS ATTENTION  3 ok, 1 missing, 2 warnings
  -> install docker, or accept that compose repos will not start
  -> armada guild pull
```

The status words are `ok`, `found`, `created`, `missing`, `stale`, `partial` and `offline` —
lowercase on the screen and lowercase in the payload, because nothing here ends a run and none
of them maps to an exit code. `NEEDS ATTENTION` is the one uppercase word in Armada's output
that is not a `Status`; it is in the payload under `data.headline`, spelled exactly as it is
printed.

`--json` returns one result per check with `status`, `detail`, and `remedy` — the exact command
that would fix it. A check that passed carries no `remedy`, and neither does a problem whose fix
is prose rather than a command.

## Dependencies

Reads `~/.armada/`. Needs network only for the guild-drift check, which degrades to `warn:
offline` without it rather than failing.

## Exit codes

`0` all ok · `1` `tool_failed` — at least one check reported `fail` · `6` `environment` — `~/.armada/` is missing entirely; run [`init.md`](init.md).

**`warn` alone does not fail**, so `doctor` is safe to run in a shell prompt.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`init.md`](init.md) · [`guild/pull.md`](guild/pull.md)
