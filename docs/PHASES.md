# Armada — phases

> **Status:** rewritten around the four modules after the M0 spike. The spike is **run** — its
> findings are §9.1 and they changed two designs before any of them were built.
>
> Armada is one system with four modules: **Manifest**, **Guild**, **Fleet**, **Helm**.
> Manifest is the module formerly known as charkit and is roughly a third built; the other
> three do not exist yet. See [`PLAN.md`](PLAN.md) for what each one is and
> [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9 for the rule that keeps them separate.

## Contents

| § | | |
|---|---|---|
| **0.1** | Architecture principles — carried forward | |
| **0.2** | SDLC principles — two retire in M1, the rest carried forward | |
| **8** | The milestones — M0 through M4 | 8.1 why this order · 8.2 M0 · 8.3 M1 · 8.4 M2 · 8.5 M3 · 8.6 M4 |
| **9** | Source material | 9.1 **M0 spike findings** · 9.2 prior art |
| **11** | Risks | |
| **12** | Notes for the implementing agent | |

---

#### 0.1 Architecture principles — carried forward, not re-litigated

The eight architecture principles in [`ARCHITECTURE.md`](ARCHITECTURE.md) §1 were agreed for
charkit and **all eight apply unchanged to all four modules**. They were written about
subprocesses, clocks and networks, not about repositories, so nothing in the widening of scope
touches them. [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9 adds one rule the four-module shape needs and changes none of the others.

#### 0.2 SDLC principles — two retire in M1, the rest carry forward

Two SDLC principles exist only to protect a **public** repository, so they end when it goes
private. **That has not happened yet — both are live and must be satisfied today** (§8.3).

| Principle | Status |
|---|---|
| Contamination grep over `src/` and `tests/` | **Live. Retires in M1.** [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4 |
| Clean-room rule for the harvester phase | **Live. Retires in M1.** [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.7 |

Both retirements are recorded rather than deleted, because a rule that vanishes without a
reason gets reinvented. What replaces them **once they go** is the fixture set: six config
fixtures become the *only* thing standing between this design and being shaped around one
repository, which makes them more load-bearing after M1 than before it.

Everything else in [`ARCHITECTURE.md`](ARCHITECTURE.md) §2 stands: TDD scope, feature branches,
conventional commits, the merge gate including its two soon-to-retire checks, `0.x` versioning, dogfooding,
and document ownership.

---

## 8. The milestones — M0 through M4

Five milestones. The first is done. Each one is independently useful, which is the property
that matters when the whole thing is built in evenings.

| M | Modules | Delivers | State |
|---|---|---|---|
| **M0** | — | Research spike: four questions, throwaway prototypes, findings | **done** — §9.1 |
| **M1** | all | One tree: rename, four crates, one binary, private repo, subtraction | next |
| **M2** | Guild | `armada init`, the interview, import from `~/.claude/`, sync | |
| **M3** | Fleet + Helm | Jobs and Drones, budgets, inbox — **and Helm and the Bridge on top** | the product |
| **M4** | Fleet | Workflow loops with real verification | blocked on `manifest check` |

### 8.1 Why this order, and the one thing that reordered it

**M3 ships Fleet and Helm together, as one milestone.** An earlier draft of this plan had
them as separate phases with the orchestrator last, on the reasoning that you cannot build an
orchestrator before there is a fleet to orchestrate. That is true and it produced the wrong
plan: a Fleet with no orchestrator is a CLI for spawning agents by hand, which is not what
anyone asked for, and shipping it as its own milestone made the orchestrator look optional. It
is not optional. It is the product. The layers below it exist to make it possible.

**Guild comes before M3 rather than after it** for a concrete reason and not a priority one:
the orchestrator's persona, its workflows and its budget defaults all live *in* the guild.
Building the guild first is what makes M3 a configuration job instead of a hardcoding job.
Guild is also the only milestone that pays off entirely on its own — one command puts a whole
working setup on a new machine — and its content already exists in `~/.claude/` waiting to be
adopted.

**M1 is subtraction and it gets more expensive every week.** Renaming crates, moving the state
directory and deleting the privacy machinery touches every file and every golden
snapshot. Doing it before Guild and Fleet add surface area is the cheapest this will ever be.

**M4 is genuinely blocked** on Manifest's `check` verb, which is not built. Nothing else is.
That is stated here rather than discovered at integration time.

### 8.2 M0 — the research spike ✓ done

Four questions, each with a kill criterion so the spike ended in decisions rather than drifting
into a build. **None of the kill criteria fired.** Findings are §9.1.

| Question | Answer |
|---|---|
| What already exists that does this? | Overlap is real and partial — §9.2 |
| Do resumable sessions hold up as the session model? | Yes, verified — §9.1 F1 |
| Can budgets be enforced without an accounting layer? | Yes, and better than designed — §9.1 F2 |
| Can the orchestrator stay aware without polling? | Yes, two mechanisms — §9.1 F3 |
| How much of Guild do Claude Code plugins carry? | Most of the volume, none of the value — §9.1 F4 |

**Done when:** ✓ satisfied. Nothing from the spike ships; the prototypes were thrown away and
the findings are recorded here.

### 8.3 M1 — one tree

Restructure and subtraction. No new capability, and it should stay that way — a milestone that
renames everything *and* adds a feature has an unreviewable diff.

| | |
|---|---|
| **Repo** | Goes private. `charkit` → `armada`. |
| **Crates** | `core`, `manifest`, `guild`, `fleet`, `helm` — mirroring [`PLAN.md`](PLAN.md)'s module structure. Today's `charkit-core` + `charkit-adapters` become `manifest`; today's `charkit-cli` becomes `helm`. |
| **Binary** | One: `armada`. No `char` shim — it was never published, so a clean break costs nothing. |
| **Config** | `char.yml` → `armada.yml`, with the existing keys under a `manifest:` section. Also `crates/core/schema/char.schema.json`, and all six `tests/fixtures/*/char.yml`. |
| **State** | `~/.char/char.db` → `~/.armada/manifest.db`; `~/.char/config.toml` → `~/.armada/machine.yml`; the workspace-local `.char/` → `.armada/`. |
| **Identifiers** | Error class `char_bug` → `armada_bug`. Docker label namespace `char.workspace` → `armada.workspace`, compose project prefix `char-<id>` → `armada-<id>`. **Both are stamped on live resources**, so M1 must reap the old namespace before it stops recognising it — see the warning below. |
| **Managed blocks** | The `<!-- char:begin -->` / `<!-- char:end -->` markers and the `char agents-md` verb ([`PLAN.md`](PLAN.md) §5.1). Existing markers in the wild must still be recognised for one release, or a re-run appends a second block instead of replacing the first. |
| **Docs** | Convert [`PLAN.md`](PLAN.md) §1–§12, [`ARCHITECTURE.md`](ARCHITECTURE.md), [`AGENTS.md`](../AGENTS.md) and [`traps.md`](traps.md) from `char` spelling to `armada`. Part II of `PLAN.md` and everything under `docs/manifest/`, `docs/guild/`, `docs/fleet/`, `docs/helm/` is already converted. Delete the transition notes on each shipped reference page once they are true. |
| **Deletes** | `xtask/src/privacy.rs`, `xtask/src/contamination.rs`, the clean-room hook and its test, and the doc sections that explain them — **only after the repo is actually private**, not before ([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4, §2.7). |
| **Boundary check** | `xtask/src/boundaries.rs` generalises from "core depends on nothing concrete" to the module dependency rule in [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9. Today it enforces only the crate layering of [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.5, because three of the four modules have no crates. |

> **The rename touches live resources, which is the one part that is not a search-and-replace.**
> Docker labels and the compose project prefix identify containers, networks and volumes that
> exist on the machine right now. A build that renames the namespace without reaping the old one
> leaves every pre-M1 resource unowned and unreclaimable — the exact failure the ownership layer
> was built to prevent ([`PLAN.md`](PLAN.md) §2.3). **`armada manifest clean` must recognise both namespaces for one
> release.** This is the only behaviour M1 is allowed to add, and it is a migration rather than
> a feature.

**Done when:** `cargo test` and `cargo xtask doclint` pass, the six golden snapshots are
regenerated by hand, `rg -i 'char(kit)?' --glob '!target'` returns only deliberate historical
references, and `armada manifest init` / `clean` / `status` behave exactly as `char init` /
`clean` / `status` did. **A behaviour change in M1 is a defect**, not a bonus — with the single
documented exception of dual-namespace reaping above.

### 8.4 M2 — Guild

The portable half of the system, and the problem this project was started to solve: every new
machine and every new repo currently means setting up Claude files, scripts, hooks, MCP servers
and plugins by hand.

The guild is **machine-global user state**, not repository content. It lives in
`~/.armada/guild/`, Armada builds it by interviewing you, and it syncs between your machines on
its own. See [`PLAN.md`](PLAN.md) §13.

| Part | Notes |
|---|---|
| `armada init` | Machine setup — checks, then *"do you already have a guild?"* → pull from remote, import a bundle, or build one. Distinct from `armada manifest init`, which sets up a workspace. |
| Import | Adopts what is already in `~/.claude/`: skills, subagents, hooks, plugin and marketplace registrations, settings, `CLAUDE.md`. The guild starts nearly complete rather than empty, which is the difference between a tool you set up once and one you abandon during setup. |
| Interview | Asks only what it cannot read: voice, expectations, how-you-work, workflow confirmation, budget ceilings. Everything it writes is a plain file you can edit afterwards. |
| Sync | `~/.armada/guild/` **is** a git repo Armada manages, pushed to a private remote named once during the interview. `export` / `import` bundles are the escape hatch for a machine that will never hold your credentials. |
| Secret guard | Import refuses to adopt credential-shaped values; those stay in `machine.yml`, which never syncs. Built here, not retrofitted. |

**Split the packaging.** §9.1 F4 found that a Claude Code plugin carries skills, subagents,
hooks, MCP servers, monitors and a `bin/` on `PATH` — but **cannot carry `CLAUDE.md` or user
memory**, and a plugin's `settings.json` supports only two keys. So Guild ships the mechanical
assets as a plugin and lets Claude Code's own installer and versioning do the distribution,
and writes by hand only what plugins cannot carry: the memory fragments and the settings keys.
`claude plugin init` scaffolds a plugin into `~/.claude/skills/` that auto-loads with no
marketplace and no install step, which is exactly the shape a personal guild wants.

**Done when:** on a machine that has never seen Armada, `armada init` → pull → a working setup,
and a `git diff` in the guild repo shows what changed since the other machine.

### 8.5 M3 — Fleet, Helm and the Bridge

The product. Everything before this exists to make this possible.

**Fleet — the agents you do not talk to.** A **Job** is a UUID, a git worktree, a port block, a
transcript and a budget; a **Drone** is the process executing it. The Drone is temporary and the
Job is not (§9.1 F1, [`PLAN.md`](PLAN.md) §14.1). Fleet mints the UUID before anything runs, so
ownership is recorded up front and cleanup can find the Job afterwards even if the directory is
gone.

| Verb | Contract |
|---|---|
| `spawn` | Classify → worktree → `manifest init` → budgeted headless turn. Classification is Fleet's, not the orchestrator's: it is needed the moment a Job can be spawned. |
| `ls` | Name, task, status, run time, spend, needs-attention. All of it read off `stream-json` (§9.1 F2). |
| `inbox` | What the fleet needs from you. |
| `answer` | Resume a Drone with your decision. |
| `board` | Prints the worktree path and resume command. Armada does not own a terminal — cmux or the Claude app opens it. |
| `kill` | `manifest clean` → drop the worktree → release the port block, in that order. |

**Helm — the one agent you do talk to.** Typing `armada` launches a Claude Code session running
an orchestrator persona from your guild, with Armada's MCP server as its toolbelt. It needs no
interface work at all, which is why it lands here rather than after everything else. **No `helm`
binary is installed** — Kubernetes owns that name ([`glossary.md`](glossary.md)).

**The Bridge — the screen you watch.** `armada bridge` renders every Job, its state, its spend
against its ceiling, and who needs an answer. It holds no state and adds no capability: every
key maps to a Fleet verb that already works from a shell ([`helm/bridge.md`](helm/bridge.md)).

An earlier draft deferred the Bridge indefinitely, on the argument that cmux and the Claude app
already list sessions. That was answered rather than overruled: a session list is not what the
Bridge shows. Job state, budget against ceiling, and the inbox are data only Armada holds,
because only Armada mints the Jobs — so deferring the view meant deferring the only view of
anything Armada knows and nothing else does. Helm still works with the Bridge unbuilt, which is
what keeps it a rendering choice rather than an architectural one ([`PLAN.md`](PLAN.md) §15.1).

**Staying aware without polling** (§9.1 F3): a plugin monitor tailing `~/.armada/inbox.jsonl`
delivers fleet events into the conversation live, and a `Stop` hook refuses to end a turn while
anything is unread. Both are configuration rather than code, and both were demonstrated in the
spike.

**Done when:** you type `armada`, say "add rate limiting to the API and find out why the
nightly job is flaky", and two isolated Jobs run, report, and bring you the one decision
that is yours — without you naming a workflow, a worktree or a port.

### 8.6 M4 — workflow loops

The loop that runs until a task is complete, terminating on a verdict or a ceiling.
[`PLAN.md`](PLAN.md) §14.3 has the envelope and the four verdicts.

**Blocked on `armada manifest check`.** A verdict is only `PASS` if it carries evidence an
external command produced; an agent asserting "tests pass" is not evidence and an exit code is.
Manifest's remaining verbs — `up`, `down`, `check`, `config`, `agents-md`, `explain` — are
therefore first-class Armada work and not background work. `check` is the one that unblocks
this milestone.

**Done when:** a bug workflow reproduces a failure, writes a test that fails first, fixes it,
gets `check` green, and lands on a local branch, with no human turn in the middle and a hard
ceiling that stops it if it cannot.

---

## 9. Source material

### 9.1 M0 spike findings

Run on a real machine against real sessions, worktrees and hooks. Every claim here is something
that executed. Two findings changed a design before it was built, which is the entire return on
running a spike at all.

#### F1 — resumable sessions are the session model ✓

`claude` accepts `--session-id <uuid>` so the **caller** assigns the identity before anything
starts, `--resume <uuid>` to continue, and `--print --output-format stream-json` for a bounded
headless turn with a live event stream.

- Two worktrees off one repo, a UUID each, concurrent headless turns, **no collision**. The
  session in the second worktree correctly reported its own branch.
- The transcript landed exactly where predicted:
  `~/.claude/projects/<cwd-slug>/<uuid>.jsonl`.
- Records carry `cwd`, `gitBranch`, `sessionId` and `timestamp` — Fleet reconstructs which
  worktree and branch a session belongs to **without recording it separately**.
- Resume recovered context after the process had exited.

**Consequence:** Fleet has no journal to invent. The transcript on disk already is one; Fleet
writes only a thin index of Job metadata on top. This also retired two earlier designs —
a hidden multiplexer, and Armada owning a pty and rendering it — both of which existed only
because of an assumption that a session must be a live terminal somebody owns.

#### F2 — budgets need no accounting layer ✓ *(design changed)*

Every turn ends with a `result` event carrying the whole ledger, and `rate_limit_event` arrives
along the way. Measured values from the spike run:

| Field | Value |
|---|---|
| `num_turns` | 2 |
| `duration_api_ms` | 2956 |
| `total_cost_usd` | 0.1724735 |
| `stop_reason` | `end_turn` |
| `usage` | input 4 · cache_creation 14815 · cache_read 44357 · output 85 |
| `rate_limit_info` | `status: allowed` · `rateLimitType: five_hour` · `resetsAt` |

**Consequence:** every ceiling in [`PLAN.md`](PLAN.md) §14.3 reads straight off this. And
`rate_limit_event` is strictly better than the fixed concurrency cap the plan had — the
orchestrator can decline to spawn when a window reset is close, which is the thing the cap was
a proxy for.

#### F3 — the inbox has two mechanisms, both verified *(design changed)*

This was the load-bearing unknown. It is now the best-supported part of the plan.

**The `Stop` hook works.** A hook returning `{"decision":"block","reason":"…"}` held the turn
open and fed the message in. The session's unprompted output relayed it, attributed it as a
hook message rather than its own finding, and ended with a question — out of a nine-line shell
script.

**Monitors are better.** A plugin may ship `monitors/monitors.json`; every stdout line from a
monitor is delivered to Claude as a **live notification during the session**. A monitor tailing
the inbox pushes fleet events mid-turn rather than at turn end.

```json
[{ "name": "armada-inbox",
   "command": "tail -F ~/.armada/inbox.jsonl",
   "description": "Fleet events needing you" }]
```

**One constraint worth knowing:** monitors run in *interactive CLI sessions only*. That fits
exactly — the orchestrator is interactive and the Drones are headless — but it means monitors
can never be a Drone-side mechanism.

**Consequence:** the daemon design is dead and so is polling. Monitors give live push, the
`Stop` hook is the backstop that guarantees nothing is lost at turn end, and both are
configuration.

#### F4 — plugins carry the volume of the Guild, not the value

| Guild asset | Plugin carries it? |
|---|---|
| Skills, subagents, hooks, MCP servers, monitors, LSP, `bin/` on `PATH` | **Yes** |
| Voice / expectations / how-you-work | **No** — plugins cannot carry `CLAUDE.md` or user memory |
| Settings — permissions, statusline, effort level | **Barely** — a plugin `settings.json` supports two keys |

**Consequence:** M2 splits cleanly. The mechanical half is nearly free; the personal half — the
most valuable part — is exactly what Armada has to write itself.

### 9.2 Prior art

Checked rather than recalled, because three separate designs in this plan were withdrawn on
discovering something already did the job.

| | Has it | Lacks it |
|---|---|---|
| **OpenCode** | A real client/server session API — list, create, delete, abort, fork at a message, parent/child hierarchies, multiple clients per server. Primary agents and subagents with parallel work. | No documented aggregating orchestrator; navigation between sessions is hierarchical rather than one agent holding the picture. No documented worktree isolation. |
| **cmux** | Parallel Claude sessions in worktrees, with a session list. | No resource ownership, no portable guild, no orchestrator. macOS only, no automation surface. |
| **Claude Platform managed agents** | A supervisor that decomposes, delegates to parallel sub-agents with isolated context, and aggregates. The closest thing that exists to Armada's orchestrator. | Server-hosted and API-level: does not run in your local worktrees, does not know your ports and containers, does not carry your guild. |

**What none of them have** is the combination this plan is actually about: an aggregating
orchestrator over sessions that are isolated by a resource-ownership layer, equipped from a
portable personal guild. Adopting OpenCode is not the shortcut it appears to be — it is a
harness switch, and the entire guild is Claude Code shaped. Its session verb set is worth
reading as a design reference for [`PLAN.md`](PLAN.md) §14 all the same.

---

## 11. Risks

| Risk | Why it bites | Mitigation |
|---|---|---|
| **Manifest stalls** | Fleet and Helm are more interesting. Manifest stops at three verbs and M4 never unblocks, because `check` never lands. | Manifest's remaining verbs are milestones, not background work. M4's blocker is named in §8.6 for this reason. |
| **Rebuilding what exists** | Three designs in this plan were withdrawn after finding the job already done — a multiplexer, a terminal emulator, a session journal. Fleet is where this keeps happening, because "orchestrate parallel agents" sounds like infrastructure. | Standing rule for Fleet: before building a mechanism, check whether Claude Code, git, or something already installed does it. Armada's own code should be policy and glue. |
| **Guild drift between machines** | A hook edited on one machine and a skill on another, neither pulled, and the two setups silently diverge. | Auto-commit on change; warn on start when the guild is behind its remote; `armada doctor` shows the delta. Conflicts surface as conflicts, never as a silent overwrite. |
| **Guild carries a secret** | An imported settings file or MCP config holds a token and it reaches a remote — private, but still a remote. | The import guard in §8.4. Built in M2, not retrofitted. |
| **Stuck loops burn quota** | Parallelism itself is fine and deliberate. The unbounded retry is what costs. | The ceilings in [`PLAN.md`](PLAN.md) §14.3, plus `rate_limit_event` awareness from §9.1 F2. |
| **Orchestrator context bloat** | If Helm reads Drone transcripts it fills its window in three days of work and starts forgetting the fleet. | Structural: the orchestrator reads **summaries only**; probe is a separate cheap model. A design constraint, not a tuning knob. |
| **Losing the anti-contamination discipline** | Going private retires the grep that stopped one repository's specifics shaping the design. The crude leaks were never the real risk — an abstraction shaped around a single repo is. | The six config fixtures now carry that job alone. Keep them, and add one whenever a new repo shape appears. |
| **Harness lock-in** | Hooks, skills, plugins and MCP are Claude Code shaped. | Keep guild content in a neutral source shape and make the writer a renderer per harness. Do not write a second renderer until a second harness is real. |

---

## 12. Notes for the implementing agent

1. **Read [`ARCHITECTURE.md`](ARCHITECTURE.md) before [`PLAN.md`](PLAN.md).** It records the
   reasoning; the plan records the specification. Where they disagree, architecture wins and one
   of them is defective.
2. **Nothing points upward.** Manifest may not reference Fleet; Guild may not reference Helm.
   [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9. The boundary check enforces it, and the moment it
   is disabled "just for this one thing", this is four tools again.
3. **Manifest stays agent-agnostic.** It is the bottom of the stack precisely because it knows
   nothing about agents. A convenience that leaks a Job id into Manifest is not a
   convenience.
4. **Prefer configuration to code in Fleet and Guild.** Workflows are files in the guild;
   the inbox is a file and two hooks; classification is one model call. If a design needs a
   daemon, re-read §9.1.
5. **The spike's findings are evidence, not recollection.** §9.1 records what ran. If something
   there turns out to be wrong, fix the finding and say which — do not quietly work around it.
6. **Six fixtures are the anti-contamination discipline now.** When a new repository shape turns
   up that the fixtures do not cover, add a fixture before adding a feature.
