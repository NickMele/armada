# Bridge

**What it is:** The Electron command-center shell, and the only engineer-facing surface in Armada — a client of Fleet rather than the daemon itself, so closing it stops nothing.

---

**Kind:** Surface.

This document exists so "Bridge" has one stable, correct link target, not to duplicate journey content. Bridge is a shell rather than a domain concept: it owns the chrome, and the work happens in the journeys that mount inside it.

## What it is

The Electron frontend — the only engineer-facing surface in Armada. Single-user, no auth layer (personal dashboard, not a shared team view). Chosen over a TUI for richer visualization: multi-Drone monitoring, diffs, real-time UI (see `../contracts/system-architecture.md`, section 4).

## Relationship to Fleet

**Bridge is a client of the Fleet daemon, not the daemon itself.** It connects to the Armada API over **one axum listener** — WebSocket for the real-time event stream, HTTP for request-response commands, same port, no second port and no gRPC. `../contracts/system-architecture.md` owns that decision and the measured `tonic` cost behind it, recorded against the decision on which web framework the api crate uses.

Closing Bridge does not stop Fleet; reopening it reconnects rather than respawning. **What Bridge shows while it cannot reach Fleet, and what it trusts on reconnect, is not yet designed** (see Open questions).

**Bridge authors no vocabulary of its own.** Job statuses, their reasons, escalation reasons and verdicts all reach the renderer as one generated TypeScript module, emitted by the same codegen that produces the `ipc` types — names from `core-model`'s enums joined to a checked-in file carrying each variant's verb, icon and status token. Bridge imports it, and never holds a status list for a filter, a reason list for a badge, or a copy of the enum→verb map.

`lib/job-states.js` is retired vocabulary: it was the previous answer and it drifted three times, most recently carrying six escalation reasons after the enum had gone to seven. The build now fails where a variant lacks a verb, an icon or a token. `../contracts/system-architecture.md`, section 6 owns the mechanism.

**Alerts show an escalation trigger's plain-language label first.** The enum name stays recoverable in the detail view.

**Bridge also calls `launchctl` directly**, outside the protocol: it bootstraps Fleet's launchd job at login and restarts it via `kickstart -k`. Why that is Bridge's job rather than an API operation, and what "Restart Fleet" actually means under launchd, are on [Fleet](fleet.md) — Daemon Lifecycle.

## Where Bridge's behavior is actually documented

Bridge has no behaviour section of its own. Its behaviour is specified across the journeys below, each covering a distinct trigger rather than a slice of shared UI machinery.

| Journey | Trigger |
| --- | --- |
| Check System Health | "Is everything okay right now?" — Doctor's module grid |
| Monitor Active Work | "What's currently running?" — lightweight heartbeat view |
| Dispatch a Job | "I want to start something" — Job Board approval flow |
| Triage Queue | "What's waiting for me?" — proactive Reviews/Alerts/Activity Feed check |
| Respond to a Push Alert | Reactive — a notification pulled you in, includes Debug/Pilot |
| Run and edit a Manifest | Run this project's checks without dispatching a job |

The module grid is [Doctor](doctor.md)'s. **The count is not a contract** — a module earns a row where Armada depends on it and it can be up or down, and Doctor carries the list.

Run and edit a Manifest reads a project's Checks and Commands, runs any one of them as a rehearsal, and edits the file.

**Redirect and Kill are both available on a healthy Drone.** A redirect is recorded on the Job.

**Watching a healthy Drone work is [Observe](observe.md)**, opened on one Job and read-only. It is not on the Board, which stays a scanning surface.

[Job Board](job-board.md) itself is a distinct concept with its own document, surfaced inside Bridge rather than a Bridge sub-page.

## Top-level shell

Bridge's shell is a **title row** across the top — the repository picker, search, Dispatch and, once Helm's dock is closed, its own reopen button — a **left column** of three resizable panels beneath it, and a **full-width panel** to their right where the journeys mount. Finer layout treatment within each journey remains UI/UX design phase work.

**The left column stacks Navigation, Stats and Fleet**, one panel each, resizing and collapsing as a single unit rather than three panels each settling their own width — Bridge/1088's replacement for the rail and the status bar. See `../contracts/design-system.md`, Left column, and Component → token mapping.

Navigation carries Overview, Job Board, Studios, Alerts, Doctor, Manifest, Cleanup and Settings. Helm is not one of them — it is a dock beside the content on every surface, toggled by `⌘J` rather than a rail digit, and above the layout breakpoint a closed dock draws nothing at all: the title row's own Helm button is the one way back. See [Helm](helm.md).

**Studios is one repository's Studios, and one open on its whiteboard.** The list names each Studio and when it was last touched; a Studio opened from it is read-only until Continue, and accepting or rejecting a proposed relation, and deleting a node, happen there and nowhere else. See [Studio](studio.md).

**Settings is Fleet's four limits and this machine's own settings, on one screen** — Helm's action authority first. It replaced a sheet reached from the status bar (#1088 removed the bar; #1089 gave the sheet's contents a rail row instead), and it is where a limit changed still takes the same way it always has.

**Cleanup was last because it was newest, and that is the rule rather than a placement.** A surface joins at the end, so `⌘1`–`⌘4` kept reaching what they reached when Cleanup, then named Held worktrees, arrived. **Overview is the one surface that broke that rule**: it is where Bridge opens (#921), so it joined first rather than last, taking `⌘1` and pushing every other surface's digit down by one — Job Board `⌘2`, Alerts `⌘3`, Doctor `⌘4`, Manifest `⌘5`, Held worktrees `⌘6`, Settings `⌘7`. Studios broke it a second time on 2026-09-17 (#1287): the owner put it third, at `⌘3`, so Alerts, Doctor, Manifest, Cleanup and Settings each moved down one and Settings reads `⌘8`. See `../contracts/design-system.md`, Two tiers, for the rail's own digit history and for Helm's move off it.

What the rail draws is what is built, which is not yet the whole roster. A surface with nothing behind it would be a promise Armada does not keep, so it holds its place in the order and its digit without drawing a row.

**Active Jobs, Reviews and the Activity Feed are not on it.** All three were lists of Jobs standing beside the Board, and the Board now holds every Job with state as a filter — see [Job Board](job-board.md). Each is that list under one filter: running, `awaiting_review`, and over. Four lists of Jobs gave four surfaces four chances to disagree about what a Job row looks like, and a person had to learn which one held which state.

**Alerts is the one that stays.** An alert is a condition on a Job rather than a status a Job holds — thrashing, fan-out abuse and evidence-suspect are none of them Job states — so it is a different population with its own level and trigger structure, not a filter of this one.

> **Rule.** Alerts lists every repository Fleet serves, and never follows the Board's own pick.
> Why: an escalation interrupts, and one held back until a person switched repositories would not.

**The Fleet panel reports Fleet and Doctor health continuously**, in the left column rather than a bar fixed to the window's bottom. The Manifest surface's Doctor strip can therefore disappear when every module passes, without its absence being ambiguous.

**The panel names Fleet's state rather than only reporting health, and it draws even when Fleet is down.** Running, not running and unreachable each get a string — see `../contracts/design-system.md`, Component → token mapping, Fleet panel. Not running and unreachable are separate because a missing runtime file and a live pid that does not answer call for different things.

Whether the panel reads the same during onboarding, before Fleet is reachable, is tracked in `../contracts/design-system.md` (`status-bar-onboarding` — the slug predates the panel and is unchanged, since a citation resolves by name).

## Still open

Almost nothing is scoped to Bridge as a config target: settings a person adjusts *in* Bridge are tagged to the concept each one affects — Job Board default view to Job Board, landing Manifest to Manifest — rather than to Bridge as a catch-all. **Notification routing is the exception**: no dependency path carries config to Bridge, so the Electron side reads its own copy or a hardcoded default. See `../contracts/configuration.md` for the tiering rule.

**A person's Bridge preferences are a different path from that one, kept by Fleet rather than resolved from `armada.yml`.** `#927`. Fleet keeps a preferences table the way it keeps a person's admission limits — one row per name, an absent row reading as the shipped default — and serves it over `get_preferences`/`save_preferences`. Bridge's main process loads it when Fleet connects, saves on change, and publishes it in `BridgeState`; while Fleet is unreachable Bridge draws the shipped default and queues no save. `where_things_are_open` is the first preference this carries — whether Job detail's *Where things are* chapter opens collapsed or expanded — and moving the Board's own view and sort onto it is left for later.

## Open questions

- **[bridge-reconnect-trust]** What does Bridge show while it cannot reach Fleet, and what does it trust on reconnect? Closing Bridge does not stop Fleet, and reopening it reconnects rather than respawning, but the reconnect behavior itself is not yet designed.
- **[bridge-notification-routing-path]** How does notification routing configuration actually reach Bridge? It is the one setting scoped to Bridge as a config target rather than to the concept it affects, and no dependency path currently carries config to Bridge — the Electron side reads its own copy or a hardcoded default instead of the resolved Kit/Machine value.
