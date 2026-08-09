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

Built **greenfield in its own repo**, not extracted incrementally through Chariot.

### 8.1 Why greenfield, and the anti-contamination strategy

The goal is that charkit carries no Chariot assumptions. Two distinct risks, and they need
different defenses:

| Contamination | Looks like | Defense |
|---|---|---|
| **Crude** | A hardcoded `backend/` path, a `tilt` import, a `.claude/worktrees` assumption | Greenfield makes it *structurally impossible* — the agent cannot see Chariot except in phase 3's harvest. Backed by a grep gate. |
| **Subtle** | An abstraction shaped around Django+Next because that is the only repo the agent ever saw | **Six fixture configs in phase 1.** Isolation does nothing here — an agent given one example generalizes from n=1 regardless of repo topology. |

**The fixture set is the more important of the two and is non-optional.** Write all six
before any code exists. An agent cannot overfit to Django+Next if the schema must also
express five other shapes on day one.

**The constraint that makes a fixture set useful:** every fixture must be able to fail the
schema in a way no other fixture can. Five Node monorepos teach nothing — they all pass or
all fail together. If adding a fixture creates no new way to be wrong, it is decoration.

| Fixture | Axis it owns | Failure it catches that nothing else does |
|---|---|---|
| `django-next` *(real)* | Maximal case — polyglot monorepo, supervisor, checks running *inside* containers, 3s→15min cost spread, **and the only fixture with a `commands:` block** (`PLAN.md` §4.5) | Schema can't express a real complex repo; `commands:` unexercised until phase 6, when it is load-bearing |
| `multi-lang` *(representative)* | A genuinely different runtime pairing | Abstraction is Django/Next-shaped |
| `go-service` | Low end — one component, one binary, one Postgres, no monorepo, **plus one secret from one provider** (`PLAN.md` §4.7) | **Over-structuring.** A trivial repo needing 40 lines of config — and secrets that only work in a complex config are secrets nobody will adopt |
| `pnpm-monorepo` | Many components, **zero** services, turbo already present, **plus a declared nested workspace** (`PLAN.md` §4.6) — so the fixture is a root manifest *and* a nested `char.yml` | Component-per-package globbing; also honestly answers "is char redundant where turbo exists?" Additionally: overlap detection, manifest-only roots, and discovery returning the same answer from any depth |
| `rails-monolith` | `setup:` as a *sequence* (bundle → db:create → migrate → seed) including a step needing `shell: true`; two services with real dependency ordering; **`owns.release:` for a database on a shared server** | `setup:` modeled as a single string; `needs:` ordering that only works for one service; a setup step that errors when its resource exists; setup that creates something `clean` cannot reach |
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

> **Read-only parallel run, from phase 3 onward.** Point charkit at the Chariot checkout,
> run `char check`, and diff the verdicts against `scripts/char`'s output. This is *not* a
> Chariot dependency and *not* a Chariot PR — zero risk to Chariot's merge gate — but it
> restores most of the continuous validation and turns phase 6 from a cliff into a
> formality. Do this at the end of every phase from 3 on.

### Phase 0 — Foundations *(human + agent, working session, no code)*

**Output:** `docs/ARCHITECTURE.md`, `AGENTS.md`, and a `CONTRIBUTING`-style README section.
Nothing else. This is a conversation that produces documents, not a build step.

**Why this is first, and not skipped:** it is the third anti-contamination defense, and it
catches what the other two miss. Chariot's `check.py` has a *structure* — 3,383 lines of it.
Without stated architecture principles, phase 3's port inherits that structure by default,
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
| 4 | **Merge gate = lint + typecheck + tests + the contamination grep** | The grep is the phase-3 acceptance test made permanent |
| 5 | **Semver from the first publish, with a changelog** | Cheap now, painful to retrofit once anything depends on it |
| 6 | **Dogfood from phase 3 onward** | charkit gets its own `char.yml` and gates itself with itself the moment `char check` runs. Strongest available forcing function, and it makes the README example real rather than illustrative. |

#### 0.3 Decisions genuinely yours — bring answers to the session

| Question | Options | Consideration |
|---|---|---|
| **Public or private repo?** | public / private-for-now | Changes whether CI is free, whether the license matters, how much README polish is warranted. Private → public later is easy; the reverse is not. |
| **License, if public** | Apache-2.0 / MIT / none | Apache-2.0 adds a patent grant and clears corporate legal at no adoption cost. Only matters if public. |
| **CI** | GitHub Actions / local gate only / both | Actions is free for public repos, and unlike Chariot there is no billing constraint. Real CI matters more for a package other repos depend on. |
| **Typing strictness** | mypy strict / basic / none | Strict from commit one is cheap; retrofitting onto 3,000 lines is not. |
| **Rust edition / MSRV** | 2021 edition | Pin an MSRV in `Cargo.toml` and raise it deliberately. Users are unaffected either way — they receive a static binary, so no toolchain is required to run `char`. |
| **Test layers** | unit only / unit + a real-subprocess integration tier | Principle 1 makes unit tests hermetic — which means **nothing exercises real process-group kill** unless you deliberately add a small integration tier. Recommend adding one: it covers the exact failure char exists to prevent. |
| **Coverage** | gated / report-only | Chariot runs report-only; same is probably right here. |

#### 0.4 Done when — ✓ satisfied

`docs/ARCHITECTURE.md` states principles 0.1 and 0.2 **with the rationale kept, not just the
rules**; `AGENTS.md` tells a future agent how to work in the repo; every question in 0.3 has
a recorded answer. **No source files exist yet.**

---

### Phase 1 — Repo skeleton + **six** config fixtures *(must land alone)*

Cargo workspace scaffolding, `clippy`, `rustfmt`. JSON Schema for `char.yml`. Then write all six configs
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

**Done when:** all six are expressible with no escape hatches and no fields invented on the
spot.

**Expect the schema to change while writing them — that is the phase working.** If
`rails-monolith` needs `setup:` to be a list and `django-next` didn't, add it now, before any
code depends on the narrower shape. The fixtures that force a change are the ones earning
their keep; note which ones did, because that record is the argument for keeping them.

**Why alone:** every later agent codes against this contract. Parallel agents cannot share a
decision that has not been made yet — they will each invent an answer and you will get three
incompatible ones.

### Phase 2 — Ownership core: `init`, `clean`, `status`

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

**And when no process outlives its workspace.** `killpg` against a `setsid`'d group is verified
to reach grandchildren (`traps.md`), so the assertion is that a spawned tree of three is zero
after the group is killed. **Test it by spawning and killing the group directly, not via
`char down`** — `down` and the compose driver are phase 4, so an earlier draft's criteria could
not be run at the end of this phase. For the same reason, the labelled containers the reap pass
removes are created with `docker run --label` in the test rather than by `char up`. Add the neighbouring rule the same test protects: **every spawned child
is waited on or explicitly reaped** — a dropped handle leaves a zombie, and a fifteen-minute
detached run accumulates them.

**And when a lease survives its holder dying:** take a lease, `kill -9` the holder, and
confirm the next claimant reclaims it once the heartbeat goes cold rather than blocking
forever. This is the mechanism ten-minute `char check` runs depend on (`PLAN.md` §4.3), so it needs a
test that kills something.

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
| **Harvester** | `~/Development/chariot/scripts/char/` | `docs/harvest.md` — a behaviour spec plus **a written list of every trap and bug-shaped branch found**. Plus the ported test *cases*. |
| **Implementer** | this plan, `ARCHITECTURE.md`, the fixtures, the harvest doc, the tests. **Never opens the Chariot repo.** | `src/` |

The harvest step is mandatory and is the whole reason a rewrite is safe here. The value in
those 3,383 lines is not the code — it is the bug fixes discovered by running against a real
repo, two of which the source flags as "Playwright traps." The uncommented ones are the
danger, and charkit has no continuous real-repo validation until phase 6, so anything lost
would not resurface until then.

Substantively: replace `CHECK_CATALOG` with the config loader, `domain` with `component`, and
strip every Chariot-specific path, turbo filter and interpreter-directory assumption.

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

**And the `commands:` dispatcher (`PLAN.md` §4.5), whole.** An earlier draft of this plan shipped the
`commands:` *schema* in phase 1 and *consumed* it in phase 6 without any phase building it —
for a feature `PLAN.md` §4.5 itself calls critical path, since it is the entire mechanism by which
Chariot keeps `worktrees` / `tickets` / `design` / `baselines` while giving up `check` and
`servers`. The surface is small but touches several subsystems:

- transparent argv passthrough and the child's exit code
- `env:` layering over the inherited environment, including `${port.NAME}` substitution
- `stdio:` — `pipe` or `inherit`, defaulting to `pipe` when secrets are granted
- `secrets:` grants
- `owns:` **evaluated as a selector** at `clean` time — a distinct code path from reading
  the `owned` table, because a command runs ad hoc and has no "while it was up" window to record
  against

It lands here rather than in phase 2 because `secrets:` is its last dependency and arrives in
this phase. Everything else it needs — port claiming, the spawn wrapper, `clean` — exists by
the end of phase 2.

> **Phase 4 is now the heaviest phase in the plan.** Both drivers, five ready-check kinds,
> compose document generation, secrets, and the dispatcher. Split it across several
> review-sized PRs; `ARCHITECTURE.md` §2.2 already makes review the binding constraint rather than phase
> boundaries.

> **Considered and rejected: moving phase 5 after Chariot adoption.** Adoption needs `check`,
> `up`/`down`/`clean`, `init` and `commands:` — not the evidence scanner, `agents-md` or the
> MCP server, whose real consumer is phase 8. Reordering would buy real-repo validation a
> phase sooner. It loses on one point, and it is decisive: Chariot would then adopt charkit
> without `config verify`, which `PLAN.md` §5 calls load-bearing. One hand-written config could survive
> that; everything after it would not.

**Done when:** a scratch repo with a bare `docker-compose.yml` plus a long-running command
comes up, gets ready-checked, and tears down completely — `docker ps` and `lsof` clean
afterwards.

**And when a `commands:` entry dispatches correctly:** subcommands and flags reach the child
untouched (`char worktrees sweep --dry-run`), the child's exit code comes back, `env:` layers
over the inherited environment, and a declared `owns:` selector is reclaimed by `char clean`
after the command has already exited.

**And when the secret path is proven negatively:** a service is granted a secret from a stub
provider, comes up with the value in its environment, and the value appears in **none** of
`.char/run/*/logs/`, `--json` output on both success and failure, `ps` output while running,
or `~/.char/char.db`. Assert on absence, with the stub returning a distinctive sentinel so
the search is unambiguous.

### Phase 5 — Bootstrap sandwich + `agents-md` + MCP *(fans out widest)*

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

**Done when:** an agent given only "set up char in this repo" produces a verifying config in
a repo it has never seen — `char config scan` → the agent authors → `char config verify`
passes, with no human in the loop.

### Phase 6 — Chariot adopts it

A Chariot PR: delete `scripts/char/check.py` and `servers.py`, take the dependency, move
everything char does not replace into a `commands:` block (`PLAN.md` §4.5), repoint `bin/char`.

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

**Done when:** `char check --all` is green in Chariot and the worktree flow still works end
to end.

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

Chariot's git dependency is repointed at a released binary as part of this phase.

**Done when:** a clean machine with no toolchain of any kind runs the one-liner and gets a
working `char`. Verify on a container with neither Rust nor Python installed — the whole point
of the binary is that neither is required.

### Phase 8 — The only test that matters

Adopt char in a repo that is *not* Django + Next (a multi-language repo), using only a
`char.yml` authored by an agent through the phase-5 sandwich.

**Pass/fail:** if it needs a change to char's own code, the abstraction is wrong.

---

## 9. Source material

The reference implementation lives at `~/Development/chariot/scripts/`. **It is Python;
charkit is Rust** (`PLAN.md` §10.1), so every row below is a source of *behaviour*, never of code. The
line counts indicate how much behaviour there is to harvest, not how much work the rewrite is.

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
| **Overfitting to Chariot** — the abstraction gets shaped around Django+Next because it is the only repo the agent has seen. Isolation does *not* prevent this. | **High** | **Six fixture configs in phase 1** (§8.1). This is the single most important guard in the plan. |
| **Six phases of drift surface in phase 6.** Isolation removes continuous real-repo validation. | High | Read-only parallel run against Chariot from phase 3 onward (§8.1). Expect substantial rework in phase 6 regardless. |
| **Crude contamination** — a Chariot path or import follows the code in during phase 3. | Med | Phase-3 acceptance test is a literal `grep -riE "chariot\|tilt\|NEXT_PUBLIC\|\.claude\|backend/\|web/" src/ tests/` returning nothing, plus a PreToolUse hook that denies the source-repo path to every agent but the harvester. **Only phase 3's harvester has Chariot access.** |
| **Config expressiveness pressure** once a second repo lands. | Med | Four substitutions plus two scoped placeholders, hard cap (`PLAN.md` §4.4). Escape hatch is a generator script. |
| **Machine-global state corruption** with several agents claiming or renewing leases simultaneously. | Low | SQLite transactions (`PLAN.md` §4.3). Was Med when this was a JSON file rewritten under an `O_EXCL` lockfile; ten-minute runs renewing heartbeats made that write pattern the contended path, which is why the store changed. |
| **`curl \| sh` is a trust ask** and some environments block it. | Low | `uvx` and `pipx` cover anyone who will not run it. Publish the script's source in-repo. |

---

## 12. Notes for the implementing agent

- **Phase 0 produces documents only.** Phase 1 lands alone and ships all **six** fixtures.
  Do not fan out until the config contract is committed.
- **Only phase 3's harvester has access to the Chariot repo.** Every other phase works from
  this document and the fixtures. If a later phase feels like it needs to look at Chariot,
  that is a signal the plan is underspecified — fix the plan, don't peek.
- Phases 2 and 4 parallelize moderately; **phase 5's evidence scanner is the widest fan-out**
  (one agent per parser).
- What does *not* compress with more agents: live verification (containers start at the speed
  they start) and human review of each PR. Review is the binding constraint.
- Every phase is a normal branch + PR. TDD throughout: failing test → minimal implementation
  → passing test.
- The check engine's tests use injected `run_fn` rather than real subprocesses — preserve
  that pattern; it is why the suite is fast and hermetic.
