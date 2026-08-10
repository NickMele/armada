# charkit

One consistent vocabulary for managing a repo's tech stack, so a coding agent working across
several repos never has to re-derive how to start, check, or clean up any of them. Six verbs,
identical everywhere; everything else is config. **POSIX only — macOS and Linux. Not
Windows**, because process groups, signals and file locks are load-bearing here rather than
incidental.

> **Status: the contract, and no runtime.** Phases 0 and 1 are complete — the architecture and
> working agreements are recorded, and the `char.yml` contract now exists as a JSON Schema, the
> structs that mirror it, and six fixture configs with a golden resolved snapshot each. `char`
> builds and answers `--version`; it has no verbs yet. See [`docs/PLAN.md`](docs/PLAN.md)
> for the full specification and phase order.

## The idea

Five things go wrong in every repo, every day: starting the apps, running linters, running
all the tests, **cleaning up after a run**, and **initialising a fresh checkout**. The last
two are the same bug — you cannot clean up what you never claimed. So char stamps every port,
container, network, volume, image and process with the workspace that made it, and `clean`
becomes a query rather than a memory. That is the whole design, and the property that follows
is the one no other tool has: **char can still reclaim a workspace's resources after the
directory is gone.** `docker compose down` needs the file.

| Verb | Contract |
|---|---|
| `char init` | Workspace ready. Runs setup, claims a port block, writes `.char/`. Idempotent. |
| `char up` | Services running and ready-checked. Records what it started. |
| `char down` | Services stopped. Port block kept. |
| `char check` | Lint, format, test. Scoped, scheduled, locked. |
| `char clean` | Releases everything this workspace owns. |
| `char status` | What's running, what's mine, what's stale. |

Every verb takes `--json`. Alongside them: `char config scan` / `config verify`,
`char agents-md`, and `char explain` — which hands back the evidence a stack trace does not
carry (the exact argv, what it waited on and who held it, whether this check failed the same
way in the last three runs). It runs no model: char's caller is already an agent, and the
useful thing char can do is give it what it cannot see.

## Install

Not yet published. Once it is (phase 7):

```sh
curl -LsSf https://raw.githubusercontent.com/<owner>/charkit/main/install.sh | sh
```

One static binary, measured at 2.09 MB stripped and treated as a floor (`PHASES.md`). **There is no runtime to install** — no interpreter, no
toolchain, nothing to provision. `cargo install charkit` is a second channel for people who
would rather build it themselves.

## License

Apache-2.0 — full text in [`LICENSE`](LICENSE).

---

# Contributing

## Start here

1. [`docs/PLAN.md`](docs/PLAN.md) — the contract: concepts, verbs, config schema, drivers.
2. [`docs/PHASES.md`](docs/PHASES.md) — what gets built, in what order, and the fixture set.
3. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — the principles **and why each one exists**.
4. [`docs/traps.md`](docs/traps.md) — measured environment behaviour; read before designing
   anything that depends on how a tool behaves.
5. [`AGENTS.md`](AGENTS.md) — the operational rules, in short form.

`ARCHITECTURE.md` deliberately keeps the reasoning alongside every rule, because a rule
without its reason gets discarded the first time it is inconvenient. It also records, in `ARCHITECTURE.md` §2.8,
which document owns which fact — this README is a derived summary and is never authoritative. **If you want to change
a principle, argue with its rationale.** If a rule turns out to have no rationale recorded,
that is a defect in the document — please raise it.

## Setup

```sh
cargo test
cargo xtask doclint      # the docs are a deliverable; they are linted like one
```

Rust stable, 2021 edition. The MSRV is pinned in `Cargo.toml`. `cargo xtask` needs no
interpreter and no virtualenv — that is why the doc lint is Rust and not a script.

## Workflow

Never commit to `main`. Work on a feature branch and let the gate push it.

```sh
git switch -c core/scheduler-reducer
# ... work ...
no-mistakes axi run --intent "<what you set out to accomplish>"
```

- **Commits are conventional**, scoped by module — `core`, `adapters`, `cli`, `schema`,
  `fixtures`, `docs`.
- **PRs are sized for review, not per phase.** Review is the binding constraint on this
  project, so a phase lands as several small PRs. `main` sitting part-way through a phase is
  expected.
- Completed phases are tagged (`phase-1`, `phase-2`, `phase-2.5`, …).

## The gate

**The GitHub Actions matrix is the gate.** It runs on every PR across `ubuntu-latest` and
`macos-latest`, and nothing merges without it. The two platforms matter here rather than being
box-ticking: signals, process groups and file locks are exactly the things that differ.

**`no-mistakes` is a recommended pre-flight, not a requirement.** It runs an agent code review
plus tests, lint and docs locally, then pushes and opens the PR — so problems surface before CI
rather than after. It is how this repo is maintained day to day and you are welcome to install
it, but your PR is judged by the same six checks either way.

A change must clear all six:

| | |
|---|---|
| **lint** | `cargo clippy -- -D warnings` and `cargo fmt --check` |
| **typecheck** | the compiler — `cargo build` failing *is* the typecheck |
| **tests** | unit, integration and e2e |
| **coverage** | ratchet — it may never drop |
| **crate boundaries** | `core` depends on nothing concrete, `adapters` depend on core traits only, `cli` is the only crate depending on both |
| **contamination** | a grep, described below — run by `cargo xtask doclint` |
| **docs** | `cargo xtask doclint` — cross-references resolve, code blocks parse, config keys appear in both an example and prose |

## Two rules that trip people up

**1. Certain strings may never appear under `src/` or `tests/`.**

```
chariot   tilt   NEXT_PUBLIC   .claude   backend/   web/
```

The check engine was ported out of a Django+Next monorepo, and this grep is what stops that
repo's specifics coming along for the ride — a hardcoded package directory, a vendor
assumption that belongs in config rather than code. It runs in the gate and **there is no
allowlist**: if it fires, the code changes, not the pattern. The exact pattern lives in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) §2.4 and is deliberately stated only there.

That includes docstrings and ported test cases — `tests/` is in scope for exactly that reason.
`tests/fixtures/` is exempt, because a fixture config describes a hypothetical repo and naming
that repo's directories is the fixture working. Use neutral examples — `foreman start`,
`root: services/api` — when you need to illustrate a command or a component root.

`docs/harvest.md` is the one exception: it exists to describe the source repo, so a ban would
forbid recording the assumptions you are meant to strip. It carries behaviour, not
implementation — no verbatim code.

A green grep only means no *crude* contamination. The subtler failure — an abstraction shaped
around one repo because that is the only repo anyone saw — is invisible to it, and is what the
six fixture configs exist to catch.

**2. Only phase 3's harvester reads the source repo.** Everything else works from the plan,
the architecture document, the fixtures and the harvest notes. If some later phase feels like
it needs to look, that means the plan is underspecified — fix the plan.

## Testing

TDD is mandatory in `core/` — failing test, minimal implementation, passing test. In
`adapters/` and `cli/` tests land in the same PR, but the order is not policed; a test-first
at the adapter boundary is usually just asserting on your own fake.

| Tier | Contents |
|---|---|
| `tests/unit/` | Pure core, no I/O. Fake `ctx.run` and **assert on the argv** — argv is where the bugs actually are. |
| `tests/integration/` | Real subprocesses and files: process-group kill leaving no orphans, concurrent port claims and lease reclamation, docker labels gone after `clean`. |
| `tests/e2e/` | The real CLI against scratch repos. |
| `tests/golden/` | One JSON snapshot per verb. Regenerate by hand — there is no update flag, on purpose. |

Use `#[coverage(off)]` or a documented exclusion, with a reason comment, for genuinely
untestable lines.

## Versioning

charkit stays at `0.x` and does not promise API stability. The consequence: since the package
version carries no compatibility signal, `schema_version` in the `--json` payloads is the only
one. Adding a field does not bump it; removing a field or changing its type does.
