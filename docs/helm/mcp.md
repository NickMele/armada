# The MCP toolbelt

One MCP server. Tools namespaced by module, so the tool list has the same shape as the system
([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.9).

> **Status: not built — M3.** Targets `rmcp` v3.x; **re-check the API before starting**, it has
> moved fast ([`traps.md`](../traps.md)).

## Synopsis

```sh
armada mcp serve [--stdio] [--json]
```

Normally registered rather than run by hand — the orchestrator launches it, and
`~/.armada/guild/mcp.yml` registers it everywhere else.

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--stdio` | flag | **default** | Serve over stdio, the transport Claude Code uses. |

## Tools

| Tool | Wraps | Notes |
|---|---|---|
| `fleet.spawn` | [`../fleet/spawn.md`](../fleet/spawn.md) | Classifies unless the caller names a workflow. |
| `fleet.status` | [`../fleet/ls.md`](../fleet/ls.md) | Returns the table as structured data. |
| `fleet.probe` | — | Summarises one Job's transcript with a cheap model. **Read-only; never resumes the Drone.** |
| `fleet.answer` | [`../fleet/answer.md`](../fleet/answer.md) | Resumes a waiting Job. |
| `fleet.kill` | [`../fleet/kill.md`](../fleet/kill.md) | Clean, then drop. |
| `manifest.check` | [`../manifest/check.md`](../manifest/check.md) | The objective gate. |
| `manifest.up` / `.down` / `.status` / `.clean` | corresponding pages | Workspace operations. |
| `manifest.explain` | [`../manifest/explain.md`](../manifest/explain.md) | Evidence a stack trace does not carry. |

**Drones get a smaller toolbelt** — they may report and ask, not spawn. A Drone able to spawn
Drones is a fork bomb with a budget.

| Drone tool | Effect |
|---|---|
| `fleet.report` | Append progress to its own Job record. |
| `fleet.ask_human` | Raise an entry to the inbox and wait. |
| `fleet.verdict` | Emit a step verdict ([`PLAN.md`](../PLAN.md) §14.3). |

## How it works

Stateless: each request is self-contained, with per-request capability negotiation and no
session to hold. That happens to match how the CLI is already built — a command is parse → call
core → render, and a stateless server is the same shape with a different renderer
([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.3).

Long operations use the **Tasks extension** rather than a bespoke polling protocol. A real
`manifest check` runs well past ten minutes, and the extension exists for exactly that:
asynchronous work with polling, mid-flight input and durable handles.

## Output

Each tool answers in the `--json` envelope of [`PLAN.md`](../PLAN.md) §3.1, so an MCP caller and
a shell caller parse identically.

## Dependencies

Fleet and Manifest. No network, no daemon — the server is started by whatever registers it and
lives as long as that session.

## Exit codes

`0` clean shutdown · `6` `environment` — the transport failed.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`helm.md`](helm.md) · [`inbox.md`](inbox.md)
