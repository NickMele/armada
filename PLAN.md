# PLAN — The Job daemon, stage one: the mechanical half

Implements [`docs/reserved/034-the-job-daemon-lands-the-work.md`](docs/reserved/034-the-job-daemon-lands-the-work.md),
stage one only — the eight items task `56bb1535` lists in dependency order. Stage one spends no
tokens: it pushes, opens, watches, merges and reaps, all on mechanical conditions. It does **not**
resume a Drone (034 §6.1's fleet-wide budget is stage two), does not rebase, and does not draw the
trail in the Bridge (`crates/helm/src/bridge.rs`/`render.rs` are being edited right now by Job
`wire-bridge-84`; this plan writes the data and exposes it through `fleet show` only).

**The bootstrap, restated from 034 §7**: this Job cannot use the daemon to land itself, because
the daemon does not exist until this Job lands. A person merges stage one's PR by hand — expected,
not a gap.

## What already exists vs. what's new

| Piece | State | Detail |
|---|---|---|
| A daemon, any daemon | **missing** | `rg -i 'daemon\|launchd\|plist\|systemd'` across `crates/` finds only prose explaining why Armada has *none* (`crates/helm/src/args.rs:954`, `crates/helm/src/verbs/fleet.rs:3505-3512`, `crates/core/src/fleet/advance.rs:102`, `docs/glossary.md:17`, `docs/PLAN.md:1925-1946` — the last already carries a correction dated 2026-08-17 citing 034 by name). Stage one is greenfield for process supervision. |
| `armada fleet tick` | done, and the template | `crates/helm/src/verbs/fleet.rs:3513` — the needs→gather→decide→after loop this plan's landing pass mirrors. Its own doc comment (fleet.rs:3505-3512) is the "why not a daemon" argument 034 reverses; that comment goes stale the moment this lands and needs a line pointing at 034, same as `docs/PLAN.md`'s correction already does. |
| `Job.transitions` / `Job.progress` | done, and explicitly not reusable | `crates/core/src/fleet/job.rs:119-120,158-159`. Doc comments already distinguish them from each other; 034 §6.5 extends the same argument to rule out both for the daemon's trail — a third field is required, not a repurposing of either. |
| `Job.facts: BTreeMap<String,String>` | done, and **not** where "main moved" goes | `crates/core/src/fleet/job.rs:181` — doc comment: *"Seeded once, before anything runs"*, for `${task.*}` substitution at spawn. Writing into it mid-run contradicts its own contract. §7 below gets its own field. |
| `~/.armada/inbox.jsonl` / `failures.jsonl` | done, the pattern to reuse | `crates/fleet/src/inbox.rs` (`Line` enum, `append()`, `read()`/fold-by-id, `home.rs:91-93` for the path) and `crates/core/src/failure.rs` (`fold()`, `fingerprint()`). No locking; atomicity is append-mode + tolerating a torn last line. `daemon.jsonl` follows this exactly. |
| Gate predicates | 8 of 10 | `crates/core/src/fleet/gate.rs` / `workflow.rs:90-101`. `branch_exists`'s `Needs::Branch` (gate.rs ~343, ~653-664) is the closest existing shape to `pr_open`/`pr_merged`'s `Needs::Pr`. |
| `armada.yml`'s `fleet:` section | **missing, but the schema already expects it** | `crates/core/schema/armada.schema.json:18` and `crates/core/src/config/model.rs:60-68` both say, in so many words, that a sibling section is coming and the nesting exists "so that adding one never has to move every key that is already there." Nobody has added it. |
| `crates/fleet/src/machine.rs` | done, the template for the daemon switch | Owns machine.yml's `fleet:` section today — `{ carry: BTreeMap<String, Vec<String>> }` (machine.rs:49-55) — with the identical read/write/preserve-other-sections discipline `crates/helm/src/machine.rs`'s `HelmSection` uses. This is where `daemon:` joins, not a new file. |
| `armada doctor` | done, and the template for the daemon check | `crates/helm/src/verbs/doctor.rs:51-83` builds an ordered `Vec<Finding>`; `helm_argv` (doctor.rs:309) already reads `machine.yml`'s `helm.enter` and reports it as a Finding — the daemon check is one more push in the same shape, reading `daemon.jsonl` instead. |
| `gh` CLI | **no precedent** | Confirmed by `rg 'gh pr\|gh api'` — the only hit is 034's own doc. Shelling to `gh` is new, and it is the daemon's alone: `land-branch/SKILL.md` (already amended for 034) keeps `git push`/`git remote` denied to a Drone. |

## Why this order

Items 1–8 below are the task's own order, and it is load-bearing, not incidental:

- **The audit trail (2) comes before push/PR (3)**, because item 3's own requirement — *"a
  repository with no remote fails legibly at the land step, not inside the daemon"* — has no
  mechanism to satisfy it until the trail exists. A push that fails writes a `daemon_acts` entry
  with an outcome of `failed: no remote configured`, and `fleet show` is what makes that legible.
  Build the pipe before the thing that needs to flow through it.
- **The gate predicates (5) come after `fleet.land.merge` (4)**, because `pr_open`'s `decide()`
  arm is policy-aware (see §5) and needs somewhere to read the policy from.
- **Everything through (5) has to exist before (6)**, because merging on green is the first
  irreversible daemon action and needs the trail, the PR, and the gate all already working so its
  own outcome can be recorded and verified.

## 1. `armada daemon` — process, enable/disable/status, launchd

**Machine switch** — `crates/fleet/src/machine.rs`'s `FleetSection` gains a nested field,
following the exact pattern `carry` already sets:

```rust
pub struct FleetSection {
    #[serde(default)]
    pub carry: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub daemon: DaemonSwitch,
}

#[derive(Default)]
pub struct DaemonSwitch {
    #[serde(default)]
    pub enter: bool,   // off by default — a fresh install must not act unattended
}
```

This is the same field name (`enter`) `crates/helm/src/machine.rs`'s `HelmSection` uses, and for
its stated reason: *"whether this box may act unattended is a fact about the box."* `read`/`write`
already preserve unknown sections; adding a field to a section this module owns needs no new
preserve-logic, only a new `#[serde(default)]` field (the same additive discipline `job.rs` uses
throughout).

**Process module** — new `crates/fleet/src/daemon.rs`:
- `pidfile(armada_home) -> PathBuf` (`home.rs`-style one-liner, `~/.armada/daemon.pid`).
- `is_running(armada_home) -> Option<u32>` — reads the pidfile, checks liveness with the same
  `pgid_is_live`-family check `crates/fleet/src/own.rs`/`crates/helm/src/verbs/status.rs` already
  use for Drones. A stale pidfile (process gone) reads as not running, not as an error — same
  fail-safe direction `crates/helm/src/machine.rs:122-130` documents for `read`.
- `start(armada_home)` / `stop(armada_home)` — detached spawn reusing the `setsid` shape
  `crates/fleet/src/drone.rs` already uses for a Drone, running a new hidden `armada daemon run`
  entry point (the actual watch loop, §6). Writes/removes the pidfile. Every start/stop appends a
  `daemon.jsonl` line (§2) — **the first daemon act ever recorded is starting**, matching 034
  §6.5's "written from the first action."

**launchd (macOS only)** — `#[cfg(target_os = "macos")]`. A plist template at
`~/Library/LaunchAgents/com.armada.daemon.plist` running `armada daemon run`,
`RunAtLoad`/`KeepAlive` true, installed with `launchctl bootstrap`/`load` on `enable`, removed with
`unload` on `disable`. **On any other OS, `armada daemon enable` refuses legibly** — `bad_config`
or an equivalent named error, not a silent no-op — since 034 only asks for macOS in stage one and
a switch that claims success while doing nothing is exactly the silent-stall shape 034 exists to
end.

**CLI verbs** — new `crates/helm/src/verbs/daemon.rs`, modeled on `crates/helm/src/verbs/helm.rs`'s
`enable()`/`disable()`/shared `switch()` (helm.rs:198-230) for the machine.yml half, plus a new
`status()` that reports both the switch (`DaemonSwitch.enter`) and the live process
(`daemon::is_running`) — two facts, since a switch that is on with no live process is exactly the
gap `armada doctor` (§8) exists to surface. `crates/helm/src/args.rs` gains a `daemon` arm
alongside `"doctor"`/`"helm"` (args.rs:1169-1172), and `fn daemon()` peeks `rest.first()` for
`enable`/`disable`/`status`/`run`, mirroring `fn helm()` (args.rs:1717-1791) — **note `helm` has no
`status` verb** (status lives in `armada doctor` for Helm), so `daemon status` is new shape, not a
copy-paste. `main.rs` dispatch follows `Invocation::HelmEnable`/`Disable` (main.rs:447-452).

## 2. The audit trail — before anything it will record

**On the Job** — `crates/core/src/fleet/job.rs` gains one field, additive:

```rust
/// Every act the daemon has taken about this Job (`034` §6.5).
///
/// Not `transitions` — a push is not a state change. Not `progress` — a
/// second writer there makes "who said this" unanswerable. A third voice,
/// because it is a third kind of fact: not a step boundary, not the Drone's
/// own words, but something Armada itself did *to* this Job unattended.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub daemon_acts: Vec<DaemonAct>,
```

```rust
pub struct DaemonAct {
    pub id: String,               // ties an outcome update to its intent row
    pub at: String, pub at_ms: u64,
    pub act: DaemonActKind,       // Pushed, Opened, ChecksGreen, Merged, Pulled,
                                   // ReRan, Reaped, MarkedMainMoved,
                                   // ReportedFailure, RefusedToMerge
    pub target: String,           // branch, PR number, or run id
    pub outcome: Option<DaemonOutcome>,   // None until the act settles
    pub outcome_at: Option<String>,
}
```

Two new functions on `Job`, both following `record()`'s existing read-compare-write shape
(job.rs:684-713): `begin_daemon_act(&mut self, act, target) -> String` (pushes an entry with
`outcome: None`, returns the id) and `settle_daemon_act(&mut self, id, outcome)` (finds the entry
by id, fills `outcome`/`outcome_at`). **Write the intent before the irreversible action, the
outcome after** — 034 §6.5's own words — so a crash mid-merge leaves a `Merged` act with no
outcome rather than nothing at all, which is the one case an audit trail exists for.

**`~/.armada/daemon.jsonl`** — for what is about no Job: the daemon starting, stopping, reaching
its (stage-two) limit, failing to reach `gh`. New `crates/fleet/src/daemon_log.rs`, the identical
shape `inbox.rs` and `failure.rs` already use:

```rust
enum Line { Started { at, at_ms, pid }, Stopped { at, at_ms, reason },
            GhUnreachable { at, at_ms, detail } }
fn append(armada_home: &Path, line: &Line) -> std::io::Result<()>   // OpenOptions append, no lock
fn fold(text: &str) -> Vec<Entry>                                  // skip unparseable lines
fn last(entries: &[Entry]) -> Option<&Entry>                        // for `armada doctor` (§8)
```

`home.rs` gains `daemon_log(armada_home) -> PathBuf` (`armada_home.join("daemon.jsonl")`),
one-liner, matching `inbox`/`tick_lock`/`worktree` (home.rs:75-93).

**Surfaced by `fleet show`** — `crates/core/src/envelope.rs`'s `ShowData` (2332-2453) gains
`pub daemon_acts: Vec<DaemonActRow>`, built in `crates/helm/src/verbs/fleet.rs`'s `show()`
(~1502-1523) alongside `progress`/`transitions`, from `record.daemon_acts` directly — no new I/O,
the field is already on the loaded `Job`. This is the whole of what stage one owes the Bridge:
Job `wire-bridge-84` draws it later, from data that already exists on `ShowData`.

## 3. Push and PR, from the Job's own record

New `crates/fleet/src/land.rs` (impure — shells to `git`/`gh`), functions:
- `push(worktree, branch) -> Result<(), LandError>` — `git push -u origin <branch>`. `LandError`
  distinguishes *no remote configured* from *push rejected* from *`gh`/`git` not on `PATH`*, because
  the first is 034's "fails legibly" case and the others are ordinary transient failures.
- `open_pr(worktree, branch, &Job) -> Result<PrHandle, LandError>` — `gh pr create --title … --body
  …`. The body is built by a pure helper (`crates/core/src/fleet/land.rs` or a function in
  `job.rs` itself), from `record.task`, the `plan` step's `artifact_exists: PLAN.md` content (read
  from the worktree — this plan's own text, for every Job after this one), and the transitions log
  filtered to `PASS` events — **not** a Drone's own summary, which is 034 §4's whole reason the
  daemon pushes instead of the Drone.

**On failure, no silent daemon-only log.** A push/open that fails calls
`Job::begin_daemon_act(Pushed, branch)` immediately (intent), and
`settle_daemon_act(id, Failed("no remote configured"))` (or the specific `LandError`) as soon as it
knows — recorded on the **Job**, so `armada fleet show <job>` names it, satisfying 034's "fails
legibly at the land step" without inventing a second failure channel. `daemon.jsonl` only gets a
line if `gh`/`git` themselves are unreachable (a machine fact, not a Job fact).

## 4. `armada.yml`'s `fleet:` section

**Schema** — `crates/core/schema/armada.schema.json`: add `fleet` to top-level `properties` (it
stays out of `required`, since absence is meaningful — see below), and a new `$defs/fleet`:

```json
"fleet": {
  "additionalProperties": false,
  "properties": {
    "land": {
      "additionalProperties": false,
      "properties": { "merge": { "enum": ["auto", "never"] } }
    }
  }
}
```

**Model** — `crates/core/src/config/model.rs`: `Document` (60-74) gains
`pub fleet: Option<FleetSection>`, a small `FleetSection { land: Option<LandSection> }` /
`LandSection { merge: Option<LandMerge> }` / `enum LandMerge { Auto, Never }`, all
`#[serde(deny_unknown_fields)]` like `Document` already is.

**The boundary that keeps Manifest ignorant** — `crates/core/src/config/resolve.rs:77-79`'s
`parse()` today does `from_str::<Document>(text).map(|d| d.manifest)`, discarding everything else.
**That line does not change.** A new sibling function, `resolve::land_merge(text: &str) ->
LandMerge` (defaulting `Auto`/`Never`-absent-and-unparseable to `Never` per 034 §6.4 — *"never must
be the default when the section is absent"*), lives beside `parse()` in `armada_core::config` and
is called **only** from `crates/fleet` (the daemon reading its own policy). `crates/manifest`
never gains a new import, `armada_core::config::resolve::parse`'s manifest-only unwrap never
widens, and `xtask boundaries` staying silent about this is expected, not a gap — see the risk
section below.

## 5. `pr_open` / `pr_merged`

`crates/core/src/fleet/workflow.rs`: `Predicate` gains `PrOpen`, `PrMerged` (90-101's word table:
`"pr_open"`, `"pr_merged"`). `crates/core/src/fleet/gate.rs`: `Needs` gains `Pr`; `Facts` (451-466)
gains `pub pr: Option<PrFact>` where `PrFact { number: u64, open: bool, merged: bool }`. `needs()`
maps both new predicates to `Needs::Pr`. The impure gather in `crates/helm/src/verbs/fleet.rs`'s
`gather()` (called from `tick`'s `pass`) runs `gh pr view <branch> --json state,mergedAt` when it
sees `Needs::Pr`.

**The per-repository resolution, decided here rather than left open.** Workflows are guild data,
re-read fresh every tick by name (`workflow.rs`'s own doc: *"a file in your guild… syncs between
machines"*) — the same `land` step's YAML is shared across every repository a person's guild
touches, so the step's `must:` word in `templates/guild/workflows/{feature,bug}.yml` **cannot
differ by repository as literal text**, and does not need to: it changes to `must: pr_open`,
unconditionally, in both starter templates. What differs per repository is what `pr_open` *requires*:
`decide()`'s `Predicate::PrOpen` arm reads the resolved `fleet.land.merge` (§4's `land_merge()`,
threaded in as a new parameter to `decide()` — the one place this module stops being config-blind,
and only for this one predicate) and holds on "open or merged" under `never`, "merged" under
`auto`. This exactly matches 034's own sentence — *"`auto` lands on `pr_merged`, `never` lands on
`pr_open`"* — as an outcome, while keeping the shipped YAML identical for every repository.
`Predicate::PrMerged` ships as a second, unconditional, always-strict predicate (`decide()` holds
only on `merged`) — not used by either starter template, available to a guild author who writes a
custom step that must wait for a real merge regardless of repository policy.

Tests: `needs()`/`decide()` unit tests mirroring `branch_exists`'s (gate.rs ~978-986,
~1213-1241) — `pr_open` holds on an open unmerged PR under `never` and does not hold under `auto`
until merged; `pr_merged` holds only on `merged` regardless of policy. Each written failing first
against today's `gate.rs`, which has no `Needs::Pr` arm to hold at all.

## 6. Merge on green → pull → re-run → reap

New orchestration in `crates/fleet/src/land.rs`, run by the daemon's own loop (`armada daemon run`,
§1) rather than `armada fleet tick` — **tick's own doc comment (fleet.rs:3505-3512) is the "why not
a daemon" argument this plan reverses; it needs a line pointing at 034 alongside the correction
`docs/PLAN.md:1939-1946` already carries**, or a future reader hits two documents disagreeing,
which `docs/glossary.md` exists to prevent (034 §2's own opening argument).

Per Job at the `land` step with an open PR and `fleet.land.merge == Auto`, once its checks read all
green (`gh pr checks <branch>`):
1. `begin_daemon_act(Merged, pr_number)`; `gh pr merge --merge` (or squash — **open question,
   flagged below**); `settle_daemon_act` with the exit.
2. Pull `main` in the shared clone Jobs branch from (**needs the exact path — open question**):
   `git fetch origin main:main` (or checkout+pull if the ref cannot fast-forward), recorded as
   `Pulled`.
3. Re-run checks on the updated `main` — reusing the existing `armada manifest check --detach`
   shell (`gate.rs`'s doc: *"the shell… starts an `armada manifest check --detach`"*), recorded as
   `ReRan`. **This is the step that catches two green-alone PRs that are not green together** — 034
   §3's own reason for it not being redundant.
4. If the re-run is also green: reap this Job's worktree — `crates/fleet/src/worktree.rs`'s
   `holds_uncommitted_work` guard still applies (a merged branch's worktree should be clean, but
   the guard is not skipped on that assumption), recorded as `Reaped`.
5. If the re-run is **red**: this is new information a green PR did not carry. Stage one's answer
   matches its own boundary — **record `ReportedFailure` and raise one inbox entry; no resume**,
   the identical shape 034's CI-failure case takes, not a special case.

`fleet.land.merge == Never`: steps stop after `pr_open` holds (§5) — the daemon still pushed,
opened, and watches checks, and reports failures the same way, it simply never reaches step 1
above. Matches 034 §6.4: *"`never` is not a degraded mode."*

## 7. `main` moved — a fact, and nothing more

**Not `Job.facts`** (see the table above — that field is spawn-time-only by its own contract).
`crates/core/src/fleet/job.rs` gains one new additive field instead:

```rust
/// Set once, when the daemon pulls a `main` this Job's branch was not
/// forked from. A fact for the Drone to notice on its own next turn — not
/// an instruction, and not a rebase (034 §3: "asked for, never imposed").
#[serde(default, skip_serializing_if = "Option::is_none")]
pub main_moved_at: Option<String>,
```

After step 6.2 (pull) succeeds, the daemon walks every other `RUNNING`/`PAUSED` Job in the same
repository (the same in-scope iteration `crates/helm/src/verbs/fleet.rs`'s `pass()` already does
for `tick`) and sets this field via a `begin_daemon_act(MarkedMainMoved, job_id)` /
`settle_daemon_act` pair on **its own** record (the mover), while writing `main_moved_at` directly
on **each other Job's** record — two different writes, since the audit trail is about what the
daemon did and this field is the fact it left behind. Stage one builds no `fleet_rebase` tool and
no consumer of this field beyond it existing and being readable — 034 explicitly reserves both for
later (*"a Drone may then ask…"*), and the task names the rebase out of stage one by name.

## 8. `armada doctor` reads the daemon

`crates/helm/src/verbs/doctor.rs`: one more push in `run()` (51-83), following `helm_argv`'s exact
shape (machine.yml → `Finding`): a new `daemon()` helper reads `daemon::is_running` (§1, the
pidfile/liveness check — the *actual* process, not just the switch) and `daemon_log::last` (§2,
`daemon.jsonl`'s most recent entry) and reports a `Finding` naming both — running-or-not, and what
it last did, with `remedy: Some("armada daemon enable")` when the switch is off. **This is the row
that names the keystroke** — the task's own words, and 020's eight-hour stall is the reason it is
not optional: a Job whose `land` step gate never holds because the daemon that would push its
branch is not running must be distinguishable, on the screen, from a Job whose PR is legitimately
still red.

## Testing order (each new test shown failing first)

1. `crates/fleet/src/machine.rs` — `DaemonSwitch` round-trips, defaults off, preserves `carry`
   (mirrors machine.rs's own existing test block).
2. `crates/fleet/src/daemon.rs` — `is_running` against a live pgid, a stale pidfile, no pidfile.
3. `crates/core/src/fleet/job.rs` — `begin_daemon_act`/`settle_daemon_act`: an act with no outcome
   round-trips, a settled act's `outcome_at` is set, `daemon_acts` stays empty (and absent from
   JSON, via `skip_serializing_if`) for a Job the daemon has never touched.
4. `crates/fleet/src/daemon_log.rs` — append/fold mirrors `inbox.rs`'s tests: a torn last line is
   skipped, not fatal; `last()` on empty is `None`.
5. `crates/core/src/config/{schema,model,resolve}` — `fleet:` absent parses as `Never`; present
   with `merge: auto` parses `Auto`; an unknown key under `fleet:` is `bad_config` (schema's
   `additionalProperties: false`); `crates/manifest`'s own tests are run unchanged and must still
   pass with **no new import** — the boundary is proven by *absence* of a change there, not a new
   test.
6. `crates/core/src/fleet/gate.rs` — `pr_open`/`pr_merged` `needs()`/`decide()`, both policies,
   both predicates, per §5.
7. `crates/fleet/src/land.rs` — `push`/`open_pr` against a fake `Run`, no-remote case produces the
   specific `LandError` and the daemon-act failure shape from §3.
8. Full sequence (fake `gh`/`git`): green PR + `auto` → merge, pull, re-run, reap, in order, each
   recorded; red re-run → `ReportedFailure` + one inbox entry, no resume attempted (assert this
   directly — a regression here silently reintroduces stage two).
9. `crates/helm/src/verbs/doctor.rs` — daemon `Finding` for running/not-running/never-enabled,
   each with the right `remedy`.
10. `cargo xtask boundaries` and `armada manifest check` at the end, not as a substitute — see below.

## The risk `cargo xtask boundaries` cannot catch

Restating 034's own claim in my words, verified against `xtask/src/boundaries.rs` rather than
assumed: it is a `cargo metadata` crate-dependency-graph checker (confirmed at
`xtask/src/boundaries.rs:32-37`'s own comment — *"nothing stops someone adding `armada-manifest` to
`core`'s manifest, which compiles fine"* is the general form of this exact gap). It enforces
`Module::may_depend_on` from Cargo dependency edges, and `core` sits below every module and is
always permissible (ARCHITECTURE.md 558-585's own note) — confirmed empirically too:
`crates/manifest/src/discovery.rs` already imports `armada_core::config::declared_workspaces`, so
`crates/manifest` → `armada-core` is an existing, legitimate, already-permitted edge. **That is
exactly why the graph check cannot catch this violation**: there is no forbidden edge to add.
`land_merge()` (§4) lives in `armada_core::config::resolve`, a module `crates/manifest` is already
allowed to call into; the violation would be `crates/manifest` (or `armada_core::config::resolve`
itself) actually calling it — a data-flow fact invisible to a dependency graph, which only sees
`manifest → core` and reports it fine either way. The narrower, more likely version of the same
gap: someone widens `resolve::parse`'s existing `map(|d| d.manifest)` (resolve.rs:77-79) to also
fold `document.fleet` into the `Config`/`ResolvedConfig` that already flows, unchanged, into every
Manifest-facing verb (`crates/helm/src/verbs/check.rs`, `status.rs`, etc.) — no new Cargo edge, no
new import, compiles clean, and puts agent-shaped policy (`fleet.land.merge`) in front of code that
today provably never sees a Job. The backstop is the same one the command-centre plan named: code
review at `implement`'s `review` gate, reading this section and checking that `land_merge()` has
exactly one caller, in `crates/fleet`, and that `resolve::parse`'s `map(|d| d.manifest)` line is
untouched, and that `crates/manifest` gains no new reference to `land_merge` or `document.fleet`
at all.

## Open questions — resolved before implement, except where named

1. **Merge strategy** (`--merge` vs `--squash` vs `--rebase` on `gh pr merge`). **Not resolved
   here** — this is a repository preference (squash-merge is common, plain merge preserves the
   Drone's own commit history that `land-branch/SKILL.md` §"Make the history readable" already
   asks for) and belongs in `fleet.land:` beside `merge:`, e.g. `fleet.land.strategy: squash|merge`,
   defaulting to `merge` since a Drone's history was already asked to be clean before landing. Flag
   for the person approving this plan — it is a one-line schema addition either way and does not
   change the shape of §4 or §6, only their content.
2. **Where "pull main locally" pulls into.** `crates/fleet/src/worktree.rs` manages per-Job
   worktrees off a shared repository, but no code path found in this exploration names a single
   canonical "the main checkout" location — every worktree shares one object store per Job-spawn's
   `git worktree add`, and worktrees for *future* Jobs are created from whatever `main` looks like
   at spawn time (`git worktree add` reads refs, not a checked-out tree). Resolved as: `git fetch
   origin main:main` in the shared bare/origin the worktrees were added from is sufficient — it
   updates the ref every future `worktree add` reads — and no separate checkout is needed. Confirm
   this against `crates/fleet/src/worktree.rs`'s actual `add` call at implement time; if a
   worktree literally named `main` also exists and is checked out, it needs the same fetch **and**
   a fast-forward `pull`, not just the ref update.
3. **`gh` authentication.** Assumed already configured on any machine where `armada daemon enable`
   is run (the same assumption `land-branch/SKILL.md` makes about the environment a Drone runs in).
   `armada doctor`'s daemon check (§8) should report `gh auth status` failing as a Finding rather
   than let it surface only as a mysterious push failure — small addition to §8, worth doing in the
   same pass since the check already runs a command and reads its exit code.
