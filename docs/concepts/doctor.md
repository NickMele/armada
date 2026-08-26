# Doctor

**What it is:** Health check surface, one of Bridge's seven. Passive per-module grid, and the hard gate on first-run dispatch.

---

**Kind:** Surface.

Formalises Doctor, the health check surface inside [Bridge](bridge.md). The definition was split across the Check System Health and First-Run Onboarding user journeys, [Fleet](fleet.md), Bridge and `../contracts/iconography.md`; this document is the citable source and those pages link here.

## What it is

Doctor is one of Bridge's **seven** surfaces — seven since Manifest was added. It is a **passive scan, not an action**. It answers "is everything okay right now" by reporting, and it changes nothing.

**Doctor owns no probe logic and no state.** It invokes one probe per module through one `HealthReport` contract and renders what comes back, plus one client-side fact. Probes live beside their subjects, never in Doctor.

- **Per-module status**, each module independently pass, warn or fail. No blended score, so a problem in one module is never obscured by a healthy one.
- **Checked on demand.** Doctor itself never pushes.
- **Doctor never enters Alerts.** A health check is a standing condition, not a queued decision: a failing module has no workspace, no spend, no step and no Job id, and Alerts stays a list of Jobs.
- **A failure reaches you as a one-line neutral strip** above the Triage Queue tabs, naming which modules are failing and whether work is stopped, and disappearing when they all pass.
- Doctor sits under the Bridge section label in the sidebar with the other six surfaces, reachable in sidebar order. Its nav icon is `stethoscope` at 16px.

### Everything is probed. Nothing self-reports — with one stated exception

**The exception is Armada API.** Once Fleet and Armada API are one process, a probe for "is the transport reachable" and a probe for "is the daemon alive" read the same fact.

**Armada API's row is Bridge's own client-side connection state, not a probe.** Bridge already knows whether its socket is open, and asking a probe would mean asking the process it cannot reach. That state is genuinely different information — wrong port, stale connection, Bridge-side socket failure.

**One probe per module, plus one client-side fact.**

| Property | Why it follows |
| --- | --- |
| One code path | Doctor asks, everything answers, same `HealthReport` shape |
| Health is a pull, not a push | Self-reporting implies a push nobody wanted |
| No stale state | A self-report is as fresh as its last write; a probe is true when you asked |
| Uniform test surface | `testkit` fakes every module identically — unhealthy Fleet as testable as Docker |

**Probe logic lives beside its subject.**

| Module | Probes from | Why there |
| --- | --- | --- |
| Git, Docker, Claude, Keychain | `adapters` | Already owns talking to things outside Armada |
| Kit, Machine, Manifest | `config` | Reads and validates their own files |
| SQLite | `store` | The only crate holding the connection |
| Fleet | Its pidfile | No process is asked anything |

**`HealthReport` lives in `core-model` and the trait in `adapter-traits`.** Nothing reaches across a crate boundary to read a status, and `testkit` can fake an unhealthy anything.

**Health gets no crate of its own.** Revisit if health grows history, thresholds or a scheduler.

Why: no v1 measurement backs the seam, and a `health` crate would either
duplicate `adapters` or thinly wrap it. Grouping by surface rather than by
capability is the shape that grew v1's `core` to 38,470 lines. Merging two
crates later is an afternoon.

**`fleet-bin doctor --json` invokes the probes** — a short-lived process Bridge spawns, neither Fleet nor a second daemon. Why: Doctor is already specified as a pull, on demand, holding no state, a shape that fits a process existing for a quarter of a second.

| Measurement | Result |
| --- | --- |
| Eight probes, one cold process | 1068 ms |
| The same process, warm | 253 ms |
| `docker info` and `claude --version` | 98% of it, subprocess spawns in any process |
| Fleet's row — pidfile → `kill(pid, 0)` | 26 µs |
| Probes run concurrently, cold | About 760 ms, `max()` not `sum()`, bounded by Docker |

**Fleet's own row is an honest external probe**, because the prober does not ask Fleet anything, and it needs no second daemon. Running the probes concurrently is worth doing, not required.

## The module grid

**The grid as the rule below generates it:** Fleet, Armada API, Kit, Machine, Manifest, SQLite, Git, Docker, Claude, Keychain, System stats.

### What earns a row

**Armada depends on it, and it can be up or down.** Why: a row is whatever needs surfacing for a person to be confident the systems are in place and working.

**The count is not a contract.** The rule generates the list and the number is its output. Carry the rule and the current list, never a number in a sentence.

**Doctor reports service health, not Job-readiness.** A module passes when the thing it names is working, whether or not anything currently uses it. Docker running with no Check invoking it is a true pass.

The narrower rule — a row exists where failure prevents a Job from running —
excludes Docker outright, because Docker blocks nothing when no Manifest's
Checks use it.

**There is no fourth result.** Service health needs no conditional logic and no `n/a` for conditional modules.

**The rule is deliberately wide.** It bounds the grid at "things Armada depends on" rather than at a narrower set, which admits SQLite and Keychain. Network reachability would also qualify and is not yet a row.

Each row states what its probe reads and where the boundaries sit.

| Module | What the probe reads | Pass | Warn | Fail |
| --- | --- | --- | --- | --- |
| Fleet | Daemon alive and answering — pidfile → `kill(pid, 0)`, 26 µs | Alive, answering | **Never warns** | Not answering |
| Armada API | **Not a probe.** Bridge's own client-side connection state | Connected | **Never warns** | Wrong port, stale connection, Bridge-side socket failure |
| Docker | Is the daemon reachable | Reachable | **Never warns** | Not reachable |
| SQLite | `armada.db` opens, schema version matches, WAL is writable | Opens and writes | Schema behind, migration pending | Locked, corrupt, or the volume is full |
| Keychain | The macOS Keychain is unlocked and Armada's brokered scope readable | Unlocked, readable | **Never warns** | Locked or access denied — secret brokering fails |
| System stats | CPU and memory headroom against the Machine threshold | Above threshold | Below threshold — Drones queue rather than spawn | Insufficient to run anything |
| Claude | CLI present, authenticated, quota against the Machine floor | Authenticated, quota above the floor | Quota below the reserved floor — dispatch gated | Not authenticated, or CLI absent |
| Manifest | Every known `armada.yml` parses, schema current, no drift | All parse, all schemas current | Some parse and some do not, or some have drifted | None parse, or none found |
| Kit | `kit.yml` present, parses, schema version current | Parses, schema current | Parses, schema behind, migration pending | Missing or unparseable |
| Machine | `machine.yml` present, parses, schema version current | Parses, schema current | Parses, schema behind, migration pending | Missing or unparseable |
| Git | Binary present, version against the supported minimum | Present, version adequate | Present, version below the supported minimum | Absent |

**"Restart Fleet" means skip the throttle wait, not recover.** Under launchd, restart is automatic and uncapped. Where Fleet exited 0 deliberately, launchd leaves it down by design and Doctor must show the reason rather than a restart button.

### Graded and binary — deliberately

**Fleet, Armada API, Docker and Keychain have no warn state and never will.** Each is reachable or it is not, and there is nothing between. Forcing a middle onto them would mean inventing a threshold nobody can justify.

**Crash-looping belongs in Alerts, not Doctor.** "Answering but crash-looping" is real, and under launchd it is a likely condition rather than a hypothetical, since restart is automatic and uncapped. It is history, not current state, and Doctor holds no state by design.

**Warn is the row you fix before it becomes a fail.** All but one are "works now, will break later" — old git, stale Kit schema, stale Machine schema, stale SQLite schema, low quota, drifted Manifest. Only System stats is degradation in the live sense.

**Thresholds are settings, not constants.** Headroom %, quota floor % and the supported git minimum are Machine-level and tunable post-ship. Several already exist as rows in the Configuration Settings registry.

### Denial-frequency rollup

**After N denials of the same command across Jobs, Doctor surfaces a rollup** with a suggested change to confirm or decline. It is never auto-applied.

Where the rollup sits relative to the module grid is unspecified — see Open questions.

## The first-run hard gate

Doctor is step 3 of four in First-Run Onboarding: Guild Init, Set Up a Project, Check System Health, Dispatch a Job. The name of the first step, and whether the sequence holds four steps or five, is tracked in [Kit](kit.md).

- On a fresh install, dispatching before Doctor reports green is **blocked**.
- This is **the one place in Armada where step order is enforced** rather than left as a recommendation.
- Gating is **between steps only**. Nothing inside a step is gated.
- Once onboarding ends, no gating remains anywhere in the app. Doctor never gates dispatch again, and all four surfaces are freely revisitable.

The reason is diagnosis: dispatching before Fleet, Git and Docker are verified produces an opaque failure with no context on why.

**This gate is Doctor's first consumer**, and the first point at which a module's probe has to exist.

## Relationship to Fleet and to Bridge

**Fleet has no engineer-facing surface of its own.** Its own health status is one module among several in Doctor's grid. Fleet is infrastructure, and you never go to Fleet the way you go to Job Board or Helm.

On a Fleet crash, auto-restart is attempted first; if that fails, Doctor shows Fleet as fail and an explicit "Restart Fleet" action becomes available as a manual fallback.

**Fleet against Armada API.** Fleet is the daemon. Armada API is the `api` crate, the transport Bridge reconnects to on reopen. Fleet's module answers whether the daemon is healthy; Armada API's answers whether Bridge can reach it.

**They are one process.** `api` runs in-process with `fleet` — see [Fleet](fleet.md), Daemon Lifecycle. Doctor still reports while Fleet is down, through `fleet-bin doctor --json`, a short-lived probe Bridge spawns rather than a second daemon.

**Daemon, Armada Server and sidecar are retired names.** Fleet is the daemon and is the sidecar the original architecture decision named; the transport module is Armada API, matching the `api` crate. See `../contracts/system-architecture.md`.

**Doctor is a surface, not a peer concept to Bridge.** Bridge is the Electron shell; Doctor is one of the seven surfaces mounted inside it. Doctor's behaviour is documented in the Check System Health journey, which is why Bridge's own document carries a pointer rather than a description.

## Result vocabulary

**pass / warn / fail**, on the existing `completed-success`, `awaiting-review` and `completed-failed` status tokens — green, amber and red. `../contracts/design-system.md` maps those tokens one to one onto the Job state machine and names no token for a Doctor result.

**Colour is shared with Job states; Doctor has no tokens of its own.** A health grid rendering green, amber and red in values nobody else uses makes a problem harder to spot, and one colour vocabulary across the app is worth more than a private one here.

The result words carry no glyphs. A result is the word `pass`, `warn` or `fail` in the status colour: `circle-check` and `circle-x` are reserved to Judge criterion verdicts, so Doctor cannot own them, and a column of words scans better than a column of glyphs and survives greyscale.

The earlier circle-wrapped glyph set, which the wrap kept from reading as a Job badge:

```
pass   circle-check   / CircleCheck   --status-completed-success
warn   triangle-alert / TriangleAlert --status-awaiting-review
fail   circle-x       / CircleX       --status-completed-failed
```

16px, inheriting the cell's status colour. Doctor is the one place `triangle-alert` lives; `octagon-alert` is reserved to `stalled` and kept out of generic warnings. The bare `check` and `x` belong to the badge set and are not used here.

## Open questions

- **[doctor-icon-and-word]** Does a Doctor health row carry both an icon and a status word, or the icon alone? Blocked on the Doctor layout, which is not designed.
- **[doctor-restart-fleet-placement]** Where does the "Restart Fleet" action render, and does it live in the Fleet module row?
- **[doctor-denial-rollup-placement]** Where does the denial-frequency rollup sit relative to the module grid?
