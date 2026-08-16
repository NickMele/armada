# Armada — command reference

One page per command. Every page has the same five sections: **Arguments**, **How it works**,
**Output**, **Dependencies**, **Exit codes**.

> **Read the status line on each page.** Armada is partly built. A page describing something
> that does not exist yet says so at the top and names the milestone
> ([`PHASES.md`](../PHASES.md) §8).

**Terms are defined once, in [`glossary.md`](../glossary.md)** — Job, Drone, Helm, Bridge, Board,
and the three status enums. These pages use them without re-explaining them.

## Top level

| Page | Command | Status |
|---|---|---|
| [`init.md`](init.md) | `armada init` — set up **this machine** | M2 |
| [`doctor.md`](doctor.md) | `armada doctor` — what this machine is missing | M2 |
| [`settings.md`](settings.md) | `armada settings` — every setting, its value, where it lives, whether it syncs | shipped, read-only |
| [`render.md`](render.md) | Output, colour and the palette — how every verb renders | M1.5 |
| [`helm/helm.md`](helm/helm.md) | `armada` — enter Helm | M3 |
| [`helm/bridge.md`](helm/bridge.md) | `armada bridge` — the live screen | M3 |

## Manifest — what a workspace is and how to operate it

| Page | Command | Status |
|---|---|---|
| [`manifest/init.md`](manifest/init.md) | `armada manifest init` | shipped |
| [`manifest/status.md`](manifest/status.md) | `armada manifest status` | shipped |
| [`manifest/clean.md`](manifest/clean.md) | `armada manifest clean` | shipped |
| [`manifest/commands.md`](manifest/commands.md) | `armada manifest commands` · `armada manifest <repo-command>` | shipped |
| [`manifest/skills.md`](manifest/skills.md) | `armada manifest skills` · `render` | `skills` shipped · `render` not built |
| [`manifest/components.md`](manifest/components.md) | `armada manifest components` | shipped |
| [`manifest/up.md`](manifest/up.md) | `armada manifest up` | shipped |
| [`manifest/down.md`](manifest/down.md) | `armada manifest down` | shipped |
| [`manifest/check.md`](manifest/check.md) | `armada manifest check` | shipped, `--detach` / `--status` included |
| [`manifest/config.md`](manifest/config.md) | `armada manifest config scan\|verify` | shipped |
| [`manifest/explain.md`](manifest/explain.md) | `armada manifest explain` | not built |

## Guild — your portable setup

| Page | Command | Status |
|---|---|---|
| [`guild/init.md`](guild/init.md) | `armada guild init` | shipped |
| [`guild/project.md`](guild/project.md) | `armada guild project` | shipped |
| [`guild/ls.md`](guild/ls.md) | `armada guild ls` | shipped |
| [`guild/show.md`](guild/show.md) | `armada guild show` | shipped |
| [`guild/edit.md`](guild/edit.md) | `armada guild edit` | shipped |
| [`guild/delete.md`](guild/delete.md) | `armada guild delete` | shipped |
| [`guild/push.md`](guild/push.md) | `armada guild push` | shipped |
| [`guild/pull.md`](guild/pull.md) | `armada guild pull` | shipped |
| [`guild/upgrade.md`](guild/upgrade.md) | `armada guild upgrade` | shipped |
| [`guild/export.md`](guild/export.md) | `armada guild export` | shipped |
| [`guild/import.md`](guild/import.md) | `armada guild import` | shipped |

## Fleet — the agents you do not talk to

| Page | Command | Status |
|---|---|---|
| [`fleet/spawn.md`](fleet/spawn.md) | `armada fleet spawn` | shipped |
| [`fleet/ls.md`](fleet/ls.md) | `armada fleet ls` | shipped |
| [`fleet/show.md`](fleet/show.md) | `armada fleet show` — one Job, and why it needs you | shipped |
| [`fleet/inbox.md`](fleet/inbox.md) | `armada fleet inbox` | shipped |
| [`fleet/answer.md`](fleet/answer.md) | `armada fleet answer` | shipped |
| [`fleet/board.md`](fleet/board.md) | `armada fleet board` | shipped |
| [`fleet/kill.md`](fleet/kill.md) | `armada fleet kill` | shipped |
| [`fleet/pause.md`](fleet/pause.md) | `armada fleet pause` | shipped |
| [`fleet/resume.md`](fleet/resume.md) | `armada fleet resume` | shipped |
| [`fleet/reap.md`](fleet/reap.md) | `armada fleet reap` | shipped |
| [`fleet/tick.md`](fleet/tick.md) | `armada fleet tick` — the workflow loop: gate the step, then advance, retry or stop | shipped |

## Helm — the one agent you do talk to

| Page | Topic | Status |
|---|---|---|
| [`helm/helm.md`](helm/helm.md) | `armada helm` — the conversation | M3 |
| [`helm/enable.md`](helm/enable.md) | `armada helm enable` · `disable` — the switch that lets `armada helm` enter | M3 |
| [`helm/bridge.md`](helm/bridge.md) | `armada bridge` — the live screen, and the palette | M3 |
| [`helm/mcp.md`](helm/mcp.md) | The MCP toolbelt Helm drives | M3 |
| [`helm/inbox.md`](helm/inbox.md) | How Fleet events reach you | M3 |

## Conventions used on every page

| | |
|---|---|
| `--json` | Every command accepts it and answers in the envelope of [`PLAN.md`](../PLAN.md) §3.1. Arguments tables omit it. |
| `--color <when>` | `auto` (default), `always`, `never`; `NO_COLOR` wins. Every command accepts it, and Arguments tables omit it for the same reason ([`render.md`](render.md)). |
| `-C <path>` | Which repository to branch from. **[`fleet/spawn.md`](fleet/spawn.md) alone accepts it.** Every other verb takes its workspace from where you are standing, and `cd` is the interface until something needs otherwise ([`manifest/config.md`](manifest/config.md)). |
| Paths | Written relative to the repo or as `~/`. Never absolute. |

### Exit codes are a function of `error.class`

**One rule, uniform across every command: the exit code is determined by `error.class`, or `0`
when there is no error.** Terminal state never determines it — a `FAILED` verdict is exit `3`
when the config was wrong and exit `1` when the tests were.

| Code | `error.class` | Meaning |
|---:|---|---|
| `0` | *(none)* | Success. |
| `1` | `tool_failed` | Something Armada invoked returned non-zero. |
| `2` | `bad_invocation` | Wrong arguments, unknown selector, unknown name. |
| `3` | `bad_config` | The config is missing, invalid, or inconsistent. `next_action` is required. |
| `4` | `timeout` | |
| `5` | `aborted` | |
| `6` | `environment` | Fix the machine and retry unchanged. |
| `70` | `armada_bug` | Internal error; retrying will not help. |
| `130` / `141` | — | SIGINT / SIGPIPE. |

A `commands:` child's exit code passes through **verbatim** and is not remapped
([`manifest/commands.md`](manifest/commands.md)).

Each page below names the classes its command can produce rather than restating this table —
a rule stated in two places drifts.
