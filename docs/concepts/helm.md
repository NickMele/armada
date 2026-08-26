# Helm

**What it is:** Orchestrator Agent session assisting with the Fleet — conversational, not a Drone.

---

**Kind:** Agent, Surface.

Defines Helm — a conversational orchestrator agent for reasoning across the Fleet, distinct from Drones. Companion to the main Armada brief.

## What it is

Not a Drone. Drones execute code against a workflow inside a repo, one-shot, headless. Helm is a different agent type entirely: a **persistent conversational session** with tool access to query and act on Fleet state — Jobs, Drones, evidence, escalations — rather than a structured, single-run execution.

## What it's for

| Capability | Example |
| --- | --- |
| Cross-Job pattern reasoning | "Why do all my stalled Jobs this week share a cause?" — reasoning across every Job in the selected Manifest, not one Job at a time |
| Natural-language control surface | Talk to it instead of clicking through Bridge / Debug / Pilot UI for exploratory questions |
| Planning / dispatch help | Describe a goal, Helm breaks it into candidate Jobs for you to review and queue |

This is additive to Bridge, not a replacement: Bridge is state-monitoring (what's happening), Helm is reasoning/synthesis across that state (why, and what to do about it).

## Placement

Inside Bridge, **scoped to the selected Manifest**. Selecting a Manifest opens a Helm session for it; Helm sees that Manifest's Jobs, Drones, evidence and escalations, and nothing outside it.

**Fleet-wide reasoning is not a Helm capability.** An earlier draft placed Helm as a standalone top-level surface reasoning across the entire Fleet. Dropped Aug 2026 — Bridge already carries a selected Manifest and the [Job Board](job-board.md) is per-Manifest by definition, so a Fleet-wide Helm would be the only thing in the product ignoring that selection. Anything genuinely cross-Manifest stays a Bridge job.

## Tools

Helm reaches Fleet through the **Fleet MCP**, the HTTP request-response surface Bridge also uses. Not a hand-picked toolset.

**What Helm is told at session start** — its toolset, the selected Manifest, its resolved authority and Voice — is specified on `../contracts/agent-prompt.md`, section 2.

Helm's advantage is not more access than you have. It is reasoning over results and chaining calls you would otherwise make one at a time — "why is Job 12 failing", "what needs my attention, one by one", "what is open that we could dispatch".

**The inventory is written**: 14 queries, 12 commands, 8 events. See `../contracts/system-architecture.md`, section 6. Helm gets all 14 queries and one command directly.

**Helm gets a strict subset, not a mirror.** The 8 WebSocket events are Bridge-only — an agent cannot be interrupted mid-turn. **Helm polls instead**, resolved Aug 2026: `get_events_since(cursor)` at the start of each turn, returning a count plus one line per event kind rather than the events themselves. Helm is never more than one turn stale, and fetches detail through the other queries only when it bears on what was asked. Fleet waking the session was rejected — it would make Helm a second notification channel alongside Alerts.

Every call is scoped to the selected Manifest — **except Doctor, which Helm reads machine-wide (decided Aug 2026)**. Doctor's modules are machine-level by nature: the Fleet daemon, disk, Armada API reachability. They are not Manifest-scoped and cannot be, so under strict scoping Helm could not answer whether the daemon is healthy — in a session whose whole purpose is *why, and what to do about it*. Machine health is often the cause: three Jobs stalled because the disk filled is exactly the cross-Job pattern Helm exists to find, and a strictly-scoped Helm would see three stalls and no reason. It leaks nothing, because the Manifest boundary exists to stop one project's work being visible from another, and machine health is **shared context rather than another project's work**.

What Helm may *do* rather than read is a separate limit — see Action authority below.

## Action authority

Helm's write access is deliberately narrow and maps onto the Intervention Ladder — what you do about a problem, ordered by how much you take over. It does **not** map onto alert levels, which are a separate axis.

| Rung | Action | Can Helm do this directly? |
| --- | --- | --- |
| 1 | Redirect — structured instruction sent to a Drone | **Yes** — Helm can act on this on your behalf |
| 2 | Kill & Redispatch — kill Drone, dispatch a fresh one with new context | **No** — always routes through your explicit approval, even if Helm drafted the plan |
| 3 | Break-glass Pilot — raw terminal takeover | **No, by definition.** A human at a keyboard is what rung 3 is |
| — | Any Job-level dispatch | **No** — the primary autonomy control stays strict and human-gated, no exception for Helm-originated proposals. Helm may *draft* a Job, which then sits at the same approval gate every Job sits at |

Rationale for the line at rung 1: stopping or redirecting something already approved is a much lower-stakes action than starting something new. Job dispatch is the system's core autonomy control (see [Fleet](fleet.md) — Scheduling and gating) and stays untouched regardless of who is proposing it.

**Pause is not on this ladder.** Anything escalated is already paused — Fleet never auto-kills, so a stalled or thrashing Drone is held with its worktree intact by the time you or Helm see it. **Helm may pause a healthy running Job — decided Aug 2026**, as part of the ruling that Redirect, Kill and Pause are all available on a healthy Drone: pause is either a healthy-Drone action or it is not an action at all. Note the rung-1 line is unchanged by that — a Helm-initiated **Redirect** on a healthy Drone is recorded on the Job, the same as a human one.

## Audit trail

Every Helm-initiated action is logged as its own distinct event type — never conflated with your manual actions (which are already distinguished from Drone evidence, per the Debug/Pilot design) or with Drone self-reports. Three-way separation: Drone evidence, human manual action, Helm-initiated action.

## Session model

**Session per Manifest.** Selecting a Manifest in Bridge opens its Helm session; the session belongs to that Manifest rather than to a question.

An earlier draft specified session-per-topic, opened for an investigation and closed when done. Dropped Aug 2026 — it requires deciding when a topic has ended, which is a judgement nobody wants to make mid-investigation. Manifest selection is a boundary that already exists.

Session retention and expiry stay Machine-configurable.

## Budget & cost

| Aspect | Resolution |
| --- | --- |
| Hard gating | None — Helm is not capped by the $ / quota-% gating that applies to Drone/Job spend |
| Visibility | Helm surfaces its own usage/cost so you can see what it's consuming, even though nothing enforces a cap |
| Rationale | Deliberate exception to the Known Risks "cost / rate-limit blowup" mitigation elsewhere in the brief — justified because Helm is a human-driven tool you're actively steering in real time, not autonomous background spend. Revisit if usage patterns prove this wrong. |

## Voice & conduct

Helm is the only surface in Armada that speaks in first person. These are behavioural constraints, not styling. Source: `../contracts/design-system.md`.

**First person is Helm's alone, and only for what Helm itself did.** Bridge and Fleet never say "I". Helm says "I" for its own Redirects, its own reasoning and its own suggestions. When reporting a Fleet-originated event, Helm uses the same impersonal phrasing Bridge does — "Drone 4 stopped reporting", never "I paused Drone 4".

This is load-bearing for the Audit Trail above. If Helm narrates Fleet's work as its own, the three-way separation between Drone evidence, human action and Helm action stops being legible in the one place a human actually reads it.

**Answer, plus at most one observation.** Helm answers what was asked. It may add a single observation, subject to two conditions: it actually went and looked, and the observation is flagged as its own inference. No second and third observation, no throat-clearing openers.

> Job 12 failed at step 3. `pnpm test` exited 1 on 4 assertions.

> I checked the last three jobs in `api`. That suite has now failed on the same assertion twice. The test may be the problem, not the drone.

**Hedge by source.** Helm is one of only two things in Armada permitted to state a hypothesised cause (Judge is the other). Fleet never guesses in its own voice. When Helm speculates, it says so.

**Not linted.** Helm replies are governed by prompt only, not by the copy lint — a lint in a real-time conversational loop is visible lag. See `../contracts/agent-copy.md`.

## Configuration

The following settings (see `../contracts/configuration.md`) directly affect this concept:

| Setting | Scope |
| --- | --- |
| Helm action authority (Tier 1 Redirect enabled vs. read-only) | Machine |
| Helm budget soft-warning threshold | Machine |
| Helm session retention / expiry | Machine |
| Voice/tone | Machine — tunes Helm's replies within the Voice & Conduct constraints above; may adjust length and formality, may not override them |
