# Measured behaviour

Things that are true about the environment charkit runs in, which a reasonable person would
have assumed otherwise. **Every entry here was measured, not read.**

This file exists because charkit is being built greenfield, deliberately without continuous
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

Record for each: what was measured, the version it was measured against, the command that
shows it, and what breaks if you assume otherwise.

---

## SQLite — the lease mechanism depends on these

Measured 2026-08-09. **Language-neutral: every one of these applies whatever charkit is
written in.** This is the machinery `PLAN.md` §4.3 rests on.

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

Verified against the live specification and SDK, phase 0.

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
`char check` over MCP — worth using rather than inventing a bespoke polling protocol.

## Rust — two required rules in the POSIX primitives

Both sit in machinery `PLAN.md` §7 calls load-bearing, and both are one line you must not
forget. Measured 2026-08-09 against Rust 1.97.1.

### `SIGPIPE` is set to `SIG_IGN` at startup — `char status | head` panics until you fix it

Rust's runtime ignores `SIGPIPE`, so a write to a closed pipe returns `EPIPE`, `println!`
unwraps it, and the process **panics with exit 101 and a backtrace on stderr** — worse than
the Python it replaced. `#[unix_sigpipe]` is not stabilised on 1.97.1.

**Rule: restore the default disposition at the top of `main`,** before anything writes:

```rust
unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL); }
```

This yields exit **141** and a silent, correct death — the ordinary Unix behaviour. It is one of the three `unsafe`
blocks the design permits.

### `setsid` is not in `std`, and it is mutually exclusive with `process_group(0)`

`Command::process_group(0)` gives a new process *group* in the **same session**. Detaching
from the controlling terminal — what `char up` requires — needs `setsid` via `libc` inside
`pre_exec`. That is the second of three permitted `unsafe` blocks.

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

### `Child` dropped without `wait()` leaves a zombie

```
child exits, Child dropped without wait() -> ps stat "Z"
```

Rust's `Child` does **not** reap on drop, and the docs say so, but the consequence is easy to
miss: char spawns services, checks, ready-probes, docker calls and secret providers. Every one
whose handle is dropped without `wait()` leaves a `<defunct>` entry until char itself exits.

**Rule: every spawned `Child` is waited on, or explicitly reaped.** A long-lived
`char check --detach` accumulating zombies across a fifteen-minute run is the case that bites.

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

## Typer / Click exit codes *(historical — charkit is Rust)*

Measured against **Typer 0.27.1**, phase 0. **Kept because the source repo being harvested in
phase 3 is Python**, so these remain true of the behaviour being harvested — and because the
exit-code conclusions they produced (130 and 2 are conventional; broken pipe needs an explicit
decision) carried over to the Rust design intact.

### `KeyboardInterrupt` already exits 130, and usage errors already exit 2

```sh
python app.py sigint        # raises KeyboardInterrupt  -> 130
python app.py --bogus       # unknown flag              -> 2
python app.py clickerr      # typer.BadParameter        -> 2
```

Both are free — char does not need to catch and re-exit for them.

**This entry exists because it was reported as false.** A claim that Click collapses
`KeyboardInterrupt` into `Abort` and exits `1` was checked and does not hold for this version.
Re-measure after any framework upgrade: if it ever became true, it would be silent, and
`char check` interrupted by an agent would report a code meaning "the tool failed" instead of
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
- **Exit 1 is `tool_failed`** under char's own map, so `char status | head` currently reads as
  "the tool failed" to anything checking exit codes.
- `SIGPIPE=SIG_DFL` gives the correct Unix behaviour and code **141**, which `ARCHITECTURE.md`
  §1.6 now carries in its exit table alongside the signal carve-out. Note the stdlib explicitly warns against this setting; the
  warning is about libraries that need to observe the error, which char does not.

Whichever is chosen, the "exit code = `f(error.class)`" rule needs an explicit carve-out for
signal-derived codes, covering `130` and `141` together.

## Docker Compose

Measured against **Docker Compose v5.3.1**, 2026-08-09. **An earlier version of this section
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
failure charkit exists to prevent. It looks like it worked, because the new port is also
published.

### The `!override` tag is version-dependent — and that is why char does not rely on it

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
above, not on this tag.** char generates a whole document precisely so it never depends on a
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

**If you assume otherwise:** you pass `-p char-<id>` only on the run step, and the networks
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
cleartext credentials file, for every repo, including ones that never adopt char's secrets
mechanism. Those values never passed through char, so no scrubber can redact them. This is why
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
boundary** — filenames are written by whoever pushed the branch, and `char check` on a pull
request is the ordinary case.

**If you assume otherwise:** you offer `shell: true` as a convenience, someone uses it in a
check with `${files}`, and anyone who can push a branch has code execution on every machine
that checks it. This is why the schema makes that combination **unrepresentable** rather than
warning about it (`PLAN.md` §4.1).

Also non-malicious and silent: a filename containing a space is word-split into two arguments.

### `docker compose up` has no `--label` flag

```sh
docker compose up --help | grep -c label     # → 0
```

Labels reach containers only through the compose document — `labels:` on a service, and
`build.labels:` for images the build produces. Neither `up` nor `build` accepts them on the
command line.

### Override merging does work for `labels:` and `build.labels:`

Both merge as expected. Labels were never the hard part; ports were.
