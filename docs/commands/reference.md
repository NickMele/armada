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
| [`helm/helm.md`](helm/helm.md) | `armada` — enter Helm | M3 |
| [`helm/bridge.md`](helm/bridge.md) | `armada bridge` — the live screen | M3 |

## Manifest — what a workspace is and how to operate it

| Page | Command | Status |
|---|---|---|
| [`manifest/init.md`](manifest/init.md) | `armada manifest init` | shipped |
| [`manifest/status.md`](manifest/status.md) | `armada manifest status` | shipped |
| [`manifest/clean.md`](manifest/clean.md) | `armada manifest clean` | shipped |
| [`manifest/commands.md`](manifest/commands.md) | `armada manifest <repo-command>` | shipped |
| [`manifest/up.md`](manifest/up.md) | `armada manifest up` | not built |
| [`manifest/down.md`](manifest/down.md) | `armada manifest down` | not built |
| [`manifest/check.md`](manifest/check.md) | `armada manifest check` | not built |
| [`manifest/config.md`](manifest/config.md) | `armada manifest config scan\|verify` | not built |
| [`manifest/explain.md`](manifest/explain.md) | `armada manifest explain` | not built |

## Guild — your portable setup

| Page | Command | Status |
|---|---|---|
| [`guild/init.md`](guild/init.md) | `armada guild init` | M2 |
| [`guild/edit.md`](guild/edit.md) | `armada guild edit` | M2 |
| [`guild/push.md`](guild/push.md) | `armada guild push` | M2 |
| [`guild/pull.md`](guild/pull.md) | `armada guild pull` | M2 |
| [`guild/export.md`](guild/export.md) | `armada guild export` | M2 |
| [`guild/import.md`](guild/import.md) | `armada guild import` | M2 |

## Fleet — the agents you do not talk to

| Page | Command | Status |
|---|---|---|
| [`fleet/spawn.md`](fleet/spawn.md) | `armada fleet spawn` | M3 |
| [`fleet/ls.md`](fleet/ls.md) | `armada fleet ls` | M3 |
| [`fleet/inbox.md`](fleet/inbox.md) | `armada fleet inbox` | M3 |
| [`fleet/answer.md`](fleet/answer.md) | `armada fleet answer` | M3 |
| [`fleet/board.md`](fleet/board.md) | `armada fleet board` | M3 |
| [`fleet/kill.md`](fleet/kill.md) | `armada fleet kill` | M3 |

## Helm — the one agent you do talk to

| Page | Topic | Status |
|---|---|---|
| [`helm/helm.md`](helm/helm.md) | `armada helm` — the conversation | M3 |
| [`helm/bridge.md`](helm/bridge.md) | `armada bridge` — the live screen, and the palette | M3 |
| [`helm/mcp.md`](helm/mcp.md) | The MCP toolbelt Helm drives | M3 |
| [`helm/inbox.md`](helm/inbox.md) | How Fleet events reach you | M3 |

## Conventions used on every page

| | |
|---|---|
| `--json` | Every command accepts it and answers in the envelope of [`PLAN.md`](../PLAN.md) §3.1. Arguments tables omit it. |
| `-C <path>` | Run against another workspace instead of the current directory. Manifest commands only. |
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
| `70` | `armada_bug` | Emitted as `char_bug` today; renames in M1 ([`PHASES.md`](../PHASES.md) §8.3). |
| `130` / `141` | — | SIGINT / SIGPIPE. |

A `commands:` child's exit code passes through **verbatim** and is not remapped
([`manifest/commands.md`](manifest/commands.md)).

Each page below names the classes its command can produce rather than restating this table —
a rule stated in two places drifts.
