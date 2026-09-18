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

## Kit's home

**Kit lives at `~/.armada`, and Workflows are the first thing read from it.** Fleet makes the folder, with `workflows/` inside it, when it starts. A definition in `~/.armada/workflows/` replaces the one Armada carries with the same `workflow_id` in every repository on this machine, and a repository's own `.armada/workflows/` replaces both — [Workflow](workflow.md) holds the rule. #425.

**A folder a person can see and sync**, apart from the store, the runtime file and [Machine](machine.md)'s settings: Kit travels and the Machine does not. Everything else this page lists arrives in the same home under #41, and nothing else is read from it yet.

## What a person already has, read to be shown

**Armada reads the harness's own home to show what is in it** — `#1491`. Somebody arrives with a way of working already built: skills they wrote, plugins they installed, a global file saying how they want an agent to behave, servers they connected. Kit opened on *Nothing in your Kit yet* with all of it one directory away, and **see** was the verb the build was missing.

**An adapter per harness**, decided with the owner on 18 Sep. Which folder, which file and how one harness spells a skill are `crates/adapters`' to know, and a second harness is a second adapter with nothing above it moving. `get_kit_inventory` is the read and `adapter_traits::HarnessSetup` is the seam.

**Six kinds of the eight are read**: Skills, Plugins, Agent file, Sub agents, Commands and the servers a person connected. The allowlist and the models list answer *not read yet, and why* — a rule drawn out of its two tiers reads as a grant, and both tiers are #41. **A kind is never drawn as empty when nothing looked**, which is the defect this read exists to end rather than repeat one row along.

**Reading is not granting, and no part of it is a control.** What is read carries a name, the thing's own words for itself and where it came from — no address, no command, no argument list, no environment — so there is nothing in the reading a server a Drone gets could be built out of. A server a person connected outside Armada is visible here and reaches no Drone: adding one to Kit and then allowing it are the two acts they were. That is `../scope.md`'s one confinement, kept where an import would have broken it.

**A file that will not read is named, with its reason, under the kind it belongs to.** It costs that kind and nothing else, and nothing offers to repair it: the reader has no write to reach for.

**Write-back is decided and not built.** Kit writes to the harness's home rather than keeping a copy — one setup, Armada a window onto it — because two copies drift the moment either is touched and nobody can then say which a Drone ran against. The cost is that Armada would be writing into a directory it does not own, holding work a person depends on outside Armada entirely, so the read landed first and carries no write at all. #41.

**A Kit definition that does not fit a repository is left out there, and named.** One that will not parse, names a Check that repository does not declare, or shares its id with another Kit file, is set aside — two sharing an id are both left out, and named together. Fleet starts anyway and says at start which definition was left out, why, and whose runs instead: *Kit's `bug` was left out, because …; Armada's `bug` is used instead.* One bad Kit file never stops Fleet for every repository. The owner's decision. One the repository replaces is never resolved against it. A repository's own definitions stay strict, because the repository declared them.

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

**MCP servers are the first row of this that is built** — `#1275`. Fleet keeps the set, a Manifest's word is a row of its own, and `core_model::a_drone_resolves` is the whole resolution: the Manifest's word where it has one, Kit's default where it has none. Absent is not a third word. The servers a Drone is spawned against are written from that answer and nothing else, so what Bridge draws and what a Drone reads cannot disagree.

**A server added to Kit reaches no Drone.** `KitServer::added` takes no reach and there is no constructor that starts one anywhere else, so turning one on is a second act with its own operation. That is `../scope.md`'s one confinement kept: without `--strict-mcp-config` a v1 Drone came up holding every server the operator had connected, and a Kit whose rows arrived switched on would be the same defect through the front door.

**A scout gets none of them.** [Scout](scout.md) starts with no server at all, and the sources it reads are fetched by Fleet rather than opened by the agent — so "a scout's connections" is a different question from this one, and nothing here widens a scout.

**Helm's set is still the person's own, resolved by the CLI** — `#1373`, and Kit replaces none of it yet. Helm simply launches without the flag a Drone launches with.

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

**Of this surface, one row is built**: the MCP servers a Drone gets, added, removed and allowed per Manifest from the Manifest surface in Bridge (`#1275`). Kit has no rail row of its own — the rail's order is [Bridge](bridge.md)'s, and where this surface finally lives is the `kit-setup-surface-naming` question below.

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

**Not built, and nothing in `#1275` writes a person's own Claude configuration.** A server added to Kit reaches a Drone and nobody else; a terminal session is unaffected.

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
