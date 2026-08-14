# AGENTS.md

Instructions for coding agents working in this repository.

**Armada** is a CLI (`armada`) — a suite for working with coding agents across every repo and
machine. Four modules under one binary: **Manifest** (what a workspace is), **Guild** (your
portable setup), **Fleet** (the agents you don't talk to), **Helm** (the one you do). Rust
(2021 edition), POSIX only.

**Nothing points upward** — Manifest may not reference Fleet, Guild may not reference Helm.
`ARCHITECTURE.md` §1.9. That rule and the crate layering of `ARCHITECTURE.md` §1.5 are both
enforced by `cargo xtask boundaries`, which reads the crate graph and the module each crate
belongs to.

**The vocabulary is fixed in `docs/glossary.md`** — Job, Drone, Helm, Bridge, Board, and the
three status enums. Use those words and no synonyms.

**One spelling everywhere: `armada`.** M1 renamed the tool (`PHASES.md` §8.3), so the binary
is `armada`, the config is `armada.yml`, the state is `~/.armada/`, and the crates are
`armada-core`, `armada-manifest` and `armada-helm`. Three things deliberately keep the old
`char` spelling and are not typos to fix:

- `docker.rs`'s `LEGACY_LABEL_*`, read so a container stamped before the rename is still
  reclaimable — for one release (`PHASES.md` §8.3).
- The `<!-- char:begin -->` markers, recognised for the same reason (`PLAN.md` §5.1).
- The privacy gate's config: `.claude/contamination.local` and
  `CHARKIT_CONTAMINATION_EXTRA`. They are gitignored and already on every machine that has
  one, so renaming them would silently disarm the gate exactly there.

## Read these first, in order

1. **`docs/PHASES.md`** — `PHASES.md` §8.1, then **your phase only**. It tells you what you
   are building and how you will know it is done.
2. **`docs/PLAN.md`** — the contract. Always read `PLAN.md` §2 (concepts), `PLAN.md` §3 (verbs
   and the `--json` envelope) and `PLAN.md` §4 (configuration). Read `PLAN.md` §5 and
   `PLAN.md` §6 only if your phase touches the bootstrap sandwich or the service drivers.
3. **`docs/ARCHITECTURE.md`** — the principles and, more importantly, *why* each exists.
4. **`docs/traps.md`** — measured environment behaviour. Read the relevant section **before**
   designing anything that depends on how a tool behaves, and add to it when you discover
   something surprising. Every entry was measured rather than read, because the ones that
   matter are the cases where the documentation and the behaviour differ.

If you are about to argue with a rule, argue with its rationale in `ARCHITECTURE.md`. If a
rule has no rationale recorded, that is a bug in the document — say so.

**This file is a derived summary and is never authoritative.** Where it disagrees with an
upstream document, the upstream document is right and this one is the bug. Precedence and the
table of which document owns which fact are in `ARCHITECTURE.md` §2.8:

```
traps.md > ARCHITECTURE.md > PLAN.md > PHASES.md > AGENTS.md / README.md
measured   decided           specified  sequenced   derived
```

---

## Rules that are easy to break by accident

### 1. The privacy gate is permanent — and it fails loudly

**This repository is public and stays public** (`ARCHITECTURE.md` §2.4), so `cargo xtask
privacy` and `cargo xtask history` are standing checks with no retirement date.

- Never write the private source repository's name or the literal `$HOME` into a tracked file.
- If either reports `(name rule unconfigured)` it is **checking nothing** — and it now exits
  non-zero rather than passing quietly. Arm it via `.claude/contamination.local` or the
  `CHARKIT_CONTAMINATION_EXTRA` secret. `CHARKIT_PRIVACY_UNCONFIGURED_OK=1` acknowledges an
  unarmed run deliberately; reaching for it to make a failure go away is the failure.

**Two rules retired**, each on its own merits and not because the repository changed: the
contamination grep, superseded by the six fixtures, and the clean-room rule, whose harvest has
landed. `xtask/src/contamination.rs` and `.claude/hooks/clean-room.sh` are gone. Their
reasoning is kept in `ARCHITECTURE.md` §2.4 and §2.7 rather than erased, because a rule that
vanishes without a reason gets reinvented.

**What did not go away is the risk the grep was a poor proxy for.** A green grep only proved the
absence of *crude* contamination. The failure that matters is invisible to it: an abstraction
shaped around one repository because that is the only repository anyone looked at.

**So the six fixtures are now the whole of this discipline.** They live at
`tests/fixtures/<name>/` with a golden resolved snapshot beside each. The rule that replaces the
grep:

> When a repository shape turns up that the fixtures do not cover, **add a fixture before adding
> the feature**.

Still true regardless, and now a matter of taste rather than a gate: prefer neutral examples in
docs and fixtures — `cmd: foreman start`, `root: services/api` — because an example naming one
real project's layout is the first step toward code that assumes it.

### 2. Write paths relative to the repo, or as `~/`

**The `$HOME` ban is part of the privacy gate above and is live** — `cargo xtask privacy` fails
on a literal home path in a tracked file. The habit outlives the gate on its own merits: an
absolute path in a document is wrong on every machine except the one it was written on, and this
project's whole premise is working across several.

### 3. Check which milestone you are in before writing code

Full sequencing in [`docs/PHASES.md`](docs/PHASES.md) §8. Short version:

- **M0 is complete** — the research spike. Its findings are `PHASES.md` §9.1 and they are
  **evidence, not recollection**: resumable sessions, budget telemetry, the inbox mechanisms and
  plugin coverage were all measured. If one turns out to be wrong, fix the finding and say which.
- **Manifest is partly built.** The config contract is frozen: the JSON Schema is authoritative,
  the structs mirroring it are in `crates/core`, and six fixtures have a golden resolved snapshot
  each. **What phase 1 decided and the fixtures forced is `PLAN.md` §4.1.1** — read it before
  adding a config key. The ownership layer exists behind `init` / `clean` / `status` and the
  `commands:` dispatcher, and **`check` is built** — its scheduler, scope resolution, run
  directory and verdict aggregation, with what it settled and the gap it leaves open in
  `PHASES.md` §9.3. `check --detach` and `check --status` are refused by name as not built.
  `up`, `down`, `config` and `explain` are **not built**.
- **M1 has landed**, less one row: `skills:` was not built, because M1 adds no capability.
  Tracked in `PHASES.md` §8.3.
- **Guild, Fleet and Helm do not exist.** Their specification is `PLAN.md` §13–§15 and their
  usage is [`docs/commands/reference.md`](docs/commands/reference.md).

**M4's loop needs `check` detached.** A verdict is only `PASS` if it carries evidence an
external command produced, and `check` now produces it — but `--detach` and `--status` are
still refused, so a loop can run a check to completion and cannot yet start one and poll it.
Manifest's remaining verbs are first-class work, not background work.

## Architecture rules, in short

Rationale for every one of these is in `docs/ARCHITECTURE.md` §1.

| Rule | In practice |
|---|---|
| **Three seams only** | `ctx.run`, `ctx.now`, `ctx.fetch`. Docker and git are adapter modules that call `ctx.run` — they are not seams. The filesystem is never faked; use `tempfile::TempDir`. |
| **Pure core** | Decisions are functions over data. Spawning, writing, killing and labelling live in `adapters/`. |
| **Scheduler is a reducer** | `step(State, Event) -> (State, Vec<Action>)`. `Event`/`Action` are enums matched **exhaustively** — never add a `_ =>` arm, it converts a compile error into silence. The core proposes, the shell attempts, failures come back as events. Applies to the scheduler and the claim/lease loop **only** — elsewhere a plain function is a plain function. |
| **No logic in command functions** | parse args → call core → render. Nothing else. |
| **No ambient state** | No `static mut`, no global `OnceCell` holding mutable state. No `std::env::current_dir()` or `std::env::var()` below the entrypoint. The workspace rides on `Ctx`. |
| **Dependencies point inward** | `core` imports nothing concrete. `adapters` depend on core traits only. `cli` is the only crate depending on both. Enforced by the crate graph. |
| **Read verbs never mutate** | `status`, `check --status`, `explain` take no lease, may report a progress state, and their exit code describes the **query**, not the thing queried. A gate uses `--wait`, never `--status`. |
| **No model inside Armada** | Armada never calls an agent CLI to diagnose, repair or explain. `armada manifest explain` emits deterministic evidence; the caller — already an agent — does the diagnosing. Reserved shape in `PLAN.md` §7. |
| **Every verb takes `--json`** | Fixed envelope: `schema_version`, `verb`, `workspace`, `status`, `error`, `data`. Per-verb fields go **inside `data`**, never at the top level, and every plural verb uses `data.results[]` (PLAN.md §3.1). One golden snapshot per verb. |
| **One spelling for failure** | `FAILED`, never `FAIL`. Terminal: `READY` `UP` `DOWN` `CLEAN` `PASS` `OK` `SKIPPED` / `PARTIAL` / `FAILED` / `ABORTED` `DEAD` `TIMEOUT`. Progress, never terminal and never mapped to an exit code: `RUNNING` `WAITING` (with `waiting_on`). |
| **Errors are typed** | `class` ∈ {`bad_invocation`, `bad_config`, `tool_failed`, `timeout`, `aborted`, `environment`, `armada_bug`}, plus `where`. `next_action` is required for `bad_config`. The enum covers **every** non-zero exit — a hole is where a second, competing mapping grows back. |
| **Secrets never leave the shell** | Resolved values are injected into a child's env at spawn and never reach the core, argv, `--json`, logs or `.armada/`. There is no verb that returns a secret. See `ARCHITECTURE.md` §1.8. |

### Exit codes

**One rule: the code is a function of `error.class`, or `0` when there is no error.** Terminal
state never determines it — `FAILED` is exit 3 when the config was wrong and exit 1 when the
tests were.

| | | | |
|---:|---|---:|---|
| `0` | *(no error)* | `4` | `timeout` |
| `1` | `tool_failed` | `5` | `aborted` |
| `2` | `bad_invocation` | `70` | `armada_bug` |
| `3` | `bad_config` | `6` | `environment` — fix the machine, retry unchanged |
| | | `130` | SIGINT |
| | | `141` | SIGPIPE |

A `commands:` child's exit code passes through **verbatim** and is not remapped. Armada's own
codes can only occur when the child never ran; `data.dispatched` says which.

---

## Testing

**TDD is mandatory in `core/`** — failing test, minimal implementation, passing test. In
`adapters/` and `cli/`, tests land in the same PR but the order is not policed.

| Tier | What goes here |
|---|---|
| unit | Pure core. No I/O at all. Fake `ctx.run` and **assert on the argv** — argv is where the bugs are. |
| integration | Real subprocesses, real files. Process-group kill with no orphans. Concurrent claims and lease reclamation. Docker labels verified gone after `clean`. |
| e2e | The real CLI against scratch repos. |
| `tests/golden/` | One JSON snapshot per verb. **Regenerate by hand** — there is deliberately no update flag. |

**Which directory each tier lives in is `ARCHITECTURE.md` §3** — the tiers are a rule about
what a test may touch, not three directories, and unit tests are in-module.

Coverage is gated on a ratchet: it may never drop. Use `#[coverage(off)]` or a documented
exclusion, with a reason comment, for genuinely untestable lines.

---

## Workflow

**Two rules the ownership layer learned the hard way**, with the reasoning in
`ARCHITECTURE.md` §2.1.1 and §2.1.2: **invert every new assertion once and watch
it fail** — a vacuous assertion is worse than none, because it gets cited as
evidence — and **shipped behaviour that disagrees with the spec is a divergence
even if you edited no document**, so conform and record the argument where the
licensed phase will find it.

**Run `cargo xtask doclint` after editing any document.** It is a gate check. It resolves
every `§` cross-reference, parses every fenced block with a real parser, and runs `ARCHITECTURE.md` §2.4's
contamination grep from its single source. If a block is deliberately unparseable, mark it
`<!-- doclint: skip — reason -->` rather than leaving a finding for everyone to scroll past.

**Never commit to `main`.** Work on a feature branch, always.

<!-- doclint: skip — <placeholders>, not runnable shell -->
```sh
git switch -c <scope>/<short-description>
# ... work, committing as you go ...
no-mistakes axi run --intent "<what the user set out to accomplish>"
```

- **Commits are conventional**, scoped by module: `manifest`, `guild`, `fleet`, `helm`,
  `core`, `schema`, `fixtures`, `docs`. Not milestone numbers — they expire.
- **The GitHub Actions matrix is the authoritative gate** — nothing merges without it.
  **`no-mistakes` is the pre-flight you should use**: it runs an agent code review plus test
  and lint locally, then pushes and opens the PR. Drive its gates; do not edit files to fix
  findings while a run is active. Escalate any `ask-user` finding rather than deciding it.
- **PRs are sized for review, not per milestone.** Review is the binding constraint on this
  project. `main` sitting part-way through a milestone is expected and fine.
- **Tag each completed milestone**: `git tag m1`.
- A GitHub Actions matrix (`ubuntu-latest`, `macos-latest`) runs lint, typecheck and tests
  alongside `no-mistakes`. It exists to cover the platform you are not developing on.

### Dogfooding — staged

This repository has its own `armada.yml`.

**Until `check` ships, the gate runs the raw tools** — `cargo clippy`, `cargo fmt --check`,
`cargo test`. A dogfood integration test runs `armada manifest check --json` and asserts it agrees, so a
broken `check` is one failing test rather than an unmergeable repository. **Do not wire
`armada manifest check` into the gate itself yet.**

**When `check` lands, it becomes the gate.** That is the end state; the interim arrangement
exists only because a bug in a half-built tool must not be able to lock its own repository.
`check --detach` / `--status` are the M4 blocker ([`docs/PHASES.md`](docs/PHASES.md) §8.6) —
the engine itself has landed. See `ARCHITECTURE.md` §2.6.

---

## Versioning

Armada stays at `0.x` and does not promise stability. Because the package version therefore
carries no compatibility signal, **`schema_version` in the `--json` payloads is the only
one** — treat it accordingly.

Bump rule: adding a field does not bump. Removing a field or changing its type does.

---

<!-- armada:begin -->
<!-- This block is generated by `armada manifest agents-md --write` from the resolved config.
     It is populated when `agents-md` ships. Anything outside these markers is never touched. -->
<!-- armada:end -->
