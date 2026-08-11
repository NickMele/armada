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
- **Name the platform, or measure on both.** charkit supports darwin and Linux, and phase 2
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
that:** every one of these applies whatever charkit is written in, and locking, `BEGIN
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
`char check` over MCP — worth using rather than inventing a bespoke polling protocol.

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
| `0.0.0.0`, `127.0.0.1`, `::` (v4-mapped) | `EADDRINUSE` — correctly detected |
| **`::1` only, `IPV6_V6ONLY`** | **succeeds — reports FREE** |
| `SO_REUSEPORT` on holder **and** probe | **succeeds — reports FREE** |
| `SO_REUSEPORT` on holder, plain probe | `EADDRINUSE` |

**`SO_REUSEADDR` does not defeat the probe** — an earlier draft of `PLAN.md` §3.1 said it did,
and cited this file for a measurement that was never here. What defeats it is an IPv6-only
listener, and modern Node resolving `localhost` to `::1` makes that the ordinary dev server
rather than an exotic case.

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
`char status --all` run twice at once can report a `CONFLICT` that does not
exist. It is inherent to bind-probing rather than a bug to fix — `connect()`
answers a different question — so the rule is that a probe's answer is a
point-in-time reading and nothing may be serialised against it. char's own
suite serialises the runs that snapshot a port state, for this reason.

So a probe must bind **both** `127.0.0.1` and `[::1]` and treat either `EADDRINUSE` as taken,
and `char status` must connect on both families before reporting `RESERVED`. `SO_REUSEPORT` on
both sides remains undetectable; nothing char does prevents that, so it is a known limit rather
than a bug to fix.

## Rust — the POSIX primitives, and the rules each one forces

Both sit in machinery `PLAN.md` §7 calls load-bearing, and both are one line you must not
forget. Measured 2026-08-09 on darwin, against Rust 1.97.1. **Every entry below is darwin-only
and unverified on Linux unless it names a platform.** This is the section where the one entry
that *was* re-run on Linux — the zombie-only group, below — came back different, so the
absence of a Linux column here is a gap rather than a claim of agreement.

### `SIGPIPE` is set to `SIG_IGN` at startup — `char status | head` panics until you fix it

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
from the controlling terminal — what `char up` requires — needs `setsid` via `libc` inside
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
miss: char spawns services, checks, ready-probes, docker calls and secret providers. Every one
whose handle is dropped without `wait()` leaves a `<defunct>` entry until char itself exits.

**Rule: every spawned `Child` is waited on, or explicitly reaped.** A long-lived
`char check --detach` accumulating zombies across a fifteen-minute run is the case that bites.

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

**If you assume otherwise:** char confirms a kill with the signal-0 probe, and
for a caller that *parented* the group and has not reaped it the two platforms
then fail in opposite directions. On darwin the zombie-only group answers
`EPERM`, so `group_alive` is false and `stop_group` takes its first grace poll
as proof the group is empty: it returns `gone`, never waits out the grace and
never sends its SIGKILL — the right answer for the wrong reason. On Linux the
same probe succeeds, so `stop_group` waits out the whole grace period,
escalates to SIGKILL, and *still* reads the group as alive, returning
`gone: false`. A group that died on the first SIGTERM therefore reports `CLEAN`
on darwin and `FAILED` on Linux, and only one of those is even accidentally
right. This is a test-shaped hazard rather than a production one, because the
case char actually reclaims is an **orphan**: its parent is gone, so init reaps
it the moment it dies and both platforms answer `ESRCH`. **Reap before reading
the probe as "empty", and reap before counting `ps` output.**

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
`char config verify`, and in between puts the literal text `null` into a spawned process's
environment. `crates/core` rejects it at the `env:` block for exactly this reason —
see PLAN.md §4.1.1.

Two smaller facts from the same session, recorded because they change what `config verify`
needs to check: a **duplicate mapping key is an error**, not last-wins, so two components with
one name cannot reach the core; and `Error::location()` is populated for both syntax and typed
errors, which is what makes PLAN.md §4.1.1's decision 4 free.

## Typer / Click exit codes *(historical — charkit is Rust)*

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
  `ARCHITECTURE.md` §1.6 now carries in its exit table alongside the signal carve-out. Note the stdlib explicitly warns against this setting; the
  warning is about libraries that need to observe the error, which char does not.

Whichever is chosen, the "exit code = `f(error.class)`" rule needs an explicit carve-out for
signal-derived codes, covering `130` and `141` together.

## Docker CLI — reading labels back off a resource

Measured on darwin against **Docker 29.6.2**, 2026-08-10. char stamps three
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

**Consequence char acts on:** labels are read as `{{json …}}` and parsed, not as
a delimited line. A workspace path may legally contain a tab or a newline, and
`char.workspace_path` is a real path — a delimiter a value can contain is one
that eventually attributes a resource to the wrong workspace, which is the
failure the label exists to prevent.

## Docker Compose

Measured on darwin against **Docker Compose v5.3.1**, 2026-08-09. Eleven entries. All but two
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

**Not compose, and not platform-neutral.** This entry is `sh -c` word-splitting and command
substitution with no compose involved, measured on darwin. `/bin/sh` is `bash` on darwin and
`dash` on Debian and Ubuntu, and char hardcodes it in **two** places: `template::shell_argv`
for a `setup:` step, and `verbs::dispatch` inline for a `commands:` entry, which builds the
same `["/bin/sh", "-c", …]` itself because it has to append shell-quoted passthrough after
substitution. Grepping for the helper finds one of them. Both inherit whatever `/bin/sh` is on
the machine, so the shell char actually invokes differs by platform.
**The dash side is unverified.** Nothing here rests on it, because the schema makes
`shell: true` beside `${files}` unrepresentable on every platform; what has not been measured
is whether any *other* `shell: true` command char runs behaves differently under dash.

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
# a document with labels: {char.workspace: deadbeef} on the service only
docker ps      -q --filter label=char.workspace=deadbeef   # 1
docker network ls -q --filter label=char.workspace=deadbeef  # 0  <- char-deadbeef_default EXISTS
docker volume  ls -q --filter label=char.workspace=deadbeef  # 0  <- char-deadbeef_pgdata  EXISTS
```

**This is the founding bug of the project, reintroduced.** §1 of `PLAN.md` cites 29 leftover
per-worktree Docker networks exhausting the default bridge address pool. A `clean` that finds
resources by label finds the containers and leaves the network and the volumes behind — with
no verb that can ever locate them again, because the label they would be found by was never
applied.

The fix is in the same document, and it works:

```yaml
volumes:
  pgdata:  { labels: { char.workspace: cafe1234 } }
networks:
  default: { labels: { char.workspace: cafe1234 } }
```

```sh
docker network ls -q --filter label=char.workspace=cafe1234   # 1
docker volume  ls -q --filter label=char.workspace=cafe1234   # 1
```

Top-level `networks:` and `volumes:` must be **stamped separately** in the transform step.
Stamping services is not stamping the stack.

### `docker compose config` succeeds with no daemon — `up` is where it fails

```sh
DOCKER_HOST=unix:///nonexistent.sock docker compose config   # rc 0
DOCKER_HOST=unix:///nonexistent.sock docker ps               # rc 1
```

`config` is client-side. So `PLAN.md` §6.0's steps 1–3 all pass against a dead daemon and only step 4
fails — char does its whole resolve-and-transform and discovers the daemon is gone at the end.
Probe the daemon before starting work, and report it as an environment failure rather than as
the stack failing to start.

### The docker CLI has no client-side timeout

Measured against a socket that accepts the connection and never replies: `docker ps` and
`docker compose up -d` were both still running at 30 seconds with no output. There is no flag
for this. **Every docker invocation needs char's own timeout**, and the one that matters most
is the `docker ps` in `init`'s reap pass — without a timeout, a hung daemon wedges every new
workspace on the machine, including the verb whose job is recovery.

### Override merging does work for `labels:` and `build.labels:`

Both merge as expected. Labels were never the hard part; ports were.
