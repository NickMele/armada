# AGENTS.md

Instructions for coding agents working in this repository.

**charkit** is a CLI (`char`) giving coding agents one consistent vocabulary for managing a
repo's tech stack. Rust (2021 edition), POSIX only.

## Read these first, in order

1. **`docs/PHASES.md`** — its §8.1, then **your phase only**. It tells you what you are
   building and how you will know it is done.
2. **`docs/PLAN.md`** — the contract. Always read its §2 (concepts), §3 (verbs and the
   `--json` envelope) and §4 (configuration). Read its §5 and §6 only if your phase touches
   the bootstrap sandwich or the service drivers.
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

### 1. Never write these strings under `src/` or `tests/`

```
chariot   tilt   NEXT_PUBLIC   .claude   backend/   web/
```

A grep runs in the merge gate over both directories and **has no allowlist**. If it fires, the
code changes — not the pattern. **The pattern itself lives in `ARCHITECTURE.md` §2.4 and is
stated nowhere else**, including here — a copy inside a markdown table is unrunnable. `tests/` is in scope because phase 3 ports test
*cases* from the source repo, which makes them the second transcription vector.

**One exception: `docs/harvest.md`.** It is not greped, because its job is to describe the
source repo and a ban would forbid recording the assumptions you are meant to strip. It has a
different rule instead — see rule 2.

This includes docstrings, comments and test fixture strings. When you need to illustrate the
`command` driver or a component root, use neutral examples:

```yaml
# yes
cmd: foreman start
root: services/api

# no - these fail the gate
cmd: tilt up --stream
match: ["backend/**"]
```

Note the pattern is `backend/` **with a slash** — bare `root: backend` does not match it. Do
not rely on that; use neutral names regardless.

**`tests/fixtures/` is exempt.** A fixture config describes a hypothetical repo, so naming
that repo's real directories is the fixture working. Everything else under `tests/` is
covered.

The reasoning is in `ARCHITECTURE.md` §2.4. Short version: the tool was ported out of a
Django+Next monorepo, and this catches the port dragging that repo's specifics along with it.

### 2. Only phase 3's *harvester* may read `~/Development/chariot`

Every other phase, and phase 3's implementer, works from `docs/PLAN.md`, `ARCHITECTURE.md`,
the fixtures, and `docs/harvest.md`.

**This is enforced, not requested.** A `PreToolUse` hook default-denies that path for every
agent and allows it only for the harvester, inspecting the whole `tool_input` — so `Read`,
`Glob`, `Grep`, and a `Bash` line containing `rg`, `find`, `cat` or `python -c` are all
covered. If you are not the harvester, you will be denied rather than trusted.

If a phase feels like it needs to look at that repo, **the plan is underspecified — fix the
plan, do not peek.**

**Writing `docs/harvest.md`?** It describes *behaviour*, never carries *implementation*.
Prose, tables and trap descriptions; short config or regex fragments are fine; no verbatim
implementation code. The test is: could this be pasted into `src/` and compile? Transcribed
paths get caught downstream by the grep — pasted structure does not, and structure is the
contamination nothing else can catch.

### 3. Check what phase you are in before writing code

- **Phase 0 is complete.** It produced these documents.
- **Phase 1** must land alone, and ships all six fixture configs. Do not fan out until the
  config contract is committed. Full sequencing in [`docs/PHASES.md`](docs/PHASES.md).

---

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
| **Every verb takes `--json`** | Fixed envelope: `schema_version`, `verb`, `workspace`, `status`, `error`, `data`. Per-verb fields go **inside `data`**, never at the top level, and every plural verb uses `data.results[]` (PLAN.md §3.1). One golden snapshot per verb. |
| **One spelling for failure** | `FAILED`, never `FAIL`. Full enum: `READY` `UP` `DOWN` `CLEAN` `PASS` `OK` / `PARTIAL` `BLOCKED` / `FAILED` / `ABORTED` `DEAD` `TIMEOUT`. |
| **Errors are typed** | `class` ∈ {`bad_invocation`, `bad_config`, `tool_failed`, `timeout`, `aborted`, `char_bug`}, plus `where`. `next_action` is required for `bad_config`. The enum covers **every** non-zero exit — a hole is where a second, competing mapping grows back. |
| **Secrets never leave the shell** | Resolved values are injected into a child's env at spawn and never reach the core, argv, `--json`, logs or `.char/`. There is no verb that returns a secret. See `ARCHITECTURE.md` §1.8. |

### Exit codes

**One rule: the code is a function of `error.class`, or `0` when there is no error.** Terminal
state never determines it — `FAILED` is exit 3 when the config was wrong and exit 1 when the
tests were.

| | | | |
|---:|---|---:|---|
| `0` | *(no error)* | `4` | `timeout` |
| `1` | `tool_failed` | `5` | `aborted` |
| `2` | `bad_invocation` | `70` | `char_bug` |
| `3` | `bad_config` | `130` | SIGINT |
| | | `141` | SIGPIPE |

A `commands:` child's exit code passes through **verbatim** and is not remapped. char's own
codes can only occur when the child never ran; `data.dispatched` says which.

---

## Testing

**TDD is mandatory in `core/`** — failing test, minimal implementation, passing test. In
`adapters/` and `cli/`, tests land in the same PR but the order is not policed.

| Tier | What goes here |
|---|---|
| `tests/unit/` | Pure core. No I/O at all. Fake `ctx.run` and **assert on the argv** — argv is where the bugs are. |
| `tests/integration/` | Real subprocesses, real files. Process-group kill with no orphans. Concurrent claims and lease reclamation. Docker labels verified gone after `clean`. |
| `tests/e2e/` | The real CLI against scratch repos. |
| `tests/golden/` | One JSON snapshot per verb. **Regenerate by hand** — there is deliberately no update flag. |

Coverage is gated on a ratchet: it may never drop. Use `#[coverage(off)]` or a documented
exclusion, with a reason comment, for genuinely untestable lines.

---

## Workflow

**Never commit to `main`.** Work on a feature branch, always.

```sh
git switch -c <scope>/<short-description>
# ... work, committing as you go ...
no-mistakes axi run --intent "<what the user set out to accomplish>"
```

- **Commits are conventional**, scoped by module: `core`, `adapters`, `cli`, `schema`,
  `fixtures`, `docs`. Not phase numbers — they expire.
- **The GitHub Actions matrix is the authoritative gate** — nothing merges without it.
  **`no-mistakes` is the pre-flight you should use**: it runs an agent code review plus test
  and lint locally, then pushes and opens the PR. Drive its gates; do not edit files to fix
  findings while a run is active. Escalate any `ask-user` finding rather than deciding it.
- **PRs are sized for review, not per phase.** Review is the binding constraint on this
  project. `main` sitting part-way through a phase is expected and fine.
- **Tag each completed phase**: `git tag phase-3`.
- A GitHub Actions matrix (`ubuntu-latest`, `macos-latest`) runs lint, typecheck and tests
  alongside `no-mistakes`. It exists to cover the platform you are not developing on.

### Dogfooding — staged

From phase 3, charkit has its own `char.yml`.

**Through phase 6: the gate runs the raw tools** — `cargo clippy`, `cargo fmt --check`,
`cargo test`.
`tests/test_dogfood.py` runs `char check --json` and asserts it agrees. So a broken
`char check` is one failing test, not an unmergeable repository. Do not wire `char check`
into the gate itself yet.

**Once phase 6 lands, `char check` becomes the gate.** That is the end state — the interim
arrangement exists only because a bug in a half-built tool should not be able to lock its own
repo. Phase 6 is Chariot adopting charkit, so at that point a real repo is already trusting
`char check` as its merge gate. See `ARCHITECTURE.md` §2.6.

---

## Versioning

charkit stays at `0.x` and does not promise stability. Because the package version therefore
carries no compatibility signal, **`schema_version` in the `--json` payloads is the only
one** — treat it accordingly.

Bump rule: adding a field does not bump. Removing a field or changing its type does.

---

<!-- char:begin -->
<!-- This block is generated by `char agents-md --write` from the resolved config.
     It will be populated in phase 5. Anything outside these markers is never touched. -->
<!-- char:end -->
