# Spike 10 — Can a Drone be identified by something it does not hold?

**The agent CLI accepts no unix socket on `--mcp-config`, so `LOCAL_PEERCRED` is off the table —
and a per-Drone identity is available anyway, from the process on the other end of an ordinary
loopback connection.** `http`, `streamable-http`, `sse`, `ws` and `stdio` all connect against a
server that speaks them; every one of those is TCP or a pipe, and an HTTP transport pointed at a
`unix://` URL is refused by the runtime with *protocol must be http:, https: or s3:*. But the
process that opens the connection **is the CLI Fleet spawned**, and the receiving side can name it
at accept time without asking it anything. Two Drones sharing one listener and one config file were
told apart by that alone.

**The result that bears on `#50` is the second one.** A Drone that skipped the tool entirely —
`curl`, run from its own Bash, carrying a Job name of its choosing in the body — still arrived
attributable. The connection came from a `curl` whose parent was the Drone's shell, whose parent
was that Drone's CLI. The payload said one thing and the transport said another, and the transport
was right. **That property needs no confinement**, which matters because `docs/scope.md` declines
to build any: a Drone can run a shell, and a shell cannot give a process an ancestry it does not
have.

**And a negative that bears on the code as it stands.** An MCP server whose `type` the CLI does not
recognise is *skipped*, not refused. The session comes up with no Armada server, exit status zero,
nothing on stderr, and the only trace anywhere is `[WARN] --mcp-config: 1 entry warning(s):
mcpServers.armada: Skipped — unknown MCP server type "unix" for server "armada"` in a log that
exists only under `--debug`. `crates/adapters/src/mcp.rs` writes `"type": "http"` as a literal, and
the day that spelling stops being accepted a Drone will go quiet with no diagnosis available from
the outside.

Measured 2026-08-29, against the agent CLI at 2.1.251 on darwin 27.

## What it was measured over

| | |
|---|---|
| Agent CLI | 2.1.251, the native install, invoked directly rather than through any shim |
| Runtime it reports | Node v26.3.0, darwin arm64 |
| Invocation | `-p --input-format stream-json --output-format stream-json --verbose --permission-mode dontAsk --strict-mcp-config --mcp-config <file>`, first turn on stdin |
| Model | `haiku`, and the Job's model is not a variable here |
| Server | [`010-identity-server.py`](010-identity-server.py) — one MCP server, three socket families, logging only what the receiving side can observe about its caller |
| Configs | one per case, written under the spike's own directory. The operator's own `mcp.json` was not read or written |

**What differs from a real Drone.** No `setsid`, a smaller `--allowedTools`, and a model chosen for
cost. None of the three touches which transport connects or what a socket says about its peer; the
argument list is otherwise the one `crates/adapters/src/harness.rs` renders.

## What `--mcp-config` accepts

The document is validated against a union the binary carries, and `strings` on the shipped bundle
prints it: `stdio`, `sse`, `sse-ide`, `ws-ide`, `http` (with `streamable-http` folded onto it),
`ws`, `sdk` and `claudeai-proxy`, keyed under `mcpServers`. `--mcp-config` is variadic and takes a
path **or the document itself**; `claude mcp add -t` offers a smaller set than the file does, and
`claude mcp list` ignores `--mcp-config` altogether, so neither is a reading of what the file takes.

| `type` | in the schema | connected | what carries the bytes |
|---|---|---|---|
| absent | yes, defaults to stdio | **yes** | a pipe pair to a child process |
| `stdio` | yes | **yes** | same |
| `http` | yes | **yes** | TCP, one connection for the session |
| `streamable-http` | yes, rewritten to `http` | **yes** | same |
| `sse` | yes | **yes** | TCP, two connections — a held `GET` and the `POST`s |
| `ws` | yes | **yes** | TCP, one connection, `Sec-WebSocket-Protocol: mcp` |
| `sdk` | yes | no — *Failed to connect SDK MCP server: Request timed out* after 60s | in-process, for an embedding SDK |
| `sse-ide`, `ws-ide` | yes | not tested — both require an `ideName` and are the editor's channel | TCP |
| `claudeai-proxy` | yes | not tested — carries a server id issued elsewhere | HTTPS |
| `unix` | **no** | skipped, silently | — |

[`010-results.json`](010-results.json) is that table as the CLI reported it, and the server logs
beside it are the same runs from the other side.

**`sse` and `ws` are not paper entries.** Both failed on the first pass because the spike's server
did not speak them, and both connected once it did — the CLI sent a real `GET` with
`Accept: text/event-stream` for one and a real RFC 6455 handshake for the other.

## Why there is no unix socket

Every spelling with a plausible claim on one was tried against a real `AF_UNIX` listener holding
the same server:

| config | what happened |
|---|---|
| `{"type":"unix","url":"…sock"}` | skipped, `mcp_servers: []`, warning only under `--debug` |
| `{"type":"http","url":"unix://…sock"}` | `TypeError [ERR_INVALID_ARG_VALUE]: protocol must be http:, https: or s3:` |
| `{"type":"http","url":"http+unix://…"}` | same |
| `{"type":"http","url":"…","socketPath":"…"}` | `socketPath` is not in the schema and is dropped; the URL is then unreachable |
| `{"type":"ws","url":"ws+unix://…sock:/mcp"}` | WebSocket transport created, connection failed — the client is the platform `WebSocket`, not a library with unix support |
| `{"type":"ws","url":"ws://unix:…sock:/mcp"}` | same |

So the peer credential a unix socket would have handed over for free is not reachable. What it
would have cost, had it been:

| | how the pid arrives | median over 20 |
|---|---|---|
| `AF_UNIX`, `getsockopt(SOL_LOCAL, LOCAL_PEERPID)` | the kernel answers | **1µs** |
| TCP loopback, `lsof -nP -i TCP:<peer port>` | recovered out of band | **67ms** |

[`010-peer-cost.py`](010-peer-cost.py) is that measurement. **67ms is paid once per connection, at
accept, not per call** — the `http` transport opened one connection and held it for the session,
and `sse` opened two. It is not on the path of a tool call.

## What a loopback connection says about its caller

[`010-two-drones.py`](010-two-drones.py) is `#50`'s shape: two CLIs, one config file, one listener,
both told to call the same tool with a different Job name.

| spawned | server saw | recovered pid | tool call |
|---|---|---|---|
| Drone A, pid 52342 | connection 1 | 52342, `claude` | `{"job": "JOB-A"}` |
| Drone B, pid 52343 | connection 2 | 52343, `claude` | `{"job": "JOB-B"}` |

Neither Drone named itself to the transport and neither could have. The pid came from kernel state,
the payload agreed with it, and Fleet already knows the pid of every CLI it spawned.

**The interesting case is the one that does not use the tool.** In
[`010-shell-forge.py`](010-shell-forge.py) a Drone is granted Bash and told to POST a `tools/call`
by hand, naming a Job that is not its own:

```
connection 1 pid=65352 claude              <- the CLI's own MCP client
connection 2 pid=66240 curl                <- the same Drone, by hand
  ancestry: 66240 curl <- 66238 zsh <- 65352 claude
  tool_call conn 2 args={"job": "JOB-Z"}
```

The body claimed `JOB-Z`. The socket said pid 65352, which is Drone A. A Drone can bypass the tool,
and bypassing it does not buy the Drone another Drone's identity, because the ancestry of anything
it can start runs back through its own process. That is what makes this a transport property rather
than a confinement one — and it is the same reason `Working`'s single slot works today, generalised
from one slot to many.

**The first attempt at this run is in the transcript and is worth keeping.** Asked to send a Job
name of `FORGED`, the Drone refused on the grounds that the request looked like impersonation. The
run reported above uses a neutral name.

## The stdio side of the question

The stdio transport is the one where the pipe is the identity, and it is: the spike's server, spawned
from the config, recorded its parent as the CLI process itself — a direct child, not a shell's
grandchild. So Fleet spawning `claude`, and `claude` spawning a relay, gives the relay a
kernel-supplied parent that Fleet can match against the pid it holds.

**The Job id would still be config-borne.** The server is started by the client from a `command` and
`args` in the config file, so anything Fleet puts there is a value the CLI hands to a process it
starts — the same trust level as today's URL, and readable by the Drone for the same reason:
`--mcp-config <path>` is in argv, `ps` prints a same-uid process's argv, and the file is the same
uid. The pipe's *parentage* is what is not config-borne. Recorded in
[`010-server-stdio.jsonl`](010-server-stdio.jsonl), which also shows that a stdio server inherits the
CLI's whole environment.

## Two things found on the way

**`--mcp-config` takes the document inline, and `ps` prints it.**
[`010-inline.py`](010-inline.py) passes the JSON itself instead of a path; the server connects, and
`ps -o args=` on the running Drone shows the whole document including the URL. That is the opposite
of what `crates/adapters/src/mcp.rs` and `crates/armada/src/serve.rs` are arranging with a `0600`
file outside the worktree, so the inline form is a worse option than the one already chosen, not a
lighter one.

**The CLI runs an inbound unix socket of its own.** Under `--debug` it logs, at `INFO`, a socket
named for its own pid and the token that authenticates a message injected on it — a channel for
delivering a user turn to a running session. It is the direction spike 4 went the other way about,
and it is the vendor's, not Fleet's. Noted because it is the one unix socket in the picture, and it
is not the one this spike was looking for.

## What this does not answer

- **Whether a pid is a durable key.** A pid is unique while the process lives and reusable after it
  dies. Nothing here measured what a Fleet holding a pid-to-Job map does when a Drone exits between
  a connection being accepted and a call arriving on it.
- **What the ancestry walk costs when it is Rust rather than `ps`.** Every number above for the TCP
  side is `lsof` and a `ps` per hop, which is the shape of the measurement and not the shape of an
  implementation.
- **Whether `ws` is a good idea.** It is the only accepted transport where every JSON-RPC message
  rides one socket, which is the neatest fit for a per-connection identity. Nothing here measured
  its reconnection behaviour, and `#50` decides what to build.
- **What happens with more than two Drones**, or with a Drone that opens many connections at once.
