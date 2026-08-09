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

`journal_mode=WAL` persists in the database file once set, but no driver sets it for you. And
`busy_timeout` defaults vary: some drivers ship 5000, others ship **0**. Both pragmas are
required lines in char's connection setup, not tuning knobs.

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

Both sit in machinery §7 of the plan calls load-bearing, and both are one line you must not
forget. Measured 2026-08-09 against Rust 1.97.1.

### `SIGPIPE` is set to `SIG_IGN` at startup — `char status | head` panics until you fix it

Rust's runtime ignores `SIGPIPE`, so a write to a closed pipe returns `EPIPE`, `println!`
unwraps it, and the process **panics with exit 101 and a backtrace on stderr** — worse than
the Python it replaced. `#[unix_sigpipe]` is not stabilised on 1.97.1.

**Rule: restore the default disposition at the top of `main`,** before anything writes:

```rust
unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL); }
```

This yields exit **141** and a silent, correct death — the ordinary Unix behaviour. It is one
of exactly two `unsafe` blocks the design permits.

### `setsid` is not in `std` — detaching a process group needs `unsafe pre_exec`

`Command::process_group(0)` gives a new process *group* in the **same session**. Detaching
from the controlling terminal — which is what `char up` requires — needs `setsid` via the
`libc` crate inside `pre_exec`. That is the second permitted `unsafe` block.

**Everywhere else, `unsafe` is denied crate-wide.** Two exceptions, both recorded here, both
in the POSIX layer, both a single call.

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
- `SIGPIPE=SIG_DFL` gives the correct Unix behaviour and a code that **appears in no exit-code
  table in any charkit document**. Note the stdlib explicitly warns against this setting; the
  warning is about libraries that need to observe the error, which char does not.

Whichever is chosen, the "exit code = `f(error.class)`" rule needs an explicit carve-out for
signal-derived codes, covering `130` and `141` together.

## Docker Compose

Measured against **Docker Compose v2.24.3-desktop.1**, phase 0.

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

### The `!override` tag needs Compose ≥ 2.24.4 and is silently *ignored* below it

```sh
# override.yml:  ports: !override ["5460:5432"]
docker compose -f docker-compose.yml -f override.yml config
# on 2.24.3 → published: "5432"  AND  published: "5460"
#             the tag is ignored; ordinary append merging happens
```

**Corrected 2026-08-09.** An earlier version of this entry recorded the result as
`published: "5432"` alone — base values, override discarded. That is wrong and does not
reproduce; the tag is ignored entirely, so you get the append behaviour documented above. The
conclusion is unchanged and slightly stronger: below 2.24.4 you get a port collision with no
error.

**This entry was wrong in the one file whose stated rule is "re-run rather than re-trust."**
The original was recorded from a `grep` that showed only the first match. Re-run every command
in this file before relying on it; at least one has already failed to reproduce.

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

### `docker compose up` has no `--label` flag

```sh
docker compose up --help | grep -c label     # → 0
```

Labels reach containers only through the compose document — `labels:` on a service, and
`build.labels:` for images the build produces. Neither `up` nor `build` accepts them on the
command line.

### Override merging does work for `labels:` and `build.labels:`

Both merge as expected. Labels were never the hard part; ports were.
