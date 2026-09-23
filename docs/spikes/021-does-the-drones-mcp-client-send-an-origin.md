# Spike 21 — Does the Drone's MCP client send an `Origin`?

**No.** A real agent CLI, pointed at an HTTP MCP server, sends nine headers and
`Origin` is not among them. So Fleet refusing every request that carries one —
`api.from_a_page`, `#1460` — cannot refuse a Drone submitting evidence.

| Header | Value on both requests |
|---|---|
| `accept` | `application/json, text/event-stream` |
| `accept-encoding` | `identity` |
| `content-type` | `application/json` |
| `user-agent` | `claude-code/2.1.280 (sdk-cli)` |
| `mcp-protocol-version` | `2026-07-28` |
| `mcp-method` | `server/discover`, on the first request only |
| `connection`, `host`, `content-length` | as the transport sets them |

## Why it had to be measured

Two MCP paths reach Fleet and only one of them was known.

| Endpoint | Written as | Who writes the request head |
|---|---|---|
| The agent's door, `/agent/mcp` | a program — `armada mcp` — in `crates/adapters/src/mcp.rs` | this repository, in `crates/armada/src/loopback.rs`, and a test holds it to sending no `Origin` |
| The Drone's Evidence endpoint, `/mcp` | an address, in the same file | the agent CLI's own HTTP client |

Both are written in one file with different transports, which is what makes
the second look answered by the first. It is not: no code in this repository
and no Node `fetch` writes that head. `docs/spikes/006-will-a-drone-use-the-evidence-tool.md`
recorded the calls a Drone made and not the headers they carried.

The consequence of guessing wrong was every Job failing at its Evidence call.

## How

A Node server on `127.0.0.1:40199` logging every request head and answering one
`initialize` result; a `.mcp.json` naming it as an `http` server; a real agent
CLI run against that configuration with `-p` and the probe's tools allowed. Two
requests arrived, a discovery and a call, and the table above is both of them.

Measured on 2026-09-23 against `claude-code/2.1.280 (sdk-cli)`, which is a
version: the reading is attributable to that build and not to agent CLIs in
general.

## What follows if it ever changes

**Not an exemption for `/mcp`.** The answer is to write the Drone's Evidence
server the way the door is already written — a `StdioServer` naming
`armada mcp`, so the request head comes from `loopback.rs` like every other
call this repository makes, and the port is read from the runtime file at
connect time rather than frozen into a URL at spawn. `crates/adapters/src/mcp.rs`
already carries that argument in its doc comment, for the door.
