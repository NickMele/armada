# PLAN — Draw the command centre

Implements [`docs/reserved/033-the-command-centre-designed.md`](docs/reserved/033-the-command-centre-designed.md).
The mock-ups there are the spec; this plan says what code makes them real, in
what order, and what proves each piece before it's built on.

## What already exists vs. what's new

| Piece | State | Detail |
|---|---|---|
| `render/frame.rs` primitives | **partial** | `focus`, `titled_box`, `hjoin` are real and tested. `hjoin` joins exactly **two** boxes — a three-panel row nests it (`hjoin(hjoin(a, b, gap, w), c, gap, w)`), no API change needed. `shed_to_narrow`/`apply_shedding` are **stubs that return input unchanged** — the narrow-width behaviour in 033's 96-column mock (whole panels dropped, not just columns) has to be built from nothing. |
| `render.rs` `pub mod frame` | done, unused | Zero call sites anywhere in `crates/helm/src` today. |
| `Frame.windows: Vec<Window>` | done | Both usage windows already flow through `armada_core::envelope::Window` → `bridge::frame()`. |
| `armada_fleet::inbox` | done | `Entry{uuid, job_uuid, job, kind, raised_at, raised_ms, body, answered, closed}`, `read()`. Already wrapped by `verbs::fleet::inbox()` → `InboxData{results: Vec<InboxRow>, open}`. |
| MANIFEST data | done | `verbs::check::status()` → `CheckData`; `verbs::status::run()` → `StatusData{results: Vec<ResultRow>, unreclaimed}` for the workspace/lease half of the panel. |
| GUILD data | done | `verbs::guild::ls()` → `GuildListData{items: Vec<GuildItemRow>, facts: Vec<String>, ...}`. |
| SYSTEM data | done, needs widening | `verbs::doctor::run()` → `DoctorData{results: Vec<Finding>}`, and `Finding{check, status, detail, remedy}` is already the FACT/STATUS/DETAIL shape the mock draws. Today's checks (`drift`, `store`, `docker_disk`, `drone_argv`, `helm_argv`, `directories`) don't yet emit rows for "drones N live" or "docker up \<version\>" by those exact names — see §3. |
| Job detail data | done | `ShowData` already carries `gate`, `transitions: Vec<TransitionRow>`, `progress: Vec<NoteRow>`, `asked: Vec<InboxRow>`, `budget`/`budget_remaining`, `drone_pgid`, `drone_alive`, `worktree`, `branch` — the five detail panels are a reducer over fields that exist, not new data. |
| Panel focus / multi-cursor | **partial, cherry-picked from `armada/bridge-command-centre`** | `Panel` enum, `Panel::next()`/`from_digit()`, `Key::Tab`, `Screen.focus`, and `watching()` routing (`Up/Down/j/k/Enter/d/a/x/p/r` all gated `on_jobs`, global keys `n`/`c`/`?`/`q` ungated) are built and tested — 5 new tests in `crates/core/src/fleet/bridge.rs`, all red-then-green per this plan's own rule. `shed_to_narrow` is also done for real (`KeyPair`-based, movement-never-verbs-do, 8 new tests in `frame.rs`). **Corrected in the implement step**: this row previously said INBOX's cursor was blocked on `InboxData` being a `helm`-layer type — that was wrong, unverified against the code. `InboxData`/`InboxRow` are defined in `crates/core/src/envelope.rs`, the *same crate* `core::fleet::bridge` is in; there is no crate boundary in the way. **Still deferred, for a different and real reason**: `press(screen, rows, key)` has 88 call sites in this file's own tests, every one of which would need a new parameter to carry an inbox row count for `Up`/`Down`/`j`/`k` wrapping — mechanical churn disproportionate to what it buys this pass (INBOX has no verb wired to a row yet either, per `commands/helm/bridge.md`). INBOX stays reachable by `Tab`/`2` like MANIFEST/GUILD/SYSTEM, rendered but cursorless, until a row action on it earns the churn. Nothing in `crates/helm/src/render.rs`, `bridge.rs`'s `paint`/`detail_pane`, or `verbs/bridge.rs`/`verbs/doctor.rs` (§2–§5) has been touched — the panels described in this plan draw nothing yet. |
| MANIFEST verb signatures (§2, §7) | **corrected in the implement step: `done` was wrong** | PLAN.md's §2 sketch assumed `check::status(run, now, place)` and `status::run(run, now, place)` — checked against the real code (`crates/helm/src/verbs/check.rs:761`, `crates/helm/src/verbs/status.rs:59`) and both actually take `app: &mut App<R, C, F>`, not bare `run`/`now`/`place`. `App` opens `manifest.db` and reads `MachineConfig`/`boot_id` (`crates/helm/src/app.rs:746`'s `build()`), and `main.rs`'s dispatch (`main.rs:911`) deliberately routes `Bridge` *around* `app::build` — every other verb pays that cost once per invocation, and the Bridge was kept out of that group on purpose so a 2-second redraw stays "a directory read, a transcript tail and a `ps`" (`verbs/bridge.rs`'s own doc comment). Wiring `App` into the Bridge's long-lived `watch()` loop is a real, bounded change (build it once at entry, not per frame) but it reverses a deliberate architectural line and touches `main.rs`, which PLAN.md's "files touched" list never named. **Descoped from this implement pass**, proposed separately (`fleet.propose`, subject `manifest`) rather than decided unilaterally. `BridgeView`/`read_all` ships with `fleet`/`inbox`/`guild`/`system` only; the MANIFEST panel is not drawn this pass. |

## Implement step — closed out

Built, tested (`cargo fmt`/`clippy -D warnings`/`nextest run --workspace` 2080/2080/`xtask doclint`/
`xtask boundaries` all clean): `ShowData.steps` (§5's open question 2), `verbs::bridge::BridgeView`/
`read_all` (§2, minus MANIFEST — see above), `doctor.rs`'s `drones_live` (§3 — `stale_pgids` also
needs `App`/Manifest's registry and is descoped with MANIFEST, not built under a different, weaker
metric), the seven `render.rs` panel functions (§4) and the Job detail screen's six (§5), `bridge.rs`'s
`paint()`/`detail_pane` assembling them at the 138-column and under-138 widths (§4.1, §6). Every new
function has a passing test; several are demonstrated red-without-the-change explicitly
(`step_rows`, `read_all`'s wiring) and the rest are new code with no prior passing state to regress
from. `TIMELINE`/`REPORTS` reading different `ShowData` fields is enforced by both a test and by
the two functions taking different argument types (`&[TransitionRow]` vs `&[NoteRow]`) — the two
cannot be swapped without a compile error.

**Simplifications made under time, named so a later pass can pick them up rather than rediscover
them:**
- JOBS box reuses `bridge_table`'s existing column set (`JOB STATUS ID [WORKFLOW] STEP [TASK] RUN
  [TURNS] SPENT NEEDS YOU`) rather than the mock's own narrower `JOB ID STATUS STEP ITER SPENT
  TIME`, and marks the cursor with the single-table Bridge's `›` caret column rather than 033's
  `▸`-in-the-leading-space (`frame::focus`). Full parity is real work (a second column set, a
  `TransitionRow`-shaped iteration count nowhere on `JobRow` today) rather than a rendering choice.
- INBOX's `ORIGIN` column shows `InboxRow.kind` (`NEEDS_HUMAN`/`BLOCKED`/`IDLE`) as raised, not
  033's `ASKED`/`FAILURE`/`REPORT` — tied to the `/asked /failure /report` filter grammar §3.3's
  open question already named as new grammar, not built this pass.
- GUILD box draws `GuildListData.facts` (already-summarised strings) rather than a table over
  `items`; SYSTEM box draws every `Finding` as `FACT`/`DETAIL` without the mock's exact five-row
  set (drones/docker/volumes/disk/stale-pgid) — `drones` is real and new, `docker`/`volumes`/`disk`
  already exist under `preflight`/`docker_disk`'s own check names, `stale pgid` needs Manifest's
  registry (App) and is not faked under a different meaning.
- The Job detail's `WORKFLOW` box has no `TIME` column — `StepRow` (this pass's own new type)
  carries `id`/`status`/`must`, not per-step elapsed time, which is not on any existing record;
  `TIMELINE`'s `WHEN` column is the same data.
- Golden fixtures (§6.5) for the fleet screen at 138/96 columns and the Job detail screen are not
  built. The existing `tests/golden/render/bridge*` fixtures (the `--once`/`--json` single-table
  render, untouched by this pass) still pass. Freezing the live-screen layout needs a
  `bridge::paint()`-shaped harness the golden-fixture system doesn't have yet (today's harness
  renders `Output` envelopes through `render::human`, and the live screen isn't one) — building
  that harness is real, separable work from drawing the panels.

None of these narrow what any panel is *for*; each is named so `armada bridge` at a live terminal
and this document agree about what is real.

## 1. Core state (`crates/core/src/fleet/bridge.rs`)

New, additive to the existing `Mode`/`Key`/`Action` set — nothing existing changes shape:

- `pub enum Panel { Jobs, Inbox, Manifest, Guild, System }`, `Panel::next()` wrapping for `Tab`, `Panel::from_digit(char) -> Option<Panel>` for `1`–`5`.
- `Screen` gains `pub focus: Panel` (default `Panel::Jobs`) and a cursor per row-bearing panel (`Jobs` keeps today's `cursor: Cursor` over `Frame.rows`; `Inbox` gets its own `Cursor` over `InboxData.results`). MANIFEST/GUILD/SYSTEM are read-only lists in the mock — no verb (`enter`, `d`, `a`, `x`, `p`, `r`) acts on one of their rows — **but they still take `focus`.** `Tab`/`1`-`5` must be able to land on them: a digit key that changes nothing, or a `Tab` that skips three of five panels, is a key that lies about what it does. So `Panel::focus_five()`/`Panel::next()` cycle all five regardless of which carry a `Cursor`; a focused panel with no `Cursor` just draws no focus marker on any row (nothing in `focus()` to call) and `Up`/`Down` on it are no-ops until one of these panels grows a row action worth a cursor.
- `Key::Tab` and digit chars: `Char('1'..='5')` already arrive as `Key::Char`; `watching()` gets a branch that, when the char is `'1'..='5'` and no filter/compose box is open, sets `screen.focus` instead of falling through to whatever `'1'` used to mean (today nothing — unbound).
- `press()`'s dispatch stays mode-keyed at the top level; inside `Mode::Watching` it now branches further on `screen.focus` for `Up/Down/j/k/Enter/d/a` etc., routing to the focused panel's row list. `x`/`p`/`r` remain JOBS-only per 033 (INBOX has no abort/pause/reap).
- **Movement never sheds; verbs do** (033's own words) is a rendering-time rule, not a state one — `press()` doesn't change based on width, only `frame.rs`'s `shed_to_narrow` does. Keep it that way: no `width` parameter threaded into `core`.

Every new variant/field ships with the failing test first: e.g. `tab_cycles_focus_through_five_panels`, `digit_keys_jump_focus_and_do_nothing_while_filtering`, `enter_on_an_inbox_row_boards_its_job` — written red against today's `Screen`, then made green.

## 2. Verbs (`crates/helm/src/verbs/bridge.rs`)

One new aggregate read, next to today's `read()`/`data()`:

```rust
pub struct BridgeView {
    pub fleet: Frame,
    pub inbox: InboxData,
    pub manifest: CheckData,      // + StatusData for leased workspaces
    pub guild: GuildListData,
    pub system: DoctorData,
}

pub fn read_all<R: Run, C: Clock>(run: &R, now: &C, place: &Where, filter: Option<&Filter>) -> Result<BridgeView, ArmadaError> {
    Ok(BridgeView {
        fleet: crate::verbs::bridge::read(run, now, place, filter)?,
        inbox: unwrap_inbox(crate::verbs::fleet::inbox(now, place, None, false)?),
        manifest: unwrap_check(crate::verbs::check::status(run, now, place)?),
        guild: unwrap_guild(crate::verbs::guild::ls(run, place, false, false, false)?),
        system: unwrap_doctor(crate::verbs::doctor::run(run, place)?),
    })
}
```

Every call is exactly the signature today's single-purpose command uses — `place`, `run`, `now`, no Job id. That's not incidental; see §5.

**Implemented shape, corrected against the audit table's MANIFEST row above:** `BridgeView` ships with `fleet`, `inbox`, `guild` and `system` — `manifest: CheckData` is not in it this pass. `check::status`/`status::run` need `App`, which the Bridge's call site does not build (see the audit table); `read_all` calls exactly `crate::verbs::fleet::inbox`, `crate::verbs::guild::ls` and `crate::verbs::doctor::run` with the same argument shapes those verbs already take, none of which admits a Job id.

## 3. SYSTEM panel — widen `doctor.rs`, don't add a verb

`doctor::run` already returns `Vec<Finding>`. The mock's five rows map onto new `Finding`-producing helpers alongside the existing `drift`/`store`/`docker_disk`:

| Mock row | New helper | Source |
|---|---|---|
| `drones N live` | `fn drones_live(runner, armada_home) -> Finding` | count of Job records with a live `drone_pgid` — same `pgid_is_live` check `verbs/status.rs` already uses |
| `docker up 29.7.2` | fold into `docker_disk`'s existing docker-reachability check | it already probes docker; add the version string to its `Finding.detail` |
| `volumes 171 · 12.0 GB` | already in `docker_disk` | confirm field names match the mock's `·` join |
| `disk 176 GB free` | already in `docker_disk` via `machine_detail(&DiskUsage)` | confirm wording |
| `stale pgid 18 · reap` | `fn stale_pgids(runner, armada_home) -> Finding` | reuse `status.rs`'s `Unreclaimed`/stale-pgid detection, folded to a count + remedy `reap` |

This stays inside `crates/helm/src/verbs/doctor.rs` — a wider `Vec<Finding>`, not a new crate dependency, not a new verb signature.

## 4. Rendering (`crates/helm/src/render/frame.rs`, new `render/panels.rs` or similar, `bridge.rs`)

Build bottom-up, each with a failing test before the code:

1. **`shed_to_narrow`, for real.** Give it what 033 actually specifies: at ≥138 columns draw all seven boxes; under a threshold (the 96-column mock is the fixture), draw only `ARMADA` header (collapsed to one line per window), `JOBS`, `INBOX`, `KEYS` — MANIFEST/GUILD/SYSTEM drop whole, and JOBS/INBOX shed columns the same way `bridge_table` already does today (`WORKFLOW` → `TURNS` → `TASK`). This needs a real signature change from "layout in, layout out" to something that knows which boxes are which, or (simpler, matches "frame must know nothing about Jobs/Manifest/Guild") the *caller* in `bridge.rs` picks which `titled_box` calls to make based on width, and `shed_to_narrow` stays scoped to what it already half-does: trimming the key line's verbs at narrow widths within one box. Resolve this ambiguity in the research pairing (see Open questions) before writing code — it changes whether `shed_to_narrow` grows a parameter or `bridge.rs` grows a width branch.
2. **Header box**: `titled_box("ARMADA", [one line: id, cwd, both windows], width)`, reusing today's window-formatting logic from `render::bridge_summary_pieces`.
3. **JOBS box**: today's `bridge_table` output, run through `Table::spans(style, width)`, wrapped in `titled_box`, with `focus()` applied per row using `screen.focus == Panel::Jobs`.
4. **INBOX box**: new `Table` (`FROM`, `ORIGIN`, `DETAIL`, `WAITING`) over `InboxData.results`, same `focus()` treatment keyed on `Panel::Inbox`.
5. **MANIFEST / GUILD / SYSTEM boxes**: `Table`s over `CheckData`/`StatusData`, `GuildListData`, `DoctorData` respectively — no focus marker (no cursor), per §1.
6. **KEYS box**: the full legend at wide widths; at narrow widths the movement legend (`↑↓←→ or hjkl move`, `tab next panel`, `enter act`) stays and the verb pairs (`d detail`, `n new job`, `a answer`, `p pause`, `x abort`, `r reap`, `t tick`) drop behind `?` — the 96-column mock (033) is the fixture for this exact split. This is `render::bridge_keys`'s existing priority-drop logic, relocated into a box; **movement never sheds, verbs do**, because a dropped verb is one keypress away behind `?` and a dropped movement key is a screen you cannot get around.
7. Assemble: `hjoin(titled_box(JOBS), titled_box(INBOX))`, `hjoin(hjoin(MANIFEST, GUILD), SYSTEM)`, stacked under the header and above KEYS.

## 5. Job detail, full screen

`detail_pane` in `bridge.rs` is rewritten (still fed only by `ShowData`, still "one description emitted three ways," `PLAN.md §3.1.1`):

- **Identity block**: uuid, workflow, branch, started-ago, state + reason, task — from `ShowData` fields already present.
- **WORKFLOW** + **NEEDS YOU**: `hjoin`'d pair. WORKFLOW is a `Table` over a per-step reduction (needs `transitions` + workflow step list — the workflow's step order isn't on `ShowData` today; check whether it's derivable from `transitions` alone or needs one more field. Flag as open question below). NEEDS YOU is `asked[0]` if present.
- **TIMELINE · what the gate did**: `Table` over `transitions: Vec<TransitionRow>` — `STEP`, `EVENT`, `EVIDENCE` (the `must` predicate + exit code, from `TransitionRow.evidence`), `WHEN`.
- **REPORTS · the Drone's own words**: a **separate** `Table` over `progress: Vec<NoteRow>` — `STEP`, `THE DRONE SAID` (`NoteRow.body`, truncated). These two tables share no code path beyond `Table`/`titled_box` — deliberately, since 033 §"The detail view names its two tables" is explicit that collapsing them is how a Drone's claim gets read as gate evidence. A test (`timeline_and_reports_read_from_different_fields`) asserts the two tables never source a row from the same `ShowData` field.
- **FACTS**: `Table` over `drone_pgid`/`drone_alive`, `worktree`, `branch`, `cost_usd`/`budget.cost_usd`, `turns`/`budget.attempts`, `tokens` — the mock's `cost SPENT $1.25 of $5.00` / `exchanges SPENT 19 of 20` rows.
- Full-screen assembly: identity block, then `hjoin(WORKFLOW, NEEDS_YOU)`, then TIMELINE alone (full width), then `hjoin(REPORTS, FACTS)`, then its own KEYS box (`esc back`, `a answer`, `t tick`, `r retry`, `$ raise budget`, `p pause`, `x abort` — per 033, not the fleet screen's legend).

## 6. Testing order (each failing before it passes, per repo convention)

1. `core::fleet::bridge` — `Panel`, focus cycling, digit jump, per-panel routing of existing verbs. Pure unit tests, no rendering.
2. `render/frame.rs` — real `shed_to_narrow` behaviour (panel-drop at 96 cols), any `hjoin`/`titled_box` extension the layout needs.
3. `verbs/bridge.rs` — `read_all` / `BridgeView`, and `doctor.rs`'s new `Finding` rows (each with its own red test using a fake `Run`, matching existing `doctor.rs` test style).
4. `bridge.rs` `paint()` — one test per new panel proving it draws (mirroring the existing `a_frame_draws_the_columns_the_summary_and_the_keys` style), then the 138-col and 96-col fixtures.
5. Golden fixtures (`crates/helm/tests/render_golden.rs`-style, `tests/golden/render/`) for the fleet screen at 138 and 96 columns, and the Job detail screen. **No update flag exists on purpose** — on first run each mismatch writes a `.actual` next to the fixture; each one gets read and diffed by hand before `mv`-ing it over the checked-in fixture, never scripted.
6. `cargo xtask boundaries` and `armada manifest check` run at the end, not as a substitute for the above — see §7 for why boundaries alone doesn't prove anything.

## 7. The risk, answered

In my own words: ARCHITECTURE.md §1.9 says Armada is four modules stacked so nothing points upward — Helm may depend on Fleet, Guild and Manifest; Fleet on Guild and Manifest; Guild and Manifest depend on nothing and may not name each other. The rule that actually matters is the negative one: **Manifest must keep knowing nothing about agents.** The moment a Job id gets threaded into a Manifest call "just this once," Manifest stops being a tool anyone can run by hand or from CI and becomes part of the agent framework — a one-way door. `cargo xtask boundaries` reads the crate graph, so it proves no crate declares a dependency edge it shouldn't. It cannot prove the thing that actually matters here: that no *function call* inside `armada-helm` (which is already permitted to depend on `armada-manifest`) hands a Job-shaped value to one of Manifest's own functions. That is a data-flow property, invisible to a dependency graph, and this feature is exactly where it gets likely — one screen now reads all four modules at once, through seven panels instead of one table, so there are seven places a shortcut like "just pass the job id in so the row can label itself" could get taken.

This plan's `BridgeView`/`read_all` (§2) is the concrete answer, not just the module layout: it reads MANIFEST, GUILD and SYSTEM data by calling the exact same verb functions `armada manifest check`, `armada guild ls` and `armada doctor` already call today — through `crates/helm/src/verbs`, never by importing `armada_manifest`/`armada_guild` types directly into a panel-drawing function and never by adding a parameter to `check::status`, `guild::ls` or `doctor::run` that a Job id could fill. Concretely:

- Every MANIFEST/GUILD/SYSTEM read in `read_all` calls `check::status(run, now, place)`, `guild::ls(run, place, ...)`, `doctor::run(run, place)` — **the identical signatures `armada manifest check`, `armada guild ls`, `armada doctor` already call today, unchanged.** None of these functions has a parameter a Job id could be passed through even if a panel wanted to; the cage is in the argument list, not in a rule someone has to remember. `read_all` is not permitted to grow one — that's the one-line review rule for anyone touching it later.
- The only place a Job id exists in scope at the same time as a Manifest/Guild/System value is `bridge.rs`'s top-level `paint`/`watch`, which already holds a `Frame` (Job rows) alongside whatever `BridgeView` returns. The boundary is that nothing downstream of `read_all` — no panel-drawing function — takes both a `job`/`JobRow`/`Target` value and a `BridgeView.manifest`/`.guild`/`.system` field as arguments. Each panel-drawing function is typed to take exactly one of those two families, never both, which makes the mistake a type error the compiler catches rather than a convention a reviewer has to remember.
- **What would break it, stated plainly so it's recognisable:** a MANIFEST or SYSTEM panel-drawing function gaining a `job: &str` or `target: &Target` parameter "just to label the row it's next to" — or `read_all` growing a `job` argument that `check::status`/`guild::ls`/`doctor::run` then get threaded through "for one filtered case." Either would compile clean, add no new Cargo.toml edge, and `cargo xtask boundaries` would report nothing wrong, because Helm is already permitted to depend on Manifest — the graph has no way to see that the *call* now carries agent-shaped data. That's the actual gap this plan is closing structurally rather than by convention.
- Because `cargo xtask boundaries` cannot catch this class of bug, the backstop is code review at the `implement` step's `review` gate reading this section and checking `read_all`'s signature and every panel-drawing function's argument list against it — named here so that reviewer doesn't have to rediscover the risk from scratch.

## Open questions — resolved before implement

1. **`shed_to_narrow`'s shape** (§4.1). **Resolved, and already built the recommended way.** The cherry-picked code took the latter option: `shed_to_narrow` now takes `(movement: &[KeyPair], verbs: &[KeyPair], quit: &KeyPair, width: usize) -> Vec<Span>` and stays scoped to trimming one key line — it knows nothing about panels or boxes. Whole-box dropping at 96 columns (MANIFEST/GUILD/SYSTEM disappearing) is still unbuilt and is `bridge.rs`'s job: the caller branches on width and chooses which `titled_box`/`hjoin` calls to make, exactly as recommended. `frame.rs` still knows nothing about Jobs, Manifest or Guild.
2. **WORKFLOW panel's step list** (§5). **Resolved: it needs a new field.** Read `ShowData` (`crates/core/src/envelope.rs:2163`) and its producer (`verbs::fleet::show`, `crates/helm/src/verbs/fleet.rs:1222`) directly: `ShowData` carries `step` (current), `attempt`, `gate: Option<GateRow>` (current step's predicate only) and `transitions: Vec<TransitionRow>` (history) — no ordered *declared* step list. `core::fleet::workflow::Workflow` already has `steps: Vec<Step>` (`crates/core/src/fleet/workflow.rs:290`) and `show()` already loads the workflow document once to find `gate_of(place, &record.workflow, &record.step)`. The implement step adds one additive field to `ShowData` (e.g. `pub steps: Vec<StepRow>`, each carrying id, `PASS`/`BLOCKED`/`QUEUED`/current and the gate's `must`), built in `show()` by walking `Workflow.steps` and folding in `transitions` + `record.step` — no new verb signature, same shape as every other widening this plan already does.
3. **INBOX `ORIGIN` filter** (`/asked /failure /report`): confirmed 033 wants it; confirmed it's new (no `ORIGIN` filter grammar exists in `docs/glossary.md` or `bridge.md` today) — implement step should treat this as new filter-expression grammar, not a lookup of something already there.

## Files touched (implement step)

`crates/core/src/fleet/bridge.rs`, `crates/helm/src/render/frame.rs`, `crates/helm/src/render.rs` (new panel-building functions), `crates/helm/src/render/table.rs` (only if a column type is missing), `crates/helm/src/bridge.rs` (`paint`, `detail_pane`, `watch`), `crates/helm/src/verbs/bridge.rs` (`read_all`/`BridgeView`), `crates/helm/src/verbs/doctor.rs` (new `Finding` rows), `crates/helm/tests/render_golden.rs` + new fixtures under `tests/golden/render/`, `docs/commands/helm/bridge.md` (amend the step-fraction sentence per 033's own note, and document the new keymap).
