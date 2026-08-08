# charkit — implementation plan

> **Status:** Phase 0 complete; phase 1 not started. This document is the complete
> specification — a fresh agent should be able to execute it without any prior conversation.
>
> **§0.1 and §0.2 are superseded by [`ARCHITECTURE.md`](ARCHITECTURE.md)**, which records what
> was actually decided. Everything else here stands.
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
- **Only phase 2's *harvester* may read the Chariot repo** (§9). If any other phase
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
| A monorepo with 8 packages | 1 | Packages are *components* inside the workspace, not workspaces. **This is the default and should stay the default** — reach for §4.6 only when packages are genuinely separate products |
| A monorepo declaring nested workspaces (§4.6) | 1 + one per declaration | The exception: `apps/foo` and `apps/bar` are separate products that happen to share a repo and need independent lifecycles |
| Two separate `git clone`s | 2 | Separate `.git`, genuinely independent |

**How the workspace root is found.** Every verb resolves it the same way, and the answer must
be identical from anywhere inside the tree, because `workspace_id` is a hash of it:

> Walk up from the caller's cwd to the git root, collecting **every** `char.yml` found.
>
> - **Exactly one** → that directory is the workspace root.
> - **Zero** → `bad_config`, naming the directories searched.
> - **Two or more** → `bad_config`, *unless* the outer one declares the inner in
>   `workspaces:` (§4.6). If it does, the innermost wins.

Anchoring on `char.yml` rather than always the git root, because the two differ in exactly
the cases that matter: in a monorepo a package may sit far below the root, and the git root
of a worktree is the worktree itself. One rule covers both. Stopping at the git root keeps a
stray `char.yml` in a parent directory from capturing an unrelated repo — and means a git
submodule, which has its own git root, is correctly its own workspace for free.

Collecting *all* of them rather than taking the nearest is what makes an accidental nested
`char.yml` fail loudly instead of silently creating a second owner for the same source. The
walk is bounded by directory depth, so it costs nothing.

Do **not** rename this concept. "Workspace" already means roughly this in VS Code,
Terraform, cargo and pnpm, so an agent arrives knowing it. Inventing vocabulary works
directly against the project's thesis that an agent learns this once. (If the overload with
pnpm/npm workspaces ever genuinely bites, the fix is `checkout`, not an invented word.)

### 2.2 Two derived identities

```python
workspace_id = sha1(realpath(workspace_root)).hexdigest()[:8]
project_id   = sha1(realpath(git rev-parse --git-common-dir)).hexdigest()[:8]
```

> **`realpath` on the second line is load-bearing and was missing from an earlier draft.**
> `git rev-parse --git-common-dir` returns a path *relative to cwd* — `.git` from the repo
> root, `../.git` from a subdirectory — so hashing it directly yields a different project id
> depending on where the command was run. That silently breaks `--project` scoping, the
> registry's project filter, and the guarantee that worktrees group with their parent. Verify
> this behaviour before changing the line; it is not obvious from the command's name.

Known and accepted: because the id derives from a path inside the parent checkout, moving or
deleting that checkout regroups every surviving worktree. It only affects the grouping key,
which owns nothing (below), and it is recoverable by recomputation.

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

Every port, container, network, **image** and process char creates is stamped with the
workspace id. That single fact is what makes `clean` correct, and it is the highest-value
primitive in the project.

- Containers/networks/images: label `char.workspace=<id>`
- Processes: tracked pid, spawned in their own process group (`start_new_session=True`),
  killed with `os.killpg`
- Ports: claimed blocks in `~/.char/registry.json`, released on `clean`, reaped when the
  workspace path stops existing

**Images are here because leaving them out makes `clean` wrong at the largest scale.** The
source repo already sweeps orphaned images and records roughly 2.1 GB per production app
build — the single biggest thing a stale workspace holds.

**But only images char causes to be *built*.** A pulled image such as `postgres:16` is shared
with everything else on the machine and was never char's to remove. Built images are stamped
through `build.labels` in the compose document char generates (§6.0). An earlier draft said
stamping meant "passing the label through to compose" — `docker compose` has no `--label`
flag, so that was wrong; the label reaches the image through the generated document instead.

### 2.4 What every child process inherits

char sets two variables in the environment of every process it spawns — services, checks and
`commands:` entries alike. Neither is declared anywhere; both are always present:

```
CHAR_WORKSPACE=a3f91c02       this workspace's id
CHAR_RUN_ID=<run-id>          the run this process belongs to, when inside one
```

`CHAR_RUN_ID` exists so a nested invocation can *join* the outer run rather than starting a
second one — a child that finds it set knows it is already inside a run and inherits its
lock rather than contending for it. The source repo already does exactly this with
`CHAR_CHECK_RUN_ID`, including reading it back to detect nesting, so this is a confirmed
requirement rather than a guess.

Automatic rather than a substitution: it needs no declaration, nothing to typo, and it works
for a script char has never been told anything about.

---

## 3. The verb surface

Six verbs, identical in every repo. This is the entire surface an agent memorizes;
everything else is config. **Every verb takes `--json`.**

| Verb | Contract | Terminal states |
|------|----------|-----------------|
| `char init` | Workspace ready: run each component's setup, claim a port block, write `.char/`. Idempotent. | `READY` `FAILED` |
| `char up` | Services running and ready-checked. Records what it started into `owned.json`. | `UP` `FAILED` `TIMEOUT` |
| `char down` | Services stopped. Port block **kept** — still your workspace. | `DOWN` |
| `char check` | Lint / format / test. Scoped, scheduled, locked, ceilinged. `--detach` / `--status` / `--wait`. | `PASS` `FAILED` `ABORTED` `DEAD` `TIMEOUT` |
| `char clean` | Release everything this workspace owns, including `.char/`. | `CLEAN` |
| `char status` | What's running, what's mine, what's stale, what a run is doing now. | `OK` |

Plus: `char config verify`, `char agents-md [--write|--check]`, and any repo-local verbs the
repo declares in `commands:` (§4.5) — which char dispatches but does not define.

**One spelling for failure: `FAILED`.** An earlier draft used `FAIL` for `check` and `FAILED`
for `init` / `up` — two tokens for one idea, in the one place the project claims six verbs
behave identically. The complete enum:

```
READY  UP  DOWN  CLEAN  PASS  OK          success
FAILED                                     did not achieve its goal
ABORTED  DEAD  TIMEOUT                     did not finish
```

### 3.1 The `--json` envelope

**Fixed in phase 1, alongside the config contract, and for the same reason.** Four things
consume it — the MCP server (phase 5), the dogfood test (phase 3), agents, and the golden
snapshots — and none of them can invent it independently without the three incompatible
answers §8 warns about.

```json
{ "schema_version": 1,
  "verb":           "check",
  "workspace":      "a3f91c02",
  "status":         "FAILED",
  "error":          null,
  "data":           { "runs": [] } }
```

| Field | Meaning |
|---|---|
| `schema_version` | One global version for the whole CLI contract. Adding a field does not bump; removing one or changing its type does. |
| `verb` | Which verb produced this |
| `workspace` | **Always the invoking workspace**, even under `--project` / `--all`. Other workspaces appear inside `data`, so the envelope shape never varies. |
| `status` | The terminal state from the table above |
| `error` | The typed error object (§1.7 of `ARCHITECTURE.md`) or `null` |
| `data` | The per-verb body. **Defined by the phase that builds the verb**, not here. |

The body is nested rather than flattened so the envelope is generically validatable — one
schema checks the wrapper, a per-verb schema checks `data` — and so a future verb can add a
field called `status` or `error` without colliding with the envelope.

### 3.2 Selectors

Check ids are derived as `<component>:<check>` (§4.1), so char always holds the complete set
of valid selectors and never has to discover anything. `char check web:e2e`,
`char check --component web` and `char check lint` all fall out of that set.

**Partial matches are normal.** `char check test` where `api:test` exists and `web:test` does
not runs `api:test` and exits 0.

**Zero matches depend on whether the name is conventional.** These six are conventional:

```
lint   types   test   e2e   build   fmt
```

- **A conventional name matching nothing** → `PASS`, empty `data.runs`, exit 0. "This
  workspace has no lint checks" is a real and unremarkable answer, and it is what lets an
  orchestrating agent run `char check lint` across five workspaces without special-casing
  the three that lack it.
- **An unconventional name matching nothing** → `bad_invocation`, exit 2, with the available
  selectors listed in `next_action`. Almost always a typo, and the error teaches the
  vocabulary rather than merely rejecting.

**Why char holds this small piece of policy.** Without it, "you typed it wrong" and "this
repo has none" are indistinguishable, and both available answers are bad: exiting 0 on a typo
means an agent reports a passing lint that never ran, while erroring on both teaches agents
to write `char check lint || true` — which suppresses *every* error the command can raise,
converting a local annoyance into a total loss of signal. The conventional set is also not an
invention: §4.1's examples and all six fixtures already use exactly these names.

**Growth rule: a name joins the set only when a fixture uses it.** Otherwise the list becomes
a bikeshed.

### 3.3 Scope lens

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
      file: [docker-compose.yml]   # a list — repos often run base + override
      ports: { pg: 5432 }          # remapped into this workspace's block (§6.0)

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

**`char clean` removes `.char/` entirely.** The workspace returns to its pre-init state.
Leaving it behind would mean `workspace.json` asserting a port block the registry no longer
records — a stale claim, which is precisely the condition `status` exists to surface.
`char init` is idempotent and recreates it.

**Log growth is a separate problem with a separate answer.** Coupling retention to `clean`
would mean either logs live forever or you lose the evidence from a failed run the moment you
release a port. Instead, at the **start of each run** char reaps old run directories, keeping
the most recent N and never touching one whose `lock` is live. N is configurable; its default
is a convention, not a measurement.

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

### 4.4 Templating: four substitutions plus one scoped read, hard cap

**Everywhere:** `${port.NAME}`, `${files}`, `${component.root}`, `${workspace.id}`.

**Inside `env:` blocks only:** `${env.NAME}` — a bare read of the ambient environment.
Unset at spawn time is a `bad_config` error naming the variable.

**No conditionals, no loops, no expression language.** `${env.NAME ?? "default"}` is
rejected by the schema, not merely undocumented.

The reason is not parser cost — it is that requests arrive one at a time, each individually
reasonable, with no natural stopping point: `${port.api}` → `${env.CI ?? 0}` → `{{#if}}` →
`{{#each}}` → a language with no debugger, no types, and no stack traces, whose bugs are
yours to diagnose from a YAML file at the exact moment an agent is blocked.

**Why the line sits here rather than at three.** `${workspace.id}` was never really outside
the cap — §6.1's `owns:` example already used it, so the plan contradicted itself. And a
*bare* `${env.NAME}` is structurally a lookup from a namespace, exactly like `${port.api}`:
there is no operator and nothing to evaluate. The slope in the paragraph above does not begin
at the read — it begins at `??`, because the moment the read exists someone asks "what if it
is unset," and that question has precisely two answers: a default operator, or a loud error.

**So the error is the stopping point, and it has to stay one.** Unset is `bad_config`, exit
3, naming the variable. That is what makes this a resting place rather than a first step.

**One cost, accepted knowingly.** `${env.NAME}` makes `char.yml` environment-dependent —
`config verify` can check that the reference is syntactically valid, but it cannot know
whether the variable will exist on another machine, so a config can verify locally and fail
in CI. Every other part of this file means the same thing everywhere. That is the price of
the read, and it is why the read is confined to `env:` blocks.

**Do not reach for `${env.NAME}` for secrets.** It requires the value to be in the ambient
environment already, which in practice means a `.env` file or a shell `export` — a file or a
history an agent can read. That moves the leak earlier rather than removing it. Secrets have
their own mechanism (§4.7).

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

### 4.5 `commands:` — repo-local verbs char does not own

The six verbs are universal. Every repo also has commands that are **only** meaningful in
that repo, and char must not swallow them or force them elsewhere. A top-level `commands:`
block, sibling of `components:`, declares them:

```yaml
commands:
  worktrees:
    cmd: uv run scripts/worktrees.py
    help: Create and tear down git worktrees
    env:
      WORKSPACE: ${workspace.id}
      REGISTRY: ${env.COMPANY_REGISTRY}     # bare read; unset is bad_config
    secrets: [GITHUB_TOKEN]                 # explicit grant, §4.7
    owns:
      containers: "label=com.example.worktree=${workspace.id}"
      files: [".worktrees/${workspace.id}"]
  tickets:
    cmd: uv run scripts/tickets.py
```

`env:` is additive — the parent environment is inherited wholesale and these are layered on
top, so a command needing `$HOME` already has it.

`stdio:` is `inherit` or `pipe`, and **its default is inferred: `pipe` when the entry grants
secrets, `inherit` otherwise.** Piping lets char scrub its output; inheriting preserves the
child's TTY, so colours, progress bars and interactive prompts work.

The default is only a default. char must not decide this by inference alone, because
inference is wrong in both directions: a `deploy.sh` that holds a token *and* prompts for
confirmation needs `inherit` despite its grant, and a command with no grant that fetches its
own token internally and logs it needs `pipe` despite having none. The repo knows; char
cannot.

**`stdio: inherit` alongside a `secrets:` grant is permitted, and disables scrubbing for that
entry.** char still writes nothing itself — the child writes straight to the terminal — but
§4.7's practical protection does not apply. Two deliberate keys in one block is a clear
enough signal of intent; making it an error would leave the interactive-command-with-a-token
case unserviceable, forcing that script to fetch its own secret and putting it *outside*
char's management rather than inside it.

> **Reserved, not built: `stdio: pty`.** A pseudo-terminal gives the child a TTY while char
> still sees the bytes, which recovers colour and progress-bar fidelity under scrubbing. It
> is cleanly POSIX, so it costs nothing that §7 has not already given up. Output-only is
> modest; interactive *input* — raw mode, `SIGWINCH` forwarding — is where it gets expensive,
> and no fixture needs it yet.

`owns:` behaves exactly as it does under `run:` (§6.1), with one difference: it is a
**selector, not a record.** char stores the declaration and `char clean` *evaluates* it
against docker and the filesystem. That works because every selector is stamped with
`${workspace.id}`, and it means no lifecycle hook and no write to `owned.json` — a command
runs ad hoc, so there is no "while it was up" window to record against. `ports:` is not
available here; the block is already claimed by `char init`.

`char worktrees prune --dry-run` runs `uv run scripts/worktrees.py prune --dry-run` from the
workspace root. char is a dispatcher here and nothing more: remaining argv passes through
untouched, and **the command's exit code is returned verbatim** rather than being mapped into
char's own codes — char did not decide the outcome, so it does not get to classify it.

The same three substitutions apply and no others (§4.4); `${files}` is simply never
populated for a `commands:` entry, since there is no scope to compute.

**A name may not shadow a built-in verb.** `config verify` rejects a `commands:` entry named
`init`, `up`, `down`, `check`, `clean`, `status`, `config` or `agents-md`. Without that rule
a repo can silently break the one guarantee the project exists to provide — that the six
verbs mean the same thing everywhere.

**Why this is in the config rather than a plugin mechanism.** It is the same argument as
§6.1: the thing a repo actually needs is a name and a command, not a lifecycle contract. This
is also what lets Chariot keep `worktrees` / `tickets` / `design` while giving up `check` and
`servers` (phase 6), so it is on the critical path rather than a nicety.

### 4.6 `workspaces:` — nested workspaces in one repo

**The default stays "packages are components."** A monorepo is one workspace, one port block,
one `.char/`, and per-package work is served by the scope lens that already exists —
`char check --component web`, `char check web:e2e`, `match:` globs scoping by changed files
(§3.2, §3.3). Reach for this section only when that is genuinely not enough.

The case it exists for: `apps/foo` and `apps/bar` are **separate products that happen to share
a repo**, and foo's services, ports and lifecycle must be independent of bar's. A root config
declares them:

```yaml
# repo root char.yml
version: 1
workspaces: [apps/foo, apps/bar]   # separate workspaces, excluded from this one
components:
  shared-lib:
    root: libs/shared
    checks: { lint: { cmd: ruff check ${files} } }
```

Each declared path holds its own `char.yml` and becomes an ordinary workspace: its own id,
its own port block, its own `.char/`. A root that is *nothing but* a manifest — `workspaces:`
with no `components:` — is legal, and is the honest shape for a repo of genuinely independent
products.

**No new runtime concepts.** Two workspaces sharing a checkout is structurally identical to
two git worktrees, which §2.2 already models as flat siblings. They share a `project_id`,
because they *are* the same repo — so `char status --project` reporting "foo is up, bar is
down" is the right answer, `char clean` still touches only your own workspace, and
`char clean --project` still touches both because that is the destructive option you have to
ask for.

**The thing that is actually illegal is overlap, not nesting.** If the root also claimed
`apps/foo` as a component root or reached into it with a `match:` glob, that subtree would
have two owners with two ids and two port blocks — the same source and services claimed
twice. So `config verify` asserts that no `components[].root` and no `match:` glob reaches
into a declared nested workspace.

**Why declared at the root rather than inferred.** Inferring — "any subtree containing a
`char.yml` is automatically excluded" — needs no configuration, but it means dropping a file
into a directory silently changes the root's behaviour, and an *accidental* `char.yml`
quietly becomes a workspace instead of an error. Declaring it keeps the stray-file case loud
(§2.1) while letting the deliberate case work.

> **Not built: config fragments.** A different need — one workspace whose config is split
> across per-package files for authoring reasons, rather than several workspaces. If that
> becomes real, the answer is an include mechanism that still resolves to a single workspace,
> **not** nested workspaces. Named here so nobody later reaches for the wrong one.

### 4.7 `secrets:` — tokens reach the process, never the transcript

char is the only thing in the stack that constructs the environment for every process in the
repo. That makes it the one place this can be fixed.

**The problem.** An agent runs `char up` and a service needs `STRIPE_SECRET_KEY`. Today that
means a `.env` file an agent will eventually read while debugging, or an `export` in a shell
history, or — worst — a token on the command line, visible in `ps` to every process on the
machine. And when a command echoes its environment on failure, char captures that into
`.char/run/<id>/logs/`, which is a file agents are *expected* to read.

```yaml
secret_providers:
  op:       { cmd: op read ${ref} }
  aws-sm:   { cmd: aws secretsmanager get-secret-value --secret-id ${ref}
                     --query SecretString --output text }
  keychain: { cmd: security find-generic-password -s ${ref} -w }

secrets:
  GITHUB_TOKEN: op://Engineering/github/token
  DB_PASSWORD:  aws-sm://prod/db#password

components:
  api:
    run:
      driver: command
      cmd: manage.py runserver 0.0.0.0:${port.api}
      secrets: [DB_PASSWORD]        # granted here, and nowhere else
```

The URI scheme selects the provider; `${ref}` is the rest of the reference.

**Five properties, each load-bearing:**

| | |
|---|---|
| **Reference, never value** | `char.yml` stays committed and diffable. It holds a pointer. |
| **Grants are explicit and per-entry** | A `run:`, `checks:` entry or `commands:` entry names what it needs. Least privilege, and `grep -n "secrets:"` answers "what can reach this token." |
| **Injected via env at spawn, never argv** | argv is world-readable through `ps`. |
| **char scrubs resolved values from everything it writes** | logs, `--json`, error messages, the live table. |
| **There is no retrieval verb** | No `char secret get`, ever. An agent can *use* a secret by running `char up`; it cannot *obtain* one. That asymmetry is the entire point. |

**char reads raw and writes scrubbed.** Scrubbing is a filter applied on the way *out*, never
a transform on the stream. So `ready: { log: <regex> }`, any `parse:` keys and exit-code
interpretation all see the real bytes, while the log file, `--json` and **the terminal** see
redacted ones. Scrubbing first would break a ready-check whose regex spans a redacted value —
`listening on postgres://.*@localhost` — and buys nothing.

The terminal counts as a write: if an agent runs `char up` and char streams service output,
that lands in the transcript. Which is why `stdio:` (§4.5) matters — char can only scrub what
it can see.

**Providers are commands, not integrations.** char must never grow 1Password, AWS or Keychain
SDKs. A provider is a command that prints a secret to stdout — char runs it through the
injected `run`, captures stdout, and never logs it. That is roughly a hundred lines with no
vendor lock-in, and it is the same instinct as §6 ("no vendor-named drivers") and §6.1
("`owns:` instead of a plugin API"). Vault, Doppler, `pass` and a homegrown script all work
on day one without char knowing they exist.

**Never cache a resolved secret.** Caching means writing it to disk, which is a new leak
surface. Resolve per spawn and let providers do their own session caching — `op` already
does, and that is correctly their problem, not char's.

**What this does and does not guarantee.** char guarantees the secret is never in `char.yml`,
never in argv, never in char's own logs or `--json`, and never retrievable through any char
verb. char *cannot* stop an agent from running `op read` itself, cannot control a command
invoked outside char, and cannot defeat deliberate exfiltration through encoding. Scrubbing
is defense-in-depth, not a proof.

The win is narrower than "foolproof" and still large: **the default path becomes safe.** The
agent runs `char up`, the service gets its token, and nothing the agent can read ever
contained it. Today the default path is unsafe, and that is the actual bug.

**Schema lands in phase 1; implementation in phase 4**, when `up` exists and there is
something to inject into.

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
- no `commands:` entry shadows a built-in verb (§4.5)
- no `components[].root` or `match:` glob reaches into a declared nested workspace (§4.6),
  and every path in `workspaces:` actually contains a `char.yml`
- every granted secret name is declared in `secrets:`, and every reference's URI scheme
  matches a declared `secret_providers:` entry (§4.7). **Never resolves a secret** — the
  reference is checked, the value is not fetched
- no `${env.NAME ?? ...}` anywhere, and no `${env.NAME}` outside an `env:` block (§4.4)

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
| `compose` | **Resolve → transform → emit.** See §6.0 — this is not a matter of adding flags to `docker compose`. |
| `command` | Spawns detached in its own process group, records the pid, waits on the ready-check, kills the whole group on `down`. Covers a supervisor, `pnpm dev`, `manage.py runserver`, a Procfile line — anything. |

### 6.0 The compose driver

An earlier draft specified this as *"shells out to `docker compose` with a project name
derived from the workspace id, port mappings rewritten into the claimed block,
`--label char.workspace=<id>`."* **Two thirds of that is impossible.** `docker compose` has no
`--label` flag, and port mappings cannot be rewritten from the command line at all. Only the
project name was achievable. See §6.2 for what was measured.

The mechanism is four steps:

```
1. RESOLVE   docker compose -f <base…> -p char-<id> \
                 --project-directory <workspace-root> config
             → one canonical document, with interpolation, extends:, anchors
               and relative paths already resolved

2. TRANSFORM ports[].published      → the claimed block
             labels.char.workspace  → <id>          (every service)
             build.labels.char.workspace → <id>      (services that build)

3. EMIT      .char/compose.yml

4. RUN       docker compose -f .char/compose.yml -p char-<id> \
                 --project-directory <workspace-root> up -d
```

**Why generate a whole file rather than an override.** Because an override cannot do the one
thing it would be for: compose **appends** to `ports:` rather than replacing, so the base
port stays published and every workspace still binds it — the exact collision this project
exists to prevent. The `!override` tag fixes that only on Compose ≥ 2.24.4 and **silently
does nothing below it**, reverting to base ports with no error.

**Why char never parses compose semantics.** Step 1 hands that entire problem to compose
itself. char rewrites two keys in a document compose has already normalised, which is why
this works on any version and why `extends:`, YAML anchors and `${VAR}` interpolation are not
char's problem.

`.char/compose.yml` is generated, gitignored, and removed by `clean` along with the rest of
`.char/` (§4.2). It is also inspectable and diffable, which is what makes a wrong port
obvious rather than mysterious.

**Ownership falls out.** Containers and networks carry both `com.docker.compose.project=char-<id>`
(compose applies it automatically from `-p`) and `char.workspace=<id>` (from the transform).
`clean` uses the latter, so it stays driver-agnostic.

**Images, narrowed.** char labels only images it causes to be *built*, via `build.labels`. A
pulled image such as `postgres:16` is shared with the rest of the machine and was never
char's to remove. This corrects an earlier claim in §2.3 that stamping meant "passing the
label through to compose" — it does not — and it matches the evidence, which is ~2.1 GB per
production app **build**.

**`file:` accepts a list.** Repos commonly already run base-plus-override, and step 1 must
receive the same file set they do. char also ignores ambient `COMPOSE_FILE` and
`COMPOSE_PROJECT_NAME`, passing `-f` and `-p` explicitly every time, so the result does not
depend on the caller's environment.

### 6.2 Measured Docker behaviour — do not re-derive

Verified against Docker Compose v2.24.3 during phase 0. Each of these was found by testing,
not by reading documentation, and each would have cost a phase-4 debugging session.

| # | Behaviour | Consequence |
|---|---|---|
| 1 | `docker compose up` has **no `--label` flag** | Container labels can only come from the compose document |
| 2 | An override file **appends** to `ports:` — base `5432:5432` plus override `5460:5432` publishes **both** | An override cannot remap a port. This is the trap that makes §6.0 necessary |
| 3 | The `!override` tag requires Compose ≥ 2.24.4 and **silently reverts to base values below it**, with no error | A version floor is not a sufficient guard when the failure below it is silent |
| 4 | `docker compose config` **bakes the project name into generated network names** | `-p char-<id>` must be passed on the *resolve* step, not only the run step, or networks are named for the directory |
| 5 | `config` resolves `build.context` to an **absolute** path | Emitting into `.char/` is safe, provided `--project-directory` is the workspace root |
| 6 | Override merging **does** work for `labels:` and `build.labels:` | Labels were never the hard part; ports were |

A running list of measured environment behaviour lives in [`traps.md`](traps.md). Add to it
whenever a phase discovers something that a reasonable person would have assumed otherwise.

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
        images: "label=char.workspace=${workspace.id}"
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
- **Secrets management beyond injection.** §4.7 resolves a reference and injects it. char
  does **not** store, generate, rotate, share or sync secrets, and does not implement a
  provider — a provider is a command that prints to stdout. The moment char holds a secret at
  rest it has become a secrets manager, and there are better ones.
- **Windows support.** Process groups, signals and file locks are load-bearing. Say
  POSIX-only in the README's first paragraph.
- **Multi-repo workspaces — reserved, not built.** `components[].root` is already a path;
  letting it point outside the workspace root (`root: ../api`) would get ~80% of multi-repo
  for free. Costs: a git diff *per* root rather than one, and the id can no longer be "hash
  of one path". Worth doing, but only once the two-repo setup actually exists.

---

## 8. Phases

Built **greenfield in its own repo**, not extracted incrementally through Chariot.

### 8.1 Why greenfield, and the anti-contamination strategy

The goal is that charkit carries no Chariot assumptions. Two distinct risks, and they need
different defenses:

| Contamination | Looks like | Defense |
|---|---|---|
| **Crude** | A hardcoded `backend/` path, a `tilt` import, a `.claude/worktrees` assumption | Greenfield makes it *structurally impossible* — the agent cannot see Chariot except in phase 2's harvest. Backed by a grep gate. |
| **Subtle** | An abstraction shaped around Django+Next because that is the only repo the agent ever saw | **Six fixture configs in phase 1.** Isolation does nothing here — an agent given one example generalizes from n=1 regardless of repo topology. |

**The fixture set is the more important of the two and is non-optional.** Write all six
before any code exists. An agent cannot overfit to Django+Next if the schema must also
express five other shapes on day one.

**The constraint that makes a fixture set useful:** every fixture must be able to fail the
schema in a way no other fixture can. Five Node monorepos teach nothing — they all pass or
all fail together. If adding a fixture creates no new way to be wrong, it is decoration.

| Fixture | Axis it owns | Failure it catches that nothing else does |
|---|---|---|
| `django-next` *(real)* | Maximal case — polyglot monorepo, supervisor, checks running *inside* containers, 3s→15min cost spread, **and the only fixture with a `commands:` block** (§4.5) | Schema can't express a real complex repo; `commands:` unexercised until phase 6, when it is load-bearing |
| `multi-lang` *(representative)* | A genuinely different runtime pairing | Abstraction is Django/Next-shaped |
| `go-service` | Low end — one component, one binary, one Postgres, no monorepo, **plus one secret from one provider** (§4.7) | **Over-structuring.** A trivial repo needing 40 lines of config — and secrets that only work in a complex config are secrets nobody will adopt |
| `pnpm-monorepo` | Many components, **zero** services, turbo already present, **plus a declared nested workspace** (§4.6) — so the fixture is a root manifest *and* a nested `char.yml` | Component-per-package globbing; also honestly answers "is char redundant where turbo exists?" Additionally: overlap detection, manifest-only roots, and discovery returning the same answer from any depth |
| `rails-monolith` | `setup:` as a *sequence* (bundle → db:create → migrate → seed); two services with real dependency ordering | `setup:` modeled as a single string; `needs:` ordering that only works for one service |
| `python-ml` | No web services, **no ports at all**, a 20-minute check, GPU as a non-port exclusive resource | Port machinery that doesn't gracefully no-op; `exclusive:` that assumes "a port" |

**Cost is low because fixtures are configs, not checkouts.** You don't need a Rails app — you
need a plausible `char.yml` for one plus a golden resolved snapshot.

**Evidentiary weight differs, and it is weaker than an earlier draft of this section
claimed.** `multi-lang` was originally marked *(real)* — "the second repo" — but no such
repository was ever identified, so it is representative like the other four. **Exactly one
fixture, `django-next`, is drawn from a real repo.** The remaining five prove *schema shape
only*; a hypothetical config cannot surface a runtime surprise. Six green fixtures must never
be read as "validated against six repos" — the honest reading is "validated against one repo
and five thought experiments."

**This weakens the plan's top-rated risk, and the weakening is not cosmetic.** §11 rates
overfitting to a single repo as the highest risk in the project, and names the fixture set as
its mitigation. That mitigation now rests on one real data point. Two consequences follow:

- Phase 8 — "the only test that matters" — is no longer a confirmation of something the
  fixtures already suggested. It is the **first** contact with a second real repo, and
  therefore the first opportunity to discover that the abstraction is Django+Next-shaped.
  Budget for it failing.
- If a genuinely different second repo becomes available before phase 1 finishes, promoting
  `multi-lang` back to real is the single highest-value change available to this plan.

Phase 8's target repository is also unnamed. If it ever turns out to be the same repo a
fixture was drawn from, the final validation is circular and does not count.

Greenfield was chosen over extract-through-Chariot for two reasons beyond contamination:

1. **Structural guarantees beat policed ones.** "The agent cannot see Chariot" is stronger
   than "the agent is told not to look."
2. **Extract-later has a specific, historically common failure mode.** Once phases 2–4 land
   inside Chariot the daily pain is gone, nothing forces the split, and it quietly never
   happens. Greenfield forces the code to be extractable because it has nowhere else to live.

**What greenfield gives up, and how to buy it back:** continuous validation against a real
repo. Fixtures catch config-model failures but not runtime ones — you would not discover
"the scheduler deadlocks when two exclusive resources overlap under load" until phase 6.

> **Read-only parallel run, from phase 2 onward.** Point charkit at the Chariot checkout,
> run `char check`, and diff the verdicts against `scripts/char`'s output. This is *not* a
> Chariot dependency and *not* a Chariot PR — zero risk to Chariot's merge gate — but it
> restores most of the continuous validation and turns phase 6 from a cliff into a
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

> **⚠️ Superseded. Phase 0 is complete — several of these were overridden.**
> [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) §1 records what was actually decided, with the
> reasoning. Notably: **three** injected seams rather than six (row 1); the dependency arrow
> in row 5 was **backwards** and now points inward; an exit-code map and a seventh principle
> (typed, attributed failures) were added. The table below is kept as the record of what was
> proposed, not as instructions.

| # | Principle | Why it earns its place |
|---|---|---|
| 1 | **Every outside-world interaction sits behind an injected seam** — subprocess, filesystem, docker, git, clock, network | This is why Chariot's 2,694 test lines run hermetically with no mocking framework: `run_fn` is *passed in*, not imported. It is the load-bearing pattern in the existing code, the one thing worth copying wholesale, and the same instinct as Chariot's own adapter-first rule. |
| 2 | **Pure core, imperative shell** | Config resolution, scope computation, scheduling, verdict aggregation = pure functions over data. Spawning, writing, labeling live at the edge. Most tests then need no fixture at all. |
| 3 | **The CLI is a thin wrapper over an importable library** | Already forced by the MCP server sharing the logic layer — but state it, because the failure mode is logic quietly accumulating in command functions. Every command: parse args → call library → render. |
| 4 | **No ambient state** | Workspace is resolved once and passed explicitly, never read from a global or inferred mid-call. Chariot's `--target` threading is the precedent, and it is what makes `--project` / `--all` scoping tractable. |
| 5 | **Dependencies point one way: core → adapters** | An adapter may never import the core's decision logic. Enforceable with a lint rule; worth doing. |
| 6 | **Every verb answers in a machine-readable shape** | `--json` is not an afterthought on some subset. The renderer is the only thing differing between human and agent output. |

#### 0.2 SDLC principles — recommended

> **⚠️ Superseded. See [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) §2.** Notably: TDD is
> **scoped** (mandatory in the core, test-alongside at adapters) rather than absolute; PRs
> are sized for review with no phase branches; the merge gate runs `no-mistakes` plus a
> GitHub Actions matrix; versioning stays `0.x` with no `1.0` commitment; and dogfooding is a
> **test** until phase 6, only becoming the gate once Chariot depends on it.

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

#### 0.4 Done when — ✓ satisfied

`docs/ARCHITECTURE.md` states principles 0.1 and 0.2 **with the rationale kept, not just the
rules**; `AGENTS.md` tells a future agent how to work in the repo; every question in 0.3 has
a recorded answer. **No source files exist yet.**

---

### Phase 1 — Repo skeleton + **six** config fixtures *(must land alone)*

uv package scaffolding, pytest, ruff. JSON Schema for `char.yml`. Then write all six configs
from the table in §8.1 under `tests/fixtures/<name>/char.yml`. Tests are schema validation
plus a golden resolved-config snapshot for each. **No runtime.**

The schema must cover the full contract, including the parts implemented later:
`components:` (§4.1), `commands:` (§4.5), `workspaces:` (§4.6), and `secrets:` /
`secret_providers:` (§4.7). Secrets are **schema-only in this phase** — validated and
resolvable as references, never fetched. Everything after this phase codes against whatever
lands here, so a key missing now is a contract change later.

**Done when:** all six are expressible with no escape hatches and no fields invented on the
spot.

**Expect the schema to change while writing them — that is the phase working.** If
`rails-monolith` needs `setup:` to be a list and `django-next` didn't, add it now, before any
code depends on the narrower shape. The fixtures that force a change are the ones earning
their keep; note which ones did, because that record is the argument for keeping them.

**Why alone:** every later agent codes against this contract. Parallel agents cannot share a
decision that has not been made yet — they will each invent an answer and you will get three
incompatible ones.

### Phase 2 — Rebuild the check engine, generalized *(clean-room, two agents)*

**This is a clean-room rewrite, not a copy** — see [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.7
for the full reasoning. The scheduler is a reducer and the original's is not, so the hardest
part was being rewritten regardless; and reshaping foreign code into `core`/`adapters`/`cli`
is more work than writing to the principles directly.

| Agent | Reads | Produces |
|---|---|---|
| **Harvester** | `~/Development/chariot/scripts/char/` | `docs/phase2-harvest.md` — a behaviour spec plus **a written list of every trap and bug-shaped branch found**. Plus the ported test *cases*. |
| **Implementer** | this plan, `ARCHITECTURE.md`, the fixtures, the harvest doc, the tests. **Never opens the Chariot repo.** | `src/` |

The harvest step is mandatory and is the whole reason a rewrite is safe here. The value in
those 1,632 lines is not the code — it is the bug fixes discovered by running against a real
repo, two of which the source flags as "Playwright traps." The uncommented ones are the
danger, and charkit has no continuous real-repo validation until phase 6, so anything lost
would not resurface until then.

Substantively: replace `CHECK_CATALOG` with the config loader, `domain` with `component`, and
strip every Chariot-specific path, turbo filter and `uv run --directory` assumption. Test
cases are ported, not rewritten — the assertions survive, but the harness is rebuilt around
`Ctx` and its three seams, and the scheduler tests change shape because the scheduler did.

**Done when:** the ported suite is green against the phase-1 fixtures, **and** the
contamination grep (§11) returns nothing.

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
recorded into `owned.json`, port remapping into the claimed block. Plus **secret resolution
and injection** (§4.7) — this is the phase where there is finally something to inject into.

**Done when:** a scratch repo with a bare `docker-compose.yml` plus a long-running command
comes up, gets ready-checked, and tears down completely — `docker ps` and `lsof` clean
afterwards.

**And when the secret path is proven negatively:** a service is granted a secret from a stub
provider, comes up with the value in its environment, and the value appears in **none** of
`.char/run/*/logs/`, `--json` output on both success and failure, `ps` output while running,
or `.char/owned.json`. Assert on absence, with the stub returning a distinctive sentinel so
the search is unambiguous.

### Phase 5 — Bootstrap sandwich + `agents-md` + MCP *(fans out widest)*

The layer-1 evidence scanner is a dozen independent parsers — the most parallelizable work
in the plan, one agent each. Plus schema/example emission, `config verify`, the managed
AGENTS.md block, and the MCP server.

**Done when:** an agent given only "set up char in this repo" produces a verifying config in
a repo it has never seen.

### Phase 6 — Chariot adopts it

A Chariot PR: delete `scripts/char/check.py` and `servers.py`, take the dependency, move
everything char does not replace into a `commands:` block (§4.5), repoint `bin/char`.

**The full dispatch surface, confirmed by inspection rather than assumed:**

| Chariot command | Subcommands | Disposition |
|---|---|---|
| `check` | passthrough | charkit `check` |
| `stack` | start, stop, restart, open, clean | charkit `up` / `down` / `clean` — **except `open` and `restart`**, which have no charkit verb |
| `clean` | — | charkit `clean` |
| `worktrees` | sweep, clean, merge | `commands:` — minus the resource half, which moves to `char init` / `char clean` |
| `tickets` | stale | `commands:` |
| `design` | passthrough | `commands:` |
| **`baselines`** | — | `commands:` — **not accounted for in any earlier draft of this plan**, and `baselines.py` is among the larger modules in the directory |

`stack restart` is `char down && char up` and can simply go. `stack open` is repo-specific
(it opens URLs) and becomes a `commands:` entry.

Subcommands are real — `char worktrees sweep`, `char tickets stale` — so `commands:` argv
passthrough must be transparent, as §4.5 specifies. `bin/char` execs an absolute path
resolved from the git root with no `uv run --directory`, so commands running from the
workspace root need no working-directory key.

**Take the dependency from git, not PyPI.** `uv` supports a git source, so this phase does
not wait on publishing — and getting a real repo onto charkit is worth more than getting the
packaging right first. Publishing follows in phase 7, with a consumer already attached.

**The worktree hand-off is the actual seam of this phase.** `worktrees.py` currently both
creates a worktree and allocates its resources. Those halves split:

| Today, in `worktrees.py` | After |
|---|---|
| Create the git worktree | Stays repo-local — a `commands:` entry |
| Allocate non-colliding ports, set up env/DB | `char init` in the new worktree |
| Sweep orphaned containers and networks | `char clean --orphaned` |

So the repo-local command shrinks to "create the worktree, then shell out to `char init`."
Ownership inference from compose's `working_dir` label goes away entirely, because charkit
stamps `char.workspace=<id>` itself.

**Done when:** `char check --all` is green in Chariot and the worktree flow still works end
to end.

**Expect the most rework here** — six phases of drift surface in this one PR.

### Phase 7 — Publish

PyPI as `charkit` with `bin: char`. The ~40-line installer:

```sh
command -v uv >/dev/null || curl -LsSf https://astral.sh/uv/install.sh | sh
uv tool install charkit     # provisions a Python if the machine has none
```

No bundling needed — uv installs Python itself, so this is dependency-free from the user's
side at ~2 MB rather than a 15–40 MB PyInstaller binary with macOS signing overhead.
Homebrew tap is a nice second channel, later.

Chariot's git dependency is repointed at the published package as part of this phase.

**Done when:** a clean machine with no Python runs the one-liner and gets a working `char`.

### Phase 8 — The only test that matters

Adopt char in a repo that is *not* Django + Next (a multi-language repo), using only a
`char.yml` authored by an agent through the phase-5 sandwich.

**Pass/fail:** if it needs a change to char's own code, the abstraction is wrong.

---

## 9. Source material

The reference implementation lives at `~/Development/chariot/scripts/`:

| Path | Lines | Role in this plan |
|------|------:|-------------------|
| `char/check.py` | 3,383 | Harvest in phase 2. Scope → schedule → run → parse → report, run lock, live table, `--again`. Contains `CHECK_CATALOG` (replace) and load-bearing comments about Playwright traps (translate into the fixture config, not the code). **Only one of the two traps is here — the other is in `baselines.py`, so harvest both.** |
| `char/_shared.py` | 337 | Harvest in phase 2. `run_fn` injection, target resolution, git worktree list. |
| `char/worktrees.py` | 679 | Reference for phase 3. Orphan container/network sweep — note it infers ownership from compose's `working_dir` label; charkit stamps its own instead. |
| `char/servers.py` | 436 | Reference for phase 4. Tilt-shaped; becomes config, not code. |
| `char/__main__.py` | 521 | Reference. Typer dispatch pattern. |
| `char_mcp/server.py` | ~95 | Reference for phase 5. |
| `char_test/` | 2,694 | **Harvest in phase 2 — port the cases, rebuild the harness.** `run_fn`-injected, asserts on behavior not implementation — this is the single most valuable asset. Only check-id fixtures should need editing. |
| `char/baselines.py` | 762 | **Not previously listed. Harvest for traps in phase 2 even though the code does not move.** A Playwright snapshot review aid — pixel-diffs darwin/linux snapshot pairs and renders an HTML page for a human. Holds at least one of the two Playwright traps this table attributes to `check.py`: with the default `updateSnapshots: "missing"`, an absent snapshot is *written* and the test *passes*, so a first containerised run reported 29/29 having compared 17 brand-new images against themselves. Couples only to `_shared` (`CheckError`, `RunFn`, `default_run_fn`), so phase 6 inlines three symbols and registers it as a `commands:` entry. |
| `char/tickets.py` | 51 | Small. Becomes a `commands:` entry in phase 6. |
| `bin/char` | ~25 | Copy the pattern. A bash dispatcher that resolves the git root from the *caller's* cwd at every invocation and execs `$root/scripts/char/__main__.py "$@"` — which is why one symlink works from inside any worktree. |

> **Counts re-measured. An earlier draft of this table was uniformly stale — everything had
> grown 1.4–2.4× since it was written.**
>
> | File | Earlier draft | Measured | |
> |---|---:|---:|---|
> | `check.py` | 1,632 | **3,383** | 2.07× |
> | `_shared.py` | 140 | **337** | 2.4× |
> | `worktrees.py` | 397 | **679** | 1.71× |
> | `__main__.py` | 345 | **521** | 1.51× |
> | `servers.py` | 321 | **436** | 1.36× |
>
> `scripts/char` totals **6,169** lines. `char_mcp/` and `char_test/` were not re-measured
> and should be assumed stale by a similar factor.
>
> **Consequence for phase 2:** the harvest is roughly double what the plan assumed. That
> makes the harvest step *more* valuable rather than less — twice the lines means twice the
> uncommented bug fixes to lose in a rewrite — but it is a scope change, and phase 2 should
> land as several review-sized PRs rather than one.
>
> **Re-measure before scoping any phase against this table.** It went stale once already.

---

## 10. Decisions already made — do not relitigate

| Decision | Choice | Why |
|----------|--------|-----|
| Language | **Python** | A `curl \| sh` installer gives the same "one line, fresh machine" property `npx -y` has. The language was never the requirement. Keeps 6,169 working lines in `scripts/char` alone, plus the test suite. |
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
| **Six phases of drift surface in phase 6.** Isolation removes continuous real-repo validation. | High | Read-only parallel run against Chariot from phase 3 onward (§8.1). Expect substantial rework in phase 6 regardless. |
| **Crude contamination** — a Chariot path or import follows the code in during phase 2. | Med | Phase-2 acceptance test is a literal `grep -riE "chariot\|tilt\|NEXT_PUBLIC\|\.claude\|backend/\|web/" src/` returning nothing. **Only phase 2's harvester has Chariot access.** |
| **Config expressiveness pressure** once a second repo lands. | Med | Three substitutions, hard cap. Escape hatch is a generator script. |
| **Registry corruption** with two agents claiming simultaneously. | Med | `O_EXCL` lockfile; claims idempotent by workspace id. |
| **`curl \| sh` is a trust ask** and some environments block it. | Low | `uvx` and `pipx` cover anyone who will not run it. Publish the script's source in-repo. |

---

## 12. Notes for the implementing agent

- **Phase 0 produces documents only.** Phase 1 lands alone and ships all **six** fixtures.
  Do not fan out until the config contract is committed.
- **Only phase 2's harvester has access to the Chariot repo.** Every other phase works from
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
