# charkit — implementation plan

> **Status:** approved design, not started. This document is the complete specification —
> a fresh agent should be able to execute it without any prior conversation.
>
> **Binary name:** `char` · **Package name:** `charkit` (PyPI + npm, both verified free)
> **Language:** Python 3.12+ · **Platform:** POSIX only (macOS/Linux). Not Windows.

---

## 0. Start here

**Do not write any code yet. Phase 0 is a working session with the human.**

Read §2 (concepts), §4 (config) and §8 (phases) first — you need the shape of the thing to
have a useful conversation about how to build it. Then work through **Phase 0 — Foundations**
(§8) together:

1. Walk the human through the recommended architecture principles (0.1) and SDLC principles
   (0.2). Your job is to explain the *reasoning* and invite disagreement, not to collect
   agreement. A principle nobody argued with is usually one nobody read.
2. Get an answer to every question in 0.3. Those have no defensible default.
3. Write `docs/ARCHITECTURE.md`, `AGENTS.md`, and the README's contributing section.
   **Keep the rationale, not just the rules** — a rule without its reason gets discarded the
   first time it is inconvenient.
4. Stop. Phase 1 is a separate session.

Only then does building start — and phase 1 is still not the CLI. It is the repo skeleton
plus six fixture configs and their schema. Expect the schema to change while writing them;
that is the phase working, not a setback. Record which fixture forced which change, because
that record is the justification for keeping the fixture.

Four rules that hold for the whole project:

- **Phase 0 produces documents, not code.** If a source file appears, the phase went wrong.
- **Phase 1 must land alone.** Every later phase codes against the config contract it
  establishes. Parallel agents cannot share a decision that has not been made yet — they will
  each invent an answer and you will get three incompatible ones.
- **Phase 2 is the only phase permitted to read the Chariot repo** (§9). If any other phase
  feels like it needs to look, the plan is underspecified — fix the plan, don't peek.
- **Do not relitigate §10.** Those decisions were made deliberately, with rationale recorded.

---

## 1. What this is

A CLI that gives coding agents one consistent vocabulary for managing a repo's tech stack,
so an agent working across several repos never has to re-derive how to start, check, or
clean up any of them.

It exists because five things go wrong in every repo, every day:

| # | Verb | The failure today |
|---|------|-------------------|
| 1 | Start the apps | Tilt here, `docker compose` there, a Procfile elsewhere. The agent guesses. |
| 2 | Run linters | Which tool, which scope, which fix flag, from which directory. |
| 3 | Run all tests | Including the ones needing services up, a browser, and a free port. |
| 4 | **Clean up after itself** | Nothing knows what a run created, so nothing can reliably destroy it. |
| 5 | **Initialize the code** | At the repo root *and* in a fresh worktree — deps, env, DB, non-colliding ports. |

**4 and 5 are the same bug**: you cannot clean up what you never claimed, and claiming
happens at init. That observation is the entire design.

### Evidence this is a real problem

From the source repo (`~/Development/chariot`), `scripts/char/worktrees.py:110` exists
because 29 leftover per-worktree Docker networks exhausted Docker's default bridge address
pool and broke Postgres startup for every subsequently allocated worktree — *"accumulated
exactly because nothing ever called this."* That is failure mode #4, already paid for once.

---

## 2. Core concepts

### 2.1 Workspace

**A workspace is one directory tree containing a `char.yml`, which gets its own runtime
state.** In practice: a checkout.

| Shape | Workspaces | Why |
|-------|-----------|-----|
| A repo, cloned once | 1 | One config, one port block, one `.char/` |
| A repo + 4 git worktrees | **5** | **The case that matters.** Same committed `char.yml`, five ids, five non-overlapping port blocks, five independent lifecycles. This is what lets five agents run concurrently on one machine. |
| A monorepo with 8 packages | 1 | Packages are *components* inside the workspace, not workspaces |
| Two separate `git clone`s | 2 | Separate `.git`, genuinely independent |

Do **not** rename this concept. "Workspace" already means roughly this in VS Code,
Terraform, cargo and pnpm, so an agent arrives knowing it. Inventing vocabulary works
directly against the project's thesis that an agent learns this once. (If the overload with
pnpm/npm workspaces ever genuinely bites, the fix is `checkout`, not an invented word.)

### 2.2 Two derived identities

```python
workspace_id = sha1(realpath(workspace_root)).hexdigest()[:8]
project_id   = sha1(git rev-parse --git-common-dir).hexdigest()[:8]
```

- **workspace id** — owns ports, containers, networks, processes, locks. One per checkout.
- **project id** — owns nothing. Purely the grouping key: every worktree shares one
  `--git-common-dir` with the checkout it came from.

Both are *derived*, never stored as truth, so they survive a deleted `.char/` and can be
recomputed by anything. `realpath` matters — symlinked checkouts must not get two identities.

**Workspaces in a project are siblings, not parent and children.** The root checkout is
just another workspace with no authority over the worktrees. This is load-bearing: model it
as a hierarchy and `char clean` in the root implies cascading into the worktrees, killing
services another agent is actively using. Flat siblings plus an explicit `--project` flag
makes the destructive step something you have to ask for.

### 2.3 Ownership

Every port, container, network and process char creates is stamped with the workspace id.
That single fact is what makes `clean` correct, and it is the highest-value primitive in the
project.

- Containers/networks: label `char.workspace=<id>`
- Processes: tracked pid, spawned in their own process group (`start_new_session=True`),
  killed with `os.killpg`
- Ports: claimed blocks in `~/.char/registry.json`, released on `clean`, reaped when the
  workspace path stops existing

---

## 3. The verb surface

Six verbs, identical in every repo. This is the entire surface an agent memorizes;
everything else is config. **Every verb takes `--json`.**

| Verb | Contract | Terminal states |
|------|----------|-----------------|
| `char init` | Workspace ready: run each component's setup, claim a port block, write `.char/`. Idempotent. | `READY` `FAILED` |
| `char up` | Services running and ready-checked. Records what it started into `owned.json`. | `UP` `FAILED` `TIMEOUT` |
| `char down` | Services stopped. Port block **kept** — still your workspace. | `DOWN` |
| `char check` | Lint / format / test. Scoped, scheduled, locked, ceilinged. `--detach` / `--status` / `--wait`. | `PASS` `FAIL` `ABORTED` `DEAD` `TIMEOUT` |
| `char clean` | Release everything this workspace owns. | `CLEAN` |
| `char status` | What's running, what's mine, what's stale, what a run is doing now. | informational |

Plus: `char config verify`, `char agents-md [--write|--check]`.

### 3.1 Scope lens

`status` and `clean` are the two verbs where "just me" isn't always right. Same flag on both.

| Scope | Covers | Answers |
|-------|--------|---------|
| *(no flag)* | this checkout | "Are my services up? Is a run in flight? What ports do I hold?" |
| `--project` | every workspace sharing this `--git-common-dir` | "What's going on across everything I have open on this repo?" — the orchestrating agent's view |
| `--all` | every workspace on the machine | "What is char holding anywhere?" |

`--orphaned` is a separate, always-safe filter that composes with any scope: it only touches
workspaces whose directory no longer exists, so it can never disturb a live agent.
`--project` on `clean` **will** stop other worktrees' services — which is exactly why it is
not the default.

---

## 4. Configuration

### 4.1 `char.yml` — committed

**One `components:` list.** A component is a named thing that may have source to check
(`checks:`), a process to run (`run:`), or both. Do not split these into separate `units:`
and `services:` blocks — they are two *axes*, not two kinds of thing, and splitting them
makes the both-axes case (an API server) read as duplication.

```yaml
version: 1

components:

  # runs only — not your source
  postgres:
    run:
      driver: compose
      file: docker-compose.yml
      ports: { pg: 5432 }        # remapped into this workspace's block

  # BOTH axes
  api:
    root: backend                # may point outside the workspace root (reserved, see §7)
    match: ["backend/**"]        # scoping by changed files
    setup: uv sync               # what `char init` runs
    run:
      driver: command
      cmd: manage.py runserver 0.0.0.0:${port.api}
      ports: { api: 8000 }
      ready: { http: "http://127.0.0.1:${port.api}/healthz" }
      needs: [postgres]
    checks:
      lint:
        cmd: ruff check ${files}
        fix: ruff check --fix ${files}
        timeout: 120
      types: { cmd: mypy . }
      test:
        cmd: pytest ${files}
        timeout: 600
        cost: 4                  # CPU slots
        needs: [postgres]

  # checks only — a library, never runs
  web:
    root: web
    match: ["web/**"]
    setup: pnpm install --frozen-lockfile
    checks:
      lint:  { cmd: pnpm eslint ${files}, fix: pnpm eslint --fix ${files} }
      types: { cmd: pnpm typecheck }
      test:  { cmd: pnpm vitest run, cost: 2 }
      e2e:
        cmd: pnpm e2e
        scope: component         # never file-scoped
        timeout: 900
        cost: 4
        exclusive: [browser]     # named resource, never shared
        needs: []                # boots its own servers — see §4.4
```

Check ids are **derived** as `<component>:<check>` — `api:lint`, `web:e2e`. Never written by
hand, so they cannot drift, collide, or be typo'd. Selectors that fall out for free:
`char check web:e2e`, `char check --component web`, `char check lint`.

`char up` starts every component with a `run:`. `char check` runs every component with
`checks:`.

### 4.2 `.char/` — gitignored, per-workspace runtime state

```
.char/
  workspace.json   id, project id, abs path, port block, created_at
  owned.json       container ids, networks, pids, ports
  run/<run-id>/
    lock           pid + heartbeat mtime
    state.json     per-check status, verdict
    logs/<component>.<check>.log
```

### 4.3 `~/.char/registry.json` — machine-global

The only cross-workspace state.

```json
{ "a3f91c02": {
    "path":       "/repo/.claude/worktrees/feature-x",
    "project":    "7c21ab90",
    "ports":      [5460, 5469],
    "claimed_at": "2026-08-07T14:02:11Z" } }
```

The `project` field is the whole implementation of `--project`: filter the registry by it,
then read each workspace's `owned.json`. Writes go through an `O_EXCL` lockfile; claims are
idempotent by workspace id.

### 4.4 Templating: exactly three substitutions, hard cap

`${port.NAME}`, `${files}`, `${component.root}`. **No conditionals, no loops, no expression
language.**

The reason is not parser cost — it is that requests arrive one at a time, each individually
reasonable, with no natural stopping point: `${port.api}` → `${env.CI ?? 0}` → `{{#if}}` →
`{{#each}}` → a language with no debugger, no types, and no stack traces, whose bugs are
yours to diagnose from a YAML file at the exact moment an agent is blocked.

**Escape hatch for repos that genuinely need more:** write a generator script that *emits*
`char.yml`, committed and diffable. `char config verify --check` can then assert the
generated file is in sync. This is deliberately the same pattern as cdktf → Terraform JSON.

> **Considered and rejected: a Tiltfile-style Starlark config.** It is the strongest
> objection to the above — jumping straight to a real evaluator means you never *invent*
> conditionals, you inherit them, so there is no slope to slip down. It loses on one
> specific ground, and it is the ground this project stands on: the primary author and
> reader of this file is an agent. YAML can be schema-constrained on write and parsed on
> read; Starlark must be *executed* to know what it means, which means `char config verify`
> would have to run untrusted repo code — killing layer 3 of the bootstrap sandwich (§5).

---

## 5. Bootstrap: the three-layer sandwich

**Do not write a stack-detection engine.** Do not infer intent. The split:

| Layer | Who | Produces |
|-------|-----|----------|
| **1. Deterministic scan** | char (`char init`) | An **evidence report**, never a config |
| **2. Authoring** | the agent | The `char.yml`, from evidence + schema + a worked example |
| **3. Deterministic verify** | char (`char config verify`) | Pass/fail with fix suggestions |

Layer 1 is safe precisely because it reports **facts, never intent**. "These 14 scripts
exist in package.json" cannot be wrong; "your test command is `pnpm test`" can. It emits:

- lockfiles and package managers found
- every `package.json` script, verbatim
- `pyproject` tool sections, `Makefile` targets
- compose services and their declared ports
- CI workflow steps — the best existing evidence of "what we actually run"
- workspace globs / monorepo layout

Layer 2 supplies what no scan can: which of four test scripts is canonical, which suite is
slow enough to deserve `cost: 4`, which two cannot share a browser, what genuinely needs
Postgres.

**Layer 3 is load-bearing.** Agents *will* hallucinate config — a plausible script name that
does not exist, a flag from a different version. `config verify` catches it in seconds
instead of on the first real run, in a fresh worktree, at the worst moment. It checks:

- schema validation
- **dry-invokes every `cmd` and `fix`** (`--help` / `--version` / `--dry-run`)
- `needs:` refs resolve to real components
- `exclusive:` names used more than once (a lone name is a typo)
- declared ports fit the block
- every `match:` glob hits at least one file

### 5.1 `char agents-md`

Writes a managed block into `AGENTS.md`, generated from the *resolved* config so it lists
real component and check names.

- `--write` rewrites only between `<!-- char:begin -->` / `<!-- char:end -->`; anything
  outside is untouched. No markers → appends once, at the end.
- `--check` exits non-zero if the block is stale, so it can be a `custom` check in
  `char.yml`.
- Bare invocation prints to stdout, for repos that do not want a managed block.

---

## 6. Service drivers

**Two drivers only. No vendor-named drivers — no `tilt`, no `bazel`, no `make`.**

| Driver | Behavior |
|--------|----------|
| `compose` | Shells out to `docker compose` with a project name derived from the workspace id, port mappings rewritten into the claimed block, `--label char.workspace=<id>` so `clean` finds everything. |
| `command` | Spawns detached in its own process group, records the pid, waits on the ready-check, kills the whole group on `down`. Covers Tilt, `pnpm dev`, `manage.py runserver`, a Procfile line — anything. |

Tilt is just a long-running command with a ready-check:

```yaml
components:
  stack:
    run:
      driver: command
      cmd: tilt up --stream
      ready: { http: "http://localhost:10350/" }
      stop: tilt down
```

Ready-check kinds: `http`, `tcp`, `log` (regex on stdout), `exec` (command exits 0), `none`.
Each with its own timeout.

**Supervision depth: start-and-track only.** Spawn, ready-check, record pid, kill the group
on `down`. Logs go to files. If a service crashes, `char status` reports it dead — char does
**not** restart it. No log aggregation, no `char logs -f`, no restart-on-crash.

### 6.1 `owns:` — the extension point instead of a plugin API

Do **not** build a driver plugin system. It means a public lifecycle contract, error
semantics, and a versioning story — permanent API surface for a third driver that may never
arrive. `driver: command` already *is* the plugin system.

The one thing a first-class driver gives you that a bare command cannot is knowing what it
created, so `clean` can reclaim it. So let a `command` component declare that directly:

```yaml
components:
  cluster:
    run:
      driver: command
      cmd: ./scripts/kind-up.sh ${port.api}
      stop: ./scripts/kind-down.sh
      ready: { exec: "kubectl get ns example" }
      owns:
        containers: "label=io.x-k8s.kind.cluster=char-${workspace.id}"
        ports: [api]
        files: [".kube/char-${workspace.id}.conf"]
```

~60 lines instead of a plugin API, no versioned contract, and `clean` stays correct for
resources char never created directly. If a third real driver ever proves necessary, `owns:`
is the interface you would have designed anyway.

---

## 7. Non-goals

Each of these is a plausible-sounding feature that multiplies maintenance without moving any
of the five verbs.

- **Inferring intent from a repo scan.** Layer 1 reports facts only.
- **Task dependency DAG / build caching.** turbo and nx own this. Six verbs need no build graph.
- **A driver plugin system.** See §6.1.
- **Mandatory output parsing.** Optional `parse:` keys only. Exit code plus captured stream
  must *always* be a complete answer, or every upstream tool release breaks you.
- **A growing MCP surface.** One thin wrapper over the same importable layer. The CLI with
  `--json` works in harnesses with no project-scoped MCP at all.
- **Windows support.** Process groups, signals and file locks are load-bearing. Say
  POSIX-only in the README's first paragraph.
- **Multi-repo workspaces — reserved, not built.** `components[].root` is already a path;
  letting it point outside the workspace root (`root: ../api`) would get ~80% of multi-repo
  for free. Costs: a git diff *per* root rather than one, and the id can no longer be "hash
  of one path". Worth ~2 days, only once the two-repo setup actually exists.

---

## 8. Phases

Built **greenfield in its own repo**, not extracted incrementally through Chariot.

### 8.1 Why greenfield, and the anti-contamination strategy

The goal is that charkit carries no Chariot assumptions. Two distinct risks, and they need
different defenses:

| Contamination | Looks like | Defense |
|---|---|---|
| **Crude** | A hardcoded `backend/` path, a `tilt` import, a `.claude/worktrees` assumption | Greenfield makes it *structurally impossible* — the agent cannot see Chariot except in phase 2. Backed by a grep gate. |
| **Subtle** | An abstraction shaped around Django+Next because that is the only repo the agent ever saw | **Six fixture configs in phase 1.** Isolation does nothing here — an agent given one example generalizes from n=1 regardless of repo topology. |

**The fixture set is the more important of the two and is non-optional.** Write all six
before any code exists. An agent cannot overfit to Django+Next if the schema must also
express five other shapes on day one.

**The constraint that makes a fixture set useful:** every fixture must be able to fail the
schema in a way no other fixture can. Five Node monorepos teach nothing — they all pass or
all fail together. If adding a fixture creates no new way to be wrong, it is decoration.

| Fixture | Axis it owns | Failure it catches that nothing else does |
|---|---|---|
| `django-next` *(real)* | Maximal case — polyglot monorepo, supervisor, checks running *inside* containers, 3s→15min cost spread | Schema can't express a real complex repo |
| `multi-lang` *(real)* | The second repo — a genuinely different runtime pairing | Abstraction is Django/Next-shaped |
| `go-service` | Low end — one component, one binary, one Postgres, no monorepo | **Over-structuring.** A trivial repo needing 40 lines of config |
| `pnpm-monorepo` | Many components, **zero** services, turbo already present | Component-per-package globbing; also honestly answers "is char redundant where turbo exists?" |
| `rails-monolith` | `setup:` as a *sequence* (bundle → db:create → migrate → seed); two services with real dependency ordering | `setup:` modeled as a single string; `needs:` ordering that only works for one service |
| `python-ml` | No web services, **no ports at all**, a 20-minute check, GPU as a non-port exclusive resource | Port machinery that doesn't gracefully no-op; `exclusive:` that assumes "a port" |

**Cost is low because fixtures are configs, not checkouts.** You don't need a Rails app — you
need a plausible `char.yml` for one plus a golden resolved snapshot. An afternoon for all six.

**Evidentiary weight differs, though.** The first two are real and verifiable against actual
repos. The other four are representative and prove *schema shape only* — a hypothetical
config cannot surface a runtime surprise. That is fine for what they are for (catching an
abstraction that fits only one repo), but six green fixtures must not be read as "validated
against six repos."

Greenfield was chosen over extract-through-Chariot for two reasons beyond contamination:

1. **Structural guarantees beat policed ones.** "The agent cannot see Chariot" is stronger
   than "the agent is told not to look."
2. **Extract-later has a specific, historically common failure mode.** Once phases 2–4 land
   inside Chariot the daily pain is gone, nothing forces the split, and it quietly never
   happens. Greenfield forces the code to be extractable because it has nowhere else to live.

**What greenfield gives up, and how to buy it back:** continuous validation against a real
repo. Fixtures catch config-model failures but not runtime ones — you would not discover
"the scheduler deadlocks when two exclusive resources overlap under load" until phase 7.

> **Read-only parallel run, from phase 2 onward.** Point charkit at the Chariot checkout,
> run `char check`, and diff the verdicts against `scripts/char`'s output. This is *not* a
> Chariot dependency and *not* a Chariot PR — zero risk to Chariot's merge gate — but it
> restores most of the continuous validation and turns phase 7 from a cliff into a
> formality. Do this at the end of every phase from 3 on.

### Phase 0 — Foundations *(human + agent, working session, no code)*

**Output:** `docs/ARCHITECTURE.md`, `AGENTS.md`, and a `CONTRIBUTING`-style README section.
Nothing else. This is a conversation that produces documents, not a build step.

**Why this is first, and not skipped:** it is the third anti-contamination defense, and it
catches what the other two miss. Chariot's `check.py` has a *structure* — 1,632 lines of it.
Without stated architecture principles, phase 2's port inherits that structure by default,
because "make it work like it did" is the path of least resistance. Deciding the target shape
first turns the port from a copy into **a rewrite into a known architecture**, and gives the
reviewer an objective standard to reject against.

#### 0.1 Architecture principles — recommended; confirm or override

| # | Principle | Why it earns its place |
|---|---|---|
| 1 | **Every outside-world interaction sits behind an injected seam** — subprocess, filesystem, docker, git, clock, network | This is why Chariot's 2,694 test lines run hermetically with no mocking framework: `run_fn` is *passed in*, not imported. It is the load-bearing pattern in the existing code, the one thing worth copying wholesale, and the same instinct as Chariot's own adapter-first rule. |
| 2 | **Pure core, imperative shell** | Config resolution, scope computation, scheduling, verdict aggregation = pure functions over data. Spawning, writing, labeling live at the edge. Most tests then need no fixture at all. |
| 3 | **The CLI is a thin wrapper over an importable library** | Already forced by the MCP server sharing the logic layer — but state it, because the failure mode is logic quietly accumulating in command functions. Every command: parse args → call library → render. |
| 4 | **No ambient state** | Workspace is resolved once and passed explicitly, never read from a global or inferred mid-call. Chariot's `--target` threading is the precedent, and it is what makes `--project` / `--all` scoping tractable. |
| 5 | **Dependencies point one way: core → adapters** | An adapter may never import the core's decision logic. Enforceable with a lint rule; worth doing. |
| 6 | **Every verb answers in a machine-readable shape** | `--json` is not an afterthought on some subset. The renderer is the only thing differing between human and agent output. |

#### 0.2 SDLC principles — recommended

| # | Principle | Note |
|---|---|---|
| 1 | **TDD throughout** — failing test → minimal implementation → passing test | Non-negotiable given this tool becomes a merge gate |
| 2 | **Branch + PR per phase, never commit to main** | Chariot enforces this with a `PreToolUse` hook — port the hook on day one rather than relying on discipline |
| 3 | **Conventional commits** | Matches Chariot's existing history (`build(family):`, `char check:`) |
| 4 | **Merge gate = lint + typecheck + tests + the contamination grep** | The grep is the phase-2 acceptance test made permanent |
| 5 | **Semver from the first publish, with a changelog** | Cheap now, painful to retrofit once anything depends on it |
| 6 | **Dogfood from phase 3 onward** | charkit gets its own `char.yml` and gates itself with itself the moment `char check` runs. Strongest available forcing function, and it makes the README example real rather than illustrative. |

#### 0.3 Decisions genuinely yours — bring answers to the session

| Question | Options | Consideration |
|---|---|---|
| **Public or private repo?** | public / private-for-now | Changes whether CI is free, whether the license matters, how much README polish is warranted. Private → public later is easy; the reverse is not. |
| **License, if public** | Apache-2.0 / MIT / none | Apache-2.0 adds a patent grant and clears corporate legal at no adoption cost. Only matters if public. |
| **CI** | GitHub Actions / local gate only / both | Actions is free for public repos, and unlike Chariot there is no billing constraint. Real CI matters more for a package other repos depend on. |
| **Typing strictness** | mypy strict / basic / none | Strict from commit one is cheap; retrofitting onto 3,000 lines is not. |
| **Python floor** | 3.12 / 3.11 | 3.12 matches Chariot. Lower only if a target repo's environment forces it. |
| **Test layers** | unit only / unit + a real-subprocess integration tier | Principle 1 makes unit tests hermetic — which means **nothing exercises real process-group kill** unless you deliberately add a small integration tier. Recommend adding one: it covers the exact failure char exists to prevent. |
| **Coverage** | gated / report-only | Chariot runs report-only; same is probably right here. |

#### 0.4 Done when

`docs/ARCHITECTURE.md` states principles 0.1 and 0.2 **with the rationale kept, not just the
rules**; `AGENTS.md` tells a future agent how to work in the repo; every question in 0.3 has
a recorded answer. **No source files exist yet.**

---

### Phase 1 — Repo skeleton + **six** config fixtures *(must land alone)*

uv package scaffolding, pytest, ruff. JSON Schema for `char.yml`. Then write all six configs
from the table in §8.1 under `tests/fixtures/<name>/char.yml`. Tests are schema validation
plus a golden resolved-config snapshot for each. **No runtime.**

**Done when:** all six are expressible with no escape hatches and no fields invented on the
spot.

**Expect the schema to change while writing them — that is the phase working.** If
`rails-monolith` needs `setup:` to be a list and `django-next` didn't, add it now, before any
code depends on the narrower shape. The fixtures that force a change are the ones earning
their keep; note which ones did, because that record is the argument for keeping them.

**Why alone:** every later agent codes against this contract. Parallel agents cannot share a
decision that has not been made yet — they will each invent an answer and you will get three
incompatible ones.

### Phase 2 — Port the check engine, generalized

Copy `check.py` and `_shared.py` from `~/Development/chariot/scripts/char/` — same language,
so this is a real copy, not a rewrite. Replace `CHECK_CATALOG` with the config loader,
`domain` with `component`, strip every Chariot-specific path, turbo filter, and
`uv run --directory backend` assumption. Port `test_check.py` alongside.

**Done when:** the ported suite is green against the phase-1 fixture, **and**
`grep -riE "chariot|tilt|NEXT_PUBLIC|\.claude" src/` returns nothing.

### Phase 3 — Ownership core: `init`, `clean`, `status`

Workspace id, project id, `.char/`, `~/.char/registry.json` with `O_EXCL` claiming, resource
labeling, the process-group spawn/kill wrapper, and the scope lens.

**Done when:** two directories claim non-overlapping blocks concurrently;
`char status --project` from either reports both; deleting one and running
`char clean --orphaned` releases its block without disturbing the live one.

**Start the read-only parallel run here** (§8.1) and repeat it at the end of every
subsequent phase.

### Phase 4 — Services: `up` / `down`

Both drivers, five ready-check kinds, `needs:` ordering, `owns:`, everything started
recorded into `owned.json`, port remapping into the claimed block.

**Done when:** a scratch repo with a bare `docker-compose.yml` plus a long-running command
comes up, gets ready-checked, and tears down completely — `docker ps` and `lsof` clean
afterwards.

### Phase 5 — Bootstrap sandwich + `agents-md` + MCP *(fans out widest)*

The layer-1 evidence scanner is a dozen independent parsers — the most parallelizable work
in the plan, one agent each. Plus schema/example emission, `config verify`, the managed
AGENTS.md block, and the MCP server.

**Done when:** an agent given only "set up char in this repo" produces a verifying config in
a repo it has never seen.

### Phase 6 — Publish

PyPI as `charkit` with `bin: char`. The ~40-line installer:

```sh
command -v uv >/dev/null || curl -LsSf https://astral.sh/uv/install.sh | sh
uv tool install charkit     # provisions a Python if the machine has none
```

No bundling needed — uv installs Python itself, so this is dependency-free from the user's
side at ~2 MB rather than a 15–40 MB PyInstaller binary with macOS signing overhead.
Homebrew tap is a nice second channel, later.

**Done when:** a clean machine with no Python runs the one-liner and gets a working `char`.

### Phase 7 — Chariot adopts it

A Chariot PR: delete `scripts/char/check.py` and `servers.py`, take the dependency, keep
`worktrees` / `tickets` / `design` as repo-local commands via a `commands:` block, repoint
`bin/char`.

**Done when:** `char check --all` is green in Chariot and the worktree flow still works end
to end.

**Budget extra time here** — seven phases of drift surface in this one PR.

### Phase 8 — The only test that matters

Adopt char in a repo that is *not* Django + Next (a multi-language repo), using only a
`char.yml` authored by an agent through the phase-5 sandwich.

**Pass/fail:** if it needs a change to char's own code, the abstraction is wrong.

---

## 9. Source material

The reference implementation lives at `~/Development/chariot/scripts/`:

| Path | Lines | Role in this plan |
|------|------:|-------------------|
| `char/check.py` | 1,632 | Copy in phase 2. Scope → schedule → run → parse → report, run lock, live table, `--again`. Contains `CHECK_CATALOG` (replace) and load-bearing comments about two Playwright traps (translate into the fixture config, not the code). |
| `char/_shared.py` | 140 | Copy in phase 2. `run_fn` injection, target resolution, git worktree list. |
| `char/worktrees.py` | 397 | Reference for phase 3. Orphan container/network sweep — note it infers ownership from compose's `working_dir` label; charkit stamps its own instead. |
| `char/servers.py` | 321 | Reference for phase 4. Tilt-shaped; becomes config, not code. |
| `char/__main__.py` | 345 | Reference. Typer dispatch pattern. |
| `char_mcp/server.py` | ~95 | Reference for phase 5. |
| `char_test/` | 2,694 | **Copy in phase 2.** `run_fn`-injected, asserts on behavior not implementation — this is the single most valuable asset. Only check-id fixtures should need editing. |
| `bin/char` | ~25 | Copy the pattern. Resolves the git root from the *caller's* cwd at every invocation, which is why one symlink works from inside any worktree. |

---

## 10. Decisions already made — do not relitigate

| Decision | Choice | Why |
|----------|--------|-----|
| Language | **Python** | A `curl \| sh` installer gives the same "one line, fresh machine" property `npx -y` has. The language was never the requirement. Keeps 5,649 working lines and 2,694 executable test lines. |
| Package name | **`charkit`** | `char` is taken on both PyPI and npm. Binary stays `char`; the package name appears once, in the bootstrap line. |
| Distribution | **PyPI + `install.sh`** | uv provisions Python, so no bundling. Homebrew later. |
| Supervision | **Start-and-track only** | Restart-on-crash and log aggregation are a permanent bug class for marginal gain. |
| Config shape | **One `components:` list** | `units` + `services` were the same thing split in two; the both-axes case read as duplication. |
| Config format | **YAML, statically verifiable** | Generator script is the escape hatch. Starlark would force `config verify` to execute untrusted code. |
| Driver extensibility | **`owns:`, not a plugin API** | Gets the one real benefit of a custom driver at ~60 lines. |
| Concept naming | **Keep "workspace"** | Already means this in VS Code / Terraform / cargo / pnpm. Do not invent vocabulary for concepts that already have names. |
| Build order | **Greenfield repo, Chariot last** | Isolation requested; keeps Chariot's merge gate out of the blast radius. |

---

## 11. Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Overfitting to Chariot** — the abstraction gets shaped around Django+Next because it is the only repo the agent has seen. Isolation does *not* prevent this. | **High** | **Six fixture configs in phase 1** (§8.1). This is the single most important guard in the plan. |
| **Seven phases of drift surface in phase 7.** Isolation removes continuous real-repo validation. | High | Read-only parallel run against Chariot from phase 3 onward (§8.1). Budget extra time for phase 7 regardless. |
| **Crude contamination** — a Chariot path or import follows the code in during phase 2. | Med | Phase-2 acceptance test is a literal `grep -riE "chariot\|tilt\|NEXT_PUBLIC\|\.claude\|backend/\|web/" src/` returning nothing. **Phase 2 is the only phase with Chariot access.** |
| **Config expressiveness pressure** once a second repo lands. | Med | Three substitutions, hard cap. Escape hatch is a generator script. |
| **Registry corruption** with two agents claiming simultaneously. | Med | `O_EXCL` lockfile; claims idempotent by workspace id. |
| **`curl \| sh` is a trust ask** and some environments block it. | Low | `uvx` and `pipx` cover anyone who will not run it. Publish the script's source in-repo. |

---

## 12. Notes for the implementing agent

- **Phase 0 produces documents only.** Phase 1 lands alone and ships all **six** fixtures.
  Do not fan out until the config contract is committed.
- **Phase 2 is the only phase with access to the Chariot repo.** Every other phase works from
  this document and the fixtures. If a later phase feels like it needs to look at Chariot,
  that is a signal the plan is underspecified — fix the plan, don't peek.
- Phases 3 and 4 parallelize moderately; **phase 5's evidence scanner is the widest fan-out**
  (one agent per parser).
- What does *not* compress with more agents: live verification (containers start at the speed
  they start) and human review of each PR. Review is the binding constraint.
- Every phase is a normal branch + PR. TDD throughout: failing test → minimal implementation
  → passing test.
- The check engine's tests use injected `run_fn` rather than real subprocesses — preserve
  that pattern; it is why the suite is fast and hermetic.
