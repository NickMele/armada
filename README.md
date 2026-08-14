# Armada

One suite for working with coding agents across every repo and every machine you use. Four
modules under one binary. **POSIX only — macOS and Linux. Not Windows**, because process groups,
signals and file locks are load-bearing here rather than incidental.

> **Status: one module built, three to go.** **Manifest** — the workspace layer, formerly
> Armada — has `init`, `clean`, `status` and repo `commands:` working over a machine-global
> store. `up`, `down`, `check`, `config` and `explain` are not built. **Guild**, **Fleet** and
> **Helm** do not exist yet. The M0 research spike is **done** and its findings are recorded
> in [`docs/PHASES.md`](docs/PHASES.md) §9.1. See [`docs/commands/reference.md`](docs/commands/reference.md) for
> what each command does and whether it exists.

## The idea

Every new project means setting up agent files, scripts, hooks, MCP servers and plugins again.
Every parallel agent means another chance to collide on a port or strand a container. And
running five agents at once means five things to watch instead of one.

Armada is four modules that stack, each depending only on the ones below it
([`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) §1.9):

| Module | Answers | State |
|---|---|---|
| **[Helm](docs/commands/helm/helm.md)** | The one agent you talk to. It delegates, aggregates, and brings you decisions. The [Bridge](docs/commands/helm/bridge.md) is the screen you watch while it works. | M3 |
| **[Fleet](docs/commands/fleet/spawn.md)** | The agents you don't. Jobs and Drones in isolated worktrees, with port blocks, budgets and verdicts. | M3 |
| **[Guild](docs/commands/guild/init.md)** | You — voice, skills, hooks, subagents, workflows. Global, interviewed once, synced between machines. | M2 |
| **[Manifest](docs/commands/manifest/init.md)** | What a repo is and how to operate it. Knows nothing about agents, deliberately. | partly built |

**Nothing points upward.** Manifest may not reference Fleet; Guild may not reference Helm.
That rule is what keeps Manifest usable by hand, by a script, by CI and by four parallel agents
at once. **Terms are defined once, in [`docs/glossary.md`](docs/glossary.md)** — Job, Drone,
Helm, Bridge, Board, and the three status enums.

### Why Manifest is the foundation and not a detail

Five things go wrong in every repo, every day: starting the apps, running linters, running the
tests, **cleaning up after a run**, and **initialising a fresh checkout**. The last two are the
same bug — you cannot clean up what you never claimed. So Manifest stamps every port, container,
network, volume, image and process with the workspace that made it, and cleanup becomes a query
rather than a memory.

The property that follows is the one no other tool has: **it can still reclaim a workspace's
resources after the directory is gone.** `docker compose down` needs the file. That is exactly
what a fleet of throwaway agent worktrees requires, which is why the hardest part of this system
is also the part that is already built.

| Verb | Contract |
|---|---|
| [`init`](docs/commands/manifest/init.md) | Workspace ready. Runs setup, claims a port block. Idempotent. |
| [`up`](docs/commands/manifest/up.md) | Services running and ready-checked. |
| [`down`](docs/commands/manifest/down.md) | Services stopped. Port block kept. |
| [`check`](docs/commands/manifest/check.md) | Lint, format, test. Scoped, scheduled, locked. |
| [`clean`](docs/commands/manifest/clean.md) | Releases everything this workspace owns. |
| [`status`](docs/commands/manifest/status.md) | What's running, what's mine, what's stale. |

Every verb takes `--json`.

## Install

Not yet published. One static binary — **there is no runtime to install**, no interpreter, no
toolchain. `tmux` is not required and no terminal multiplexer is bundled: agent sessions are
ordinary resumable Claude Code sessions, so anything that opens one already works.

## Documentation

| | |
|---|---|
| [`docs/commands/reference.md`](docs/commands/reference.md) | **Start here for usage.** One page per command: arguments, how it works, output, dependencies. |
| [`docs/glossary.md`](docs/glossary.md) | The vocabulary, defined once. Read it before arguing about a name. |
| [`docs/PLAN.md`](docs/PLAN.md) | The contract — concepts, verbs, config schema, drivers, and the three new modules. |
| [`docs/PHASES.md`](docs/PHASES.md) | Milestones M0–M4, the spike findings, risks. |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | The principles **and why each one exists**. |
| [`docs/traps.md`](docs/traps.md) | Measured environment behaviour. Read before designing anything that depends on how a tool behaves. |
| [`AGENTS.md`](AGENTS.md) | The operational rules, in short form. |

`ARCHITECTURE.md` keeps the reasoning alongside every rule, because a rule without its reason
gets discarded the first time it is inconvenient. It records in [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) §2.8 which document owns which
fact — this README is a derived summary and is never authoritative. **If you want to change a
principle, argue with its rationale.** If a rule turns out to have no rationale recorded, that
is a defect in the document.

---

# Contributing

## Setup

```sh
cargo test
cargo xtask doclint      # the docs are a deliverable; they are linted like one
```

Rust stable, 2021 edition. The MSRV is pinned in `Cargo.toml`. `cargo xtask` needs no
interpreter and no virtualenv — that is why the doc lint is Rust and not a script.

## Workflow

Never commit to `main`. Work on a feature branch.

- **Commits are conventional**, scoped by module — `manifest`, `guild`, `fleet`, `helm`,
  `core`, `schema`, `fixtures`, `docs`.
- **PRs are sized for review, not per milestone.** Review is the binding constraint, so a
  milestone lands as several small PRs. `main` sitting part-way through one is expected.

## The gate

The GitHub Actions matrix runs on every PR across `ubuntu-latest` and `macos-latest`. The two
platforms matter rather than being box-ticking: signals, process groups and file locks are
exactly the things that differ.

| | |
|---|---|
| **lint** | `cargo clippy -- -D warnings` and `cargo fmt --check` |
| **typecheck** | the compiler — `cargo build` failing *is* the typecheck |
| **tests** | unit, integration and e2e |
| **coverage** | ratchet — it may never drop |
| **module boundaries** | the dependency rule of [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) §1.9 |
| **docs** | `cargo xtask doclint` — cross-references resolve, code blocks parse, config keys appear in both an example and prose |

Two further checks run today and **retire in M1**, when the repository goes private: the
contamination grep (`cargo xtask contamination`) and the privacy gate (`cargo xtask privacy`),
alongside a clean-room hook. They exist because this repository is public, and they stay in
force until it is not. Their reasoning is recorded in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) §2.4 and §2.7 so that the retirement does not
read as an unexplained hole, because a rule that vanishes without a reason gets reinvented.

**What replaces them once they go is the fixture set.** Six config fixtures will then be the only
thing standing between this design and being shaped around a single repository. A green grep
never caught the real failure anyway — an abstraction shaped around one repo because that is the only repo anyone
saw is invisible to a grep, and it is exactly what the fixtures exist to catch. Add one whenever
a new repository shape turns up.

## Testing

TDD is mandatory in `core/` — failing test, minimal implementation, passing test. Elsewhere
tests land in the same PR, but the order is not policed.

| Tier | Contents |
|---|---|
| unit | Pure core, no I/O. Fake `ctx.run` and **assert on the argv** — argv is where the bugs actually are. |
| integration | Real subprocesses and files: process-group kill leaving no orphans, concurrent port claims, labels gone after `clean`. |
| e2e | The real CLI against scratch repos. |
| `tests/golden/` | One JSON snapshot per verb. Regenerate by hand — there is no update flag, on purpose. |

Use `#[coverage(off)]` or a documented exclusion, with a reason comment, for genuinely
untestable lines.

## Versioning

Armada stays at `0.x` and does not promise API stability. The consequence: since the package
version carries no compatibility signal, `schema_version` in the `--json` payloads is the only
one. Adding a field does not bump it; removing a field or changing its type does.

## License

Apache-2.0 — full text in [`LICENSE`](LICENSE).
