# Measured behaviour

Things that are true about the environment Armada runs in, which a reasonable person would
have assumed otherwise. **Every entry here was measured, not read.**

This file exists because Armada is being built greenfield, deliberately without continuous
validation against a real repository until phase 6. That trade buys isolation and costs
empirical feedback — so the feedback that *is* obtained has to be written down or it is lost
between phases.

## How to use this file

- **Before** designing a mechanism that depends on a tool's behaviour, read the relevant
  section. If it is not covered, test it — do not infer it from documentation.
- **After** discovering something surprising, add it here with the command that demonstrates
  it, so the next reader can re-run rather than re-trust.
- An entry earns its place if believing the opposite would produce a plausible design that
  silently does not work. Entries that merely record how something works belong in the
  design documents instead.
- **Name the platform, or measure on both.** Armada supports darwin and Linux, and phase 2
  put a darwin-only result in this file as though it were general — it was wrong on Linux,
  and CI found it. An unqualified entry here reads as "true everywhere", so an entry measured
  in one place says where.
- **An entry's evidence must have been falsified once before it is cited here.** The same entry
  rested on an assertion that could not fail, so its evidence was decoration. This file's whole
  claim is that an entry was measured rather than read, and an assertion nobody inverted is
  neither. The general rule is owned by `ARCHITECTURE.md` §2.1.1; the incident that produced it
  stays here, in the correction note on the zombie-group entry that suffered it.

Record for each: what was measured, the version it was measured against, the command that
shows it, and what breaks if you assume otherwise.

---

## SQLite — the lease mechanism depends on these

Run on darwin 2026-08-09. **Language-neutral, and deliberately not platform-qualified beyond
that:** every one of these applies whatever Armada is written in, and locking, `BEGIN
IMMEDIATE` and `busy_timeout` are properties of SQLite rather than of an operating system.
This is the machinery `PLAN.md` §4.3 rests on.

### `BEGIN IMMEDIATE` is mandatory. `busy_timeout` cannot substitute for it

A DEFERRED transaction that reads and *then* writes fails the moment another writer committed
in between — **after 0.0 ms, with `busy_timeout=5000` set and WAL enabled**:

```
DEFERRED upgrade: OperationalError('database is locked') after 0.0 ms
IMMEDIATE:        succeeded
```

The error is `SQLITE_BUSY_SNAPSHOT` and it is **non-retryable by design** — the reader's
snapshot is stale, so no amount of waiting can rescue it. `busy_timeout` never applies.

**If you assume otherwise:** you write the obvious lease code — read the lease row, decide,
write it back — inside a default transaction. It works in every test with one writer, and
fails nondeterministically under exactly the contention the lease design exists to handle.
Setting `busy_timeout` looks like the fix and does nothing.

**Rule: any transaction that may write starts with `BEGIN IMMEDIATE`.** No exceptions in the
lease or claim paths.

### WAL is a property of the file, not a driver default

`journal_mode=WAL` persists in the database file once set, but no driver sets it for you.
**Measured: `rusqlite`'s `Connection::open` ships `busy_timeout=5000`** — an earlier draft of
this entry said "some drivers ship 0", which is true of other drivers and not of ours. Set both
explicitly anyway; relying on a driver default is how it changes under you.

### `SQLITE_BUSY` (5) is retryable. `SQLITE_BUSY_SNAPSHOT` (517) is not

Two different failures that both print `database is locked`. Measured:

| Contention | Extended code | Time to fail | Retry? |
|---|---:|---:|---|
| `IMMEDIATE` against a held `IMMEDIATE` | **5** | **5.21 s** — the full `busy_timeout` | **Yes.** It waited its turn and lost |
| `DEFERRED` read, then write, with a committed writer in between | **517** | **31 µs** | **No.** The snapshot is stale; waiting cannot help |

**If you assume otherwise:** you write one error handler for "database is locked" and either
retry 517 forever, or give up on 5 when waiting would have worked. **Branch on the extended
code, never the message.** Anything that waited the full `busy_timeout` is a genuine queue you
lost; anything that failed in microseconds is a design error in the transaction.

## MCP

Verified against the live specification and SDK, phase 0, on darwin — the specification's own
wording and the SDK's module layout are the same wherever it is installed.

### The base protocol is stateless as of spec revision `2026-07-28`

The specification states the base protocol as *"JSON-RPC message format, **stateless,
self-contained requests**, per-request capability negotiation."* There is no long-lived
session to hold between requests.

Nuance worth keeping: this does **not** mean initialization vanished entirely. Extensions are
*"negotiated during initialization"*, and `InitializationOptions` still exists in the SDK. A
report phrased as "no initialize, no sessions" is right about the base protocol and overstated
as a general claim.

### Python SDK 2.0 removed `FastMCP`

```sh
uv run --with mcp python -c "import mcp.server as s; print(dir(s))"
# mcp 2.0.0
# FastMCP  -> ModuleNotFoundError
# MCPServer -> present
```

**If you assume otherwise:** you follow any tutorial or existing server written against
`FastMCP` and it fails at import. Every pre-2.0 MCP example is a dead template, including the
one in PLAN.md §9.

### The Tasks extension exists for long-running operations

*"Asynchronous execution of long-running operations, with polling, mid-flight input, and
durable handles."* This is the standard shape for exposing something like a ten-minute
`armada manifest check` over MCP — worth using rather than inventing a bespoke polling protocol.

### `rmcp` is **3.1.2**, and it has the Tasks extension

Re-checked against crates.io at the start of M3, because this file already said the SDK moves
fast and the plan said "v3.x, verify before starting". What the check found, and all three
mattered:

| Claim | What is true at 3.1.2 |
|---|---|
| Feature set | `server`, `macros`, `schemars`, `transport-io` is the whole of a stdio server. `client` and every HTTP transport stay off. |
| Tasks | `rmcp::task_manager::TaskManager` implements SEP-2663 server-side — spawn, `tasks/get`, `tasks/update`, `tasks/cancel`, TTL expiry. Nothing had to be written. |
| Cost | 77 transitive packages, including tokio. It is the **only** async in this workspace, and the first dependency that brings a runtime. |

**Two API shapes that a v2 example gets wrong.** `ServerInfo` is `#[non_exhaustive]`, so
`ServerInfo { capabilities, ..Default::default() }` does not compile from another crate —
`ServerInfo::new(caps).with_instructions(…)` is the builder that does. And `#[tool_router]`'s
generated `tool_router()` is an associated *function* rather than a field, so a struct holding
a `ToolRouter` field alongside it gets a dead-code warning, not a second router.

**A tool description is `inputSchema` prose.** `schemars` lifts each argument's doc comment
into the tool's schema, so those comments are read by the model and not by a maintainer.

### Claude Code renames a dotted tool

Measured, 2.1.233. A server that advertises `fleet.status` — which the specification allows,
dots are legal in a tool name — is exposed to the model as `mcp__armada__fleet_status`. The
dot becomes an underscore.

**If you assume otherwise:** a prompt, a persona's `tools:` list or an `--allowedTools` flag
written with the documented name silently matches nothing, and the model reports it has no such
tool. The wire name stays dotted; only the client's spelling changes, so
`tools/list` is not where you will notice.

### stdout is the transport, and one line of ordinary output breaks it

Measured against a real client: `armada mcp serve` wrote its shutdown envelope to stdout
through the same path every other verb uses, and it arrived after the last JSON-RPC response as
a trailing document no parser accepts.

The verb reports on **stderr** for that reason — the same rule the spinner and the interview
prompt already follow, arriving at the one verb where stdout belongs to a protocol rather than
to a person. A failure is routed the same way, which is why `mcp::serve` returns its own error
*inside* an `Output` rather than as an `Err`: `main` renders an `Err` onto stdout under
`--json`.

## Ports — what a bind probe can and cannot see

**Measured on darwin, and not re-run on Linux.** What a `bind()` refuses is decided by the
kernel, and the `IPV6_V6ONLY` and `SO_REUSEPORT` rows below are the two that could plausibly
answer differently there — treat them as unverified on Linux until someone measures them. Do
not assume the section transfers untested: the zombie-only-group entry in the POSIX-primitives
section below is the one kernel-level result in this file that *was* re-run on Linux, and it
came back different.

### An IPv6-only listener is invisible to an IPv4 bind probe

| Holder | IPv4 `bind()` probe |
|---|---|
| `127.0.0.1` | `EADDRINUSE` — correctly detected |
| **`0.0.0.0` or `::`, probe with `SO_REUSEADDR`** | **succeeds — reports FREE** |
| **`::1` only, `IPV6_V6ONLY`** | **succeeds — reports FREE** |
| `SO_REUSEPORT` on holder **and** probe | **succeeds — reports FREE** |
| `SO_REUSEPORT` on holder, plain probe | `EADDRINUSE` |

**`SO_REUSEADDR` *does* defeat the probe against a wildcard holder, and the row above is a
correction.** Re-measured 2026-08-14 on darwin, against Docker 29.6.2, while `armada manifest
up` was reporting `RESERVED` for a container that was serving traffic:

```
holder: docker publishing 0.0.0.0:5460 and [::]:5460
  bind 127.0.0.1:5460 without SO_REUSEADDR  -> EADDRINUSE   (taken)
  bind 127.0.0.1:5460 with    SO_REUSEADDR  -> SUCCEEDS     (reads as FREE)
  bind [::1]:5460     with    SO_REUSEADDR  -> SUCCEEDS     (reads as FREE)
  connect 127.0.0.1:5460                    -> OK           (something answers)
```

Two things make this bite rather than being a footnote. **Rust's `TcpListener::bind` sets
`SO_REUSEADDR` on every socket** and the standard library offers no way to unset it, so char's
probe is the `with` row and never the `without` row. And **Docker publishes by binding the
wildcard**, so this is not an exotic holder — it is every container char has ever started.

**If you assume otherwise:** the probe reports every published port as free. `armada manifest
up` reports `RESERVED` for a healthy compose service, `armada manifest status` renders it
`DOWN` while it is serving traffic, and `init`'s `CONFLICT` detection cannot see a container at
all — which is the single holder this project exists to manage. It reads as a bug in the port
*transform*, which is where an afternoon goes.

**The fix is a second probe, not a different one.** A `connect()` sees the wildcard holder; the
bind sees a socket bound without `listen()`, which a connect cannot. They are complementary,
so `port_is_taken` asks both and takes either yes.

> **This entry has now been wrong in both directions, which is the instructive part.** An early
> `PLAN.md` §3.1 draft claimed `SO_REUSEADDR` defeated the probe and cited this file for a
> measurement that was not here — so the claim was struck, correctly, as uncited. Striking an
> uncited claim is not the same as measuring its negation, and the replacement text asserted
> the negation as though it had been. Nobody had run it either way until a container reported
> itself down. **An entry that says "X does not happen" needs the same evidence as one that
> says it does.**

**The test for this fails when run *inside* a container, and only there.**
`net::tests::a_wildcard_holder_is_seen_even_though_the_bind_probe_cannot_see_it` needs a
wildcard bind to be visible to a second socket in the same network namespace; a container's
netns does not reproduce the host's `SO_REUSEADDR` behaviour, so the test's premise does not
hold there. Measured 2026-08-15 in `rust:1-bookworm`: it fails on every run in the container
and passes on both CI runners and on a developer's machine.

**Ignore that one failure when verifying a change in a Linux container, and do not "fix" it.**
Every other test in the workspace is a real signal there — the platform difference chased in
`reading_a_frame…` below was found this way and was not a container artefact. Deciding which
of the two a Linux-only failure is, before concluding anything, is the whole skill.

**`PLAN.md` §3.1's stated reason for rejecting `connect()` is also wrong**, and is corrected
with it: it says a connect *"reports a listening-but-idle socket as free"*. Measured, a
`connect()` to a listening socket completes whether or not the listener ever calls `accept` —
idleness is invisible to it. The real limit is narrower and is the one stated above.

### A bind probe is itself a bind, so two concurrent probes collide

Measured 2026-08-10 on darwin, while writing phase 2's golden suite. Two processes (or two
threads) probing the *same* port at the same instant make one of them see the
other's momentary listener and report the port taken:

```
thread A: bind 127.0.0.1:5460 -> ok        (held for microseconds, then dropped)
thread B: bind 127.0.0.1:5460 -> EADDRINUSE
```

**If you assume otherwise:** a golden snapshot that records a port's state
becomes flaky in exactly the way that looks like a real conflict, and
`armada manifest status --all` run twice at once can report a `CONFLICT` that does not
exist. It is inherent to bind-probing rather than a bug to fix — `connect()`
answers a different question — so the rule is that a probe's answer is a
point-in-time reading and nothing may be serialised against it.

**A mutex was the wrong half of the answer, and it took three agents to see
it.** Armada's own suite used to serialise the runs that snapshot a port state,
which fixes two threads and cannot fix two *processes* — and two processes is
the case that kept failing, because every Armada on a machine claimed its first
block from the same hardcoded base. So a developer's own stack, a second agent's
suite, or a container left running from the last one made a golden snapshot
report `CONFLICT`, intermittently, from a machine that was behaving correctly.
Three separate agents hit it and two recorded it as flakiness.

**The suite was asserting on global machine state, and the fix is to stop.**
`port_base` is a `machine.yml` key (PLAN.md §4.3.1), each scratch machine takes
its own high one, and the golden redaction maps the claimed ports back onto the
documented `5460` so the snapshots keep their offsets and lose only the floor.
The intra-process mutex went with the reason for it. **A test that fails when
something unrelated is running on the machine is not flaky — it is asserting on
something it does not own.**

So a probe must bind **both** `127.0.0.1` and `[::1]` and treat either `EADDRINUSE` as taken,
and `armada manifest status` must connect on both families before reporting `RESERVED`. `SO_REUSEPORT` on
both sides remains undetectable; nothing Armada does prevents that, so it is a known limit rather
than a bug to fix.

## Rust — the POSIX primitives, and the rules each one forces

Both sit in machinery `PLAN.md` §7 calls load-bearing, and both are one line you must not
forget. Measured 2026-08-09 on darwin, against Rust 1.97.1. **Every entry below is darwin-only
and unverified on Linux unless it names a platform.** This is the section where the one entry
that *was* re-run on Linux — the zombie-only group, below — came back different, so the
absence of a Linux column here is a gap rather than a claim of agreement.

### `SIGPIPE` is set to `SIG_IGN` at startup — `armada manifest status | head` panics until you fix it

Rust's runtime ignores `SIGPIPE`, so a write to a closed pipe returns `EPIPE`, `println!`
unwraps it, and the process **panics with exit 101 and a backtrace on stderr** — worse than
the Python it replaced. `#[unix_sigpipe]` is not stabilised on 1.97.1.

**Rule: restore the default disposition at the top of `main`,** before anything writes:

```rust
unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL); }
```

This yields exit **141** and a silent, correct death — the ordinary Unix behaviour. It is one of the four `unsafe`
blocks the design permits.

### `setsid` is not in `std`, and it is mutually exclusive with `process_group(0)`

`Command::process_group(0)` gives a new process *group* in the **same session**. Detaching
from the controlling terminal — what `armada manifest up` requires — needs `setsid` via `libc` inside
`pre_exec`. That is the second of four permitted `unsafe` blocks.

**Measured: setting both fails.** `process_group(0)` *and* `pre_exec(setsid)` on the same
`Command` returns `Operation not permitted (os error 1)` — `setsid` fails when the caller is
already a process-group leader, which `process_group(0)` has just made it. **Pick `setsid`
alone.** An earlier draft of this entry stated both rules without saying they conflict.

### `killpg` against a `setsid`'d group does reach grandchildren — verified

The project's central cleanup claim, measured rather than assumed:

```
child pid 26860, pgid 26860, sid 26860   (own session, via pre_exec setsid)
sh -c 'sleep 300 & sleep 300'            -> 3 processes in the group
libc::killpg(pgid, SIGTERM)              -> rc=0
procs in group after: 0                   <-- CLEAN
```

Phase 2's done-when depends on this and nothing had confirmed it in Rust.

### `killpg(SIGTERM)` kills nothing if the group leader ignores SIGTERM

```
pgid=95793   before=3   after killpg(SIGTERM)=3   after killpg(SIGKILL)=1
```

The leader ran `trap '' TERM`, and children inherit an *ignored* disposition across `fork` and
`exec` — so one uncooperative leader immunises its whole group. The earlier entry above proves
`killpg` reaches grandchildren; it used a cooperative `sleep`, so it proves the group
mechanism and says nothing about the stop policy. **SIGTERM, wait a grace period, then
SIGKILL** — an unconditional escalation, not a retry, because a process that ignores SIGTERM
ignores the second one too.

Worse and not fixable this way: a service that calls `setsid` itself — ordinary daemonizing —
leaves the tracked group entirely, so its pgid is not the one recorded and no `killpg` reaches
it. That case is detected by the port still being bound after `down`, not prevented.

### `Child` dropped without `wait()` leaves a zombie

```
child exits, Child dropped without wait() -> ps stat "Z"
```

Rust's `Child` does **not** reap on drop, and the docs say so, but the consequence is easy to
miss: Armada spawns services, checks, ready-probes, docker calls and secret providers. Every one
whose handle is dropped without `wait()` leaves a `<defunct>` entry until Armada itself exits.

**Rule: every spawned `Child` is waited on, or explicitly reaped.** A long-lived
`armada manifest check --detach` accumulating zombies across a fifteen-minute run is the case that bites.

## Claude Code CLI — the flags a headless turn needs

### `--output-format stream-json` requires `--verbose`, and `--help` does not say so

Measured 2026-08-14 against Claude Code as installed, by a real `armada fleet spawn` whose
Drone died instantly:

```
claude --session-id <uuid> --print --output-format stream-json <prompt>
-> exit 1
   Error: When using --print, --output-format=stream-json requires --verbose
```

The requirement is enforced at argument-parse time — before any API call, so a wrong argv costs
nothing but produces no turn at all. **It is not stated in `claude --help`**: the entry for
`--output-format` lists the three formats and says only *"only works with --print"*, and
`--verbose` is described as *"Override verbose mode setting from config"*, which reads as a
preference rather than a precondition.

**What this cost, and the rule it produced.** Fleet's entire Drone-argv suite passed — unit
tests on the built vector, and an integration test running a stub that recorded what `execve`
received — while **no Drone had ever run**. Every one of those assertions was about a string
Armada built correctly and the binary rejects.

> **Asserting on argv proves you built the argv you intended. It does not prove the argv is
> accepted.** Those are different claims, and a suite that only makes the first one is green on
> a program that never starts.

Two answers, because neither is sufficient alone:

- **The requirement is data, not a literal** — `fleet::drone::STREAM_JSON_NEEDS` — and a pure
  test asserts every streaming argv carries it. That makes a future edit that drops the flag a
  red test rather than a silent regression.
- **`armada doctor` holds every flag the Drone uses against `claude --help`**, which is free and
  spends no token. It catches a flag renamed or removed by a new version. **It cannot catch a
  new *combination* rule**, because `--help` does not state this one — and `--help`
  short-circuits before argument validation, so there is no free probe that would. That
  limitation is stated rather than papered over: the check narrows the class, it does not close
  it.

### A headless session with no permission posture stalls rather than failing

Diagnosed 2026-08-15, from a day of Jobs that spawned, recorded a worktree and a port block, and
then sat at `STALLED` until their wall-clock ceiling filed an inbox entry.

**Nothing in the Drone's argv ever granted a capability.** `--strict-mcp-config` and
`--disable-slash-commands` *withhold*; every other flag described the output or named the
session. So the Drone reached its first state-mutating tool call, Claude Code asked a person for
permission, and there was no person — `--print` has no terminal to prompt on.

> **A missing capability does not fail. It waits.** That is what makes it worse than a usage
> error: a rejected argv dies in a second and shows up in the log, and this one burns the whole
> ceiling and then reports a timeout, which is a symptom of nothing.

**The mode is what decides between the two.** Read off `claude --help`, measured 2026-08-15:

```
claude --permission-mode bogus
-> error: option '--permission-mode <mode>' argument 'bogus' is invalid.
   Allowed choices are acceptEdits, auto, bypassPermissions, manual, dontAsk, plan.
```

`dontAsk` denies what the allow list does not cover and carries on. `acceptEdits` and `manual`
prompt for it — which is the same stall one flag later, and the reason the fix is not "grant
edits". `bypassPermissions` and `--dangerously-skip-permissions` are never the answer for an
unattended session; see `fleet::drone::Posture`.

### `--allowedTools` is variadic, so what follows it must be a flag and not the prompt

Measured 2026-08-15:

```
claude --allowedTools Edit --unknown-xyz
-> error: unknown option '--unknown-xyz'
```

The list ends at the next `--` word, and at nothing else. **A bare word after it is consumed as
one more tool name** — so an argv that appended the posture to the end of the flags would have
handed a Job's task to `--allowedTools` and started a turn with an empty prompt.

The answer is ordering rather than quoting: `fleet::drone::headless` emits the posture *before*
`--print`, so there is always a flag between the last rule and the prompt, and an empty list
emits no flag at all. `the_prompt_never_follows_a_variadic_list` holds it there for every shape
of posture including the two where a list is empty.

**A rule may contain a space; a list entry may not.** `--allowedTools` is documented as a
*"comma or space-separated list"*, and its own example — `"Bash(git *) Edit"` — is one argument
holding two rules, one of which contains a space. So the separator is paren-aware:
`Bash(git push:*)` is a single rule, and `Edit Write` written as one entry is two.
`Posture::wrong` refuses the second before a Drone is started.

### There is no `--mcp` flag. The corpus specified one, and it does not exist

Read off `claude --help` on 2026-08-15, while building [`armada helm`](commands/helm/helm.md).

[`PLAN.md`](PLAN.md) §15.1 and `commands/helm/helm.md` both wrote Helm's launch as:

```
claude --agent helm --mcp armada --resume <helm-session-uuid>
```

**`--mcp` is not an option Claude Code has.** The flag that registers MCP servers is
`--mcp-config <configs...>`, and it takes JSON files or strings rather than a server name — so
there is nothing for a bare name to refer to. Written as specified, the argv would have been
rejected at parse time, or worse, accepted with `armada` read as a prompt.

**The corpus is a design document, not an option list, and this is the difference stated as an
incident.** Every argv in `docs/commands/**` is a claim about another program's interface, and
the only thing that settles one is the program. Three flags in Helm's launch were checked
against `claude --help` before a line of it was written, and one of the four named in the design
was wrong.

The answer is the same shape as the `--verbose` entry above and shares its machinery:
`helm::FLAGS` states every flag the launch uses as data, and `armada doctor` holds all six
against `claude --help`. It is free and starts no session.

**No session probe for Helm, and that is deliberate.** The Drone's probe is safe because a
headless turn with closed stdin makes no API call; Helm is interactive and has no equivalent
that is provably free. A check that might open the reader's orchestrator — against their
account, on a machine they were only asking about — is not worth the coverage, so `doctor` asks
`--help` and `plugin validate` and nothing else.

### `ps -o lstart=` answers for a zombie, so a start-time probe outlives the process

Measured 2026-08-14 on darwin 27.0.0, by an assertion that failed: `armada-fleet`'s
`drone::alive` reported a Drone as alive immediately after `stop` had confirmed the group
gone.

```
child in its own session is SIGKILLed, parent does not wait
ps -o lstart= -p <pid>   ->  a start time, exit 0     (the corpse answers)
killpg(pgid, 0)          ->  the group is empty       (see the entry below)
```

Armada's liveness check is `pid_started_at` recorded against `pid_started_at` observed
(`reap::pgid_is_ours`), and both readings come from `ps -o lstart=`. A **zombie answers that
question with the same start time it had while running**, so the check says *provably ours and
still there* about a process that has exited. That is the correct answer to the question it
actually asks — "is this the same process I recorded" — and the wrong answer to the one a
reader assumes it asks.

**It does not affect Armada, and the reason is worth stating rather than being lucky about.**
The reaping parent is the `armada` invocation that spawned the Drone, and it exits moments
later; init then reaps, so the *next* invocation — which is the only thing that ever asks — sees
nothing. Every caller of `alive` is a fresh process by construction.

**Rule: never assert `!alive(...)` in the same process that started the child *unless something
has reaped it first*.** `posix::stop_group` now does that reap (see the entry below), so an
`alive` asked after a `stop` is answering about a pid that no longer exists and is trustworthy
on both platforms. An `alive` asked about a child this process signalled by hand, and never
waited on, is still reading a corpse's start time and still says yes.

The sentence this entry carried before — *"ask the group instead: a second `stop_group` reports
the group empty"* — was true on darwin and false on Linux, and the reason is the entry directly
below. It is not advice, it is the same measurement seen from the other side.

### A group holding only a zombie: `killpg` fails on darwin and **succeeds** on Linux

Measured 2026-08-10 on darwin and, through CI, on `ubuntu-latest`. A child in
its own session — so its pid genuinely is a process-group id — exits and is not
waited on:

```
ps -o stat= -p <pid>     ->  Z
killpg(pgid, 0)          -> -1  EPERM      (darwin)
killpg(pgid, 0)          ->  0  succeeds   (Linux)
kill(pid, 0)             ->  0  succeeds   (darwin — the *process* still exists)
after waitpid:
killpg(pgid, 0)          -> -1  ESRCH      (both)
```

Three things there are each easy to get backwards. A zombie is still a member of
its group, so a signal-0 probe is answering "does this group still have
members", not "is anything running". The two platforms then disagree about that
question while the corpse is unreaped. And the errno on darwin is **`EPERM`, not
`ESRCH`** — code that branches on `ESRCH` specifically sees neither the darwin
answer nor the Linux one.

**If you assume otherwise:** Armada confirms a kill with the signal-0 probe, and
for a caller that *parented* the group and has not reaped it the two platforms
then fail in opposite directions. On darwin the zombie-only group answers
`EPERM`, so `group_alive` is false and `stop_group` takes its first grace poll
as proof the group is empty: it returns `gone`, never waits out the grace and
never sends its SIGKILL — the right answer for the wrong reason. On Linux the
same probe succeeds, so `stop_group` waits out the whole grace period,
escalates to SIGKILL, and *still* reads the group as alive, returning
`gone: false`. A group that died on the first SIGTERM therefore reports `CLEAN`
on darwin and `FAILED` on Linux, and only one of those is even accidentally
right. **Reap before reading the probe as "empty", and reap before counting `ps`
output.**

**It was called a test-shaped hazard, and that was the third mistake in this
entry.** The sentence read: *"this is a test-shaped hazard rather than a
production one, because the case Armada actually reclaims is an orphan"*. Every
clause of it is true and the conclusion does not follow — Armada reclaims
orphans **and** it kills children it started itself. `armada fleet kill` calls
`fleet::drone::stop` on a Drone whose handle `drone::start` dropped on purpose,
and the invocation that started the Drone is still its parent, because
**`setsid` moves a child to a new session and never to a new parent**. Between
the spawn and this process's exit, the Drone is Armada's own unreaped child by
construction. So on Linux `armada fleet kill` reported *"the Drone would not
die"* about a Drone that died on the first SIGTERM — the exact `FAILED`-for-a-
clean-kill described above, in production, on the platform most users are on.

Measured 2026-08-15 in `rust:1-bookworm`, the same assertion both ways:

```
                                              darwin        Linux
stop(): sh -c 'sleep 60', handle dropped
  before the fix                              Stopped 0.14s Survived 5.54s
  after                                       Stopped 0.14s Stopped  0.02s
```

The 5.5 seconds is the tell: it is `GRACE` plus the post-SIGKILL wait, so the
whole escalation ran and the probe still said the group was populated.

**The fix is one reap, in the probe's own path, not three patched callers.**
`posix::stop_group` calls `posix::reap_group(pgid)` — `waitpid(-pgid, WNOHANG)`
in a loop — before *every* reading of the group, so the group is always read
after this process's own dead have stopped counting as members of it.
`stop_group_reaping` is the same function with the reap supplied by the caller:
`ProcessGroup` passes its own `try_wait`, because a bare `waitpid` would consume
an exit status that its `Child` handle is about to be asked for and leave the
handle answering `ECHILD`. Two rules fall out, and both are load-bearing:

- **`waitpid(-pgid)`, never `waitpid(-1)` and never a non-positive pgid.** `-1`
  reaps whichever child exited first — another Job's Drone, another test's group
  leader — and hands its status to nobody. `waitpid(0)` is this process's own
  group. Both are refused rather than negated.
- **`WNOHANG`, so the reap collects corpses and never waits for one.** That is
  what keeps `Survived` a real answer: it is still what a member wedged in
  uninterruptible sleep, or a corpse whose parent is some other process that
  will not wait on it, produces. A reap that blocked would have deleted the
  signal instead of correcting it.

The orphan case is unchanged and free: `waitpid` on a process that was never
this one's child returns `ECHILD`, so the reap does nothing and init's has
already happened.

> **This entry was wrong in its first form, and how it was wrong is the
> instructive part.** It read *"a zombie answers `ESRCH` to `kill(pid, 0)`"*,
> measured on darwin only, and cited an assertion that appeared to prove it. The
> assertion was **vacuous**: it passed the pid of a child spawned *without*
> `setsid` to a `killpg` probe, so it interrogated a process-group id that had
> never existed, and any answer would have looked like confirmation. Two rules
> this file already states were both broken at once — measure rather than infer,
> and measure on the platforms the project supports. It survived local review
> and was caught by CI running the same suite on Linux.
>
> The correction then made the same mistake once more, and review caught that:
> the **If you assume otherwise** paragraph above attributed the Linux sequence
> — whole grace, SIGKILL, group still alive — to both platforms, a sentence
> carried over from an earlier summary rather than read off the table a dozen
> lines above it in this very entry. Reasoning from the narrative instead of
> from the numbers is the failure this entry exists to document.
>
> And a third time, caught by CI on 2026-08-14: the entry closed by calling the
> whole thing *"a test-shaped hazard rather than a production one"*. That was
> reasoning from a category — *Armada reclaims orphans* — over a call path it
> had not traced. `fleet::drone::start` abandons a child it is still the parent
> of, so `drone::stop` is the production caller the sentence said did not exist.
> The pattern across all three corrections is the same: **each one restated a
> conclusion instead of re-reading the measurement or the code under it.**

### `std::process::exit` skips a `BufWriter` flush — and it is size-dependent

```
payload   491 bytes  ->  0 bytes delivered      <-- the entire envelope is lost
payload 20054 bytes  ->  20054 bytes delivered  <-- passes, for the wrong reason
```

`BufWriter` flushes on `Drop`; `process::exit` does not run destructors. A write larger than
the 8 KiB capacity bypasses the buffer and goes straight to the fd, which is why the large
case survives.

**If you assume otherwise:** you test the `--json` envelope with a big fixture, it passes, and
a small real payload is silently emptied — for the one consumer the contract exists to serve.
**Rule: flush explicitly before any exit path, or never write the envelope through a
`BufWriter`.**

**Everywhere else, `unsafe` is denied crate-wide.** **Three** exceptions, all in `adapters`'
POSIX process module, all a single call: `libc::signal` for SIGPIPE, `setsid` inside
`pre_exec`, and **`libc::killpg`** — which is an unsafe extern fn and is therefore rejected by
`#![deny(unsafe_code)]` despite appearing in this file's own verified snippet. An earlier
version of this note said two and omitted the one the whole cleanup model depends on.

## Secret-shaped test fixtures — GitHub push protection, and where the detector lives

Measured 2026-08-15 on darwin, against `armada-guild`'s `secrets.rs` and `cargo xtask
boundaries`.

### A realistic fixture blocks the push, and assembling it at run time is the wrong fix

`crates/guild/src/secrets.rs` tests `value_is_credential_shaped` against fixtures that must
*be* credential-shaped to exercise the code. A first version wrote them as realistic-looking
literals (`ghp_16C7e42F292c6912E7710c838347Ae178B4a`) and GitHub's push protection blocked the
push — correctly, since a scanner cannot know a `ghp_` string is synthetic.

The fix tried first was `fn shaped(prefix, rest) { format!("{prefix}{rest}") }`, splicing the
string together at run time so nothing scannable sat in the source. **That defeats the scanner
rather than satisfying it** — the realistic body was still there, just invisible to detection,
which is strictly worse than the literal it replaced.

**The actual fix is cheaper than either.** `value_is_credential_shaped`'s three tests —
registered prefix, JWT's three segments, a long opaque run — never examine what follows a
prefix or fills the run. A fixture gains zero test coverage from looking realistic, so replacing
the body with an unmistakable marker (`ghp_EXAMPLE_NOT_A_REAL_CREDENTIAL_000001`) exercises the
same code path with none of the risk. Where a vendor publishes an official placeholder (AWS:
`AKIAIOSFODNN7EXAMPLE`), prefer that over inventing one. **If you assume a scanner-evasion trick
is ever the right move here:** it survives review once, then every later reader either trusts a
literal that could now be real, or has to re-derive that it can't be — the module's whole job is
telling those apart, so a fixture that only *reads* safe is a bug planted in its own test suite.

### `xtask` cannot import the detector it would need to enforce this repo-wide — `cargo xtask boundaries` says so

Tempting next step: have `cargo xtask privacy` reuse
`armada_guild::secrets::value_is_credential_shaped` to catch a scanner-evading fixture anywhere
in the repo, not just in `secrets.rs`'s own tests. Adding `armada-guild` to `xtask/Cargo.toml`
and running `cargo xtask boundaries` on it:

```
xtask/Cargo.toml:0  `xtask` (tooling) depends on `armada-guild` (Guild) (normal) — nothing
points upward (ARCHITECTURE.md §1.9); tooling may depend on [core]
```

`boundaries.rs` grants `Tooling` (`xtask`) no entries in `may_depend_on` at all — not even
`Core` gets a free pass there, since `to == Module::Core` is checked separately and `Tooling`
still isn't `from` in that branch for anything but itself. So `xtask` cannot depend on any
Armada module crate, full stop; reusing the guild's own detector from `xtask` is not available.

**If you assume otherwise:** the alternative — reimplementing the detector's rules a second time
inside `xtask` — is the same hand-copied-list drift `xtask/src/docs.rs`'s `FIXED` corpus already
demonstrated once: `docs/reserved/*.md` used to be a retyped list there too, and it silently
stopped covering new files until it was replaced with discovery. Two copies of
`CREDENTIAL_PREFIXES` would agree only until the day someone updates one and not the other. The
property therefore lives only
in `secrets.rs`'s own tests (`every_credential_shaped_fixture_is_self_evidently_synthetic`,
iterating the same `CREDENTIAL_SHAPED_FIXTURES` list the detection test asserts against), and
not in `xtask` — this is a deliberate scope decision, not an oversight.

## Serialization and parsing — what the golden snapshots depend on

Measured 2026-08-09 on darwin, against `serde_json` 1.0.151 and `serde_yaml_ng` 0.10.0 — the
crates the binary actually uses. Key ordering and scalar coercion are decided by those crates
rather than by the host, so no platform qualifier belongs on the entries themselves.

### `serde_json::Value` sorts object keys. Structs and `BTreeMap`s do not

```
struct fields          -> declaration order      {"version":…,"components":…}
BTreeMap keys          -> sorted
serde_json::Value      -> SORTED
    input  {"zulu":1,"alpha":2,"mid":3}
    output {"alpha":2,"mid":3,"zulu":1}
```

`Value`'s default map type is a `BTreeMap`, so anything routed through one comes out
alphabetised — including a payload assembled as a `Value` rather than serialized from a struct.

**If you assume otherwise:** `ARCHITECTURE.md` §1.6 makes golden snapshots byte-compared and
regenerated by hand, and every hand-written payload in this corpus is in *reading* order —
`schema_version`, `verb`, `workspace`, `status`, `error`, `data`. Serialize the same payload
through a `Value` and it emerges alphabetised, so a snapshot copied from the documents never
matches, and the obvious fix — reordering the snapshot — hides the fact that the renderer no
longer emits the documented order. **Rule: `--json` payloads and resolved configs are
serialized from structs, never assembled as a `Value`.**

### `serde_yaml_ng` deserializes an unquoted scalar straight into a `String`

```
env: { PORT: 3000 }   -> "3000"
env: { FLAG: true }   -> "true"
env: { DEBUG: null }  -> "null"      <-- four characters, in a child's environment
serde_json, same map  -> Err("invalid type: integer 3000, expected a string")
```

A newtype whose visitor implements only `visit_str` does **not** close this — also measured.
The YAML deserializer hands a plain scalar to `visit_str` whatever it looks like, so the only
thing that discriminates is deserializing into `serde_yaml_ng::Value` and matching on the
variant.

**If you assume otherwise:** the parser is quietly more permissive than the JSON Schema, which
says an env value is a string. A config with `DEBUG: null` loads cleanly, fails
`armada manifest config verify`, and in between puts the literal text `null` into a spawned process's
environment. `crates/core` rejects it at the `env:` block for exactly this reason —
see PLAN.md §4.1.1.

Two smaller facts from the same session, recorded because they change what `config verify`
needs to check: a **duplicate mapping key is an error**, not last-wins, so two components with
one name cannot reach the core; and `Error::location()` is populated for both syntax and typed
errors, which is what makes PLAN.md §4.1.1's decision 4 free.

## Typer / Click exit codes *(historical — Armada is Rust)*

Measured on darwin against **Typer 0.27.1**, phase 0, and not re-run on Linux — the exit codes
are Click's own, but the broken-pipe entry below turns on a signal disposition and is a
kernel-adjacent claim this file has been burned by before. **Kept because the source repo being harvested in
phase 3 is Python**, so these remain true of the behaviour being harvested — and because the
exit-code conclusions they produced (130 and 2 are conventional; broken pipe needs an explicit
decision) carried over to the Rust design intact.

### `KeyboardInterrupt` already exits 130, and usage errors already exit 2

```sh
python app.py sigint        # raises KeyboardInterrupt  -> 130
python app.py --bogus       # unknown flag              -> 2
python app.py clickerr      # typer.BadParameter        -> 2
```

Both are free — Armada does not need to catch and re-exit for them.

**This entry exists because it was reported as false.** A claim that Click collapses
`KeyboardInterrupt` into `Abort` and exits `1` was checked and does not hold for this version.
Re-measure after any framework upgrade: if it ever became true, it would be silent, and
`armada manifest check` interrupted by an agent would report a code meaning "the tool failed" instead of
"you interrupted me."

### Broken pipe — measured, and both options are wrong by default

```sh
#            app.py | head -2
# Typer default        -> exit 1    , 0 bytes stderr
# SIGPIPE=SIG_DFL      -> exit 141  , 0 bytes stderr
```

**Corrected 2026-08-09.** An earlier version of this entry said the behaviour "was not
measurable in the environment where the rest of this section was verified." It measures in
about a minute; the earlier attempt was defeated by `PIPESTATUS` not populating in the
harness, not by anything about Python. Use `set -o pipefail` in a subshell and read `$?`.

Two things to note, and neither is what was predicted:

- Click **catches `BrokenPipeError` silently** — no traceback, no "Exception ignored" at
  shutdown. The predicted mechanism does not occur.
- **Exit 1 is `tool_failed`** under Armada's own map, so `armada manifest status | head` currently reads as
  "the tool failed" to anything checking exit codes.
- `SIGPIPE=SIG_DFL` gives the correct Unix behaviour and code **141**, which `ARCHITECTURE.md`
  `ARCHITECTURE.md` §1.6 now carries in its exit table alongside the signal carve-out. Note the stdlib explicitly warns against this setting; the
  warning is about libraries that need to observe the error, which Armada does not.

Whichever is chosen, the "exit code = `f(error.class)`" rule needs an explicit carve-out for
signal-derived codes, covering `130` and `141` together.

## Docker CLI — reading labels back off a resource

Measured on darwin against **Docker 29.6.2**, 2026-08-10. Armada stamps three
labels on everything it creates and reaps by them, so reading them back is the
other half of the mechanism. `ls --format` and `inspect --format` are rendered
client-side by the CLI's own templater, so these are properties of the docker
client rather than of the host.

### `docker image ls --format` cannot print labels at all, unlike every other `ls`

```sh
docker ps           --format '{{.ID}}|{{.Labels}}'    # works
docker network ls   --format '{{.ID}}|{{.Labels}}'    # works
docker volume  ls   --format '{{.Name}}|{{.Labels}}'  # works
docker image   ls   --format '{{.ID}}|{{.Labels}}'
# template parsing error: can't evaluate field Labels in type *formatter.imageContext
docker image   ls   --format '{{.ID}}|{{.Label "x"}}'
# template parsing error: can't evaluate field Label  in type *formatter.imageContext
```

**If you assume otherwise:** the obvious uniform implementation — one `ls
--format` per kind — works for three of the four kinds and *errors* on images,
which is the kind holding roughly 2.1 GB per stale workspace. So labels are read
through `inspect`, which works for every type.

### …and `inspect` keeps labels in two different places

```sh
docker inspect --type=container --format '{{index .Config.Labels "k"}}'   # containers
docker inspect --type=image     --format '{{index .Config.Labels "k"}}'   # images
docker inspect --type=network   --format '{{index .Labels "k"}}'          # networks
docker inspect --type=volume    --format '{{index .Labels "k"}}'          # volumes
```

A label that is not set renders as the literal `<no value>` for containers,
images and networks, and as an empty string for volumes — so an absent label and
an empty one are indistinguishable in the text.

**Consequence Armada acts on:** labels are read as `{{json …}}` and parsed, not as
a delimited line. A workspace path may legally contain a tab or a newline, and
`armada.workspace_path` is a real path — a delimiter a value can contain is one
that eventually attributes a resource to the wrong workspace, which is the
failure the label exists to prevent.

## Docker disk usage — `docker system df`

Measured on darwin against **Docker 29.6.2**, 2026-08-15. Armada reports how much
disk a stale workspace is holding, and `docker system df` is the only command that
answers it without walking every resource by hand. The human table and the
`--format` template are two different surfaces with two different shapes, and the
verbose form is a third — so which one is parsed is a design decision, not a
formatting preference.

### The four `TYPE` values are literal strings, and one of them has a space in it

```sh
docker system df
# TYPE            TOTAL     ACTIVE    SIZE      RECLAIMABLE
# Images          1         0         4.171MB   4.171MB (100%)
# Containers      0         0         0B        0B
# Local Volumes   171       0         12.01GB   12.01GB (100%)
# Build Cache     0         0         0B        0B
```

The columns are TYPE / TOTAL / ACTIVE / SIZE / RECLAIMABLE, and the type names are
`Images`, `Containers`, `Local Volumes`, `Build Cache`. **`Local Volumes` is two
words** — it is not `Volumes`, and it is not lowercase.

**If you assume otherwise:** a parser that keys the rows on `volumes` matches
nothing and reports zero volume bytes, which is the single largest number on this
machine — 12.01 GB of 12.02 GB total. Splitting the human row on whitespace is the
same bug from the other direction: `Local Volumes` splits into two fields and every
column after it shifts left by one, so TOTAL reads as `Volumes`.

### `--format '{{json .}}'` emits one object per line — not an array — and every value is a string

```sh
docker system df --format '{{json .}}'
# {"Active":"0","Reclaimable":"12.01GB (100%)","Size":"12.01GB","TotalCount":"171","Type":"Local Volumes"}
```

Four such lines, one per type. The keys are `Active`, `Reclaimable`, `Size`,
`TotalCount`, `Type`. **All five values are strings, including the counts** —
`"171"`, not `171`. And `Reclaimable` is a compound: a size *and* a percentage in
one field, `"12.01GB (100%)"`.

**If you assume otherwise:** `serde_json::from_str::<Vec<_>>` on the whole stdout
fails, because the payload is four concatenated objects rather than a JSON array —
it has to be read line by line. A struct typing `TotalCount` as `u64` fails to
deserialise on every run. And `Reclaimable` parsed as a size yields garbage,
because the trailing `(100%)` is part of the value.

### `-v` is a different shape entirely, and its volume `Labels` is a comma-joined string

```sh
docker system df -v --format '{{json .}}'
# a SINGLE object: {"Images":[…],"Containers":[…],"Volumes":[…],"BuildCache":[…]}
# each volume row carries Name, Size, Labels, Mountpoint, Links, and others
# "Labels":"com.docker.compose.project=abc,com.docker.compose.volume=api_pgdata"
```

Adding `-v` does not add detail to the non-verbose shape — it replaces it. The
four top-level keys are arrays, and **`Labels` on a volume row is a single
comma-joined `k=v` string**, not the object `docker inspect` returns.

**If you assume otherwise:** you read ownership out of that string by splitting on
`,` and then `=`. The entry above — "…and `inspect` keeps labels in two different
places" — already records that a label value may legally contain a delimiter, and
`armada.workspace_path` is a real path, so a value containing a comma or an equals
sign attributes a volume to the wrong workspace. That is the exact failure the
label exists to prevent, arriving through the one surface that flattens labels back
into text.

**So ownership is read via `docker volume ls --filter label=…` or `docker
inspect`, and `df -v` is used only for the SIZE, matched by `Name`.**

### Every size is a human string, base 1000, and there is no raw-byte field anywhere

```sh
# unit spellings observed across 171 volumes and 1 image:  B  kB  MB
# examples seen:  0B   138.3kB   72MB   4.171MB   12.01GB
```

Lowercase `k`, capital `B`. Docker's humaniser is **base 1000**, so `kB` is 1000
bytes and not 1024; `GB`, `TB`, `PB` follow the same rule. No field in either
payload carries the raw byte count, so the string is the only number there is.

**If you assume otherwise:** you parse `kB` as 1024 and every reported figure is
2.4 % low by the gigabyte, which is small enough to survive review and large enough
to make a reclaim estimate wrong. Matching units case-insensitively is the other
half of the trap: `MB` and `mB` are the same to a lowercased match, and `B` is a
suffix of all of them, so a naive "ends with B" test strips the unit off `138.3kB`
and reads 138.3 bytes.

### A `--format` template naming a field that does not exist fails loudly — which is the reason to use one

```sh
docker system df --format '{{json .Nope}}'
# rc 1, nothing on stdout, and on stderr:
# template parsing error: template: :1:7: executing "" at <.Nope>: can't evaluate
# field Nope in type *formatter.diskUsageImagesContext
```

**This is the good case and it is why the template is parsed rather than the human
columns.** A renamed or removed field is an *error*, not a silently shifted column
— the failure announces itself on the first run after a docker upgrade instead of
producing a plausible wrong number. Note the type in the message is
`diskUsageImagesContext` even though the run covered all four types, so the type
name is not a reliable hint about which row failed.

### `docker system df` requires the daemon — there is no client-side answer

```sh
DOCKER_HOST=unix:///nonexistent.sock docker system df
# rc 1
# failed to connect to the docker API at unix:///nonexistent.sock; check if the
# path is correct and if the daemon is running: dial unix /nonexistent.sock:
# connect: no such file or directory
```

**If you assume otherwise:** you place a disk-usage line beside something that
works offline. The entry "`docker compose config` succeeds with no daemon — `up`
is where it fails" records the client-side counterpart; `df` has no such half. A
dead daemon fails the whole reading, and it is an environment failure to report,
not a zero to display.

### Both forms returned in ~0.34 s with 171 volumes — and still need Armada's own timeout

```sh
docker system df        # ~0.34 s wall, 171 volumes
docker system df -v     # ~0.34 s wall, 171 volumes
```

Cheap enough to run on a read-only check without thinking about it. But the entry
"The docker CLI has no client-side timeout" applies here unchanged — **`df` has no
deadline flag either**, so a hung daemon hangs it for as long as Armada is willing
to wait, and 0.34 s of typical cost says nothing about the tail.

### The standing conclusion

**`docker system df` output is not a stable API.** So Armada parses the
`--format '{{json .}}'` template rather than the human columns, treats a size it
cannot parse as *unknown* rather than as zero, and never lets a parse failure turn
into a confident `0 B`. A reported zero must mean the daemon said zero.

## Docker Compose

Measured on darwin against **Docker Compose v5.3.1**, 2026-08-09. Thirteen entries. All but two
are `docker compose config` or CLI-surface behaviour, which is resolved client-side and is
therefore the compose CLI's rather than the host's; the exceptions are the `${files}` entry,
which is a shell's behaviour, and the service-labels entry, which is the daemon's, and each
says so where it appears. **An earlier version of this section
was headed v2.24.3-desktop.1, which is not what is installed** — so every entry below was
re-run against v5.3.1 before being trusted. Four reproduced unchanged; one is version-dependent
and is marked.

### An override file appends to `ports:` — it does not replace

Base `docker-compose.yml` with `ports: ["5432:5432"]` plus an override with
`ports: ["5460:5432"]` publishes **both**.

```sh
docker compose -f docker-compose.yml -f override.yml config
# → published: "5432"   AND   published: "5460"
```

**If you assume otherwise:** you write an override to remap ports into a per-workspace block,
every workspace still binds the base port, and concurrent workspaces collide — the exact
failure Armada exists to prevent. It looks like it worked, because the new port is also
published.

### A bare `ports:` entry publishes on an *ephemeral* host port — `published` absent means random, not none

```sh
# docker-compose.yml:  ports: ["6379"]
docker compose config
# → {mode: ingress, target: 6379, protocol: tcp}     ← no `published` key
docker compose up -d && docker port <container> 6379/tcp
# → 0.0.0.0:55918
```

Measured against Docker 29.6.2 and Compose v5.3.1 while fixing this in `compose.rs`; recorded
here because two code comments cited this file for it and it was never written down.

**If you assume otherwise:** you read "no `published` key" as "this entry does not publish" and
skip it, which is what an earlier version of the transform did. The key that exposes a container
port *without* binding a host one is `expose:`, which is a different key. The consequence is the
exact failure the port block exists to prevent, arriving silently: the service comes up outside
the claimed block, and a `tcp:` ready-check waits on the claimed port until it times out.

**Every entry under `ports:` publishes, so every entry is rewritten or refused** — including one
Armada cannot parse, because skipping that one leaves compose to place the port.

### `${VAR:-default}` in a `ports:` entry contains a colon, and splitting on every colon cuts inside it

`ports: ["${POSTGRES_PORT:-5432}:5432"]` is not `IP:HOST:CONTAINER`. A naive `split(':')` yields
`["${POSTGRES_PORT", "-5432}", "5432"]`, and taking the second-to-last segment as the host port
reports `-5432}`. `${VAR:?err}` and `${VAR:+alt}` have the same shape, and so does a bracketed
IPv6 bind address, `[::1]:6379:6379`.

**If you assume otherwise:** the report is wrong in a way that looks like a rendering glitch
rather than a parse error, so it survives review. It is not an exotic spelling — a variable with
a default is how a compose file supports both a fixed port and a per-worktree override, which is
precisely the shape a repository adopting Armada already has.

**A `:` splitter for a compose port entry has to step over `${…}` and `[…]`**, and there is one
of them: [`compose::parse_port`](../crates/core/src/compose.rs).

### The `!override` tag is version-dependent — and that is why Armada does not rely on it

```sh
# override.yml:  ports: !override ["5460:5432"]
docker compose -f docker-compose.yml -f override.yml config

# v5.3.1  → published: "5460"                    the tag works
# v2.24.3 → published: "5432" AND "5460"         the tag is IGNORED, silently
```

**This entry has been wrong twice, in the file whose whole rule is re-run rather than
re-trust.** First it recorded base-values-only, from a `grep` that showed one match. Then it
asserted the tag is ignored full stop, from a version that is no longer installed.

**What survives is the design conclusion, and it survives on the *plain* override behaviour
above, not on this tag.** Armada generates a whole document precisely so it never depends on a
merge feature whose behaviour changes between versions and fails silently in the older
direction. If a repo's developers are split across Compose versions — which is normal — an
override-based design is correct on some machines and collides on others.

**If you assume otherwise:** a version floor looks like a sufficient guard. It is not, because
the failure below the floor is silent — one stale CI image or one developer on an older
Docker Desktop reintroduces the collision with nothing to indicate it.

### `docker compose config` bakes the project name into network names

Running `config` without `-p` emits `networks.default.name` derived from the *directory*, and
that value persists into any file generated from the output.

```sh
docker compose -f docker-compose.yml config | grep -A1 '^networks:'
# → name: <directory>_default
```

**If you assume otherwise:** you pass `-p armada-<id>` only on the run step, and the networks
are named for whatever directory the resolve happened to run in — so ownership by project
label does not group the way you expect.

### `config` resolves `build.context` to an absolute path

```sh
docker compose -f docker-compose.yml config | grep context
# → context: /absolute/path/to/dir
```

**Useful rather than dangerous:** it is what makes it safe to emit a generated compose file
into a different directory, provided `--project-directory` is set to the original root.

### `docker compose config` inlines `.env` and `${VAR}` secrets into its output

```sh
printf 'SECRET_TOKEN=sentinel\n' > .env
# service has: env_file: [.env]  and  environment: {INLINE: ${SECRET_TOKEN}}
docker compose -f st.yml --project-directory . config
#   environment:
#     INLINE: sentinel
#     SECRET_TOKEN: sentinel
```

**If you assume otherwise:** you persist the resolved document as a debugging aid and create a
cleartext credentials file, for every repo, including ones that never adopt Armada's secrets
mechanism. Those values never passed through Armada, so no scrubber can redact them. This is why
`PLAN.md` §6.0 pipes the document to `docker compose -f -` instead of writing it.

`docker compose -f -` accepts a document on stdin and produces identical resolved output —
verified.

### `${files}` under a shell is arbitrary code execution

```sh
# these filenames are legal on POSIX and git emits them raw under -z
sub/semi;echo INJECTED.py
sub/dollar$(id).py

sh -c "eslint $FILES"
#   ;echo INJECTED  -> runs as a separate command
#   $(id)           -> runs, and its output is substituted into the argument
```

Measured. **`${files}` is the only substitution whose values come from outside the trust
boundary** — filenames are written by whoever pushed the branch, and `armada manifest check` on a pull
request is the ordinary case.

**If you assume otherwise:** you offer `shell: true` as a convenience, someone uses it in a
check with `${files}`, and anyone who can push a branch has code execution on every machine
that checks it. This is why the schema makes that combination **unrepresentable** rather than
warning about it (`PLAN.md` §4.1).

Also non-malicious and silent: a filename containing a space is word-split into two arguments.

**Not compose, and not platform-neutral.** This entry is `sh -c` word-splitting and command
substitution with no compose involved, measured on darwin. `/bin/sh` is `bash` on darwin and
`dash` on Debian and Ubuntu, and Armada hardcodes it in **two** places: `template::shell_argv`
for a `setup:` step, and `verbs::dispatch` inline for a `commands:` entry, which builds the
same `["/bin/sh", "-c", …]` itself because it has to append shell-quoted passthrough after
substitution. Grepping for the helper finds one of them. Both inherit whatever `/bin/sh` is on
the machine, so the shell Armada actually invokes differs by platform.
**The dash side is unverified.** Nothing here rests on it, because the schema makes
`shell: true` beside `${files}` unrepresentable on every platform; what has not been measured
is whether any *other* `shell: true` command Armada runs behaves differently under dash.

### `docker compose up` has no `--label` flag

```sh
docker compose up --help | grep -c label     # → 0
```

Labels reach containers only through the compose document — `labels:` on a service, and
`build.labels:` for images the build produces. Neither `up` nor `build` accepts them on the
command line.

### Service labels do **not** reach the network or the volumes

**Daemon-side, not client-side:** this is what the daemon creates from the document, not what
`config` renders, so the section's client-side note does not cover it. Measured on darwin.

```sh
# a document with labels: {armada.workspace: deadbeef} on the service only
docker ps      -q --filter label=armada.workspace=deadbeef   # 1
docker network ls -q --filter label=armada.workspace=deadbeef  # 0  <- armada-deadbeef_default EXISTS
docker volume  ls -q --filter label=armada.workspace=deadbeef  # 0  <- armada-deadbeef_pgdata  EXISTS
```

**This is the founding bug of the project, reintroduced.** §1 of `PLAN.md` cites 29 leftover
per-worktree Docker networks exhausting the default bridge address pool. A `clean` that finds
resources by label finds the containers and leaves the network and the volumes behind — with
no verb that can ever locate them again, because the label they would be found by was never
applied.

The fix is in the same document, and it works:

```yaml
volumes:
  pgdata:  { labels: { armada.workspace: cafe1234 } }
networks:
  default: { labels: { armada.workspace: cafe1234 } }
```

```sh
docker network ls -q --filter label=armada.workspace=cafe1234   # 1
docker volume  ls -q --filter label=armada.workspace=cafe1234   # 1
```

Top-level `networks:` and `volumes:` must be **stamped separately** in the transform step.
Stamping services is not stamping the stack.

### `docker compose config` succeeds with no daemon — `up` is where it fails

```sh
DOCKER_HOST=unix:///nonexistent.sock docker compose config   # rc 0
DOCKER_HOST=unix:///nonexistent.sock docker ps               # rc 1
```

`config` is client-side. So `PLAN.md` §6.0's steps 1–3 all pass against a dead daemon and only step 4
fails — Armada does its whole resolve-and-transform and discovers the daemon is gone at the end.
Probe the daemon before starting work, and report it as an environment failure rather than as
the stack failing to start.

### The docker CLI has no client-side timeout

Measured against a socket that accepts the connection and never replies: `docker ps` and
`docker compose up -d` were both still running at 30 seconds with no output. There is no flag
for this. **Every docker invocation needs Armada's own timeout**, and the one that matters most
is the `docker ps` in `init`'s reap pass — without a timeout, a hung daemon wedges every new
workspace on the machine, including the verb whose job is recovery.

### Override merging does work for `labels:` and `build.labels:`

Both merge as expected. Labels were never the hard part; ports were.

### `docker compose` is a CLI plugin, so it disappears under a scratch `$HOME`

Measured on darwin 2026-08-14, against Docker **29.6.2** and Compose **v5.3.1**, while
writing the compose driver's integration test. The daemon is reachable and the subcommand is
not:

```sh
env -i PATH="$PATH" HOME=/tmp/scratch docker version   # rc 0 — the daemon answers
env -i PATH="$PATH" HOME=/tmp/scratch docker compose version
# docker: unknown command: docker compose

env -i PATH="$PATH" HOME=/tmp/scratch DOCKER_CONFIG="$REAL_HOME/.docker" docker compose version
# Docker Compose version v5.3.1
```

The CLI looks for plugins under `$DOCKER_CONFIG`, which defaults to `$HOME/.docker`. So a
`$HOME` that is not the user's has a working `docker` and no `docker compose`.

**If you assume otherwise:** char's own end-to-end harness gives every test a scratch `$HOME`
— that is what makes `~/.armada/manifest.db` a fresh machine-global store rather than the
developer's — and the compose driver then fails there with `tool_failed` and docker's generic
*"Run 'docker --help' for more information"*, on a machine where the driver works perfectly by
hand. It reads exactly like a bug in the argv, which is where an hour goes.

**This is a property of the harness, not of char.** A real invocation runs under the user's own
`$HOME` and finds the plugin. The integration suite passes `DOCKER_CONFIG` through explicitly;
nothing in `crates/` compensates for it, and nothing should.

## A wedged Docker daemon fails tests that are not about Docker

Measured on darwin, 2026-08-15, with several agents running the docker suites against one daemon
at once. `docker version` — the cheapest call there is — did not return within 120 seconds, and
`docker ps` did not either. The daemon was not down; it was accepting connections and not
answering, which is the case
[the CLI has no client-side timeout](#the-docker-cli-has-no-client-side-timeout) is about.

**The failure surfaces somewhere else entirely.** `crates/helm/tests/owned_processes.rs`'s
`a_group_stamped_by_another_boot_is_dropped_without_a_signal` spawns `sleep 60`, records it under
a foreign `boot_id`, runs `armada manifest clean`, and asserts the group is **still alive** —
because Armada must not signal a group it cannot prove is its own.

```sh
# healthy daemon: clean returns in a second or two, the sleeper is alive, the test passes
# wedged daemon: clean burns its docker deadlines and takes 91s, the sleeper exits on its own
cargo test -p armada-helm --test owned_processes a_group_stamped_by_another_boot
# → "Armada signalled a group it could not prove was its own"
```

The assertion message is a lie in this case: Armada signalled nothing. The process died of old
age while `clean` waited on docker.

**If you assume otherwise:** you read the message, go looking for a regression in
`reap::pgid_is_ours` or `stop_owned_processes`, and find nothing, because nothing is wrong there.
Confirmed pre-existing by running the identical test at the branch point — 91.00s and the same
failure, against 91.10s on the branch.

**The tell is the duration.** Any test whose fixture has a lifetime shorter than the wall time of
a docker-touching verb is measuring the daemon rather than the code. Check `docker version`
before believing a failure in this class, and re-run once it answers.

## Two golden snapshots fail under parallel load and pass alone

`status_matches_its_snapshot` and `up_and_down_match_their_snapshots` (both in
`crates/helm/tests/golden.rs`) each failed once during a full `cargo test --workspace`, and each
passed when run alone and on the next full run. Both compare a whole envelope, and both carry a
`port_block`.

**Do not treat a single failure in either as a real defect.** Re-run before believing it. An
agent spent an hour this morning diagnosing a different intermittent failure that turned out to
be four stray `armada-test-*` docker networks left behind by an earlier session, and reported it
as pre-existing breakage in the code it was sent to fix.

**What is actually unproven** is whether these two share something — a scratch `$HOME`, a port
claim, or the machine-global store — that makes them order-dependent when the suite runs
concurrently. Nobody has looked. Until somebody does, this is a known-flaky note and not a
diagnosis, and writing it down is what stops the next agent rediscovering it at full price.

**If you assume otherwise:** you will either chase a phantom or, worse, "fix" a golden fixture
to match a run that was wrong — and golden fixtures have no update flag precisely so that
adopting one is a deliberate act.

## `echo --help` prints usage on Linux and prints `--help` on darwin

Measured 2026-08-15 against GNU coreutils 9.1 (`rust:1-bookworm`) and the `/bin/echo` shipped
with darwin 27.0.

```
# Linux, coreutils 9.1
/bin/echo --help        -> "Usage: /bin/echo [SHORT-OPTION]... [STRING]..." , exit 0
/bin/echo --help foo    -> "--help foo"
/bin/echo -n --help     -> "--help"
dash builtin: echo --help -> "--help"

# darwin 27.0, BSD echo
/bin/echo --help        -> "--help"
/bin/echo --version     -> "--version"
```

**Only the sole-argument case differs.** GNU's `echo` runs its long-option parser exactly when
there is one argument and it is `--help` or `--version`; with anything else on the line every
word is data, which is why `echo prune --dry-run -- -x` agrees on both platforms and `echo
--help` does not. The shell builtins agree with BSD, so `sh -c 'echo --help'` hides the
difference entirely — a probe that goes through a shell will report no problem.

**What it cost.** `crates/helm/tests/e2e.rs` used `cmd: echo` for the `commands:` entry that
proves `armada manifest <name> --help` reaches the child rather than Armada's help page — the
rule `crates/helm/src/args.rs` exists to keep. The fixture chosen to demonstrate the
pass-through was the one program that consumes `--help` itself. It passed on the author's Mac
and failed on both Linux jobs with *"a commands: entry's --help was swallowed"*, which names
Armada as the culprit; Armada was passing the flag through correctly and GNU `echo` was eating
it. The fixture is now `./echoer.sh`, `printf '%s\n' "$*"`, which has no options of its own.

**If you assume otherwise:** you will read a red Linux job as a parser regression and go
looking for it in `args.rs`, where there is nothing to find. More generally: a test fixture
must not be a program that claims the argument the test is about.

## The e2e harness inherited the developer's `claude`, and CI has none

Measured 2026-08-15 on darwin 27.0 and on `ubuntu-latest` / `macos-latest` runners.

```
# a scratch machine, with PATH stripped of every directory holding a claude
armada init --defaults --json
-> exit 6, envelope on STDOUT:
   {"class":"environment","where":"claude","message":"`claude` is not on PATH"}
-> ~/.armada is not created
```

`crates/helm/tests/support/mod.rs` clears the environment and pins `$HOME` to a scratch
directory, so a reader reasonably concludes the suite is hermetic. It was not: `PATH` came
from the developer's shell, and preflight makes a missing `claude` **fatal** by design. Six
tests in `crates/helm/tests/guild.rs` passed locally for the sole reason that the author has
Claude Code installed, and failed on every runner, which does not — the `DOCKER_CONFIG` entry
above, in a different tool.

**Worse than a red job**: `armada doctor` does not stop at `claude --version`. It runs `claude
--help` and then `drone::probe_argv`, which *starts a session* and exits at EOF. On the
author's machine that was the real binary, so the suite was violating "no test starts a real
Claude session" every time it ran. Every scratch machine now finds a stub `claude` first on
`PATH`, generated from `drone::FLAGS` and `helm::FLAGS`.

**It looked platform-specific and was not.** `cargo test --workspace` stops at the first test
binary that fails, and on Linux `tests/e2e.rs` sorts before `tests/guild.rs` and was failing
for the unrelated `echo` reason above — so `tests/guild.rs` never ran there. The six failures
were reported as macOS-only because Linux never reached them.

**If you assume otherwise:** you will believe `env_clear()` makes a harness hermetic. It
removes the environment; it does not remove the machine. `PATH` is still an input, and every
program reachable through it is one the test did not write.

## git refuses its own auto-detected identity when the hostname has no domain

Measured 2026-08-15 against git 2.39.5 (`rust:1-bookworm`) and git 2.54.0 (darwin 27.0).

```
# $HOME points at an empty directory, so there is no .gitconfig to read.
# darwin, hostname Nicks-MacBook-Pro.local
env -i HOME=$scratch PATH=$PATH git commit -m x
-> exit 0, author "Nick Mele <nickmele@Nicks-MacBook-Pro.local>"

# Linux container, hostname a hex id that does not canonicalise
-> exit 128, "Author identity unknown"
```

**Absence of a `user.email` is not the trigger; an unusable *guess* is.** git falls back to
`getpwuid` for the name and `user@hostname` for the address, then discards the address unless
the hostname already contains a `.` or `getaddrinfo` canonicalises it into something that does.
Every developer machine satisfies that and a container generally does not, which is why this
reads as a platform difference and is really a hostname difference — the same code fails on a
Mac whose hostname has been set to a bare word.

**What it cost.** `crates/helm/tests/support/mod.rs` clears the environment and points `$HOME`
at a scratch directory, which removes the developer's identity without supplying one. Ten
tests in `crates/helm/tests/guild.rs` — every verb that commits — failed on Linux with
`Author identity unknown` while passing on the author's machine. The harness now writes a
`.gitconfig` into the scratch `$HOME`.

**If you assume otherwise:** you will conclude that pointing `$HOME` somewhere empty is enough
to isolate git. It isolates the *config*; the identity then comes from the host's passwd file
and hostname, which is a machine input by another route.

## Reading a process's start time spawns `ps` on darwin and spawns nothing on Linux

Measured 2026-08-15 against darwin 27.0 and `rust:1-bookworm`.

`machine::process_start_at` is one function with two bodies:

```
#[cfg(target_os = "linux")]      read /proc/<pid>/stat, field 22   -> no subprocess
#[cfg(not(target_os = "linux"))] run.call(["ps", "-o", "lstart=", "-p", pid]) -> one subprocess
```

Both answer the same question. Only Linux answers it without a fork, so **a `Run` harness that
records what was spawned sees one call per liveness check on darwin and zero on Linux** — and
the platform doing less work is the one that looks like it did nothing.

**What it cost.** `reading_a_frame_resumes_nothing_and_writes_nothing` guarded against a
vacuous pass by requiring at least one recorded call per redraw, and failed on Linux with *"a
frame asked the machine nothing at all"*. It reads as a regression in
`posix::stop_group`'s reaping — the change that had just merged — and is unrelated to it:
`stop_group` is on `drone::stop`, which `fleet kill` calls, and liveness goes through
`drone::alive` instead. Nothing was broken; the assertion was written in darwin's spelling.

**It was not a container artefact**, unlike `net::tests::a_wildcard_holder…` in the ports
section above — which was the first thing to rule out, and the two look identical until you
do. The branch here is `cfg(target_os = "linux")`, so it applies to any Linux rather than to a
namespace, and it failed 3 times out of 3. Every ubuntu job would have hit it the moment CI got
that far, which it never had.

**The general rule this earned:** a test that proves something happened by counting spawned
processes is asserting an implementation's spelling. Where the answer is visible in the output,
assert the answer — `job::observe_state` reads a live Drone with no finished turn as `RUNNING`
and a dead one as `STALLED`, which is the same assertion on both platforms. Where it is not
visible, say which platform you are counting for.

**If you assume otherwise:** you will read a call log as a portable record of what a function
did, and conclude a platform is broken when it is merely cheaper.

## An assertion built from `stderr` alone reports nothing when Armada refuses

Measured 2026-08-15, darwin 27.0.

Armada reports a refusal as an envelope on **stdout**, `--json` or not, and writes nothing to
`stderr`. So the conventional assertion message —

```
assert!(out.status.success(), "export failed: {}", String::from_utf8_lossy(&out.stderr));
```

— renders as `export failed: ` with the reason sitting unread in the buffer beside it. Six CI
failures carried that empty message for a day and nobody could act on any of them.

Use `support::why(&output)`, which prints the exit status and both streams. The first run after
it was added named the cause in full on the first line.

**If you assume otherwise:** you will write a test whose failure output is the word "failed",
and the cost lands on whoever reads the job — usually not you, and usually much later.

## `script(1)` does not answer the cursor query, so a live table draws nothing under it

Measured 2026-08-15, darwin 27.0.

`render/live.rs` reserves a **ratatui inline viewport**, and reserving one begins with a Device
Status Report — `ESC [ 6 n` — that the terminal is expected to answer with the cursor's row.
`script -q out.raw armada fleet spawn …` gives the program a real pty, so `stderr_is_tty` is
true and `Live::start` succeeds, but `script` itself never answers the query. The viewport gets
no row to anchor to and every frame after it is a no-op.

The capture then contains the query, a few colour resets, and the final table — which reads
exactly like progress being broken:

```
b'\x1b[6n\r\n\x1b[39m\x1b[49m\x1b[0m\x1b[?25l …  STATUS  STEP  DETAIL  TIME\r\n …'
```

Two further things have to be right before frames appear, and each fails the same silent way:

- **The pty needs a window size.** `pty.fork()` leaves it at 0×0, so `terminal_size` reports
  nothing, the table renders into no columns, and the frames are present but empty.
- **A read can split an escape sequence in half**, so a replayer that parses each `read()`
  independently paints the digits of a truncated `ESC [ 38 ; 2 ; …` onto its own screen.

A harness that forks a pty, sets `TIOCSWINSZ`, answers the DSR with something like `ESC [ 20 ; 1 R`
and buffers partial escapes shows the frames. There is one in this task's scratch notes rather
than in the repo, because nothing in the suite draws a viewport: `main::reporter` picks `Silent`
whenever stderr is not a terminal, which `cargo test` never is.

**If you assume otherwise:** you will conclude that live progress does not work, and "fix" a
verb that was already reporting correctly — or, worse, replace the viewport with appended lines
and break the contract in `PLAN.md` §3.1.1 that keeps a captured stderr clean.

## `ratatui::Terminal::clear()` re-queries the cursor — a second `ESC [ 6 n`, at teardown

Measured 2026-08-15, darwin 27.0. This is not the `script(1)` artefact above — it reproduces at a
real terminal, through a pty harness that answers every DSR query correctly (`pyte`-driven, not a
fixed reply), and it is what `armada failures`, the Bridge's *"Which workflow is this?"* and
`armada manifest check` all actually showed at a real prompt.

`Terminal::with_options` sends one `ESC [ 6 n` to anchor an inline viewport's first row — the
query the entry above is about. `Terminal::clear()`, called by every widget when it gives the
viewport's rows back, sends a **second** one: it queries the cursor position again so it can
restore it after clearing. A `Live`/`select::ask` session that opens and closes several widgets
in a run sends one of these per teardown, each a write and a blocking read with a 2s timeout, and
each an opportunity for the reply to arrive late or to be read by whichever `event::read()` call
happens to be waiting next — reproduced by holding a keypress until just after the query is
answered: the widget renders correctly right up to that point, then the reserved lines stay
blank and whatever prints next lands at the column the stale reply implied instead of at the
margin. `armada manifest check`'s report matches this exactly — the live phase is correct, only
the *completion* is wrong — because it has nothing else touching the terminal to blame.

The fix does not patch the query: it removes the second one. Every successful `Terminal::draw`
already hands back the viewport's `Rect` in its `CompletedFrame`, so a widget that keeps the last
one can clear from it directly — `MoveTo` plus `Clear(FromCursorDown)` — instead of asking
`clear()` to ask the terminal again. `ask::terminal::clear_viewport` is that: used by
`ask/select.rs`, `ask/editor.rs` and `render/live.rs`, which each build their own `Terminal` and
so each needed the call site changed — there is no one function all three already shared.

**If you assume otherwise:** a harness that never answers the query (`script(1)`, or a fixed fake
reply that does not track the real cursor) hides this entirely — the widget fails to open at all
and falls back to the non-widget path, which looks clean for the wrong reason.

---

## `cmux` opens a directory with a bare path — there is no `open` subcommand

Measured 2026-08-16 against **cmux 0.64.22 (102)**, darwin 27.0, installed as an app bundle
(`/Applications/cmux.app/Contents/Resources/bin/cmux`) rather than by a package manager.

<!-- doclint: skip — the output below is cmux's, quoted verbatim -->
```
$ cmux --help
cmux - control cmux via Unix socket

Usage:
  cmux <path>                Open a directory in a new workspace (launches cmux if needed)
  cmux [global-options] <command> [options]
```

The whole handoff [`020`](reserved/020-the-tui-decided.md)'s workspace decision needs is the
first line: **a bare path, no verb.** cmux has a large subcommand surface — `settings`, `config`, `restore`,
`tab-action` and a dozen more — and every plausible guess at a name for this one (`cmux open`,
`cmux new`, `cmux workspace`) is not among them. `launches cmux if needed` is the other half:
Armada does not have to check whether the app is already running.

**Detection is `--help`, and it is free.** It prints the usage above and starts nothing, so
probing costs no window and no socket connection — which is what makes it safe to run before the
Bridge takes the alt screen. `armada_core::fleet::bridge::cmux_offers_open` looks for the literal
`cmux <path>` in that output, so a cmux whose CLI moves on is detected as *absent* and `↵` falls
back to printing the worktree path and the resume command.

**If you assume otherwise:** a guessed subcommand exits non-zero with a usage message, which the
Bridge would report as a one-line notice under the table — so the key would appear bound, appear
to run something, and never open anything. That is the same class of silent failure
[`020`](reserved/020-the-tui-decided.md)'s state-word decision is about, arriving through the key
its workspace decision added.
