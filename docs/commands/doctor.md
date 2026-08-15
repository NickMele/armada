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
2. **`~/.armada`** — the directory exists, with `guild/`, `jobs/` and `workspaces/` inside it.
   Each missing path is named, along with what writes there, because a directory is worth
   restoring only if something needs it.
3. **Guild drift** — is `~/.armada/guild/` behind, ahead of, or diverged from its remote, and by
   how many commits. This is the check that earns the command: two machines silently diverging
   is the guild's main failure mode ([`PHASES.md`](../PHASES.md) §11).
4. **Projection** — for the current workspace, whether the guild content Claude Code is
   actually reading matches what the guild says it should be.

## Output

**One table per check**, headed by the check's name, and a `→` line under every row that asks
you to do something. Frozen byte for byte by `tests/golden/render/doctor.plain`.

```
  git
    ok       2.51.0

  claude
    ok       2.0.14

  docker
    missing  compose driver unavailable
             -> install docker, or accept that compose repos will not start

  ~/.armada
    missing  jobs/ and workspaces/ are missing; Jobs and worktrees go there
             -> armada init --force

  guild
    stale    3 commits behind origin
             -> armada guild pull
    partial  voice.md still as imported
             -> write ~/.armada/guild/voice.md in your own words

  manifest.db
    ok       2 workspaces, 0 orphans

NEEDS ATTENTION  3 ok, 2 missing, 2 warnings
```

**Grouped, because a check can report more than once.** `guild` is drift plus one row per
fragment still as imported, and a flat table scatters those among `docker` and `manifest.db`
with nothing to say which belong together. `armada init` is not grouped: it ticks each check off
exactly once, so a flat list already is the grouping.

**Every row that is not `ok` carries a fix line, and that is enforced by the type rather than by
this sentence.** A check reports a problem through a constructor whose remedy is not optional
(`armada_core::envelope::Finding::needs`), so a row a reader can do nothing with fails to
compile. It is a command where one exists and a sentence where none does — *write
`~/.armada/guild/voice.md` in your own words* is a fix; *out of date* is not.

The status words are `ok`, `found`, `created`, `missing`, `stale`, `partial` and `offline` —
lowercase on the screen and lowercase in the payload, because nothing here ends a run and none
of them maps to an exit code. `NEEDS ATTENTION` is the one uppercase word in Armada's output
that is not a `Status`; it is in the payload under `data.headline`, spelled exactly as it is
printed.

`--json` returns one result per check with `status`, `detail`, and `remedy` — the exact command
that would fix it, or the sentence that says how. A check that passed carries no `remedy`;
everything else always does.

## Dependencies

Reads `~/.armada/`. Needs network only for the guild-drift check, which degrades to `warn:
offline` without it rather than failing.

## Exit codes

`0` all ok · `1` `tool_failed` — at least one check reported `fail` · `6` `environment` — `~/.armada/` is missing entirely; run [`init.md`](init.md).

**`warn` alone does not fail**, so `doctor` is safe to run in a shell prompt.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`init.md`](init.md) · [`guild/pull.md`](guild/pull.md)
