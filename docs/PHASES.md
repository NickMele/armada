# charkit — phases

> **Sequencing, not contract.** What gets built, in what order, and how each phase knows it is
> done. The *contract* — verbs, config schema, `--json` envelope, identities, drivers — lives
> in [`PLAN.md`](PLAN.md) and is frozen once phase 1 lands.
>
> **Precedence:** [`traps.md`](traps.md) › [`ARCHITECTURE.md`](ARCHITECTURE.md) › `PLAN.md` ›
> this file › `AGENTS.md`. See `ARCHITECTURE.md` §2.8. Where this file states a behaviour that
> contradicts an owner above it, the owner is right and this file is the defect.
>
> **Read your phase only.** Plus §8.1, which explains the anti-contamination strategy every
> phase operates under.

## Contents

| § | | |
|---|---|---|
| **8** | Phases 0–8, and §8.1 the six-fixture set | Read §8.1, then your phase |
| **9** | Source material — the repo being harvested | Phase 3's harvester only |
| **11** | Risks | Once |
| **12** | Notes for the implementing agent | Once |

*Section numbers are kept from `PLAN.md` rather than renumbered — they have been renumbered
twice already and roughly thirty live cross-references depend on them. A gap is cheaper than a
third renumbering.*

---

## 8. Phases

Built **greenfield in its own repo**, not extracted incrementally through the source repo.

### 8.1 Why greenfield, and the anti-contamination strategy

**"The source repo" is the private polyglot monorepo char is being ported out of**, and
whose `scripts/char/` §9 describes. This repo never names it or its path — both are supplied
locally, from an untracked file or an environment variable
([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.7). Every reference below uses that term.

The goal is that charkit carries no source-repo assumptions. Two distinct risks, and they need
different defenses:

| Contamination | Looks like | Defense |
|---|---|---|
| **Crude** | A hardcoded `backend/` path, a `tilt` import, a `.claude/worktrees` assumption | Greenfield makes it *structurally impossible* — the agent cannot see the source repo except in phase 3's harvest. Backed by a grep gate. |
| **Subtle** | An abstraction shaped around the source repo because that is the only repo the agent ever saw | **Six fixture configs in phase 1.** Isolation does nothing here — an agent given one example generalizes from n=1 regardless of repo topology. |

**The fixture set is the more important of the two and is non-optional.** Write all six
before any code exists. An agent cannot overfit to one repo's shape if the schema must also
express five other shapes on day one.

**The constraint that makes a fixture set useful:** every fixture must be able to fail the
schema in a way no other fixture can. Five Node monorepos teach nothing — they all pass or
all fail together. If adding a fixture creates no new way to be wrong, it is decoration.

> **A different *language* is not an axis.** `multi-lang`'s row originally read "a genuinely
> different runtime pairing", which cannot stress the schema at all: `cmd:` is free text, so an
> Elixir service and a Python one produce byte-for-byte identical *shapes* and differ only in
> the strings inside them. That is `PLAN.md` §4's thesis working — stack diversity lands in config
> *values*, never config *shape*. The row now names the three structural things that fixture
> actually is the only one to exercise. Watch for the same mistake when adding a seventh: the
> test is "what new way to be wrong does this create", not "what technology does this cover".

| Fixture | Axis it owns | Failure it catches that nothing else does |
|---|---|---|
| `polyglot-web` *(real)* | Maximal case — polyglot monorepo, supervisor, checks running *inside* containers, 3s→15min cost spread, **and the only fixture with a `commands:` block** (`PLAN.md` §4.5) | Schema can't express a real complex repo; `commands:` unexercised until phase 6, when it is load-bearing |
| `multi-lang` *(representative)* | **Ready-check kinds beyond `http`** (`exec`, `log`, `tcp` — and it is the only fixture with `exec:`); a **cross-component `${port.NAME}` reference**; `owns.files` naming a path **outside** the component's `root:` | Ready-checks that only handle `http`; ports treated as a per-component namespace rather than a workspace-global one; `owns.files` assumed to sit under `root:` |
| `go-service` | Low end — one component, one binary, one Postgres, no monorepo, **plus one secret from one provider** (`PLAN.md` §4.7) | **Over-structuring.** A trivial repo needing 40 lines of config — and secrets that only work in a complex config are secrets nobody will adopt |
| `pnpm-monorepo` | Many components, **zero** services, turbo already present, **plus a declared nested workspace** (`PLAN.md` §4.6) — so the fixture is a root manifest *and* a nested `char.yml` | Component-per-package globbing; also honestly answers "is char redundant where turbo exists?" Additionally: overlap detection, manifest-only roots, and discovery returning the same answer from any depth |
| `rails-monolith` | `setup:` as a *sequence* (bundle → db:create → migrate → seed) including a step needing `shell: true`; two services with real dependency ordering; **`owns.release:` for a database on a shared server** | `setup:` modeled as a single string; `needs:` ordering that only works for one service; a setup step that errors when its resource exists; setup that creates something `clean` cannot reach |
| `python-ml` | No web services, **no ports at all**, a 30-minute check, GPU as a non-port exclusive resource | Port machinery that doesn't gracefully no-op; `exclusive:` that assumes "a port"; an `acquire_timeout` sized against the wrong hold — this is the longest exclusive hold in the set, and `PLAN.md` §4.3 is sized against it |

**Cost is low because fixtures are configs, not checkouts.** You don't need a Rails app — you
need a plausible `char.yml` for one plus a golden resolved snapshot.

**Evidentiary weight differs, and it is weaker than an earlier draft of this section
claimed.** `multi-lang` was originally marked *(real)* — "the second repo" — but no such
repository was ever identified, so it is representative like the other four. **Exactly one
fixture, `polyglot-web`, is modelled on a real repo.** The remaining five prove *schema shape
only*; a hypothetical config cannot surface a runtime surprise. Six green fixtures must never
be read as "validated against six repos" — the honest reading is "validated against one repo
and five thought experiments."

**This weakens the plan's top-rated risk, and the weakening is not cosmetic.** §11 rates
overfitting to a single repo as the highest risk in the project, and names the fixture set as
its mitigation. That mitigation now rests on one real data point. Two consequences follow:

- Phase 8 — "the only test that matters" — is no longer a confirmation of something the
  fixtures already suggested. It is the **first** contact with a second real repo, and
  therefore the first opportunity to discover that the abstraction is source-repo-shaped.
  Budget for it failing.
- If a genuinely different second repo becomes available before phase 1 finishes, promoting
  `multi-lang` back to real is the single highest-value change available to this plan.

Phase 8's target repository is also unnamed. If it ever turns out to be the same repo a
fixture was drawn from, the final validation is circular and does not count.

Greenfield was chosen over extracting in place for two reasons beyond contamination:

1. **Structural guarantees beat policed ones.** "The agent cannot see the source repo" is
   stronger than "the agent is told not to look."
2. **Extract-later has a specific, historically common failure mode.** Once phases 2–4 land
   inside the source repo the daily pain is gone, nothing forces the split, and it quietly
   never happens. Greenfield forces the code to be extractable because it has nowhere else to
   live.

**What greenfield gives up, and how to buy it back:** continuous validation against a real
repo. Fixtures catch config-model failures but not runtime ones — you would not discover
"the scheduler deadlocks when two exclusive resources overlap under load" until phase 6.

> **Read-only parallel run, from phase 3 onward.** Point charkit at the source repo's
> checkout, run `char check`, and diff the verdicts against `scripts/char`'s output. This
> creates *no* dependency there and opens *no* PR against it — zero risk to that repo's merge
> gate — but it restores most of the continuous validation and turns phase 6 from a cliff into
> a formality. Do this at the end of every phase from 3 on.

### Phase 0 — Foundations *(human + agent, working session, no code)*

**Output:** `docs/ARCHITECTURE.md`, `AGENTS.md`, and a `CONTRIBUTING`-style README section.
Nothing else. This is a conversation that produces documents, not a build step.

**Why this is first, and not skipped:** it is the third anti-contamination defense, and it
catches what the other two miss. The source repo's `check.py` has a *structure* — 3,383 lines
of it. Without stated architecture principles, phase 3's port inherits that structure by
default, because "make it work like it did" is the path of least resistance. Deciding the
target shape first turns the port from a copy into **a rewrite into a known architecture**,
and gives the reviewer an objective standard to reject against.

#### 0.1 Architecture principles — recommended; confirm or override

> **⚠️ Superseded. Phase 0 is complete — several of these were overridden.**
> [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) §1 records what was actually decided, with the
> reasoning. Notably: **three** injected seams rather than six (row 1); the dependency arrow
> in row 5 was **backwards** and now points inward; an exit-code map and a seventh principle
> (typed, attributed failures) were added. The table below is kept as the record of what was
> proposed, not as instructions.

| # | Principle | Why it earns its place |
|---|---|---|
| 1 | **Every outside-world interaction sits behind an injected seam** — subprocess, filesystem, docker, git, clock, network | This is why the source repo's 2,694 test lines run hermetically with no mocking framework: `run_fn` is *passed in*, not imported. It is the load-bearing pattern in the existing code, the one thing worth copying wholesale, and the same instinct as that repo's own adapter-first rule. |
| 2 | **Pure core, imperative shell** | Config resolution, scope computation, scheduling, verdict aggregation = pure functions over data. Spawning, writing, labeling live at the edge. Most tests then need no fixture at all. |
| 3 | **The CLI is a thin wrapper over an importable library** | Already forced by the MCP server sharing the logic layer — but state it, because the failure mode is logic quietly accumulating in command functions. Every command: parse args → call library → render. |
| 4 | **No ambient state** | Workspace is resolved once and passed explicitly, never read from a global or inferred mid-call. The source repo's `--target` threading is the precedent, and it is what makes `--project` / `--all` scoping tractable. |
| 5 | **Dependencies point one way: core → adapters** | An adapter may never import the core's decision logic. Enforceable with a lint rule; worth doing. |
| 6 | **Every verb answers in a machine-readable shape** | `--json` is not an afterthought on some subset. The renderer is the only thing differing between human and agent output. |

#### 0.2 SDLC principles — recommended

> **⚠️ Superseded. See [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) §2.** Notably: TDD is
> **scoped** (mandatory in the core, test-alongside at adapters) rather than absolute; PRs
> are sized for review with no phase branches; the merge gate runs `no-mistakes` plus a
> GitHub Actions matrix; versioning stays `0.x` with no `1.0` commitment; and dogfooding is a
> **test** until phase 6, only becoming the gate once the source repo depends on it.

| # | Principle | Note |
|---|---|---|
| 1 | **TDD throughout** — failing test → minimal implementation → passing test | Non-negotiable given this tool becomes a merge gate |
| 2 | **Branch + PR per phase, never commit to main** | The source repo enforces this with a `PreToolUse` hook — port the hook on day one rather than relying on discipline |
| 3 | **Conventional commits** | Matches the source repo's existing history (`build(family):`, `char check:`) |
| 4 | **Merge gate = lint + typecheck + tests + the contamination grep** | The grep is the phase-3 acceptance test made permanent |
| 5 | **Semver from the first publish, with a changelog** | Cheap now, painful to retrofit once anything depends on it |
| 6 | **Dogfood from phase 3 onward** | charkit gets its own `char.yml` and gates itself with itself the moment `char check` runs. Strongest available forcing function, and it makes the README example real rather than illustrative. |

#### 0.3 Decisions genuinely yours — bring answers to the session

| Question | Options | Consideration |
|---|---|---|
| **Public or private repo?** | public / private-for-now | Changes whether CI is free, whether the license matters, how much README polish is warranted. Private → public later is easy; the reverse is not. |
| **License, if public** | Apache-2.0 / MIT / none | Apache-2.0 adds a patent grant and clears corporate legal at no adoption cost. Only matters if public. |
| **CI** | GitHub Actions / local gate only / both | Actions is free for public repos, and unlike the source repo there is no billing constraint. Real CI matters more for a package other repos depend on. |
| **Typing strictness** | mypy strict / basic / none | Strict from commit one is cheap; retrofitting onto 3,000 lines is not. |
| **Rust edition / MSRV** | 2021 edition | Pin an MSRV in `Cargo.toml` and raise it deliberately. Users are unaffected either way — they receive a static binary, so no toolchain is required to run `char`. |
| **Test layers** | unit only / unit + a real-subprocess integration tier | Principle 1 makes unit tests hermetic — which means **nothing exercises real process-group kill** unless you deliberately add a small integration tier. Recommend adding one: it covers the exact failure char exists to prevent. |
| **Coverage** | gated / report-only | The source repo runs report-only; same is probably right here. |

#### 0.4 Done when — ✓ satisfied

`docs/ARCHITECTURE.md` states principles 0.1 and 0.2 **with the rationale kept, not just the
rules**; `AGENTS.md` tells a future agent how to work in the repo; every question in 0.3 has
a recorded answer. **No source files exist yet.**

---

### Phase 1 — Repo skeleton + **six** config fixtures *(must land alone)*

> **✓ Complete.** The record of what it decided, and of every change the six fixtures forced,
> is [`PLAN.md`](PLAN.md) §4.1.1 — including the five things `PLAN.md` specified without
> settling, and three defects the fixtures found in the corpus itself. The config contract is
> frozen from here.

**The workspace root and `xtask/` already exist** — they landed before this phase because the
doc lint they carry checks the corpus this phase codes against, and it found a corrupted JSON
payload and seven dangling cross-references on its first run. Phase 1 adds `crates/core`,
`crates/adapters` and `crates/cli` as members; it does not create the workspace.

Cargo workspace members, `clippy`, `rustfmt`. JSON Schema for `char.yml`. Then write all six configs
from the table in §8.1 under `tests/fixtures/<name>/char.yml`. Tests are schema validation
plus a golden resolved-config snapshot for each. **No runtime.**

**Also ships here: the clean-room enforcement hook.** A `PreToolUse` hook in the repo's
`.claude/settings.json` that default-denies the source-repo path for every agent and allows it
only for phase 3's harvester (`ARCHITECTURE.md` §2.7). It lands in phase 1 rather than phase 3
because a guard added at the moment it is first needed has already been unenforced for every
commit before that.

The schema must cover the full contract, including the parts implemented later:
`components:` (`PLAN.md` §4.1), `commands:` (`PLAN.md` §4.5), `workspaces:` (`PLAN.md` §4.6), and `secrets:` /
`secret_providers:` (`PLAN.md` §4.7). Secrets are **schema-only in this phase** — validated and
resolvable as references, never fetched. Everything after this phase codes against whatever
lands here, so a key missing now is a contract change later.

**`cargo xtask doclint` must stay green**, and it is a gate check. It resolves every `§`
against the heading index, parses every fenced YAML/JSON/shell block, diffs config keys in
examples against keys in prose, and runs `ARCHITECTURE.md` §2.4's contamination grep — reading the pattern out
of `ARCHITECTURE.md` rather than carrying a copy, so there is still exactly one place it lives.
It also runs `ARCHITECTURE.md` §2.4's privacy gate over every tracked file, which is what keeps
the source repo's name and anyone's home directory out of the prose the grep does not cover.
A block that is deliberately unparseable carries `<!-- doclint: skip — reason -->`.

**Done when:** all six are expressible with no escape hatches and no fields invented on the
spot.

**Expect the schema to change while writing them — that is the phase working.** If
`rails-monolith` needs `setup:` to be a list and `polyglot-web` didn't, add it now, before any
code depends on the narrower shape. The fixtures that force a change are the ones earning
their keep; note which ones did, because that record is the argument for keeping them.

**Why alone:** every later agent codes against this contract. Parallel agents cannot share a
decision that has not been made yet — they will each invent an answer and you will get three
incompatible ones.

### Phase 2 — Ownership core: `init`, `clean`, `status`, `commands:`

> **✓ Complete.** What it settled, and what it sends back to `PLAN.md`, are recorded at
> the end of this section. The five things it had to decide — the shape of `Ctx` and the three
> seam traits, the claim loop's own `step()`, where `~/.char/config.toml` is read, the envelope
> renderer and its snapshots, and `char.db`'s DDL — are settled from here, because every later
> phase codes against them.

Workspace id, project id, `.char/`, `~/.char/char.db` with lease-based claiming, resource
labeling, the process-group spawn/kill wrapper, and the scope lens.

**This phase moved ahead of the check engine, and the reason is a dependency, not a
preference.** `char check` is *scoped, scheduled, locked and ceilinged* (`PLAN.md` §3): it writes
`.char/run/<run-id>/{lock,state.json,logs/}`, sets `CHAR_WORKSPACE` and `CHAR_RUN_ID` on
every child (`PLAN.md` §2.4), and reaps old run directories at run start (`PLAN.md` §4.2). Every one of those
depends on workspace resolution, the workspace id and `.char/` — all of which live here. With
the old ordering, the check engine had to either invent its own workspace resolution and run
lock for this phase to replace — the three-incompatible-answers failure §8 warns about — or
ship a `check` that could not lock or scope. `PLAN.md` §2.3 calls ownership "the highest-value
primitive in the project"; it is also the foundational one.

**Done when:** two directories claim non-overlapping blocks concurrently;
`char status --project` from either reports both; and **deleting one directory outright, then
running `char init` in a third, automatically reclaims the deleted one's block, plus containers
and networks created for the test with `docker run --label`** — reported, not silently — without
disturbing the live one. `char clean --orphaned`
does the same on demand.

**And when no process outlives its workspace — tested against an *uncooperative* service.**
`killpg` against a `setsid`'d group is verified to reach grandchildren (`traps.md`), so the
assertion is that a spawned tree of three is zero after the group is killed. **A cooperative
`sleep` passes this while proving nothing**: measured, a leader running `trap '' TERM` leaves
3 of 3 alive after `killpg(SIGTERM)`, because children inherit an ignored disposition across
`fork` and `exec`. So the suite needs three cases — cooperative, SIGTERM-ignoring (must die on
the SIGKILL escalation), and self-`setsid` (must be *detected and reported*, since no `killpg`
can reach it). **Test it by spawning and killing the group directly, not via
`char down`** — `down` and the compose driver are phase 4, so an earlier draft's criteria could
not be run at the end of this phase. For the same reason, the labelled containers the reap pass
removes are created with `docker run --label` in the test rather than by `char up`. Add the neighbouring rule the same test protects: **every spawned child
is waited on or explicitly reaped** — a dropped handle leaves a zombie, and a fifteen-minute
detached run accumulates them.

**And when a lease survives its holder dying:** take a lease, `kill -9` the holder, and
confirm the next claimant reclaims it once the heartbeat goes cold rather than blocking
forever. This is the mechanism ten-minute `char check` runs depend on (`PLAN.md` §4.3), so it needs a
test that kills something.

**And the `commands:` dispatcher (`PLAN.md` §4.5), whole except secret grants.** An earlier
draft shipped the `commands:` *schema* in phase 1 and *consumed* it in phase 6 with no phase
building it; a later one put it in phase 4, the heaviest phase, on the reasoning that
`secrets:` is its last dependency. **That reasoning was backwards** — a grant is a later
addition to a dispatcher, not a prerequisite for one. Everything the dispatcher genuinely needs
is in this phase: the spawn wrapper, port claiming, `clean`, and the run lease `commands:`
entries now take (`PLAN.md` §4.3). Grants arrive in phase 4 and change nothing already shipped.

Moving it here is what makes phase 2.5 possible, and `PLAN.md` §4.5 calls it critical path: it
is the entire mechanism by which the source repo keeps `worktrees` / `tickets` / `design` /
`baselines` while giving up `check` and `servers`. The surface is small but touches several
subsystems:

- transparent argv passthrough and the child's exit code
- `env:` layering over the inherited environment, including `${port.NAME}` substitution
- `stdio:` — `pipe` or `inherit`, defaulting to `pipe` when secrets are granted
- `owns:` **evaluated as a selector** at `clean` time — a distinct code path from reading
  the `owned` table, because a command runs ad hoc and has no "while it was up" window to record
  against

**Done when** subcommands and flags reach the child untouched (`char worktrees prune
--dry-run`), the child's exit code comes back **verbatim and unremapped**, `env:` layers over
the inherited environment, and a declared `owns:` selector is reclaimed by `char clean` after
the command has already exited.

#### What phase 2 settled

Five things `PLAN.md` specified without deciding, and the answers every later phase now codes
against:

| # | Decision | Why it landed there |
|---|---|---|
| 1 | **`Ctx` carries three trait-bound seams**, and `Run` takes a `RunRequest` with an **already-split argv**. | The split — quote handling and `${files}` expansion — is a pure decision, so the seam never re-parses anything and a fake asserts the exact vector. That is the whole reason there are three seams and not six. |
| 2 | **The claim loop's reducer is `step(ClaimState, ClaimEvent) -> (ClaimState, Vec<ClaimAction>)`**, with `Attempt` meaning *re-decide and try*. | `ARCHITECTURE.md` §1.2 gives the scheduler's enums as a floor; this loop needed its own, and deciding it now is what stops phase 3 inventing a second, incompatible one. Losing a port-block race and losing a lease race are the same shape, so one reducer covers both. |
| 3 | **`~/.char/config.toml` is read once, in `adapters::machine`, called from the entrypoint.** Absence is the documented defaults; an unreadable or mistyped file is `environment`. | Keeps phase 1's property that `Defaults` is passed in and never read. `$HOME`, cwd and the environment are read in `main` and nowhere else — which is also what lets the whole suite point char at a `TempDir`. |
| 4 | **One golden snapshot per verb under `tests/golden/`, serialized from structs, redacted for ids and paths, with no update flag.** | Measured: a `serde_json::Value` sorts object keys while struct fields emit in declaration order, so a payload routed through one comes out alphabetised and no hand-written snapshot ever matches. |
| 5 | **`char.db` DDL as `PLAN.md` §4.3 states it**, `user_version = 1`, a namespace UUID written at creation, and `port_from`/`port_to` as two inclusive integer columns. | Leases are keyed `(kind, key)`, which makes cpu-slots and exclusives machine-wide by construction; the run lease's key is therefore the **workspace id**, or five worktrees would contend on one lease. |

Plus one number `PLAN.md` never states: **the port base is 5460**, taken from `PLAN.md` §3.1's own payload
rather than invented, with the ceiling at 32767 so a block never lands inside Linux's ephemeral
range.

#### Three gaps closed after the phase merged

A conformance pass before phase 3 opened found three things the done-whens did
not reach, and closing them cost less than phase 3 inheriting them:

| Gap | Why it was worth closing before phase 3 |
|---|---|
| **`PLAN.md` §3.1's aggregation precedence had no implementation.** `ErrClass::severity()` existed and was unit-tested, while `init` and `clean` each counted rows for themselves. `init`'s version was a live bug: it took the *first* failed row and hardcoded `tool_failed`, so a `setup:` command missing from `PATH` reported exit 1 — whose documented response is "that is a real result, report it" — instead of the `bad_config` exit 3 the caller had to act on. | The rule's stated purpose is that *two implementations cannot disagree*, and there were already two. `check` is where it becomes load-bearing, so phase 3 would have written a third. `envelope::aggregate` is now the only one. |
| **`clean --orphaned --force-rebuild` was a stub** returning `bad_invocation`. | `PLAN.md` §4.3 specifies it as the way out of a `char.db` char cannot read, and *the recovery path must not need the thing that is broken* — so it runs at the entrypoint, before `App` opens anything. It moves the unreadable file aside rather than deleting it, and carries the old namespace across when that is still legible, so resources already stamped stay reapable. The shipped invocation is the one `PLAN.md` spells, `--all` optional: the pass is machine-scoped either way — it enumerates every labelled resource on the daemon and removes across namespaces — so it says that in its own output rather than demanding a flag the corpus does not. `--artifacts` and `--force` *are* refused, having no meaning on a path that reads no `char.yml` and takes no lease. `--dry-run` previews it, replacement database included. |
| **The reap test covered networks and volumes, not containers.** | The code path is parameterised by kind, so a container adds little *on that argument* — which is the point: this corpus is explicit that arguments lose to measurements, and the phase's own wording is `docker run --label`. Gated on obtaining an image, skipped loudly otherwise. |

#### Two defects and one open question phase 2 sends to phase 2.5

Recorded rather than fixed, because phase 2.5 is the only phase licensed to send changes back.

| Defect | Where | What phase 2 did |
|---|---|---|
| **`owned.kind` has no `release` member**, though `PLAN.md` §6.1 requires the resolved `owns.release:` command to be recorded at `char init` into the machine-global store — a workspace-local record would be gone in the orphan case, which is the one that matters. | `PLAN.md` §4.3's `kind` list vs §6.1 | Added `release` as a `kind`. A new kind value is additive, which the 0.x rule already permits; a new table would not have been. |
| **No port base is stated anywhere.** `PLAN.md` §3.1's payload shows `5460-5469`, `PLAN.md` §4.3.1 has `port_block_size` but nothing to add it to. | `PLAN.md` §3.1 vs §4.3.1 | Took 5460 from the payload, as a constant rather than a seventh `config.toml` key — adding a key would have been a contract change. |
| **Should `--force-rebuild` require `--all`?** The recovery is machine-scoped and cross-namespace by nature: it enumerates every labelled resource on the daemon and removes on `ENOENT` whichever installation stamped it, which is the one path `PLAN.md` §2.3.1's namespace filter cannot bound. `PLAN.md` §4.3 spells the invocation without `--all`, so that is what ships and the run states its own scope in `reaped.skipped` and in the `--dry-run` preview. Whether saying so is enough, or the scope should have to be typed, is the open question. **The consequence that sharpens it:** the rebuild writes a *fresh* database, as `PLAN.md` §4.3 specifies, which discards every workspace row and port block on the machine — so live workspaces re-claim on their next `init` and may be handed different ports while their services are still bound to the old ones. | `PLAN.md` §4.3 vs §2.3.1 | Nothing — this is a proposal, and phase 2.5 is the phase with a real repo behind it to answer from. |

### Phase 2.5 — A real repo adopts the ownership layer *(first contact)*

**The source repo takes the dependency for `init` / `clean` / `status` / `commands:` only, and
keeps its own `check.py` and `servers.py` untouched.**

Everything else in this plan is validated against fixtures written by the same people who wrote
the plan. This is the first phase where a repo that was not designed around charkit has to
actually use it, and it happens **three phases earlier than it otherwise would** — phase 6 was
first contact, which meant the ownership model, the port claiming, the reaping and the label
vocabulary all reached a real repo only after `check` and `up`/`down` were built on top of them.

**It is affordable precisely here and nowhere earlier.** The subsystem with the strongest
evidence behind it is this one: the 29-leftover-networks outage (§1 of `PLAN.md`) is an
ownership failure, not a check failure. It is also the subsystem with no incumbent inside the
source repo to fight — `worktrees.py` does part of it badly and knows it. `check.py` and
`servers.py` stay, so nothing the repo depends on daily is at risk.

**Done when:** a source-repo worktree is created and destroyed entirely through `char init` and
`char clean`, five worktrees coexist with non-overlapping port blocks, `char status --project`
reports all five, and **deleting a worktree with `rm -rf` leaves nothing behind that the next
`char init` does not reclaim** — verified with `docker network ls` and `docker volume ls`
filtered by `char.workspace`, not by `docker ps` alone.

**And when the resource half of `worktrees.py` is deleted** — not wrapped, not shimmed. If it
survives as a `commands:` entry doing the same work, this phase proved nothing.

**What it is allowed to send back.** This phase may change `PLAN.md`. It is the first real
evidence the ownership model has ever received, and the fixture set rests on one data point
(§8.1) which this phase is the first chance to correct. A schema change here is cheap; the same
change discovered in phase 6 is not. Record what changed and why, in the same shape §8.1 asks
of the fixtures.

> **This phase does not read the source repo under the clean-room rule** (§11) — it *modifies*
> it, as an ordinary consumer, from the outside. The rule forbids porting that repo's
> implementation into charkit; it does not forbid charkit having a user. Phase 3's harvester is
> still the only agent that may read `scripts/char/` for its contents.

### Phase 3 — Rebuild the check engine, generalized *(clean-room, two agents)*

**This is a clean-room rewrite, not a copy** — see [`ARCHITECTURE.md`](ARCHITECTURE.md) §2.7
for the full reasoning. The scheduler is a reducer and the original's is not, so the hardest
part was being rewritten regardless.

**The language change makes this structural rather than policed.** The source is Python; the
target is Rust. Copying is not merely forbidden, it is impossible — there is no line that
could be pasted across even by an implementer trying to. §8.1 argues that structural
guarantees beat policed ones; this is the strongest form of that available, and it arrived as
a side effect of the language decision rather than by design.

It also changes what "port the test cases" can mean. The assertions survive as **data** —
given this config and these recorded command outputs, expect this verdict — while the harness
is new in every respect. Extract them as a table the Rust suite drives, not as translated
test functions.

| Agent | Reads | Produces |
|---|---|---|
| **Harvester** | The source repo's `scripts/char/`, at the locally configured path (§8.1) | `docs/harvest.md` — a behaviour spec plus **a written list of every trap and bug-shaped branch found**. Plus the ported test *cases*. |
| **Implementer** | this plan, `ARCHITECTURE.md`, the fixtures, the harvest doc, the tests. **Never opens the source repo.** | `crates/` |

The harvest step is mandatory and is the whole reason a rewrite is safe here. The value in
those 3,383 lines is not the code — it is the bug fixes discovered by running against a real
repo, two of which the source flags as "Playwright traps." The uncommented ones are the
danger, and charkit has no continuous real-repo validation until phase 6, so anything lost
would not resurface until then.

Substantively: replace `CHECK_CATALOG` with the config loader, `domain` with `component`, and
strip every source-repo-specific path, turbo filter and interpreter-directory assumption.

**`needs:` on a check gates in this phase and starts in phase 4.** The end state is that a
check needing `postgres` brings it up — one command instead of three, which matters when the
caller is an agent. But `up` does not exist yet, so here a check whose service is not running
fails with `bad_invocation` naming the service and telling the caller to run `char up`. Phase
4 replaces that error with the start. This is one behaviour built in two steps, not two
behaviours.

Two consequences of `check` eventually starting services, which must be handled in phase 4:
anything it starts is recorded as `owned` rows like any other service, so `clean` reclaims
it; and **`check` does not stop what it started.** Stopping would risk killing a service a
sibling workspace is using, which `PLAN.md` §2.2's flat-siblings model exists to prevent, and would
make the next `check` pay startup cost again.

**And when every dispatch writes its record** (`PLAN.md` §4.2): the post-substitution argv,
the env delta by name, the `${files}` set, the leases held and anything waited on with its
holder, the failure signature, **and the reducer's `Event` sequence for the run** — which
replays through `step()` and is therefore the strongest single assertion available here: replay
a recorded run and the resulting `State` must equal the one that was persisted. Written when the check runs — it cannot be recovered later,
and `char explain` in phase 5 is a reader with nothing to read without it.

#### What phase 2 learned that this phase should start with

Four hand-offs, each of them something phase 2 paid for once.

**Give the two agents separate git worktrees.** This phase is *defined* as two
agents working at the same time, and phase 2 ran with two sessions sharing one
checkout: a `git add -A` in one swept another's in-flight files into a commit
whose message described neither, and a branch switch moved the tree under the
agent that was not looking. Nothing was lost, but the recovery cost more than
the setup would have. §8.1 argues that structural guarantees beat policed ones;
`git worktree add` is that argument applied to this phase's own shape. It also
avoids a smaller trap phase 2 hit: `no-mistakes axi respond` resolves its run by
**current branch**, so a stray `git switch` reports "no active run to respond
to" rather than saying you are in the wrong place.

**The ported cases encode the source repo's behaviour, bugs included.** That is
the point of porting them — the value in those lines is the bug fixes discovered
by running against a real repo — but it cuts the other way too, and a ported
assertion that *cannot fail* is invisible while looking like coverage.
[`ARCHITECTURE.md`](ARCHITECTURE.md) §2.1.1 has the general rule; here it is
load-bearing, because the harvest is the only place a quirk can be told from a
fix and the implementer never sees the original.

**Land it as several review-sized PRs, and phase 2 is the evidence.** Phase 2
went in as one, and its review step ran for two hours across three fix rounds
before the first test ever executed. §9's re-measurement already doubled this
phase's harvest; review does not compress with it.

**Two things phase 2 built and deliberately left unwired, for this phase to
connect rather than reinvent.** `waiting_on` and the claim reducer's `Report`
action exist with no consumer, because phase 2 has no run with a `results[]` to
put a `WAITING` row in. `lease::acquisition_order` exists and nothing calls it,
because nothing schedules yet. And the scheduler's own `Event`/`Action` enums
are still unwritten: [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.2 gives their
membership as a **contract floor**, and `crates/core/src/lease.rs` is the worked
precedent for what a reducer looks like in this codebase — including the rule
that there is never a catch-all arm.

**Done when:** the ported suite is green against the phase-1 fixtures, **and** the
contamination grep (§11) returns nothing.

**Start the read-only parallel run here** (§8.1) — it diffs `char check` verdicts against
`scripts/char`, so it cannot begin before this phase — and repeat it at the end of every
subsequent phase.

### Phase 4 — Services: `up` / `down`

Both drivers, five ready-check kinds, `needs:` ordering, `owns:`, everything started
recorded as `owned` rows in `~/.char/char.db` (`PLAN.md` §4.3), port remapping via the generated compose document (`PLAN.md` §6.0). Plus
**secret resolution and injection** (`PLAN.md` §4.7) — this is the phase where there is finally
something to inject into.

**`secrets:` grants on `commands:` entries land here**, since this is the phase where secret
resolution exists. The dispatcher itself shipped in phase 2 — see there for why.

**And the dispatcher's own `owned` pgid row.** Phase 2 writes none for its synchronous child,
so char SIGKILLed mid-dispatch leaves that process group running unrecorded in `PLAN.md` §4.3's
`owned` table, with only the port probe able to find it; closing that needs the `Run` seam to
report a pgid back, which is the same extension `up` needs to spawn and track detached
services.

> **Phase 4 is now the heaviest phase in the plan.** Both drivers, five ready-check kinds,
> compose document generation, secrets, and the dispatcher. Split it across several
> review-sized PRs; `ARCHITECTURE.md` §2.2 already makes review the binding constraint rather than phase
> boundaries.

> **Considered and rejected: moving phase 5 after the source repo's adoption.** Adoption needs
> `check`, `up`/`down`/`clean`, `init` and `commands:` — not the evidence scanner, `agents-md`
> or the MCP server, whose real consumer is phase 8. Reordering would buy real-repo validation
> a phase sooner. It loses on one point, and it is decisive: the source repo would then adopt
> charkit without `config verify`, which `PLAN.md` §5 calls load-bearing. One hand-written
> config could survive that; everything after it would not.

**Done when:** a scratch repo with a bare `docker-compose.yml` plus a long-running command
comes up, gets ready-checked, and tears down completely — `docker ps` and `lsof` clean
afterwards, **and `docker network ls` and `docker volume ls` filtered by `char.workspace` are
both empty.** The last clause is the one that would have caught the bug this criterion was
written without: compose does not propagate service labels to networks or volumes, so a suite
that only checks `docker ps` passes while the founding leak accumulates.

**And when the environment failures are exercised, not just the happy path:** Docker daemon
unreachable reports class `environment` and exit 6 rather than `tool_failed`; a docker call
against a hung socket hits char's own timeout instead of blocking forever; and `char.db`
deleted mid-run is detected via the `user_version` sentinel rather than silently issuing a
duplicate port block.

**And when a `commands:` entry dispatches correctly:** subcommands and flags reach the child
untouched (`char worktrees sweep --dry-run`), the child's exit code comes back, `env:` layers
over the inherited environment, and a declared `owns:` selector is reclaimed by `char clean`
after the command has already exited.

**And when the secret path is proven negatively:** a service is granted a secret from a stub
provider, comes up with the value in its environment, and the value appears in **none** of
`.char/run/*/logs/`, `--json` output on both success and failure, `ps` output while running,
or `~/.char/char.db`. Assert on absence, with the stub returning a distinctive sentinel so
the search is unambiguous.

### Phase 5 — Bootstrap sandwich + `agents-md` + `explain` + MCP *(fans out widest)*

`char config scan` — the layer-1 evidence scanner — is a dozen independent parsers, the most
parallelizable work in the plan, one agent each. Plus schema/example emission,
`char config verify`, the managed AGENTS.md block, and the MCP server.

**`char config scan` must run in a repo with no `char.yml`.** That is the only state it is
ever useful in, and `PLAN.md` §2.1 exempts it from workspace resolution for exactly that reason.

**The MCP server targets `rmcp` v3.x and spec revision `2026-07-28`.** Verified in phase 0 and
recorded in [`traps.md`](traps.md); re-check before starting, because this moved recently.
Three consequences:

- **§9's reference implementation is a dead template.** It is Python, written against a
  pre-2.0 Python SDK; charkit is Rust on `rmcp`. Read it for *what* to expose, never for *how*.
  Re-check `rmcp`'s API before starting: it shipped three major versions in five months
  (`PLAN.md` §10.1), so recall is not current.
- **The base protocol is stateless** — self-contained requests, per-request capability
  negotiation, no session to hold. That happens to suit charkit: `ARCHITECTURE.md` §1.3
  already says a command is *parse → call core → render*, and a stateless server is the same
  shape with a different renderer. There is no session state to design.
- **Use the Tasks extension for `char check`, rather than inventing a polling protocol.** It
  exists for exactly this — asynchronous long-running operations with polling, mid-flight
  input and durable handles — and a real check runs well past ten minutes. Align
  `--detach` / `--status` / `--wait` with it rather than shipping two different
  long-operation idioms for the same run.

**And `char explain` returns the evidence bundle** (`PLAN.md` §3.4). The *verb* lands here
because it reads across everything the earlier phases produce. **The dispatch record it reads
does not** — that is written by phases 3 and 4, at the moment a check or service runs, and
those phases carry it in their own done-whens. An earlier draft put the whole feature here on
the reasoning that it "reads state phases 2–4 already produce", which is false for the part
that matters: leases held, what was waited on and who held it, and the bind state at dispatch
are all point-in-time and unrecoverable afterwards. A phase-5 verb querying for them finds
nothing.
Its own done-when is the history row: **two runs of the same bug produce the same failure
signature, and a different bug in the same check produces a different one.** That is the claim
nothing else in the corpus tests, and it is the one an agent's behaviour changes on.

**Done when:** an agent given only "set up char in this repo" produces a verifying config in
a repo it has never seen — `char config scan` → the agent authors → `char config verify`
passes, with no human in the loop.

### Phase 6 — The source repo adopts it

A PR against the source repo: delete `scripts/char/check.py` and `servers.py`, and move
everything char does not replace into a `commands:` block (`PLAN.md` §4.5).

**The dependency, `bin/char`, the `commands:` block and the whole ownership half are already
there** — phase 2.5 did that, and has been in daily use since. So this phase is narrower than
it looks: it is the `check` and `up`/`down` cutover against a repo that already trusts char
with its worktrees. The table below is the full remaining surface.

**The full dispatch surface, confirmed by inspection rather than assumed:**

| Source-repo command | Subcommands | Disposition |
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
passthrough must be transparent, as `PLAN.md` §4.5 specifies. `bin/char` execs an absolute path
resolved from the git root with no `uv run --directory`, so commands running from the
workspace root need no working-directory key.

**Take the dependency as a locally built binary, not a release.** cargo builds one from the checkout, so this phase does
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

**Done when:** `char check --all-files` is green in the source repo and the worktree flow still
works end to end.

**Expect the most rework here** — six phases of drift surface in this one PR.

### Phase 7 — Publish

Cross-compiled static binaries on GitHub Releases for `darwin`/`linux` × `arm64`/`amd64`, plus
a `~40-line` installer that selects the right one:

```sh
curl -LsSf https://raw.githubusercontent.com/<owner>/charkit/main/install.sh | sh
# detects uname -sm, downloads one small static binary, drops it on PATH
```

**There is no runtime to provision.** One static binary — measured at **2.09 MB stripped** for
a hello-world with `rusqlite` bundled, before `clap`, `serde`, `rmcp` and `tokio`, so treat
that as a floor and re-measure at this phase. The install is a single download and a `chmod`. Publishing to crates.io
is a second channel for people who would rather `cargo install`; a Homebrew tap is a third,
later.

The source repo's git dependency is repointed at a released binary as part of this phase.

**Done when:** a clean machine with no toolchain of any kind runs the one-liner and gets a
working `char`. Verify on a container with neither Rust nor Python installed — the whole point
of the binary is that neither is required.

### Phase 8 — The only test that matters

Adopt char in a repo whose stack the source repo does not share (a multi-language repo),
using only a `char.yml` authored by an agent through the phase-5 sandwich.

**Pass/fail:** if it needs a change to char's own code, the abstraction is wrong.

---

## 9. Source material

The reference implementation is the source repo's `scripts/` directory, at the locally
configured path (§8.1). **It is Python; charkit is Rust** (`PLAN.md` §10.1), so every row below
is a source of *behaviour*, never of code. The line counts indicate how much behaviour there is
to harvest, not how much work the rewrite is.

| Path | Lines | Role in this plan |
|------|------:|-------------------|
| `char/check.py` | 3,383 | Harvest in phase 3. Scope → schedule → run → parse → report, run lock, live table, `--again`. Contains `CHECK_CATALOG` (replace) and load-bearing comments about Playwright traps (translate into the fixture config, not the code). **Only one of the two traps is here — the other is in `baselines.py`, so harvest both.** |
| `char/_shared.py` | 337 | Harvest in phase 3. `run_fn` injection, target resolution, git worktree list. |
| `char/worktrees.py` | 679 | Reference for phase 2. Orphan container/network sweep — note it infers ownership from compose's `working_dir` label; charkit stamps its own instead. |
| `char/servers.py` | 436 | Reference for phase 4. Tilt-shaped; becomes config, not code. |
| `char/__main__.py` | 521 | Reference. Typer dispatch pattern. |
| `char_mcp/server.py` | ~95 | **Do not use as a template.** Written against a pre-2.0 MCP SDK; `FastMCP` no longer exists (`docs/traps.md`). Read it for *what* to expose, never for *how*. |
| `char_test/` | 2,694 | **Harvest in phase 3 — port the cases, rebuild the harness.** `run_fn`-injected, asserts on behavior not implementation — this is the single most valuable asset. Only check-id fixtures should need editing. |
| `char/baselines.py` | 762 | **Not previously listed. Harvest for traps in phase 3 even though the code does not move.** A Playwright snapshot review aid — pixel-diffs darwin/linux snapshot pairs and renders an HTML page for a human. Holds at least one of the two Playwright traps this table attributes to `check.py`: with the default `updateSnapshots: "missing"`, an absent snapshot is *written* and the test *passes*, so a first containerised run reported 29/29 having compared 17 brand-new images against themselves. Couples only to `_shared` (`CheckError`, `RunFn`, `default_run_fn`), so phase 6 inlines three symbols and registers it as a `commands:` entry. |
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
> **Consequence for phase 3:** the harvest is roughly double what the plan assumed. That
> makes the harvest step *more* valuable rather than less — twice the lines means twice the
> uncommented bug fixes to lose in a rewrite — but it is a scope change, and phase 3 should
> land as several review-sized PRs rather than one.
>
> **Re-measure before scoping any phase against this table.** It went stale once already.

---

## 11. Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Overfitting to the source repo** — the abstraction gets shaped around that one repo because it is the only repo the agent has seen. Isolation does *not* prevent this. | **High** | **Six fixture configs in phase 1** (§8.1). This is the single most important guard in the plan. |
| **Six phases of drift surface in phase 6.** Isolation removes continuous real-repo validation. | High | Read-only parallel run against the source repo from phase 3 onward (§8.1). Expect substantial rework in phase 6 regardless. |
| **Crude contamination** — a source-repo path or import follows the code in during phase 3. | Med | Phase-3 acceptance test runs the contamination grep — **defined only in `ARCHITECTURE.md` §2.4**, because a copy inside a markdown table is both unrunnable and a second thing to keep in sync — and it must return nothing. Plus a PreToolUse hook that denies the source-repo path to every agent but the harvester. **Only phase 3's harvester has source-repo access.** |
| **Config expressiveness pressure** once a second repo lands. | Med | Four substitutions plus two scoped placeholders, hard cap (`PLAN.md` §4.4). Escape hatch is a generator script. |
| **Machine-global state corruption** with several agents claiming or renewing leases simultaneously. | Low | SQLite transactions (`PLAN.md` §4.3). Was Med when this was a JSON file rewritten under an `O_EXCL` lockfile; ten-minute runs renewing heartbeats made that write pattern the contended path, which is why the store changed. |
| **`curl \| sh` is a trust ask** and some environments block it. | Low | `uvx` and `pipx` cover anyone who will not run it. Publish the script's source in-repo. |

---

## 12. Notes for the implementing agent

- **Phase 0 produces documents only.** Phase 1 lands alone and ships all **six** fixtures.
  Do not fan out until the config contract is committed.
- **Only phase 3's harvester has access to the source repo.** Every other phase works from
  this document and the fixtures. If a later phase feels like it needs to look at that repo,
  that is a signal the plan is underspecified — fix the plan, don't peek.
- Phases 2 and 4 parallelize moderately; **phase 5's evidence scanner is the widest fan-out**
  (one agent per parser).
- What does *not* compress with more agents: live verification (containers start at the speed
  they start) and human review of each PR. Review is the binding constraint.
- Every phase is a normal branch + PR. TDD throughout: failing test → minimal implementation
  → passing test.
- The check engine's tests use injected `run_fn` rather than real subprocesses — preserve
  that pattern; it is why the suite is fast and hermetic.
