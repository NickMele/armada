# `armada guild project`

Put your guild where Claude Code will read it.

> **Status: built — M2.** ([`PHASES.md`](../../PHASES.md) §8.4)

Usually reached through [`init.md`](init.md) or [`pull.md`](pull.md), which both end here. Run it
directly after editing your guild by hand, or with `--remove` to take it back off the load path.

## Synopsis

```sh
armada guild project [--remove] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--remove` | flag | off | Delete exactly what a previous projection placed, and nothing else. |

## Why this exists

**A guild is not on any tool's load path until something puts it there.** Guild skills live in
`~/.armada/guild/skills/`; Claude Code reads `~/.claude/skills/`. Until this verb existed nothing
copied between them, so a skill Armada ships and `guild init` installs answered
`Unknown command: /onboard-repo` in the session Armada handed you to
([`PHASES.md`](../../PHASES.md) §8.4).

## How it works

### What moves, and where it lands

| In your guild | Lands at | Read by Claude Code as |
|---|---|---|
| `skills/<name>/**` | `~/.claude/skills/<name>/**` | a skill, invocable as `/<name>` |
| `subagents/<name>.md` | `~/.claude/agents/<name>.md` | a subagent — Claude Code's word for it |
| `hooks/<file>` | `~/.claude/hooks/<file>` | whatever your `settings.json` registers it as |

That table is [`import.md`](import.md)'s read backwards, and it is one table in the code for
exactly that reason: two copies of it would be two chances for projection to write into a
directory Claude Code does not read.

**Nothing else is placed.** `workflows/` is Armada's own to read; `plugins.yml` and `mcp.yml`
are registrations rather than files on a load path; and the three memory fragments are the
personal half [`PLAN.md`](../../PLAN.md) §13.3 says Guild writes by hand — a `voice.md` dropped
into `~/.claude/` is read by nothing.

### A file you edited is never overwritten

Every projection records **what it placed and the hash of each file as it placed it**
([`PLAN.md`](../../PLAN.md) §13.2). Three hashes answer every question: what the guild says,
what is on disk, and what Armada last wrote.

| On disk | Recorded | Outcome |
|---|---|---|
| absent | — | `ADDED` — placed |
| identical to the guild's | anything | `UNCHANGED` — adopted, nothing written |
| differs, and Armada wrote it | matches | `CHANGED` — brought up to date |
| differs, and you changed it | differs | `CONFLICT` — **left exactly as it is**, and reported |
| differs, and Armada never wrote it | absent | `CONFLICT` — **it is yours**, and it keeps its name |

**A file left as yours stops being Armada's.** It drops out of the manifest, so `--remove` will
not delete it and every later projection reports it rather than fighting for it. To take the
guild's copy instead, delete yours and run this again.

**Byte-identical is adopted rather than disputed.** On the machine the guild was imported *from*,
projecting finds the same bytes at the same path with no record of having written them. Calling
that a conflict would report every adopted skill as one on a fresh `guild init`, and would mean
the guild could never update one of them again. Nothing can be lost: the bytes are the same, and
the guild's copy is in a git repository with history.

### `--remove` reverses exactly what was placed

It reads the manifest and never the guild, which is what makes *exactly* true: a guild that has
grown a skill since does not widen the reversal, and one that has lost a skill does not narrow
it. A file you edited survives it, because an edited file is no longer what was placed.

### The manifest never syncs

`~/.armada/projection.json` records what was placed on **this** machine
([`PLAN.md`](../../PLAN.md) §13.1). Syncing it would tell a second machine that files it has
never written are Armada's to overwrite. It sits outside `~/.armada/guild/`, so it cannot be
committed even by a bug.

## Output

```
  STATUS     ITEM    DETAIL
  ADDED      skills  add-migration, onboard-repo
  CHANGED    agents  helm.md
  CONFLICT   hooks   edited here; delete it to take the guild's
  UNCHANGED  skills  triage-flake

NEEDS ATTENTION  ~/.claude/, 2 placed, 1 updated, 1 left as yours
```

**One row per area, not per file** — a guild with forty skills would otherwise print forty rows
on a verb whose whole job is to be glanced at. Frozen byte for byte by
`tests/golden/render/guild-project.plain`.

`--json` returns `at`, one result per area, the summary `facts`, and `kept` — how many files were
left exactly as they were because you had edited them.

## Dependencies

An initialised guild. `--remove` needs only the manifest, so it works on a machine whose guild
has since been deleted.

## Exit codes

`0` projected · `2` `bad_invocation` — there is no guild here · `6` `environment` — `~/.claude/` could not be written.

**Files left as yours do not fail the verb.** Armada declining to overwrite your work is the
behaviour, not an error; the row and the headline say so and the exit code stays `0`.

Full table and the one rule behind it: [`../reference.md`](../reference.md).

## See also

[`init.md`](init.md) · [`pull.md`](pull.md) · [`../doctor.md`](../doctor.md)
