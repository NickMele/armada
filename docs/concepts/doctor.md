# Doctor

**What it is:** Health check surface, one of Bridge's seven. Passive per-module grid, and the hard gate on first-run dispatch.

---

**Kind:** Surface.

Formalises Doctor, the health check surface inside [Bridge](bridge.md). The definition was split across the Check System Health and First-Run Onboarding user journeys, [Fleet](fleet.md), Bridge and `../contracts/iconography.md`, and no two of them agreed on how many modules the grid holds. This document is the citable source; those pages now link here.

Four source conflicts were resolved Aug 2026 and are recorded below with the call made. The pages they contradicted are listed at the end.

## What it is

Doctor is one of Bridge's **seven** surfaces. It is a **passive scan, not an action**. It answers "is everything okay right now" by reporting, and it changes nothing.

**Doctor owns no probe logic and no state.** It invokes one probe per module through one `HealthReport` contract and renders what comes back, plus one client-side fact. Probes live beside their subjects, never in Doctor.

- **Per-module status**, each module independently pass, warn or fail. No blended score, so a problem in one module is never obscured by a healthy one.
- **Checked on demand.** Doctor itself never pushes. ~~A module fail does reach Alerts at **Waiting**.~~ **Reversed 2026-08-21** from the design of the Triage Queue journey: a health check is a standing condition, not a queued decision — a failing module has no workspace, no spend, no step and no Job id, and nothing about it is *waiting* in the sense every Alerts row is. **Doctor never enters Alerts.** A failure reaches you as a one-line neutral strip above the Triage Queue tabs, naming which modules are failing and whether work is stopped, and disappearing when they all pass. Alerts stays a list of Jobs.
- Doctor sits under the Bridge section label in the sidebar with the other six surfaces, reachable in sidebar order. Its nav icon is `stethoscope` at 16px. **Seven since 2026-08-21**, when Manifest was added.

### Everything is probed. Nothing self-reports — with one stated exception

Resolved Aug 2026, replacing an earlier framing in which Armada's own modules reported their own health and only external tools were probed. That split was never load-bearing and cost a second code path.

**The exception is Armada API.** Once Fleet and Armada API are one process, a probe for "is the transport reachable" and a probe for "is the daemon alive" read the same fact — which is the *Daemon-vs-Fleet* duplication this document already removed once, arriving under new names. The fix is not to delete a row but to stop calling it a probe: **Armada API's row is Bridge's own client-side connection state**, which is genuinely different information (wrong port, stale connection, Bridge-side socket failure) and needs no probe at all. One probe per module, plus one client-side fact.

| Property | Why it follows |
| --- | --- |
| One code path | Doctor asks, everything answers, same `HealthReport` shape |
| Health is a pull, not a push | Matches what Doctor already is. Self-reporting implies a push nobody wanted |
| No stale state | A self-reported status is only as fresh as its last write. A probe is true when you asked |
| Uniform test surface | `testkit` fakes every module identically, so an unhealthy Fleet is as testable as an unhealthy Docker |

**Where probe logic lives.** Beside its subject. Git, Docker, Claude and Keychain probe from `adapters`, the crate that already owns talking to things outside Armada. Kit, Machine and Manifest probe from `config`, reading and validating their own files. SQLite probes from `store`, the only crate holding the connection. Fleet probes from its pidfile. `HealthReport` lives in `core-model` and the trait in `adapter-traits`, so nothing reaches across a crate boundary to read a status and `testkit` can fake an unhealthy anything.

**A dedicated `health` crate was rejected.** No v1 measurement backs the seam, it would duplicate or thinly wrap `adapters`, and grouping by surface rather than by capability is the shape that grew v1's `core` to 38,470 lines. Merging two crates later is an afternoon. Revisit if health grows history, thresholds or a scheduler.

**Which process invokes the probes is resolved Aug 2026: neither Fleet nor a second daemon.** `fleet-bin doctor --json` is a short-lived process Bridge spawns — eight probes in one cold process, measured at 1068 ms cold and **253 ms warm**, of which `docker info` and `claude --version` are 98% and are subprocess spawns whichever process holds them. Fleet's own row becomes an honest external probe (pidfile → `kill(pid, 0)`, 26 µs) **without a second daemon**, because the prober does not ask Fleet anything. Doctor is already specified as a pull, on demand, holding no state — a shape that fits a process existing for a quarter of a second. Run the probes concurrently and the total is `max()` not `sum()`, about 760 ms cold, bounded by Docker. Worth doing, not required.

## The module grid

**The grid as the rule below generates it:** Fleet, Armada API, Kit, Machine, Manifest, SQLite, Git, Docker, Claude, Keychain, System stats.

### What earns a row

**Armada depends on it, and it can be up or down.** Resolved Aug 2026.

Doctor reports **service health**, not Job-readiness. A module passes when the thing it names is working, whether or not anything currently uses it. Docker running with no Check invoking it is a true pass — the service is healthy, which is what the row claims.

An earlier candidate rule — a row exists where failure prevents a Job from running — was rejected. It excluded Docker, which blocks nothing when no Manifest's Checks use it, and it would have forced a fourth **n/a** result to describe conditional modules. Service health needs no conditional logic and no fourth result.

**The rule is deliberately wide.** It bounds the grid at "things Armada depends on" rather than at a narrower set, which admits SQLite and Keychain — both added Aug 2026, neither previously listed. Network reachability would also qualify and is not yet a row.

Each row states what its probe reads and where the boundaries sit.

| Module | What the probe reads | Pass | Warn | Fail |
| --- | --- | --- | --- | --- |
| Fleet | Is the daemon process alive and answering — pidfile → `kill(pid, 0)`, measured at 26 µs. An honest external probe, because the prober is not Fleet | Alive, answering | **Never warns** | Not answering. Under launchd, restart is automatic and uncapped — so "Restart Fleet" means **skip the throttle wait**, not recover. Where Fleet exited 0 deliberately, launchd leaves it down by design and Doctor must show the reason rather than a restart button |
| Armada API | **Not a probe.** Bridge's own client-side connection state — it already knows whether its socket is open, and asking a probe would mean asking the process it cannot reach. Resolved Aug 2026 with the topology decision | Connected | **Never warns** | Not connected — wrong port, stale connection, Bridge-side socket failure |
| Docker | Is the daemon reachable | Reachable | **Never warns** | Not reachable |
| SQLite | **Added Aug 2026.** `armada.db` opens, schema version matches, WAL is writable | Opens and writes | Schema behind, migration pending | Locked, corrupt, or the volume is full |
| Keychain | **Added Aug 2026.** The macOS Keychain is unlocked and Armada's brokered scope is readable | Unlocked, readable | **Never warns** | Locked or access denied — secret brokering fails |
| System stats | CPU and memory headroom against the Machine threshold | Above threshold | Below threshold — Drones queue rather than spawn | Insufficient to run anything |
| Claude | CLI present, authenticated, quota against the Machine floor | Authenticated, quota above the floor | Quota below the reserved floor — dispatch gated | Not authenticated, or CLI absent |
| Manifest | Every known `armada.yml` parses, schema current, no drift | All parse, all schemas current | Some parse and some do not, or some have drifted | None parse, or none found |
| Kit | `kit.yml` present, parses, schema version current | Parses, schema current | Parses, schema behind, migration pending | Missing or unparseable |
| Machine | `machine.yml` present, parses, schema version current | Parses, schema current | Parses, schema behind, migration pending | Missing or unparseable |
| Git | Binary present, version against the supported minimum | Present, version adequate | Present, version below the supported minimum | Absent |

### Graded and binary — deliberately

Fleet, Armada API, Docker and Keychain have **no warn state and never will**. Each is reachable or it is not, and there is nothing between. Forcing a middle onto them would mean inventing a threshold nobody can justify.

The honest middle for Fleet would be "answering but crash-looping," which is real — and under launchd it is now a *likely* condition rather than a hypothetical, since restart is automatic and uncapped. It stays rejected because it is **history, not current state**, and Doctor holds no state by design. It belongs in Alerts.

**What warn means across the modules that have it:** all but one are "works now, will break later" — old git, stale Kit schema, stale Machine schema, stale SQLite schema, low quota, drifted Manifest. Only System stats is degradation in the live sense. Warn is the row you fix before it becomes a fail.

**Thresholds are settings, not constants.** Headroom %, quota floor % and the supported git minimum are Machine-level and tunable post-ship. Several already exist as rows in the Configuration Settings registry.

One further thing lands here: after N denials of the same command across Jobs, Doctor surfaces a **denial-frequency rollup** with a suggested change to confirm or decline. It is never auto-applied. Named in an early implementation step from the retired nine-phase plan (Pattern-learning and pre-authorization). Where the rollup sits relative to the module grid is unspecified (see Open questions).

## The first-run hard gate

Doctor is step 3 of four in First-Run Onboarding: Guild Init, Set Up a Project, Check System Health, Dispatch a Job. The name of the first step, and whether the sequence holds four steps or five, is tracked in [Kit](kit.md).

- On a fresh install, dispatching before Doctor reports green is **blocked**.
- This is **the one place in Armada where step order is enforced** rather than left as a recommendation.
- Gating is **between steps only**. Nothing inside a step is gated.
- Once onboarding ends, no gating remains anywhere in the app. Doctor never gates dispatch again.

The reason is diagnosis: dispatching before Fleet, Git and Docker are verified produces an opaque failure with no context on why.

**This gate is Doctor's first consumer**, and the first point at which a module's probe has to exist. An implementation step from the retired nine-phase plan (First-Run Onboarding hard-gate) is where the probes were built to land.

## Relationship to Fleet and to Bridge

**Fleet.** Fleet has no engineer-facing surface of its own. Its own health status is **one module among several** in Doctor's grid. Fleet is infrastructure, and you never go to Fleet the way you go to Job Board or Helm. On a Fleet crash, auto-restart is attempted first; if that fails, Doctor shows Fleet as fail and an explicit "Restart Fleet" action becomes available as a manual fallback.

**Fleet against Armada API.** Fleet is the daemon. Armada API is the `api` crate, the transport Bridge reconnects to on reopen. Fleet's module answers whether the daemon is healthy; Armada API's answers whether Bridge can reach it.

**They are one process.** `api` runs in-process with `fleet` — see [Fleet](fleet.md), Daemon Lifecycle. Doctor still reports while Fleet is down, through `fleet-bin doctor --json`, a short-lived probe Bridge spawns rather than a second daemon.

**Bridge.** Doctor is a **surface, not a peer concept to Bridge**. Bridge is the Electron shell; Doctor is one of the seven surfaces mounted inside it. Doctor's behaviour is documented in the Check System Health journey, which is why Bridge's own document carries a pointer rather than a description.

## Result vocabulary

**pass / warn / fail**, rendered circle-wrapped so a Doctor result never reads as a Job state.

```
pass   circle-check   / CircleCheck   --status-completed-success
warn   triangle-alert / TriangleAlert --status-awaiting-review
fail   circle-x       / CircleX       --status-completed-failed
```

16px, inheriting the cell's status colour. Doctor is the one place `triangle-alert` lives, which is why `octagon-alert` was reserved to `stalled` and kept out of generic warnings. The bare `check` and `x` belong to the badge set and are not used here.

**Colour is shared with Job states, the glyph is not.** Three new Doctor tokens were rejected: a health grid that renders green, amber and red in values nobody else uses makes a problem harder to spot, not easier, and one colour vocabulary across the app is worth more than a private one here. The circle wrap is what keeps a Doctor cell from reading as a Job badge, which is why it survives the shared palette rather than being dropped alongside it.

## What is already resolved

| Question | Resolution |
| --- | --- |
| Is Doctor an action or a scan? | **Passive scan.** Checked on demand, with no push notification on module failure |
| One blended health score, or per-module? | **Per-module**, each independently pass, warn or fail. No blended score |
| Does Doctor run its own checks? | **No.** Doctor owns no probe logic and no state. It invokes one probe per module through one `HealthReport` contract, plus one client-side fact, and renders what comes back. Probes live beside their subjects, in `adapters`, `config` and `store`, and are invoked by `fleet-bin doctor --json` — a short-lived process Bridge spawns. **Everything is probed and nothing self-reports, with Armada API as the one stated exception** — resolved Aug 2026 |
| How many modules? | **Wrong question — amended 2026-08-21.** The count is not a contract; the rule generates the list and the number is its output. A module earns a row where **Armada depends on it and it can be up or down** — it is whatever needs surfacing for a person to be confident the systems are in place and working. This document has asserted eight, nine and ten at different times, and each was treated as a fact to reconcile. A count stated in prose only goes stale and then gets copied, which is how "nine" reached three other pages. Carry the rule and the current list, never a number in a sentence. Currently: Fleet, Armada API, Kit, Machine, Manifest, SQLite, Git, Docker, Claude, Keychain, System stats |
| Does Fleet get a surface of its own? | **No.** Fleet's own health status is one module in Doctor's grid, and that is Fleet's only engineer-facing surface |
| Is Doctor a peer concept to Bridge? | **No.** One of Bridge's seven surfaces (six until Manifest was added 2026-08-21) |
| Does Doctor gate dispatch outside first run? | **No.** The hard gate is unique to First-Run Onboarding. Afterwards all four surfaces are freely revisitable with no gating between them |
| Which words, and which colours? | **pass / warn / fail**, on the existing `completed-success`, `awaiting-review` and `completed-failed` tokens. Resolved Aug 2026 |
| Do Doctor results reuse the Job badge glyphs? | **No glyphs at all — amended 2026-08-21.** A result is the word `pass`, `warn` or `fail` in the status colour. `circle-check` and `circle-x` were reserved to Judge criterion verdicts on 21 Aug, so Doctor cannot own them; a column of words also scans better than a column of glyphs, and survives greyscale |
| What is the transport module called? | **Armada API**, matching the `api` crate. Renamed from "Armada Server" Aug 2026, on the same reasoning that dropped "Daemon" as a redundant name for Fleet. See `../contracts/system-architecture.md` |

## Where the sources disagreed

Recorded rather than silently reconciled. All four have a call as of Aug 2026, and the contradicted pages are listed in the next section rather than edited here.

- **The module count. The First-Run Onboarding journey says nine and names eight.** Its step 3 calls it a 9-module grid, then lists Fleet, Guild, Manifest, Daemon, Git, Docker, Claude, System. The Check System Health journey names nine, and its ninth is Armada Server. It also writes "System stats" where the other writes "System". [Bridge](bridge.md) repeats "9-module grid" and names none. **Resolved: the rule generates the list, and no page carries a count.** Check System Health's list stands minus Daemon, and "System stats" is the module name. The number all three sources assert is the part that goes stale, which is why the resolution is a rule rather than a figure.
- **Fleet and Daemon were listed as two separate modules.** Fleet is the daemon everywhere else in the brief, and the sidecar of the original architecture decision. No source ever said what Daemon covers. **Resolved: Daemon drops** as a third name for Fleet. Armada API survives as a distinct module on a narrower reading: transport reachability rather than daemon health. Renamed from "Armada Server" Aug 2026 to match the `api` crate.
- **Three sources placed Doctor's build in three different phases of the retired nine-phase plan.** One early step built a Doctor grid as a minimal surface. The phase named for Manifest Setup & Doctor had a Done-when requiring the module grid to report accurately, yet none of its eleven steps built the checks. Check System Health's design note said the checks were built in that phase and the grid surfaced later. **Resolved by reframing:** Doctor owns no probe logic, so no step builds "Doctor's checks" and none is missing. Probes live beside their subjects, and the first consumer is the First-Run Onboarding hard-gate step. The earlier minimal-surface step lost the Doctor grid, since nothing consumed it that early and its own Definition of Done never mentioned it.
- **Colour words against result words.** Check System Health says red, yellow, green. `../contracts/iconography.md` says pass, warn, fail. `../contracts/design-system.md` maps status tokens one to one onto the Job state machine and names no token for a Doctor result. **Resolved: pass / warn / fail on the existing status tokens**, which are green, amber and red and so absorb the journey's colour words. Circle-wrapped glyphs stand.

## Pages these resolutions contradicted

**Correction pass complete, Aug 2026.** All seven pages below were corrected. The table is kept as a record of what changed and where, not as outstanding work.

| Page | What contradicted this document | Corrected |
| --- | --- | --- |
| First-Run Onboarding journey | Step 3 said "9-module grid" and listed Daemon and System | Modules named in full, Armada API included, no count in the sentence |
| Check System Health journey | Nine-row module table including Daemon. Colour words. Design note claimed a retired-plan phase builds Doctor's checks | Eight rows, pass/warn/fail, rollup framing, design note rewritten |
| [Bridge](bridge.md) | Journey table said "Doctor's 9-module grid" | Eight, linked here |
| Early minimal-surface step, retired nine-phase plan | Detail named the Doctor grid as one of its minimal surfaces | Dropped, with the reason recorded |
| Manifest Setup & Doctor phase, retired nine-phase plan | Done-when asserted a module grid no step owned | Rewritten to module self-reporting, rendered by the hard-gate step |
| First-Run Onboarding hard-gate step, retired nine-phase plan | Definition of Done did not state that module health reporting lands here | New first DoD line, with the eight modules named |
| `../contracts/iconography.md` | Doctor section named the three glyphs but no tokens | Tokens added, plus the rejected-tokens reasoning |

## Open questions

- **[doctor-icon-and-word]** Does a Doctor health row carry both an icon and a status word, or the icon alone? Blocked on the Doctor layout, which is not designed.
- **[doctor-restart-fleet-placement]** Where does the "Restart Fleet" action render, and does it live in the Fleet module row?
- **[doctor-denial-rollup-placement]** Where does the denial-frequency rollup sit relative to the module grid?
