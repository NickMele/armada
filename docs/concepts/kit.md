# Kit

**What it is:** The tool set you bring — Skills, MCP servers, sub agents, Agent files, Plugins, Commands, the allowlist, the models list. Holds defaults a Manifest may extend or restrict.

---

**Kind:** Entity.

Defines Kit — the tool set you bring. Companion to `../contracts/configuration.md`.

**Guild is retired from the lexicon.** It was two concepts wearing one name, split into Kit and [Machine](machine.md).

## What it is

The tool set you bring: Skills, MCP servers, sub agents, Agent files, Plugins, Commands, the allowlist, the models list. Settings **plus** onboarding, not an occasional-visit config screen. Kit is where your tooling gets set up in the first place (Init), and where you return to edit it afterward.

**Two tiers.** Kit holds your defaults; a [Manifest](manifest.md) holds its own version; they merge with a strategy and a resolution order. **Kit rows are the only ones with a Manifest tier.**

**Kit sets defaults a Manifest may extend or restrict — not a ceiling a Manifest may only narrow.** Nothing above the Manifest constrains anything; the config direction rule is withdrawn. Resolution *across* several peer Manifests is a separate rule and is unaffected — see [Drone](drone.md).

## Scope

The machine's AI tool set and defaults, and everything in `../contracts/configuration.md` tagged "Kit only" or "Kit → Manifest."

Resources, budget, timing, interface and notification routing are **not** here — those are [Machine](machine.md). Setting up your *first project* is also out of scope — that's Manifest's Init, chained together in the broader First-Run Onboarding journey (not yet designed).

## Navigation — two functional groups

The original four groups split across the two concepts: AI Behavior and the tooling half of Safety are Kit; Resources & Budget, Interface & Notifications and Helm action authority are [Machine](machine.md).

| Group | Contents |
| --- | --- |
| AI Behavior | Skills, MCP, Agent files, Plugins, Way I work, Expectations, Workflows, Sub agents, Models, AI-assisted prompt |
| Safety — the tooling half | Allowlist defaults, destructive-op list defaults |

- Agent files are the global "how I work" file — Kit-only, no Manifest-level counterpart, see [Manifest](manifest.md).
- Sub agents are global; a Manifest can layer project-specific ones on top.
- Helm action authority is not here. It is [Machine](machine.md), and was mis-tagged with a Manifest tier before the split.

## Destructive-op list defaults

**Governs Drone-initiated operations only** — what a Drone must stop and ask you about mid-run. Your own Bridge actions (kill a Drone, override a dispatch freeze, force-merge) always confirm and are not configurable.

**Finding, not yet designed, and filed rather than replaced.** Union-only was what guaranteed a Manifest could never remove a confirmation Kit required; with the direction rule withdrawn, nothing does.

## Two-tier inheritance

**Kit holds the default allowlist, and a [Manifest](manifest.md) can extend or restrict it per project.** The same inheritance pattern governs Skills, MCP and Plugins. **Extend or restrict, in either direction, is the model wording for every two-tier setting.**

**Known cost: allowlist rot.** Two-tier inheritance keeps changes scoped, but upkeep is ongoing as new tools are needed. No automated solution exists; worth monitoring rather than solving now.

**Findings the withdrawn direction rule leaves open**, filed rather than replaced:

- Nothing prevents a Manifest removing a Judge trigger.
- Nothing prevents a Manifest selecting a model outside the Kit set.
- Nothing prevents a Manifest removing a required destructive-op confirmation.
- A Command that bypasses the allowlist bypasses the denial record, which was the part that was working.

**Budget is not here.** Billing mode, cost caps and the quota floor are [Machine](machine.md).

## Init

Guided, sequential walkthrough of the groups above, with sensible defaults pre-filled — you adjust as you go rather than starting from a blank form. The tool half only; resources, budget and interface are [Machine](machine.md) Init, and first-project setup is Manifest's (see Scope).

What this surface is called, and whether it is its own journey, is tracked in Open questions below.

## Actions

| Action | What it does |
| --- | --- |
| Edit / View | Standard settings editing across the functional groups above |
| Push to Claude | Keeps your live local Claude environment in step with your Kit |
| Import / Export | Carries one Kit across your own several machines |
| Upgrade | Schema version auto-increment plus migration scripts |

"Sync" is not a standalone action. The name conflated two distinct ideas, split into Push to Claude and Import / Export.

### Push to Claude

Keeps your live local Claude environment in step with your Kit's Skills, MCP and Agent files.

**Automatic.** When you edit your Kit, Fleet pushes the change out; you never have to trigger it. Non-conflicting items merge silently.

**A same-named conflict with config Claude already has outside Armada surfaces for you to resolve**, rather than being silently overwritten. That is the one time you are involved.

A manual re-push exists as recovery — after resolving a conflict, or repairing a Claude install — not as the normal path.

### Import / Export

Two mechanisms, both carried over from the v1 prototype:

- Git-repo push/pull, for continuous cross-machine portability.
- One-off file export/import, for a simple copy.

**Your Kit travels, the Machine does not.**

### Upgrade

**Same pattern as Manifest and Job schema versioning:** auto-increment schema version plus migration scripts, applied automatically on Fleet startup.

## Configuration

The settings in `../contracts/configuration.md` that directly affect this concept are its "Kit only" and "Kit → Manifest" rows. This document's config analysis added these settings to that registry:

- Kit-level allowed/default models list
- AI-assisted prompt toggle
- Import/Export Git repo target
- Push to Claude conflict-resolution policy
- Push to Claude target (which local Claude install)

**The last three classify as [Machine](machine.md) and are flagged as low-confidence.** They configure the mechanism by which a Kit travels, which sits awkwardly across the split.

## Still open

First-Run Onboarding (Kit Init → Machine Init → first Manifest → Doctor check → first dispatch) is analyzed but not yet designed — tracked as a user journey, not as an open item here.

## Open questions

- **[kit-setup-surface-naming]** What is the Kit setup surface called, and is it its own journey? Whether First-Run Onboarding uses the step names above, and whether Kit Init and Machine Init are one step or two, is open.
