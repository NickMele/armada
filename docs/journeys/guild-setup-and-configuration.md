# Guild Setup & Configuration

**What it is:** Two related but distinct moments — setting a new machine up for the first time, and coming back later to change a default.

Design fidelity: not set. Analysis: Complete. UI/UX design: Not started.

---

*Numbering note: the design project has not yet drawn this journey — `UI/UX Design` reads `Not started` and its own page names no `Journey N - ...` file. Journeys 1 through 4 and 9 carry numbers stated in the design project itself; this journey and the six after it are numbered here only to give the file set a stable order, continuing after the highest number the design project has assigned. Renumber this file if the design project later assigns a different one.*

*This journey's own title predates the Guild → Kit / Machine split (retired Aug 2026) and is left as Notion has it, per the retitling question still open below.*

**Trigger:** First-time Init, or an ongoing settings change.

**Concepts touched:** Kit, Machine.

**Milestone:** Reach.

## Section 1 — First-Time Init

**Trigger:** You just installed Armada on this machine.

Guided, sequential walkthrough of Guild's four functional groups (AI Behavior, Resources & Budget, Safety, Interface & Notifications), sensible defaults pre-filled, you adjust as you go. Machine-level only — does not extend into setting up your first project. See Set Up a Project (Manifest) and First-Run Onboarding, the two journeys this section chains into.

What this journey is called, and whether one surface still walks all four groups, is open — see Open questions below. The groups divide across Kit and Machine.

## Section 2 — Ongoing Edits

**Trigger:** You want to change a default — add a Skill, adjust budget caps, update the allowlist, etc.

Navigate to the relevant functional group → edit → changes apply immediately to Kit's own config. If the change affects Skills/MCP/Agent files, **Push to Claude** carries it into your live Claude environment automatically — no separate action to remember. The exception is a same-named conflict with config Claude already has outside Armada, which surfaces for you to resolve (see Kit for conflict-resolution behavior).

Also covers Import/Export (Git-repo or file-based, for moving one Kit config across your own machines) and Upgrade (automatic schema migration on Fleet startup).

## What is already decided and landed

- **Push to Claude's conflict-resolution behaviour is configurable, not fixed.** Now a config row ("Push to Claude conflict-resolution policy"), user-configurable. Non-conflicting items merge silently; same-named conflicts with pre-existing Claude config surface for resolution.
- **Machine Voice cannot widen or narrow the agent-copy lint, in either direction.** Voice sets tone — length and formality of runtime-generated prose. The lint blocks generated-text tells — importance puffery, weasel attribution, faux-insight setups. A warmer Voice needs friendlier phrasing; it does not need puffery, so the failure a configurable lint was meant to prevent never arises. The lint also now hard-gates commit messages and PR descriptions, which land in a real repository permanently, so a Voice setting able to widen it would let a tone preference weaken a gate. Under the Kit/Machine split, Voice is a Machine setting — one tier, no project-level counterpart, no merge — so "narrow versus widen" was never the right frame for it in the first place.
- **Which Workflow thresholds had no config row — closed 21 Aug 2026.** Four rows were added: **Review gate policy** (`manifest_rule:review_gate`), **Loop iteration cap** (`iteration_cap`, 5), **Poke limit** (`poke_limit`, 2), and **Drone heartbeat interval** (`heartbeat_interval_minutes`, 5 min) — split out from the existing liveness *timeout* row, because an interval and a timeout are different quantities and the interval must be shorter than the timeout, validated at config load. Review gate policy is filed Manifest-only, not Kit → Manifest, because a per-repo override that loosens a Kit-set rule would have widened it — the same shape as Auto-merge policy. The Judge confidence threshold row is superseded: the confidence notion was dropped (see Triage Queue), so it has no referent. Still outstanding: the Workflow-type registry names `requires_worktree`, `requires_checks` and `ref_source`, none of which appear on the Workflow concept page or in any sample WorkflowDef.

## Open questions

- **[kit-machine-setup-surface-naming]** What are the Kit and Machine setup surfaces called, and is first-run setup one journey and one gated step, or two?
  Kit and Machine each carry an Init section on their own concept pages — Kit's covers the tool half, Machine's the installation half — and each states that the two chain together in First-Run Onboarding, but neither names the surface it describes. Kit's own Open Items name the chain as `Kit Init → Machine Init → first Manifest → Doctor check → first dispatch`: five steps, two named Inits. First-Run Onboarding's Flow holds four steps and one, named Guild Init. The concept pages and the journey pages disagree on both the names and the count. The four functional groups are already divided across the two concepts — Kit takes AI Behavior and the tooling half of Safety; Machine takes Resources & Budget and Interface & Notifications, with Helm action authority among them — but this journey's Section 1 still walks all four in one sequence, so the grouping is in question and not only the name. Kit travels between machines and Machine does not: Kit carries Import/Export, Machine has no counterpart, and First-Run Onboarding's fork at entry (import an existing Kit, or start fresh) applies to Kit alone, so a single combined step forks on half its content. This is the same open question First-Run Onboarding carries — one answer settles both pages.

- **[kit-import-missing-files]** What happens on Kit import when referenced Claude files are missing?
  Guild Init forks at entry: import an existing Kit from another machine, or start fresh. The import path asks only where the Kit lives and performs the import. A Kit references concrete Claude files — agent files, Skills, MCP servers, sub-agents, plugins — and those may not exist on the importing machine. Open: does import fail, warn, or import with the missing entries marked; if marked, what is that state called, since it is not "drifted," which means present-but-different; and does a Drone spawned under a Kit with missing references fail at spawn, or start without the capability. That last question matters more than it looks — a Drone silently missing a Skill it was supposed to have is the `silent-deny-no-commit` failure shape in a different costume: it proceeds without a capability and produces confidently incomplete work. This is the same question First-Run Onboarding files as "what happens on the Import path when the imported Kit references Claude files that do not exist on this machine."

- **[drone-secrets-handling]** How are secrets handled for Drones?
  Established principle: a Drone never holds secrets directly — Fleet brokers. Unresolved is the concrete mechanism, including unattended 1Password auth. Sharpened 22 Aug 2026 by the adversarial review: Checks are invoked by Fleet and Commands by the Drone, which is already decided and is exactly the boundary a brokering mechanism needs — a secret that only ever reaches Fleet-invoked execution is never in a Drone's process at all. All options are bounded by the unattended 1Password spike: if Fleet cannot read the vault with no human present, the answer collapses toward environment-variable injection regardless of preference. Also open on Set Up a Project (Manifest), which shares this item.

## Related

Kit, Machine — the two concept pages this journey's four functional groups split across. First-Run Onboarding — the hard-gated sequence this journey's Section 1 is step one of.

This journey has no number because the design project has not drawn it. A number in a filename here means a `Journey N` drawing exists to match it; inventing one would assert a correspondence that does not.
