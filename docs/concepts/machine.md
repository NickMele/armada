# Machine

**What it is:** How this installation behaves — resources, timing, budget, interface and notification routing. One tier, no project-level counterpart, no merge.

---

**Kind:** Entity.

Defines Machine — how this installation of Armada behaves. Split from [Kit](kit.md).

## What it is

The settings that describe **this installation** — its resources, its timing, its budget, and how loudly it speaks to you. Machine is the half of the old Guild concept that no project has an opinion about.

**A Machine setting has one value.** There is no project-level counterpart, no merge strategy and no resolution order. A [Manifest](manifest.md) never participates.

## The test

**Does a project-level version of this setting make sense?** If yes, it belongs to [Kit](kit.md) — a project adding a Skill is ordinary. If no, it is Machine.

There is no project-level version of *how loudly Armada notifies you*; the notion does not parse.

**Your Kit travels. The Machine does not.**

**Port block granule is the one knowing exception.** A project-level version does parse — a monorepo of compose stacks wants a larger granule than a repo of libraries — so the test above says Kit.

It stays Machine because the preference is unfelt: demand already drives a claim's width, and what the granule buys is headroom for a mid-Job widening to extend in place, which is rare. The signal to move it is a repo where widenings routinely fail to extend and re-claim, changing port numbers under a running worktree.

## Two functional groups

### Resources & Budget

- CPU/mem headroom threshold
- Fleet health-check and resource-poll interval
- SQLite WAL checkpoint interval
- Concurrency cap
- $ cost cap
- Quota % floor
- DAG scheduling tiebreak
- Network-loss retry policy
- Worktree root path
- Log retention and pruning
- Port range base
- Port range ceiling
- Port block granule

### Interface & Notifications

- Job Board default view
- Landing Manifest
- Notification routing
- Voice/tone
- Helm budget soft-warning threshold
- Helm session retention
- Helm action authority

**Helm action authority is Machine, not Kit.** Why: what Helm may do is a property of the installation, not of a repo.

## Notification routing

**Routing is a Machine setting with one value and no merge.** What bounds it is a **product rule on `../contracts/design-system.md`**, not a config tier: the loudness order, and the rule that an approval may never be promoted to push.

That rule holds because escalations and approvals mean different things. An escalation means work has stopped and nothing progresses until a person looks. An approval means work is waiting to start and will keep.

## Voice/tone

**Voice governs runtime-generated prose only** — Judge summaries, Helm replies, Job summaries. Static UI chrome is not configurable.

Voice may adjust length and formality. It may not override the voice principles, the lexicon or the status grammar: "terse" and "explanatory" are legal values, "playful" is not. Bounded by the `../contracts/design-system.md` contract.

## Budget and usage control — two gate types, selected here

Which one applies is a property of the machine, not the project.

| Machine type | Gate |
| --- | --- |
| Work machine, API-billed | **$ cost cap** per Job or Drone |
| Personal machine, subscription plan | **Gate on % of the 5-hour or weekly quota window remaining** |

The quota gate is the same mechanism as CPU and memory gating. A floor of quota is reserved for manual and interactive use, so Fleet cannot lock you out of your own Claude.

## Init

The installation half of first-run setup: resources, budget mode, notification routing, interface defaults. The tool half belongs to [Kit](kit.md). Both chain together in the broader First-Run Onboarding journey.

What this combined surface is called, and whether it is its own journey and its own gated step, is tracked in [Kit](kit.md).

## Configuration

All rows in the Configuration Settings registry tagged `Machine`. A Machine row carries no `Merge strategy` and no `Peer polarity` — both read `n/a — single layer` by construction.

## Open questions

- **[machine-voice-owning-tier]** Does Voice belong to Machine or to Kit? Voice is a Machine setting by the group mapping above, but `../contracts/agent-prompt.md` places it in prompt layer 2 beside Skills, the Agent file and Sub agent definitions — all of which are Kit. The two readings disagree about which concept owns it.
- **[machine-manifest-budget-precedence]** Does a Manifest budget cap override the Machine cap, or are they independent single-layer caps? A Manifest budget cap exists as a Manifest-only row, which means budget demonstrably *does* have a project-level version — the one place the Kit/Machine test gives an unclear answer. Precedence between the two caps is separately open.
