# Spike 12 — Does peer-process attribution survive five Drones and concurrent connections?

**It survives five, and the race that spike 10 flagged does not resolve wrong — it resolves
absent.** Five Drones, one listener, one shared config: every connection attributed, every payload
agreeing with its transport, one connection each. Then 384 connections were engineered to lose their
peer, by a client that writes its pid and calls `_exit` before the lookup can finish. Every one that
failed came back **empty**, and not one came back naming another process. A late lookup is a Drone
Fleet cannot identify, which is a refusal; it is not a Drone identified as a different Drone, which
would be a Job's work credited to another.

**There is a misattribution, and it is not a race.** A local port number is not unique on a host:
two processes may each hold port 24101 as long as they are talking to different places. Put a second
Drone's unrelated connection on the same local port and *every* lookup keyed on that number alone —
`lsof -nP -i TCP:<port>`, which is what spike 10 used — names the wrong live pid, deterministically,
whenever it happens to scan that process first. It is not rare-and-timing-dependent, it is a wrong
question. **The pair (local port, remote port) is the right one**, it is in the same kernel record,
and matching on both was right in every ordering the reproduction was run in.

**And 67ms is the price of `lsof`, not the price of the question.** Asking `proc_pidfdinfo` about the
pids Fleet already holds answers the same thing in **22µs** synthetically and **95–270µs** against a
real agent CLI — between 240 and 2,900 times cheaper, with no subprocess, and it is the route that
can match the pair. On the current route the cost is not on the tool-call path but it is in the
accept loop: five Drones starting together had their identities established over 472ms, against
67ms for the same five Drones with the lookup removed.

Measured 2026-08-29, against the agent CLI at 2.1.251 on darwin 27.

## What it was measured over

| | |
|---|---|
| Agent CLI | 2.1.251, the native install, invoked directly — `claude` on this machine's `PATH` is an unrelated wrapper and was not used |
| Invocation | spike 10's, unchanged: `-p --input-format stream-json --output-format stream-json --verbose --replay-user-messages --model haiku --permission-mode dontAsk --allowedTools … --strict-mcp-config --mcp-config <file>` |
| Server | [`010-identity-server.py`](010-identity-server.py) **unchanged**, so the only thing that varies from spike 10 is the number of Drones and the fact that they start together |
| Fleet size | five, all spawned inside 13ms, all from one config file naming one `http` endpoint |
| Lookups | [`012-peerpid.py`](012-peerpid.py) — four routes to the same answer, offsets into `struct socket_fdinfo` calibrated at runtime rather than hardcoded |
| Ground truth | the runner spawned every process it scores, so "the server knew which Drone" is a comparison against a pid this side already held |
| Configs | written under the spike's own directory and deleted after. The operator's own MCP config was not read or written, and the Fleet running on this machine was not touched |

**What differs from a real Drone.** No `setsid`, a smaller `--allowedTools`, and `haiku` for cost —
spike 10's three, unchanged. One more matters here: this spike ran inside an agent session, so every
ancestry chain below runs back through that session's CLI rather than through Fleet. The shape is
the same, the root is not.

## Five Drones, one listener

[`012-five-drones.py`](012-five-drones.py). All five started within 13ms of each other, all pointed
at one endpoint by one file, each told to call the same tool with its own Job name.

| conn | logged at | peer port | recovered pid | Job | payload said |
|---|---|---|---|---|---|
| 1 | +0ms | 58367 | 7731 | JOB-C | JOB-C |
| 2 | +123ms | 58368 | 7732 | JOB-D | JOB-D |
| 3 | +279ms | 58370 | 7729 | JOB-A | JOB-A |
| 4 | +377ms | 58372 | 7733 | JOB-E | JOB-E |
| 5 | +472ms | 58376 | 7730 | JOB-B | JOB-B |

Five connections, five Drones, one connection each, no failures — and the order the server met them
in is not the order they were spawned in, which is the point: nothing about the sequence carried the
identity. Recorded in [`012-server-five-drones.jsonl`](012-server-five-drones.jsonl) and
[`012-five-drones.json`](012-five-drones.json).

**The cheaper route was asked the same live connections and agreed on all five**, at 95–270µs
against a real CLI — more than the 22µs a small client costs, because the scan walks every open
socket of a Node process, and still three orders of magnitude under `lsof`.

## What the lookup costs

[`012-lookup-cost.py`](012-lookup-cost.py), with five child processes holding five live loopback
connections and the runner knowing which pid opened which port. Twenty samples each, median.

| route | how the pid arrives | median | right of 20 |
|---|---|---|---|
| `lsof -nP -i TCP:<port>` | scans every process on the machine | **64ms** | 20 |
| `lsof -nP -p <five pids> -a -i TCP` | the same tool, bounded to what Fleet holds | **42ms** | 20 |
| `netstat -anv -p tcp` | one process, a `process:pid` column | 3.1ms | **0** |
| `proc_pidfdinfo` over the five pids | a syscall per open fd, no subprocess | **22µs** | 20 |
| `getsockopt(LOCAL_PEERPID)` on `AF_UNIX` | the kernel answers | 0.6µs | — |

**`netstat` is not a slow route, it is not a route.** On darwin 27 it prints no `127.0.0.1` TCP rows
at all — connections `lsof` lists from the same process at the same instant are simply absent from
its output. That is why it scores zero rather than a time.

**Bounding `lsof` to five pids saves a third and no more**, because what it costs is a process
spawn and a kernel table walk, not the breadth of the question. The cheap answer is not a better
invocation of `lsof`; it is not invoking `lsof`.

## Trying to make it resolve wrong

[`012-race.py`](012-race.py) and [`012-race-client.py`](012-race-client.py). Three hazards, each with
a ground truth the runner spawned. `none` and `wrong` are counted separately throughout and never
added: one is a Drone that cannot be identified, the other is a Drone identified as another Drone.

### A — the same local port against the same listener is refused

A process binds a local port and connects. A second process binds the same local port, with
`SO_REUSEADDR`, and connects to the same listener while the first connection is still open on the
server side. It gets `OSError: [Errno 48] Address already in use` at `connect`, and the server
accepts one connection rather than two.

**So the obvious reuse hazard is closed by the kernel, and closed by the very condition that would
make it dangerous.** A four-tuple is unique while it exists; the server holding the old connection
open is what keeps it existing. The window in which the pid is ambiguous *for that listener* is the
window in which the port cannot be taken.

### B — one local port number, two live holders: reproduced

The hazard the kernel does not close is a second connection to somewhere *else*. Drone A's session
connection to Fleet from local port 24101, and Drone B's connection to anything at all from local
port 24101, are both legal, both live, and indistinguishable to a lookup keyed on 24101.

| scan order | `lsof -i TCP:<port>` | `lsof -p <pids>` | `proc_pidfdinfo`, local port | `proc_pidfdinfo`, port pair |
|---|---|---|---|---|
| impostor spawned second | right | right | right | right |
| impostor spawned first | **wrong** | **wrong** | **wrong** | right |

Three of four routes named a pid that was not the peer. Which way they go is decided by scan order
and nothing else, so "right when the impostor came second" is not a route being sometimes right —
it is the same wrong question getting lucky.

**The fix is free.** `insi_fport` sits four bytes from `insi_lport` in the same record. Matching both
was right in both orderings, and this is the route that costs 22µs.

### C — the peer exits before the lookup finishes

The client exits `hold` ms after connecting; the lookup starts at accept. 24 connections per row.

| peer lives | `lsof -i TCP:<port>` (~62ms) | `proc_pidfdinfo` (~47µs) |
|---|---|---|
| 0ms | 0 right, 24 none | 0 right, 24 none |
| 1ms | 0 right, 24 none | **23 right**, 1 none |
| 2–25ms | 0 right, 24 none | **24 right** |
| 30ms | 6 right, 18 none | 24 right |
| 35ms | 17 right, 7 none | 24 right |
| 40ms | 23 right, 1 none | 24 right |
| 45ms and above | 24 right | 24 right |

**Across all 384 connections, in both routes, the wrong column is zero.** The window is not a race,
it is a deadline: a peer that does not outlive the lookup is unidentifiable, and it is unidentifiable
as *nothing*, because a dead process holds no socket and the port it held is not yet anyone else's.

The deadline is about 40ms for `lsof` — under its own 62ms cost, because the process it wants is met
partway through the scan rather than at the end — and about 2ms for the syscall route. **Every
connection an agent CLI session makes is far longer-lived than either**, which is why the n=5 run
lost nothing. It is short-lived callers that this bounds, and the short-lived caller in the picture
is the next section.

## Many connections at once, from one Drone

[`012-parallel-curl.py`](012-parallel-curl.py). One Drone, granted Bash, told to fire four requests
at once by hand — spike 10's bypass, in a burst. The framing is neutral for spike 10's reason: asked
to send a Job name of `FORGED`, that spike's Drone refused it as impersonation, which is a fact about
the model and not about the transport.

| conn | logged at | pid | ancestry | payload |
|---|---|---|---|---|
| 1 | +0ms | 5752 | the Drone's own MCP client | — |
| 2 | +7293ms | 6340 | curl ← zsh ← **5752** | JOB-P1 |
| 3 | +7388ms | 6341 | curl ← zsh ← **5752** | JOB-P2 |
| 4 | +7479ms | 6343 | curl ← zsh ← **5752** | JOB-P4 |
| 5 | +7568ms | 6342 | curl ← zsh ← **5752** | JOB-P3 |

Four concurrent connections, four distinct pids, one shell, one Drone. Every one traced home, and
the payloads arrived out of order — which is what a burst does and what a per-connection identity is
indifferent to. Recorded in [`012-server-parallel-curl.jsonl`](012-server-parallel-curl.jsonl), with
the Drone's own transcript in
[`012-transcript-parallel-curl.ndjson`](012-transcript-parallel-curl.ndjson).

**This is also where the cost stops being free.** The `http` transport opens one connection and holds
it, so a Drone using the tool costs one lookup per session. A Drone using `curl` costs one lookup per
call, and the four above were logged 90ms apart — `lsof` serialising inside the accept loop. **The
number of lookups is decided by the caller, not by Fleet.**

## Where the 472ms went

The five Drones' connections were logged 123ms, 156ms, 98ms and 95ms apart. Run again against a
listener that accepts and asks nothing —
[`012-server-five-drones-control.jsonl`](012-server-five-drones-control.jsonl) — the same five
arrived 21ms, 1ms, 19ms and 26ms apart.

| | first to fifth |
|---|---|
| `010-identity-server.py`, `lsof` in the accept loop | 472ms |
| the same five Drones, no lookup | 67ms |

So 405ms of the 472ms is the lookup, serialised, and the rest is five CLIs starting. It is
not on any tool call's path — the calls came later, on connections already identified — but it is on
the path of every connection behind it, and it grows with the fleet. At 22µs it does not.

## What this does not answer

- **Whether ancestry survives a reparented process.** Every chain here was walked while the whole
  line was alive. A Drone that exits while its `curl` is in flight leaves that `curl` reparented, and
  nothing above measured what the walk returns then. The direct peer pid is unaffected; the bypass
  case is the one that rests on the walk.
- **What any of this costs in Rust.** Every number is Python calling `libproc` through `ctypes`, or
  Python spawning `lsof`. The syscall count is the implementation's; the marshalling is not.
- **Whether a pid is a durable key**, still. Spike 10 left this open and it stays open: what is
  measured here is that a *stale* lookup returns nothing, not what a Fleet holding a pid-to-Job map
  across a Drone's death and a pid's reuse would do.
- **n beyond five.** Five is the number `#50` names. Nothing here says where the accept loop stops
  keeping up, only that the per-connection cost is the thing that would decide it.
- **Anything about `#50`'s readiness.** The working slot is `Option<Working>` and singular throughout
  Fleet, and `#47`'s write-scope reservation is unbuilt. This measured a signal, not a system.
