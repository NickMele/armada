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
| **0.2** | SDLC principles — one retired, one retiring, the rest carried forward | |
| **8** | The milestones — M0 through M4 | 8.1 why this order · 8.2 M0 · 8.3 M1 · 8.4 M2 · 8.5 M3 · 8.6 M4 |
| **9** | Source material | 9.1 **M0 spike findings** · 9.2 prior art · 9.3 **check-engine findings** |
| **11** | Risks | |
| **12** | Notes for the implementing agent | |

---

#### 0.1 Architecture principles — carried forward, not re-litigated

The eight architecture principles in [`ARCHITECTURE.md`](ARCHITECTURE.md) §1 were agreed for
charkit and **all eight apply unchanged to all four modules**. They were written about
subprocesses, clocks and networks, not about repositories, so nothing in the widening of scope
touches them. [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9 adds one rule the four-module shape needs and changes none of the others.

#### 0.2 SDLC principles — two retire on their own merits, the rest carry forward

**This is no longer keyed to the repository going private.** An earlier version of this plan
took it private in M1 and that decision was reversed
([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4): the repository stays public, permanently, so the **privacy gate is a standing check**
rather than a transitional one. The two rules below still retire, but each because its own job
is done — which is why M1 landing changed nothing about either.

| Principle | Status |
|---|---|
| Contamination grep over `src/` and `tests/` | **Live.** Retires because the fixtures replaced it, not because the repo changed. [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4 |
| Clean-room rule for the harvester phase | **Live.** Retires because the harvest has landed. [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.7 |

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
| **M1** | all | One tree: rename, four crates, one binary, private repo, subtraction | **done, less two rows** — §8.3 |
| **M1.5** | Manifest | The render layer: palette, tables, help, progress. Three audiences, one envelope | next |
| **M2** | Guild | `armada init`, the interview, import from `~/.claude/`, sync | |
| **M3** | Fleet + Helm | Jobs and Drones, budgets, inbox — **and Helm and the Bridge on top** | the product |
| **M4** | Fleet | Workflow loops with real verification | blocked on `check --detach` / `--status` |

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

**M1 was subtraction and it got more expensive every week.** Renaming crates, moving the state
directory and deleting the privacy machinery touches every file and every golden snapshot.
Doing it before Guild and Fleet added surface area was the cheapest it was ever going to be —
and the one row it did not carry (§8.3) gets no more expensive by waiting.

**M4 is still blocked, but on something narrower than before, and the capability it was
missing now exists.** The `check` engine has landed and dogfoods (§9.3), so a verdict can now
carry evidence an external command produced — the thing M4 was originally waiting for. What
`check` cannot do is `--detach` and `--status`, both still refused by name. That matters
because a loop runs inside a Drone's turn: the `python-ml` fixture's checks take thirty
minutes, and a Drone that blocks its whole turn on one is not viable. **A loop can run a check
to completion; it cannot yet start one and poll it.**

> **Fleet's detached Drone unblocks it, and the work left is wiring rather than design**
> (§8.5). `--detach` needs exactly what a Drone needed: start a long-lived `setsid`'d process
> group, record it as owned so `clean` reclaims it, and answer afterwards from what it wrote
> to disk. All three are built and used — `ProcessGroup::spawn`, the `owned` row with its two
> stamps, and a run directory `--status` can read the way `armada fleet ls` reads a
> transcript. The missing piece was never the flag; it was a second caller proving the shape
> works for something that outlives the command that started it.

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

Restructure and subtraction. No new capability, and it stayed that way — a milestone that
renames everything *and* adds a feature has an unreviewable diff.

**Landed, except the two rows marked below.** The state column says what each row's answer
is now; the arrows are kept because they are the migration, and a reader meeting a `char.*`
label or a `<!-- char:begin -->` marker needs to find out here why it is still recognised.

| | | |
|---|---|---|
| **Repo** | Goes private. `charkit` → `armada`. | **half done, half reversed.** The rename landed. Going private did not, and no longer will: the decision was reversed and the repository stays public ([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4), which is what unhooks the **Deletes** row below from this milestone. Renaming the GitHub repository itself is the operator's, not a build's. |
| **Crates** | `core`, `manifest`, `guild`, `fleet`, `helm` — mirroring [`PLAN.md`](PLAN.md)'s module structure. Today's `charkit-core` + `charkit-adapters` become `manifest`; today's `charkit-cli` becomes `helm`. | **done** — `armada-core`, `armada-manifest`, `armada-helm`. No code moved between crates: `core` stays pure and `manifest` is the module's shell, which is what keeps [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.5 mechanically enforced. `guild` and `fleet` get crates when they get code, and `boundaries.rs` already knows where they go. |
| **Binary** | One: `armada`. No `char` shim — it was never published, so a clean break costs nothing. | **done** |
| **Config** | `char.yml` → `armada.yml`, with the existing keys under a `manifest:` section. Also `crates/core/schema/char.schema.json`, and all six `tests/fixtures/*/char.yml`. | **done** |
| **State** | `~/.char/char.db` → `~/.armada/manifest.db`; `~/.char/config.toml` → `~/.armada/machine.yml`; the workspace-local `.char/` → `.armada/`. | **done**, and `machine.yml` is YAML rather than renamed TOML — one document language, one parser. **Its sections landed late and it cost a release**: the **Config** row above nested `armada.yml` under `manifest:` precisely so a second module could add one, `machine.yml` was left flat, and the first sibling section to arrive made every Manifest verb fail to parse it. Now namespaced the same way, with the flat form read for one release on the same terms as the label namespace below ([`PLAN.md`](PLAN.md) §4.3.1). |
| **Identifiers** | Error class `char_bug` → `armada_bug`. Docker label namespace `char.workspace` → `armada.workspace`, compose project prefix `char-<id>` → `armada-<id>`. **Both are stamped on live resources**, so M1 must reap the old namespace before it stops recognising it — see the warning below. | **done**, both namespaces read — see the warning below. The compose prefix had no code to change: the compose driver is not built ([`PLAN.md`](PLAN.md) §6.0). |
| **Managed blocks** | The `<!-- char:begin -->` / `<!-- char:end -->` markers and the `char agents-md` verb ([`PLAN.md`](PLAN.md) §5.1). Existing markers in the wild must still be recognised for one release, or a re-run appends a second block instead of replacing the first. | **markers renamed; nothing to migrate yet.** `agents-md` is not built, so no block has ever been written. The dual-recognition rule is recorded in [`PLAN.md`](PLAN.md) §5.1 for whoever builds it. |
| **Docs** | Convert [`PLAN.md`](PLAN.md) §1–§12, [`ARCHITECTURE.md`](ARCHITECTURE.md), [`AGENTS.md`](../AGENTS.md) and [`traps.md`](traps.md) from `char` spelling to `armada`. Part II of `PLAN.md` and everything under `docs/commands/` is already converted. Delete the transition notes on each shipped reference page once they are true. | **done** |
| **Deletes** | ~~`xtask/src/privacy.rs`~~ **kept — permanent**, because the repository stays public. `xtask/src/contamination.rs`, the clean-room hook, its test and their `settings.json` wiring are **deleted**, each on its own merits rather than as part of a rename ([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4, §2.7). |
| **Skills** | **Config contract landed**: `skills:` in the schema, `SkillEntry` in `model.rs`, and the `polyglot-web` fixture exercising the full shape plus the minimal one ([`PLAN.md`](PLAN.md) §4.8). **Outstanding**: resolution (`skills` does not reach `ResolvedConfig`, so nothing can read it yet), the four `config verify` cross-reference checks, and the `manifest skills` / `skills show` verbs. |
| **Ergonomics note** | M1 turns `char check` into `armada manifest check`. That is intended, but it is also when the most-used verbs get longer — [`PLAN.md`](PLAN.md) §3 records the root-alias resolution as reserved-not-built, and the rule that makes it safe. Nothing to build in M1; it is flagged so the regression is a known trade rather than a surprise. | — |
| **Boundary check** | `xtask/src/boundaries.rs` generalises from "core depends on nothing concrete" to the module dependency rule in [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9. Today it enforces only the crate layering of [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.5, because three of the four modules have no crates. | **done** — `boundaries.rs` reads the module each crate belongs to and enforces [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9's table, with [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.5 falling out of it. |

> **The rename touches live resources, which is the one part that is not a search-and-replace.**
> Docker labels and the compose project prefix identify containers, networks and volumes that
> exist on the machine right now. A build that renames the namespace without reaping the old one
> leaves every pre-M1 resource unowned and unreclaimable — the exact failure the ownership layer
> was built to prevent ([`PLAN.md`](PLAN.md) §2.3). **`armada manifest clean` must recognise both namespaces for one
> release.** This is the only behaviour M1 is allowed to add, and it is a migration rather than
> a feature.

**Done when:** `cargo test` and `cargo xtask doclint` pass, the golden snapshots are
regenerated by hand, `rg -i 'char(kit)?' --glob '!target'` returns only deliberate historical
references, and `armada manifest init` / `clean` / `status` / `check` behave exactly as
`char init` / `clean` / `status` / `check` did. **A behaviour change in M1 is a defect**, not a
bonus — with the single documented exception of dual-namespace reaping above.

**What is left of M1.** One row, and it is ordinary work rather than a blocker: `skills:`
was scheduled here to share one rewrite of the fixtures, and that saving is spent either way
now, so it can land in any milestone. The **Repo** and **Deletes** rows are not outstanding —
they were overtaken by the decision to stay public
([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.4).

### 8.3.1 M1.5 — the render layer

**Inserted after M1, because using the CLI made the gap obvious.** Everything before this
specified `--json` carefully and left human output to whatever `render.rs` happened to print.
The help text is hard to read, nothing is aligned, and nothing is coloured.

| | |
|---|---|
| **Palette** | Promoted out of the Bridge page into [`commands/render.md`](commands/render.md), shared by every coloured surface. Truecolor, no 16-colour fallback. |
| **Three audiences** | TTY human, **non-TTY human**, `--json` ([`PLAN.md`](PLAN.md) §3.1.1). The middle one is the common case: agents call this CLI and mostly do not pass `--json`. |
| **Tables** | One aligned-column renderer for `status`, `check` and later `fleet ls`. Terminal-width aware, degrading by truncating a column rather than wrapping a row. |
| **Help** | Restructured. The current output is a wall of flags with no grouping and inconsistent alignment. |
| **Progress** | Spinners and per-check progress for long runs — **on stderr**, so `\| jq` never sees a frame. |
| **Banner** | ANSI Shadow wordmark on bare `armada` and `armada init` only. Suppressed below 51 columns, on non-TTY, and under `--json` ([`commands/render.md`](commands/render.md)). |
| **Colour control** | `--color auto\|always\|never`, `NO_COLOR` honoured. Decided once, in one place. |

**Done when:** `armada manifest check` is pleasant at a terminal, identical in structure when
piped, byte-identical to today under `--json`, and `armada --help` can be read without
squinting. **No verb changes behaviour** — this is a renderer, and a behaviour change here is
a defect.

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
| Workflow validation | `templates/guild/workflows/workflow.schema.json` is authoritative for the predicate enum, step shape and budget keys; `armada guild verify` cross-checks that every `skill:` resolves, every `scope:` names real checks, and the `workflow:` graph is acyclic. Same schema-vs-verify split as `armada.yml` ([`PLAN.md`](PLAN.md) §5). |
| Starter skill | `guild init` copies [`templates/guild/skills/onboard-repo/`](../templates/guild/skills/onboard-repo/SKILL.md) — the loop that writes a repo's `armada.yml` with you, ending on a real `config verify` ([`PLAN.md`](PLAN.md) §13.4). The guild's first real content. |
| Starter workflows | `guild init` copies [`templates/guild/workflows/`](../templates/guild/workflows/) — design, plan, feature, bug — into `~/.armada/guild/workflows/` ([`PLAN.md`](PLAN.md) §14.6). M4's loop has nothing to run without them. |
| Starter persona | `guild init` copies [`templates/guild/subagents/helm.md`](../templates/guild/subagents/helm.md) into `~/.armada/guild/subagents/`, then never touches it again — it is yours from that moment ([`PLAN.md`](PLAN.md) §15.4). Without it M3 has an orchestrator with no persona to run. |
| Secret guard | Import refuses to adopt credential-shaped values; those stay in `machine.yml`, which never syncs. Built here, not retrofitted. |
| `manifest render` | Renders a repo's declared skills into a harness format ([`PLAN.md`](PLAN.md) §4.8). Lands here rather than M1 because the managed-region and reversal bookkeeping is the same machinery guild projection needs ([`PLAN.md`](PLAN.md) §13.2) — building it once, for both, is the point. |

**Split the packaging.** §9.1 F4 found that a Claude Code plugin carries skills, subagents,
hooks, MCP servers, monitors and a `bin/` on `PATH` — but **cannot carry `CLAUDE.md` or user
memory**, and a plugin's `settings.json` supports only two keys. So Guild ships the mechanical
assets as a plugin and lets Claude Code's own installer and versioning do the distribution,
and writes by hand only what plugins cannot carry: the memory fragments and the settings keys.
`claude plugin init` scaffolds a plugin into `~/.claude/skills/` that auto-loads with no
marketplace and no install step, which is exactly the shape a personal guild wants.

**Done when:** on a machine that has never seen Armada, `armada init` → pull → a working setup,
and a `git diff` in the guild repo shows what changed since the other machine.

#### What landed, and what did not

The done-when is met and run rather than reasoned about: `crates/helm/tests/guild.rs` drives the
real binary against real `git`, two scratch `$HOME`s and a bare repository standing in for the
private remote. Built: `armada init`, `armada doctor`, `armada guild init` with the five-question
interview, and `guild pull` / `push` / `export` / `import`. The starters, the secret guard and the
three agreed layouts are in and frozen.

Four things this milestone did **not** build, each with the reason it is a milestone of its own
rather than a gap:

| Not built | Why |
|---|---|
| **Projection** | `guild pull` and `import` update the guild; nothing yet re-writes the managed regions of `~/.claude/` from it, and `armada doctor`'s fourth group has nothing to compare against until something does ([`PLAN.md`](PLAN.md) §13.2). A `doctor` reporting `ok` for a check that ran nothing would be worse than one that does not report it at all. |
| **`manifest render`** | Listed in the table above because its managed-region bookkeeping is the same machinery projection needs. It is the same milestone as projection, and neither is half-useful without the other. |
| **`armada guild edit` and `guild verify`** | Reserved by name and refused by name. `edit` is `$EDITOR` plus the validation `verify` performs, so building it first would mean building half of `verify` twice. |
| **`armada doctor --fix`** | Refused by name rather than half-implemented: every finding already carries the command that fixes it, so `--fix` is a convenience over a surface that already works, and one that silently did half of what it promised would be worse than one that says it is not built. |

### 8.5 M3 — Fleet, Helm and the Bridge

The product. Everything before this exists to make this possible.

**Fleet — the agents you do not talk to.** A **Job** is a UUID, a git worktree, a port block, a
transcript and a budget; a **Drone** is the process executing it. The Drone is temporary and the
Job is not (§9.1 F1, [`PLAN.md`](PLAN.md) §14.1). Fleet mints the UUID before anything runs, so
ownership is recorded up front and cleanup can find the Job afterwards even if the directory is
gone.

| Verb | Contract |
|---|---|
| `spawn` | Classify → worktree → `manifest init` → **start a detached Drone and return**. Classification is Fleet's, not the orchestrator's: it is needed the moment a Job can be spawned. |
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
key maps to a Fleet verb that already works from a shell ([`helm/bridge.md`](commands/helm/bridge.md)).

An earlier draft deferred the Bridge indefinitely, on the argument that cmux and the Claude app
already list sessions. That was answered rather than overruled: a session list is not what the
Bridge shows. Job state, budget against ceiling, and the inbox are data only Armada holds,
because only Armada mints the Jobs — so deferring the view meant deferring the only view of
anything Armada knows and nothing else does. Helm still works with the Bridge unbuilt, which is
what keeps it a rendering choice rather than an architectural one ([`PLAN.md`](PLAN.md) §15.1).

#### What landed, and what did not

**Fleet is built and is usable from a shell.** All six verbs ship, with the Job index in
`~/.armada/jobs/`, worktrees under `~/.armada/workspaces/<repo>/<name>`, classification on
Haiku 4.5, ceilings read off the turn's `result` event, and the append-only inbox. Both agreed
layouts moved out of `tests/golden/render/pending/` into live byte comparisons **without either
being renegotiated**, which is what that directory was for.

**Drones run detached, and `spawn` returns.** A Job's Drone is started the way `armada manifest
up` starts a `command` service — `setsid`, a log file rather than a pipe, the handle dropped
without a wait, and the process group recorded as owned — so several Jobs run at once and
`armada fleet kill` and `armada manifest clean` both reclaim them. **An orphaned Drone is reaped
by the pass that already reaps an orphaned service**, and *"nothing is killed that Armada cannot
prove is its own"* stays one rule with one implementation.

> **This was got wrong first, and the correction is worth keeping.** `spawn` originally ran the
> turn to completion, because the testing instruction — fake the harness at `ctx.run` and assert
> the argv — pushed toward a Drone that `Run::call` could wait for. `Run::call` runs a child to
> completion by definition, so the seam quietly decided the design, and a blocking `spawn` can
> only run one Job at a time: the opposite of what Fleet is for. **A testing instruction is not
> an architecture**, and the give-away was that the mechanism already existed one module down.
> The suite did not have to get weaker to fix it — it got stronger, by running a stub `claude`
> that records the vector `execve` actually received.

**Nothing reports home, so the transcript is the ledger.** A detached Drone updates no record
when its turn ends; `armada fleet ls` sums every `result` event in the Job's stream and asks the
process table whether the group is still alive. That is also what keeps a Job's spend honest
across `armada fleet answer`: a resumed session appends its own `result`, so continuing adds up
rather than starting over.

> **The second thing this milestone got wrong, and it is the more instructive one.** The Drone
> argv was missing `--verbose`, which Claude Code requires alongside `--output-format
> stream-json`. Every Job spawned, claimed a worktree and a port block, and its Drone died
> instantly on a usage error — **while every test passed**, including one that ran a stub
> recording what `execve` received.
>
> **Asserting on argv proves you built the argv you intended. It does not prove the argv is
> accepted.** Those are different claims, and the whole suite only ever made the first. The
> fix is in three parts, because none of them is sufficient alone: the requirement is data with
> a pure test behind it; `armada doctor` runs the real argv against the real validator for free,
> using a probe that provably cannot spend a token; and the limitation itself is written down in
> [`traps.md`](traps.md), because the next thing Armada shells out to will have the same shape.
> **The general rule: a test that only checks what you sent is not a test that you can send it.**

| Not built | Where it goes |
|---|---|
| The MCP server, Helm, the Bridge | the two agents after this one — the three-agent shape above |
| **The skills merge** of [`PLAN.md`](PLAN.md) §14.5 | Fleet projects no merged skill set into a Job's worktree yet, and `fleet ls --skills` does not exist. A Drone resolves a skill *name* in its own worktree, so the repo already wins a collision; what is missing is the guild half and the shadow report. |
| **The workflow loop** | `spawn` runs one bounded turn and records what it spent. Advancing a step on its verdict is M4's, and the predicates are data the schema already validates. |
| **`kill --all-finished`'s full lens** | it kills every Job that is over *or* paused; `check --status` is what would let it tell a stalled Drone from a finished one. |

#### Four decisions taken before M3 was dispatched

| | Decided | Why not the alternative |
|---|---|---|
| **Testing Fleet** | Assert on the **argv** — `claude --session-id <uuid> --print --output-format stream-json --verbose` — and feed recorded `stream-json` back. **No test spawns a real session or spends tokens.** *Amended twice in flight.* (a) The Drone is not faked at all: it runs detached, which `ctx.run` cannot express, so the suite starts a **stub `claude`** that records what `execve` received. (b) Argv assertions are **not sufficient**, so `armada doctor` runs the real argv against the real validator, free. | Real sessions in the suite make a rate limit a red build and the API's latency a flaky one. Argv is where the bugs are anyway. Both amendments were forced by failures recorded above: the first because **stating the fake before the design let the seam decide the design**, the second because **a test that only checks what you sent is not a test that you can send it** — the whole suite was green while no Drone had ever started. |
| **The Bridge** | `ratatui`. | Hand-rolled ANSI over the existing render layer is ~300 lines, but then input handling and resize are also yours, and neither is interesting work. |
| **Classification** | **Haiku 4.5** (`claude-haiku-4-5-20251001`). | It runs on every spawn, so its cost is the one that compounds; picking one of four labels with a confidence is exactly its shape. Keyword rules are free and wrong often enough that the override becomes the normal path. |
| **Shape** | **Three sequential agents** — Fleet, then the MCP server, then Helm and the inbox. | One agent holding the whole milestone has no handoffs and no way to catch an early mistake before everything downstream is built on it. Fleet alone is already usable from the CLI. |

**Staying aware without polling** (§9.1 F3): a plugin monitor tailing `~/.armada/inbox.jsonl`
delivers fleet events into the conversation live, and a `Stop` hook refuses to end a turn while
anything is unread. Both are configuration rather than code, and both were demonstrated in the
spike.

**Skills merge lands here too.** Fleet projects the guild's skills and the repo's rendered ones
into each Job's worktree, with the collision policy of [`PLAN.md`](PLAN.md) §14.5 — repo wins,
shadow always reported. Helm's toolbelt gains `manifest.skills` and `manifest.skill`
([`commands/helm/mcp.md`](commands/helm/mcp.md)).

**Done when:** you type `armada`, say "add rate limiting to the API and find out why the
nightly job is flaky", and two isolated Jobs run, report, and bring you the one decision
that is yours — without you naming a workflow, a worktree or a port.

### 8.6 M4 — workflow loops

The loop that runs until a task is complete, terminating on a verdict or a ceiling.
[`PLAN.md`](PLAN.md) §14.3 has the envelope and the four verdicts.

**`check` has landed; two of its flags have not.** A verdict is only `PASS` if it carries
evidence an external command produced; an agent asserting "tests pass" is not evidence and an
exit code is. The engine that produces that exit code is built and dogfooded — scope
resolution, the scheduler, the run directory and verdict aggregation, with what it settled and
the one gap it leaves open in §9.3. What it still refuses by name is `--detach` and `--status`,
so a loop can run a check to completion but cannot yet start one and poll it. `up` and `down` have since landed with them; Manifest's
remaining verbs — `agents-md` and `explain` — are first-class Armada work and not background
work.

**The mechanism `--detach` needs is now built and in use.** Fleet's Drones are long-lived
`setsid`'d process groups recorded as owned, and `armada fleet ls` answers about them from what
they wrote to disk rather than from anything they reported (§8.5). `--detach` is the same shape
against a run directory, and `--status` the same read. That is a wiring job on a proven
mechanism rather than the open design question it was when this section was written.

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

### 9.3 Check-engine findings

Manifest's check engine — scope resolution, the scheduler, the run directory and verdict
aggregation — is built and dogfooded ([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.6). It is recorded
here for the same reason §9.1 is: these are things that ran, and later work codes against them.

#### What the check engine settled

Six things [`PLAN.md`](PLAN.md) specified without deciding, and the answers everything after
this codes against. Written down for the same reason the ownership layer's were: two
implementers deciding these separately produce two incompatible engines.

| # | Decision | Why it landed there |
|---|---|---|
| 1 | **A run id is 16 Crockford base32 characters: 10 of wall-clock milliseconds, then 6 of per-process entropy.** | `PLAN.md` writes `01J8X2` in `data.run_id`, in `.armada/run/01J8X2/logs/` and in `armada manifest explain --run 01J8X2` — and those six characters are exactly the leading edge of a time-ordered base32 id, so this is the illustration made real. Time-ordered is load-bearing: retention keeps "the most recent N", and an id that did not sort would need every run's mtime read off a filesystem that may have been restored from a backup. |
| 2 | **A CPU slot's identity is the store's, chosen all-or-nothing inside one transaction.** | `lease::acquisition_order` numbers a check's slots `0..cost`, which is right for *ordering* and wrong for *naming*: two checks each asking for slot `0` deadlock the moment the second blocks on the first — measured, and it hung `armada manifest check` on its first real run. All-or-nothing matters as much: taking three of four and waiting for the fourth lets two runs hold half the machine each, and no acquisition order fixes that, because resources within the class are interchangeable. `acquisition_order` keeps its job — how many, and exclusives before slots. |
| 3 | **`Event::Tick` is the one variant added to [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.2's floor, and the shell ticks before it starts.** | `ARCHITECTURE.md` §1.2 records the cost of a pure reducer as "`now` is carried on every event"; spelling that as a variant is the escape hatch the floor names, and it changes none of the ten variants it writes out. The ordering is not cosmetic: a `Started` that arrives first computes every deadline from a `now_mono` of zero, and the first real tick then jumps past it. Measured — every check timed out immediately and reported a duration of eleven days. |
| 4 | **A check's verdict row carries `classifies`, not `attempted`.** | The two turned out not to be the same question. A cascaded `ABORTED` never ran *and* must not set the run's class; a check blocked on a service that is not running also never ran and *must*, because `bad_invocation` outranks a test failure precisely so the caller fixes the invocation first. Naming the field for what happened would have marked a blocked check `attempted: true` while it was never attempted. |
| 5 | **A `SKIPPED` prerequisite satisfies `needs:`.** | `PLAN.md` §4.1 says a check id in `needs:` "must have **passed** in this run". Read literally that cascades an `ABORTED` through every dependent of a check that had no matching files — turning a clean tree into a failing run, which is the mirror image of the hole `--all-files` exists to close. Nothing failed, so nothing is aborted. Recorded because it is an ambiguity in the spec rather than a free choice. |
| 6 | **`--detach` and `--status` are refused by name, as not built.** | `PLAN.md` §3 gives both to `check` and neither ships yet. Refused by name rather than as an unknown flag, because the flag *is* known and the honest answer is that Armada cannot do it yet — "unknown flag" sends an agent looking for a typo. The gap is stated here rather than left to be discovered. |

#### One finding the check engine fixed rather than sent back

Recorded because it was first written down as a defect in [`PLAN.md`](PLAN.md) and turned out to
be a defect in the ownership layer's code, which is a distinction worth keeping.

**The finding as first stated was wrong.** It read: `PLAN.md` §3.1 says the top-level error is
"the strict maximum over `results[]`", `PLAN.md` §4.1 says "a cascaded `ABORTED` never sets
`error.class`", therefore the two sections disagree and something downstream has to choose
between them. They do not disagree. `PLAN.md` §3.1's precedence chain runs over `error`
**classes**, and a cascaded row carries no `error` object at all — so `PLAN.md` §3.1 is silent
about what such a row contributes, and nothing anywhere in `PLAN.md` specifies it.

What filled that silence was `envelope::implied_class`, a helper the ownership layer invented to
give a class to a row that attached none. The inference is right for `FAILED` — the alternative
is a verb reporting success while `results[]` shows a failure — and it was extended to `ABORTED`
and `DEAD` by symmetry. That extension is the only thing that produced the forbidden outcome,
and `PLAN.md` §4.1 already ruled it out.

So there was nothing to send back. The inference was narrowed instead: a row whose state means
*no verdict was reached* implies no class, while a row carrying a real `aborted` error — a claim
that hit the acquisition ceiling — aggregates like any other. The blast radius was nil, because
the check engine is the only thing in the codebase that emits an `ABORTED` row and `DEAD` is
never emitted at all.

**The correction cost less than the workaround it replaced.** Conforming to `PLAN.md` §4.1 by
filtering rows before aggregating had put the same rule in two places and made the aggregate's
own count describe the slice rather than the run; narrowing the inference deleted the filter, a
field on every verdict, and a restated message. The lesson worth carrying: *a rule stated in two
documents is worth checking for a third possibility — that one of them never made the claim.*

#### One gap the check engine leaves open

**`Event::Interrupted` had no producer. Fixed.** Both reducers handled it and both were
unit-tested on it, but nothing in the shell delivered it: there was no SIGINT handler anywhere
in `crates/`, only the SIGPIPE restore at the entrypoint. A `armada manifest check` that was
interrupted died on the default disposition instead of ending its run.

Measured before the fix, by sending SIGINT to a real run:

| | Before | After |
|---|---|---|
| exit code | **130**, correct — [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.6 makes signals the one carve-out from `exit = f(error.class)` | **130**, and still by *dying from the signal* rather than calling `exit(130)`, so job control is not fooled |
| run lease | left in `~/.armada/manifest.db` until the heartbeat went cold, so a retry inside the minute failed fast | released with the run |
| **children** | **kept running.** `setsid`'d into their own sessions, so the signal never reached them | **killed**, by the group-kill path the reducer already had |

`manifest::posix::catch_interrupts` traps SIGINT and SIGTERM and does one thing: sets an
atomic. The run loop polls it before any other observation and delivers `Event::Interrupted`
once, and the path that already existed takes over — kill each running group, mark the rest
`ABORTED`, end the run `aborted`.

**Two details that are not incidental.** A second signal restores the default disposition and
re-raises, so a second Ctrl-C always kills: a tool that traps SIGINT and then wedges is worse
than one that never trapped it. And the exit is a real re-raise rather than `exit(130)`, because
[`ARCHITECTURE.md`](ARCHITECTURE.md) §1.6 says a signal *"has no error class at all"* — mapping it to `aborted`'s exit `5` is the
mistake that section names in advance, and it is exactly the mistake catching the signal tempts
you into.

Covered by `crates/helm/tests/interrupt.rs`, which signals a real run and then looks for the
grandchild — the only way to see this, since the orphan is invisible from inside the process.

---

## 11. Risks

| Risk | Why it bites | Mitigation |
|---|---|---|
| **Manifest stalls** | Fleet and Helm are more interesting. Manifest stops where `check` left it — no `explain` — and M4's loop never gets the detached run it needs. | Manifest's remaining verbs are milestones, not background work. What `check` still does not do is named in §8.6 for this reason. |
| **Rebuilding what exists** | Three designs in this plan were withdrawn after finding the job already done — a multiplexer, a terminal emulator, a session journal. Fleet is where this keeps happening, because "orchestrate parallel agents" sounds like infrastructure. | Standing rule for Fleet: before building a mechanism, check whether Claude Code, git, or something already installed does it. Armada's own code should be policy and glue. |
| **Guild drift between machines** | A hook edited on one machine and a skill on another, neither pulled, and the two setups silently diverge. | Auto-commit on change; warn on start when the guild is behind its remote; `armada doctor` shows the delta. Conflicts surface as conflicts, never as a silent overwrite. |
| **Guild carries a secret** | An imported settings file or MCP config holds a token and it reaches a remote — private, but still a remote. | The import guard in §8.4. Built in M2, not retrofitted. |
| **Stuck loops burn quota** | Parallelism itself is fine and deliberate. The unbounded retry is what costs. | The ceilings in [`PLAN.md`](PLAN.md) §14.3, plus `rate_limit_event` awareness from §9.1 F2. |
| **Orchestrator context bloat** | If Helm reads Drone transcripts it fills its window in three days of work and starts forgetting the fleet. | Structural: the orchestrator reads **summaries only**; probe is a separate cheap model. A design constraint, not a tuning knob. |
| **Losing the anti-contamination discipline** | The grep that stopped one repository's specifics shaping the design has retired. The crude leaks were never the real risk — an abstraction shaped around a single repo is. | The six config fixtures now carry that job alone. Keep them, and add one whenever a new repo shape appears. |
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
