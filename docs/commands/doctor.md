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

Five groups of checks, in order. Each is reported independently; one failure does not stop the
rest.

1. **Tooling** — `git`, `claude`, container runtime: present, and version.
2. **Drone argv** — would a Drone actually start? Every flag Fleet's argv uses is held against
   `claude --help`, and then the argv itself is run with its prompt replaced by
   `--input-format stream-json` and nothing on stdin, so Claude Code validates every flag,
   starts the session, reaches EOF and exits **without making an API call**.
3. **`~/.armada`** — the directory exists, with `guild/`, `jobs/` and `workspaces/` inside it.
   Each missing path is named, along with what writes there, because a directory is worth
   restoring only if something needs it.
4. **Guild drift** — is `~/.armada/guild/` behind, ahead of, or diverged from its remote, and by
   how many commits. This is the check that earns the command: two machines silently diverging
   is the guild's main failure mode ([`PHASES.md`](../PHASES.md) §11).
5. **Fragments** — which of `voice.md`, `expectations.md` and `how-i-work.md` are still
   Armada's words rather than yours: `still as imported` for a carved-up `CLAUDE.md`,
   `still Armada's example text` for one import had nothing to put in. Read from a marker
   `armada guild init` writes into the file, so deleting the marker is what says *this is mine*
   ([`PLAN.md`](../PLAN.md) §13.4).
6. **Projection** — for the current workspace, whether the guild content Claude Code is
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

The status words are `OK`, `FOUND`, `CREATED`, `MISSING`, `STALE`, `PARTIAL` and `OFFLINE` —
SCREAMING on the screen and SCREAMING in the payload, like every other word a `STATUS` column
holds. None of them ends a run or maps to an exit code, and they used to be lowercase to say
so; that distinction cost more than it bought, because a reader scanning one column read `PASS`
and `ok` as two kinds of thing when they are one ([`render.md`](render.md)).
`NEEDS ATTENTION` sits in the payload under `data.headline`, spelled exactly as it is printed.

`--json` returns one result per check with `status`, `detail`, and `remedy` — the exact command
that would fix it, or the sentence that says how. A check that passed carries no `remedy`;
everything else always does.

### Why the Drone-argv check exists

Fleet's Drone argv was missing `--verbose`, which Claude Code requires alongside
`--output-format stream-json`. Every Job spawned, claimed a worktree and a port block, and its
Drone died instantly on a usage error nobody saw until [`fleet/ls.md`](fleet/ls.md) reported
`STALLED`. **Every test passed throughout**, because they all asserted on the argv Armada built
rather than on one the binary accepts — a distinction recorded in
[`../traps.md`](../traps.md).

**It costs nothing and it never will.** The probe carries no prompt and is told to read messages
from stdin, which is closed; a turn that never happens has no ledger and no cost. If a future
version ever answers it anyway, the check reports that as a finding rather than paying for it
quietly.

**What it does not cover**, stated because a check nobody knows the edges of is a check people
over-trust: `claude --help` does not document combination rules — the `--verbose` requirement is
not in it — and `--help` short-circuits before validation, so nothing can enumerate them in
advance. The probe covers the combination Armada actually uses, which is the one that matters.

## Dependencies

Reads `~/.armada/`. Needs network only for the guild-drift check, which degrades to `warn:
offline` without it rather than failing.

## Exit codes

`0` all ok · `1` `tool_failed` — at least one check reported `fail` · `6` `environment` — `~/.armada/` is missing entirely; run [`init.md`](init.md).

**`warn` alone does not fail**, so `doctor` is safe to run in a shell prompt.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`init.md`](init.md) · [`guild/pull.md`](guild/pull.md)
