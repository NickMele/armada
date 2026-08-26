# Bridge

**What it is:** The Electron command-center shell, and the only engineer-facing surface in Armada — a client of Fleet rather than the daemon itself, so closing it stops nothing.

---

**Kind:** Surface.

This document exists so "Bridge" has one stable, correct link target, not to duplicate journey content. Bridge is a shell rather than a domain concept: it owns the chrome, and the work happens in the journeys that mount inside it.

## What it is

The Electron frontend — the only engineer-facing surface in Armada. Single-user, no auth layer (personal dashboard, not a shared team view). Chosen over a TUI specifically for richer visualization: multi-Drone monitoring, diffs, real-time UI (see `../contracts/system-architecture.md`, section 4).

## Relationship to Fleet

Bridge is a client of the Fleet daemon, not the daemon itself. It connects to the Armada API over **one axum listener** — WebSocket for the real-time event stream, HTTP for request-response commands, same port, no second port and no gRPC. `../contracts/system-architecture.md` owns that decision; the measured cost of `tonic` that settled it is recorded against the decision on which web framework the api crate uses, and whether gRPC earns its place.

Closing Bridge does not stop Fleet; reopening it reconnects rather than respawning. **What Bridge shows while it cannot reach Fleet, and what it trusts on reconnect, is not yet designed** (see Open questions).

**Bridge authors no vocabulary of its own.** Job statuses, their reasons, escalation reasons and verdicts all reach the renderer as one generated TypeScript module, emitted by the same codegen that produces the `ipc` types — names from `core-model`'s enums joined to a checked-in file carrying each variant's verb, icon and status token. Bridge imports it. It never holds a status list for a filter, a reason list for a badge, or a copy of the enum→verb map. `lib/job-states.js` was the previous answer and it drifted three times, most recently carrying six escalation reasons after the enum went to seven; the build now fails where a variant lacks a verb, an icon or a token, so the renderer cannot fall behind rather than being asked not to. `../contracts/system-architecture.md`, section 6 owns the mechanism.

**Bridge also calls `launchctl` directly**, outside the protocol: it bootstraps Fleet's launchd job at login and restarts it via `kickstart -k`. Why that is Bridge's job rather than an API operation, and what "Restart Fleet" actually means under launchd, are on [Fleet](fleet.md) — Daemon Lifecycle.

## Where Bridge's behavior is actually documented

Bridge has no behaviour section of its own because its behaviour is specified across the journeys below — each covering a distinct trigger rather than a slice of shared UI machinery:

| Journey | Trigger |
| --- | --- |
| Check System Health | "Is everything okay right now?" — [Doctor](doctor.md)'s module grid. **The count is not a contract** — a module earns a row where Armada depends on it and it can be up or down, and Doctor carries the list |
| Monitor Active Work | "What's currently running?" — lightweight heartbeat view |
| Dispatch a Job | "I want to start something" — Job Board approval flow |
| Triage Queue | "What's waiting for me?" — proactive Reviews/Alerts/Activity Feed check |
| Respond to a Push Alert | Reactive — a notification pulled you in, includes Debug/Pilot |
| Run and edit a Manifest | "I want to run this project's checks without dispatching a job" — the most recently added surface. Reads a project's Checks and Commands, runs any one of them as a rehearsal, and edits the file |

[Job Board](job-board.md) itself is a distinct concept with its own document, surfaced inside Bridge rather than a Bridge sub-page.

## Top-level shell

Bridge's shell is a **left resizable rail** for navigation, a **full-width panel** to its right where the journeys mount, and a **status bar fixed to the bottom**. Finer layout treatment within each journey remains UI/UX design phase work.

The rail carries Job Board, Active Jobs, Alerts, Reviews, Activity Feed, Doctor and Manifest. Helm sits below them. **The status bar reports Fleet and Doctor health continuously**, which is what lets the Manifest surface's Doctor strip disappear when every module passes without its absence being ambiguous.

**The bar names Fleet's state rather than only reporting health, and it is present when Fleet is down.** Running, not running and unreachable each get a string — see `../contracts/design-system.md`, Status bar. Not running and unreachable are separate because a missing runtime file and a live pid that does not answer call for different things. Whether the bar reads the same during onboarding, before Fleet is reachable, is tracked in `../contracts/design-system.md`.

## Still open

Almost nothing is scoped to Bridge as a config target: settings a person adjusts *in* Bridge are tagged to the concept each one affects — Job Board default view to Job Board, landing Manifest to Manifest — rather than to Bridge as a catch-all. **Notification routing is the exception**, and it owns a live problem worth reading: no dependency path carries config to Bridge, so the Electron side reads its own copy or a hardcoded default. See `../contracts/configuration.md` for the tiering rule.

The design record behind this document also carries decisions the prose above already reflects: whether Redirect, Kill and Pause are available on a healthy Drone (decided — all three, with a redirect recorded on the Job); what the status bar says for each of Fleet's runtime states (decided — three states, each named out loud, described above); whether escalation trigger names get plain-language treatment in Alerts (decided — plain label primary, enum recoverable in the detail view); and what the desktop side reads for Job statuses and escalation reasons (decided — codegen, one generated module, described above).

## Open questions

- **[bridge-reconnect-trust]** What does Bridge show while it cannot reach Fleet, and what does it trust on reconnect? Closing Bridge does not stop Fleet, and reopening it reconnects rather than respawning, but the reconnect behavior itself is not yet designed.
- **[bridge-notification-routing-path]** How does notification routing configuration actually reach Bridge? It is the one setting scoped to Bridge as a config target rather than to the concept it affects, and no dependency path currently carries config to Bridge — the Electron side reads its own copy or a hardcoded default instead of the resolved Kit/Machine value.
