# The MCP toolbelt

One MCP server. Tools namespaced by module, so the tool list has the same shape as the system
([`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.9).

> **Status: built — M3.** `rmcp` **3.1.2**, checked against crates.io rather than taken from
> this line; what that check found is in [`traps.md`](../../traps.md), including the two API
> shapes a v2 example gets wrong and the fact that Claude Code renames a dotted tool.

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
| `manifest.explain` | [`../manifest/explain.md`](../manifest/explain.md) | Evidence a stack trace does not carry. **Not served**: the verb it would wrap is not built, and a tool that reimplemented one would be the thing §1.3 exists to stop. It lands with the verb. |
| `manifest.skills` | [`../manifest/skills.md`](../manifest/skills.md) | What this repo knows about itself. Summaries only — Helm holds a routing table, not prose. |
| `manifest.skill` | [`../manifest/skills.md`](../manifest/skills.md) | One skill resolved, **including the doc body**. |

> **`manifest.skill` is on this belt and not on the Drone's**, and that sentence used to read *a
> Drone calls this; Helm does not*, which the shipped belts have never done. The two routers are
> two types (below) and the Drone's has only `fleet.*` — so a Drone cannot read this repository's
> skills, cannot read its manifest, and learns what it learns from the commands it runs. That is
> the half [`../../reserved/008-armada-injects-its-own-skills.md`](../../reserved/008-armada-injects-its-own-skills.md)
> records as left open; widening the belt is a separate decision with a separate blast radius.

**Drones get a smaller toolbelt** — they may report, ask, propose and settle a step, not spawn. A
Drone able to spawn Drones is a fork bomb with a budget.

| Drone tool | Effect |
|---|---|
| `fleet.report` | Append progress to its own Job record. |
| `fleet.ask_human` | Raise an entry to the inbox and wait. |
| `fleet.propose` | Raise a change to this repository's `armada.yml` or to the person's guild, and **carry on** ([`../../reserved/008-armada-injects-its-own-skills.md`](../../reserved/008-armada-injects-its-own-skills.md)). |
| `fleet.verdict` | Emit a step verdict ([`PLAN.md`](../../PLAN.md) §14.3). |

**`fleet.propose` is not `fleet.ask_human` with a different word**, and the difference is the
wait. A question is something a Drone cannot proceed without an answer to, so it blocks; a
proposal is something the person should know that the Drone is not blocked on, so it files one
entry with an id and returns at once. Collapsing them would mean a Drone spending its own wall
clock ceiling to tell somebody a check name looks stale.

**A proposal is not a change.** It writes an inbox entry — `Origin::Raised`, the same id space
as everything else `armada failures show` resolves — and nothing else. Applying it is the
person's, through `armada guild edit` or an ordinary edit to `armada.yml`. Armada verifies; it
does not accept an agent's word, which is why there is no verb here that writes either file.

**The two belts are two types, not one list with a filter.** A filter is a line somebody
eventually moves, and moving it has no visible consequence until a machine is full of Drones.
`fleet.spawn` is not *blocked* from a worker; it is absent — a Drone calling it gets `tool not
found`, because no router on that belt has ever heard of it.

**Which belt is served is decided by `ARMADA_JOB`, not by a flag.** That variable is set on
every child of a Job and by nothing else, so a Drone's own environment answers the question. A
flag is a thing a registration file can get wrong, and getting it wrong in one direction is the
fork bomb.

**A Drone names no Job.** Its four tools take the handle from that same variable rather than as
an argument, so a worker cannot write another worker's record — and therefore cannot rewrite the
evidence somebody else's verdict rests on.

**A `PASS` with no evidence is refused rather than recorded.** *"A verdict is only `PASS` if it
carries evidence an external command produced"* ([`PLAN.md`](../../PLAN.md) §14.3), and this is
where that stops being a sentence in a prompt.

## How it works

Stateless: each request is self-contained, with per-request capability negotiation and no
session to hold. That happens to match how the CLI is already built — a command is parse → call
core → render, and a stateless server is the same shape with a different renderer
([`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.3).

Long operations use the **Tasks extension** rather than a bespoke polling protocol. A real
`manifest check` runs well past ten minutes, and the extension exists for exactly that:
asynchronous work with polling, mid-flight input and durable handles. `manifest.check` and
`fleet.spawn` take it; everything else answers inline, because a durable handle for a read that
takes forty milliseconds is two round trips to save none.

**The same tool answers both ways.** SEP-2663 forbids handing a task handle to a client that
did not declare the extension — it would have nothing to poll with — so a caller that cannot
poll gets the result inline from the same tool. That is one name in the tool list rather than a
`check` and a `check_status`, which is the pair the extension exists to stop anyone writing.

**Cancelling is cooperative and says so.** It records the intent and acknowledges; it does not
abort the verb underneath. A `manifest check` mid-run holds process groups and a lease, and
killing its future would leak both — the run's own interrupt path is what ends it properly.

## Output

Each tool answers in the `--json` envelope of [`PLAN.md`](../../PLAN.md) §3.1, so an MCP caller and
a shell caller parse identically — the tool result's one text block is byte-for-byte what
`armada … --json` writes to stdout.

**No `structuredContent`.** A tool returning one should declare an `outputSchema`, and `data` is
a different shape per verb: twelve schemas kept in step with twelve structs, to restate bytes
the caller already has. It would also have to be assembled as a `serde_json::Value`, whose map
is a `BTreeMap` — which alphabetises an envelope this corpus writes in reading order
([`traps.md`](../../traps.md)).

**A failed verb is a tool-level error, never a protocol error.** Clients render a JSON-RPC error
opaquely, so a `bad_config` naming the line to edit would reach the agent as "tool result
missing". The result is flagged `isError` and the envelope travels in the content, so an agent
reads exactly what a person would — the class, the `where`, and the `next_action`.

**`armada mcp serve` itself reports on stderr**, and it is the only verb that must: stdout *is*
the transport, and a summary written into it is a frame no client can parse. Measured — see
[`traps.md`](../../traps.md).

## Dependencies

Fleet and Manifest. No network, no daemon — the server is started by whatever registers it and
lives as long as that session.

## Exit codes

`0` clean shutdown · `6` `environment` — the transport failed.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`helm.md`](helm.md) · [`inbox.md`](inbox.md)
