# Rust practices

This is read before the first line is written, not after review flags something.
It documents practices that are specific to Armada — not general Rust advice,
which exists elsewhere and does not need re-stating here.

The workspace today is three crates: `core-model`, `adapter-traits`, `testkit`,
plus `xtask`. Both `core-model` and `adapter-traits` are deliberately empty —
they exist to fix a dependency shape before there is anything to put inside it.
The remaining nine crates of the twelve `.claude/agents/rust-engineer.md` describes are
not built. Three of them are named elsewhere in this repo, in doc comments and
in `xtask`'s own rules, and are referenced below as `store`, `ipc` and
`adapters` because that's what the code that already exists calls them. The
other six are not named anywhere in the repo and this document does not invent
names for them.

`cargo xtask verify-foundations` is red right now, on purpose. A gate that goes
green before the milestone it's checking is finished is the failure mode it
exists to prevent — see the module comment in `xtask/src/main.rs`. Don't chase
green; read what each failing rule says is missing.

## 1. Crate boundaries

| Crate | Status | May depend on | Checked by |
|---|---|---|---|
| `core-model` | exists, empty | nothing (no runtime, no I/O, no vendor) | `cargo tree`, manually today — see below |
| `adapter-traits` | exists, empty | nothing (same rule, same reason) | `cargo tree`, manually today |
| `testkit` | exists, empty | anything a test needs | none — never shipped, never a dependency of anything that is |
| `store` | not built | SQLite (`rusqlite` or similar); scoped so it's the *only* crate with a SQLite dependency | `xtask` rule five (JSON), and whatever crate-dependency rule eventually lands |
| `ipc` | not built | the wire types, `protocol-version.toml`, generated TS | `xtask` rule five (JSON); `cargo xtask verify-protocol` per `agents/protocol-engineer.md` |
| `adapters` | not built | vendor SDKs, freely | `xtask` rule six (vendor literal) |

`core-model` and `adapter-traits` sit under every other crate, which is exactly
why they're empty and gated hardest: a dependency added there is a dependency
added everywhere. The rule as stated in `.claude/agents/rust-engineer.md` — `cargo
tree` must show no `tokio`, no `git2`, no `reqwest` under either — is currently
**not** one of the six rules in `xtask/src/rules.rs`. Run `cargo tree -p
core-model` and `cargo tree -p adapter-traits` yourself before you add a
dependency to either; nothing in the gate or the write-hook stops you yet.
That's a gap in the tooling, not a relaxation of the rule.

### Telling when a dependency belongs in `adapters` instead

The question isn't "is this crate allowed to depend on things" — `store` and
`ipc` both will. The question is narrower: **does this dependency require the
crate to know whose API it's talking to?** If yes, it's `adapters`' job, not
yours, regardless of which crate you're standing in.

Concrete signal: you're about to `cargo add` a crate whose whole purpose is
talking to one vendor (an SDK, a client generated from one company's OpenAPI
spec, a crate named after the vendor), or you're about to write that vendor's
name — in a type, a string, or a comment — anywhere outside `crates/adapters`.
`no_vendor_literal_outside_adapters` (`xtask/src/rules.rs` rule six) catches
the literal; it does not catch the dependency by name, only by what it forces
you to say once it's there. In practice the literal shows up first, in a
comment explaining why the dependency is there, and that's usually the
tell — if you're explaining which vendor a piece of code is for, that
explanation belongs in `adapters`, and so does the code.

## 2. Type-system-first: make the wrong call unspeakable

Every v1 failure that this pattern targets was a **convention** failure — code
that was correct because everyone remembered the rule, not because the
compiler enforced it. The fix generalizes: before adding a runtime check
("don't call this in that state", "don't log this value"), ask whether a
narrower type removes the call from the call site entirely. A check can be
skipped by a future caller in a hurry; a method that doesn't exist can't be.

Three concrete shapes, all named in `.claude/agents/rust-engineer.md`:

**A Drone's VCS handle has no push method.** The type a Drone is handed is
scoped to what a Drone is allowed to do — commit and diff inside its own
worktree — and push simply isn't a method on it:

```rust
pub struct DroneVcs {
    worktree: PathBuf,
}

impl DroneVcs {
    pub fn commit(&self, message: &str) -> Result<Oid, VcsError> { /* ... */ }
    pub fn diff(&self) -> Result<String, VcsError> { /* ... */ }
    // no `push`. Whatever component owns landing a branch — Fleet, after
    // review — gets a different type with that capability. A Drone cannot
    // push because the call does not exist, not because something checks
    // and rejects it at the boundary.
}
```

**`Secret<T>` has no `Debug`, `Display`, or `Serialize`.** Not a redacting
`Debug` impl that prints `***` — no impl at all:

```rust
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn new(value: T) -> Self {
        Secret(value)
    }

    pub fn expose(&self) -> &T {
        &self.0
    }
}

// Deliberately no `impl Debug`, `impl Display`, `impl Serialize` for Secret<T>.
```

A redacting `Debug` impl is a trap: it compiles, it looks handled, and it
depends on nobody ever adding a second field that also needs redacting.
Omitting the impl means `format!("{:?}", secret)` fails to compile, and — this
is the part that makes it worth doing — the property **cascades**. A struct
that embeds a `Secret<T>` field can't `#[derive(Debug)]` either without
excluding that field explicitly, so the compiler forces the decision at every
new call site that touches a secret, not just the first one.

**`DroneSpawnConfig` has no escape hatch.** `--strict-mcp-config` is a required
constructor argument, not an `Option`, and there is no raw-argv builder:

```rust
pub struct DroneSpawnConfig {
    workspace: PathBuf,
    model: ModelId,
    strict_mcp_config: PathBuf,
    // ...
}

impl DroneSpawnConfig {
    pub fn new(workspace: PathBuf, model: ModelId, strict_mcp_config: PathBuf) -> Self {
        DroneSpawnConfig { workspace, model, strict_mcp_config }
    }
    // No `Default`. No `.arg(&str)` or `with_raw_args(Vec<String>)`. If a
    // future flag is needed, it's a new named field and a new constructor
    // parameter — never a string that bypasses the fields already here.
}
```

The failure this refuses is the config that's *correct today* because whoever
built it remembered to pass the flag, and wrong the day someone builds a
`DroneSpawnConfig` by a different path and forgets. There is no different path.

## 3. Error handling across a crate boundary

The shape an error takes once it leaves its crate — the wrapping type, the
correlation ids, the code manifest, the wire form — is fixed by
`docs/contracts/error-contract.md`. This section covers only what a crate
boundary demands of a `Result`; the contract is the authority for everything
past it.

A `Result<T, E>` that crosses a crate boundary carries a typed `E` the caller
can match on — not a `String`, not an opaque boxed error that collapses
everything to "something went wrong." The caller on the other side of a crate
boundary is, by construction, code that didn't see how the error happened; a
string forces it to either parse prose or give up and propagate blindly, which
is the convention failure again in a different shape. (A derive macro defines these
enums rather than a hand-written `impl std::error::Error` — the contract says
why, and what it costs in build time. The shape below holds either way.)

**The specific rule: a query function never returns pre-filtered results.**
If a function reads N records and M of them fail to parse, the function's
signature has to make the M visible to the caller — not decide on the caller's
behalf that M doesn't matter.

```rust
// Wrong: the store decides silently that a row that doesn't parse isn't a Job.
pub fn list_jobs(&self) -> Vec<Job> {
    self.rows()
        .into_iter()
        .filter_map(|row| Job::try_from(row).ok())
        .collect()
}

// Right: the caller sees what failed and decides what to do about it.
pub fn list_jobs(&self) -> Result<Vec<Job>, ListJobsError> {
    let mut jobs = Vec::new();
    let mut failed = Vec::new();
    for row in self.rows() {
        match Job::try_from(row) {
            Ok(job) => jobs.push(job),
            Err(e) => failed.push(e),
        }
    }
    if failed.is_empty() {
        Ok(jobs)
    } else {
        Err(ListJobsError::PartialParseFailure { jobs, failed })
    }
}
```

This is a measured v1 bug, not a hypothetical: a query function in v1 used
`.filter_map(Result::ok)` after a store call and silently dropped 21 real
Jobs. The list it returned was well-typed, compiled cleanly, and was simply
missing a fifth of its rows, with nothing in the signature saying so. The rule
exists because `Vec<T>` and `Result<Vec<T>, E>` are both plausible-looking
return types for a listing function, and only one of them makes it possible
for the caller to notice that rows went missing. Prefer the second shape (an
explicit partial-failure variant, or returning failures alongside successes)
over anything that resolves to a plain `Vec<T>` once a fallible parse is in
the path.

## 4. Where `serde_json::from_*` may appear

Exactly two places: `store` and `ipc`. Those are the two places bytes enter
the process — `store` reading its own SQLite rows back into typed values,
`ipc` reading whatever arrives on the Fleet–Bridge wire. Everywhere else, a
value already arrived as a typed Rust value through a function call or a
trait method; a `serde_json::from_*` appearing in the middle of the system
means something was serialized somewhere just to be deserialized again a few
calls later, which is either a missed opportunity to pass the value directly
or a sign the boundary is in the wrong place.

This is enforced twice, deliberately: `xtask` rule five
(`no_untyped_json_outside_store_and_ipc`) catches it after the fact, and
`hooks/guard_write.py` denies the write before it happens — the two carry the
same exemption list (`crates/store/`, `crates/ipc/`, plus `xtask/` itself,
since the gate's own source names the pattern it forbids) and a comment in
each file says the two must not drift. If you change one, change the other.

Note what this rule does *not* restrict: `#[derive(Serialize, Deserialize)]`
on a `core-model` type is fine and expected — `core-model`'s own doc comment
says serialization lives there only as derives on types defined there. The
rule is about parsing untyped bytes into a value, not about a type knowing how
to serialize itself.

## 5. Testing

**Unit tests** live beside the code they test and check what doesn't need a
process boundary: state-machine transitions on the Job record, parsing logic,
anything where the input and expected output both fit in the test function.
`core-model`'s state transitions and escalation logic are the obvious tenants
once M1 fills the crate in.

**Integration tests** live in `tests/acceptance/` at the repo root — the
directory `xtask` rule one (`acceptance_test_exists_and_fails`) is already
watching, currently failing because nothing is there yet. These are for
anything that crosses a crate boundary for real: a Drone's output stream
being parsed by whatever watches it, a Job moving through Fleet, a round trip
through `store`'s actual SQLite file.

`testkit` is the harness for the second kind. It's a fake — not a live Drone —
driven by NDJSON fixtures under `crates/testkit/fixtures/ndjson/`, and the
fixtures are weighted toward misbehaviour on purpose. `testkit`'s own doc
comment gives the reason: v1 shipped 586 commits and 2,181 passing tests, and
the Jobs that suite called done didn't do what they claimed. A fake harness
that only emits happy-path output reproduces exactly that — a fast green
suite that proves nothing about the part that actually failed. So the fixture
set is one file per **failure mode**, not per happy path, and each one is a
specification of what a misbehaving Drone's output stream really looks like,
written before the detector that's supposed to catch it:

| Fixture | Failure mode |
|---|---|
| `silence.ndjson` | Silence / stalled |
| `claims-done-no-evidence.ndjson` | Claims done, no evidence artifact |
| `plain-text-bypass.ndjson` | Claims done in plain text, bypasses the structured report |
| `thrashing.ndjson` | Thrashing / off-rails |
| `evidence-gaming.ndjson` | Evidence gaming |

`xtask` rule two enforces that all five exist; it does not and cannot check
that a fixture is a faithful specification of the failure mode it names —
that's a review question, not a gate question, when you're the one writing
or changing one. Fixtures also sit beside a pinned real capture used by a
format-drift contract test, per `testkit`'s doc comment — a fixture's shape
has to track what the real output format actually looks like, not drift from
it over time as the format changes upstream.

## 6. The 500/900 line rule

Warn at 500 lines, fail at 900 — `xtask` rule three, and the same thresholds
in `hooks/guard_write.py`, which turns the warning into an `ask` decision
before the write and the failure into a `deny`. (The hook's number for an
`Edit` is estimated from the size of the change, since the real file doesn't
exist yet when the hook runs; it's exact for `Write`.)

500 is not a hard rule because a hard rule at a line count gets satisfied by
moving code into a second file without moving the coupling — the metric goes
green, the actual problem (too much reasoning living in one place) doesn't.
The warning is a deliberate prompt to say *why* a file is still growing, not
an instruction to split it reflexively. In practice, a `.rs` file that's
pushing past 500 lines in this codebase is usually one of two things: it's
mixing a type definition with the runtime logic that operates on it (the
type-system-first pattern above tends to want the type on its own), or it's
grown a `match` arm for every new case instead of dispatching through a trait
that already exists for exactly that purpose — `adapter-traits` exists so
that kind of growth happens in `adapters`, per-vendor, instead of in one
crate's `match`.

The rule covers `.rs`, `.ts`, and `.tsx` under `crates`, `apps`, `packages`,
and `xtask`. `docs/` is prose and isn't gated by it.

## 7. Mechanical rules worth knowing exist

**Every file under `crates/*/src/` is listed in `foundations-manifest.txt`** —
one repo-relative path per line, sorted, no globs, no comments. Add the line
in the same change that adds the file. `guard_write.py` denies the write
outright if the manifest doesn't list the path first, which means: add the
manifest line, then write the file, not the other way round — the hook checks
before the file exists on disk. A glob in the manifest would defeat the point;
it would pre-authorize a whole directory, which is the exact drift the rule
exists to make visible as a diff instead.

**`unsafe_code = "forbid"`** is a workspace-wide lint in the root `Cargo.toml`.
It's not per-crate and not a `#![deny]` you'll find repeated anywhere — it's
already on for everything in this workspace.

**No vendor literal outside `adapters`** (rule six) is checked case-insensitive
against `anthropic`, `claude`, `openai`, `gpt-4`, `gemini`, `copilot`,
`github`, `gitlab`, `bitbucket`, `docker` — the vendor list is `xtask`'s own,
not the plan's, and adding a vendor to it is a deliberate one-line change to
`xtask/src/rules.rs`, not an incidental one. It scans comments along with
code, which is the point: the boundary leaks in a comment before it leaks in
a type.

## 8. Build and test time, measured on v1

Two numbers, from v1's own `armada.yml`. One was solved and the fix transfers;
the other was **not solved**, and knowing that is worth more than the fix would
have been.

### The warm number: use `cargo nextest`, not `cargo test`

**Measured at 3x on v1's workspace: 83 seconds against 27, for the same 2,034
tests.** 35 seconds of that 83 was actual CPU — most of the old number was one
test binary blocking while the rest waited their turn.

The reason is structural rather than incidental. `cargo test` runs each test
binary to completion before starting the next, and v1 had twenty of them.
`nextest` pools every test across all binaries, so a single slow binary stops
being a serial bottleneck. A workspace of twelve crates will have the same shape.

```
cargo nextest run --workspace
```

**It has to be installed before a check needs it, and v1 got that wrong.** The
setup step claimed to install it and installed nothing; the failure surfaced on
2026-08-17 as a Job whose first test run said `no such command: nextest`. A check
a Drone cannot run is a check that does not exist. Install it with `--locked`, so
the runner is the same build everywhere and an already-present version is a no-op
rather than a rebuild.

### The cold number: still unsolved, and v1 said so

Compilation after a merge cost **around four minutes**, and `nextest` does not
touch it — v1's own note calls it "a separate problem and this is not a fix for
it".

**There was no compiler cache.** No `sccache`, no `mold` or `lld`, nothing. The
only build-time measure v1 shipped was:

```toml
[profile.dev]
debug = "line-tables-only"
```

which cuts debug info to what a backtrace needs, and shortens linking.

**The cause v1 named was self-inflicted, and it is already gone here.** The cold
rebuild was triggered by a git hook that ran `cargo install --path crates/helm
--force` on every checkout and merge onto `main` — so every merge invalidated the
cache and paid for a full reinstall. Those hooks were disabled when v1 was
decommissioned on 2026-08-23 and the files deleted. **Do not reintroduce a hook
that rebuilds on merge.** If v2 wants the binary installed after a merge, that is
a thing you run when you want it, not a thing that happens to you.

**If cold builds become painful again, measure before reaching for a tool.** v1
never established how much of the four minutes was compilation versus linking
versus the forced reinstall, which is why nobody could say whether `sccache`
would have helped. The candidates worth measuring, in the order they usually pay:

| Candidate | What it addresses |
|---|---|
| A faster linker (`lld`, `mold`) | Link time, which dominates incremental rebuilds of a large binary |
| `debug = "line-tables-only"` | Already in place on v1. Less debug info to write and link |
| `sccache` | Cold rebuilds of unchanged dependencies. Pays across branches and worktrees, which matters here because **every Drone gets its own worktree** and would otherwise compile the world |
| Splitting a crate | Only if one crate is genuinely the long pole. Measure with `cargo build --timings` first |

The worktree point is the one specific to Armada: a design that gives every Job
its own checkout multiplies cold builds by the number of concurrent Jobs, so the
cache question gets sharper as Throughput arrives, not softer.

## Open questions

These are gaps this document found rather than filled, because filling them
isn't this document's job:

- **[compiler-cache]** Whether v2 adopts a compiler cache, and on what
  measurement. v1 never established how much of its four-minute cold build was
  compilation, linking or the forced reinstall.
- **[cargo-tree-gate]** The `cargo tree` check on `core-model` and
  `adapter-traits` is stated as gated in `.claude/agents/rust-engineer.md` but
  is still not one of the gate's rules. Run it by hand until someone adds it.
