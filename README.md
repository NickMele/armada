# charkit

One consistent vocabulary for managing a repo's tech stack, so a coding agent working across
several repos never has to re-derive how to start, check, or clean up any of them. Six verbs,
identical everywhere; everything else is config. **POSIX only — macOS and Linux. Not
Windows**, because process groups, signals and file locks are load-bearing here rather than
incidental.

> **Status: pre-implementation.** Phase 0 (foundations) is complete — the architecture and
> working agreements are recorded. No source code exists yet. See [`docs/PLAN.md`](docs/PLAN.md)
> for the full specification and phase order.

## The idea

Five things go wrong in every repo, every day: starting the apps, running linters, running
all the tests, **cleaning up after a run**, and **initialising a fresh checkout**. The last
two are the same bug — you cannot clean up what you never claimed, and claiming happens at
init. That observation is the whole design.

| Verb | Contract |
|---|---|
| `char init` | Workspace ready. Runs setup, claims a port block, writes `.char/`. Idempotent. |
| `char up` | Services running and ready-checked. Records what it started. |
| `char down` | Services stopped. Port block kept. |
| `char check` | Lint, format, test. Scoped, scheduled, locked. |
| `char clean` | Releases everything this workspace owns. |
| `char status` | What's running, what's mine, what's stale. |

Every verb takes `--json`.

## Install

Not yet published. Once it is (phase 7):

```sh
command -v uv >/dev/null || curl -LsSf https://astral.sh/uv/install.sh | sh
uv tool install charkit
```

`uv` provisions a Python interpreter itself, so this works on a machine with none.

## License

Apache-2.0.

---

# Contributing

## Start here

1. [`docs/PLAN.md`](docs/PLAN.md) — the complete specification. Phases, config shape, verbs.
2. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — the principles **and why each one exists**.
3. [`AGENTS.md`](AGENTS.md) — the operational rules, in short form.

`ARCHITECTURE.md` deliberately keeps the reasoning alongside every rule, because a rule
without its reason gets discarded the first time it is inconvenient. **If you want to change
a principle, argue with its rationale.** If a rule turns out to have no rationale recorded,
that is a defect in the document — please raise it.

## Setup

```sh
uv sync
uv run pytest
```

Python 3.12 or newer.

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
- Completed phases are tagged (`phase-1`, `phase-2`, …).

## The gate

`no-mistakes` is the primary gate: it runs an agent code review, then
tests, lint and docs, then pushes and opens the PR. A GitHub Actions matrix runs alongside it
on `ubuntu-latest` and `macos-latest` — it exists to cover the platform you are not developing
on, since signals and process groups are exactly the things that differ.

A change must clear all six:

| | |
|---|---|
| **lint** | `ruff` |
| **typecheck** | `mypy --strict` |
| **tests** | unit, integration and e2e |
| **coverage** | ratchet — it may never drop |
| **imports** | `import-linter`: `core` imports nothing concrete, `adapters` import core protocols only, `cli` is the only module importing both |
| **contamination** | a grep, described below |

## Two rules that trip people up

**1. Certain strings may never appear under `src/`.**

```
chariot   tilt   NEXT_PUBLIC   .claude   backend/   web/
```

The check engine was ported out of a Django+Next monorepo, and this grep is what stops that
repo's specifics coming along for the ride — a hardcoded package directory, a vendor
assumption that belongs in config rather than code. It runs in the gate and **there is no
allowlist**: if it fires, the code changes, not the pattern.

That includes docstrings and test fixtures. Use neutral examples — `foreman start`,
`root: services/api` — when you need to illustrate a command or a component root.

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
| `tests/integration/` | Real subprocesses and files: process-group kill leaving no orphans, concurrent `O_EXCL` port claims, docker labels gone after `clean`. |
| `tests/e2e/` | The real CLI against scratch repos. |
| `tests/golden/` | One JSON snapshot per verb. Regenerate by hand — there is no update flag, on purpose. |

Use `# pragma: no cover` with a reason comment for genuinely untestable lines.

## Versioning

charkit stays at `0.x` and does not promise API stability. The consequence: since the package
version carries no compatibility signal, `schema_version` in the `--json` payloads is the only
one. Adding a field does not bump it; removing a field or changing its type does.
