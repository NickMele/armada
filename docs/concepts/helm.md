# Helm

**What it is:** A dock beside the content on every Bridge surface, holding the questions waiting on you and one running conversation per repository — not a Drone.

---

**Kind:** Agent, Surface.

Companion to the main Armada brief.

## What it is

Not a Drone. Drones execute code against a workflow inside a repo, one-shot, headless. Helm is a different agent type entirely: a **persistent conversational session** with tool access to query and act on Fleet state — Jobs, Drones, evidence, escalations — rather than a structured, single-run execution.

Helm is additive to Bridge, not a replacement: Bridge is state-monitoring (what's happening), Helm is reasoning/synthesis across that state (why, and what to do about it).

## What it's for

| Capability | Example |
| --- | --- |
| Cross-Job pattern reasoning | "Why do all my stalled Jobs this week share a cause?" |
| Natural-language control surface | Talk to it instead of clicking through Bridge / Debug / Pilot UI |
| Planning / dispatch help | Describe a goal, Helm breaks it into candidate Jobs to review and queue |

Cross-Job reasoning covers every Job in the selected Manifest, not one Job at a time. The control surface is for exploratory questions.

## Placement

**Helm is a dock, not a surface.** It sits beside the content on every Bridge screen at 1100px and wider, folding to an edge strip that opens as a sheet below that; `⌘J` toggles it from anywhere, including from inside a field. It carries no rail digit and does not appear in the sidebar. See `../contracts/design-system.md`, Two tiers, for the layout and `../concepts/job-board.md` for how a question on the Board relates to a question on the dock.

The dock's upper zone lists every question waiting on a person, from every repository Fleet serves — a Drone question, a held command, a Judge refusal — each card naming its own repository. Answering one there settles it on that Job's own detail too. The lower zone holds Helm's conversation.

**One running conversation per repository, with Start fresh.** Which repository Helm answers for is set by the most recent explicit act — picking a repository, "Discuss with Helm" on a card, or the dock's own switch — never by the picker moving on its own. Helm is hosted by Fleet and resumed from the stored session when a message needs it; no process idles between messages. A conversation clears after 30 quiet days (`settings.helm-session-retention-expiry`, Machine-scoped).

**Every ask names where the person is.** Bridge sends the screen, the picked repository, the open Job chipped above the message box, and the Board or Overview cursor row with each message; Fleet hands the session one line naming it ahead of what was typed, and the thread keeps only what the person wrote. `#1075`.

**Fleet-wide reasoning is not a Helm capability.** Why: a conversation belongs to the repository it is pointed at, so a Fleet-wide Helm would be the only thing in the product ignoring that.

Anything genuinely cross-Manifest stays a Bridge job.

**Built to change.** Conversations are looked up by a key that is a repository today, and Helm's host sits behind one interface — a key per topic, or a host inside Bridge instead of Fleet, is a switch on either, not a rebuild.

## Tools

**Helm reaches Fleet through the Fleet MCP**, the HTTP request-response surface Bridge also uses. Not a hand-picked toolset. What Helm is told at session start — its toolset, the selected Manifest, its resolved authority and Voice — is specified on `../contracts/agent-prompt.md`, section 2.

Helm's advantage is not more access than you have. It is reasoning over results and chaining calls you would otherwise make one at a time — "why is Job 12 failing", "what needs my attention, one by one", "what is open that we could dispatch".

**Helm gets a strict subset, not a mirror.** `../contracts/system-architecture.md`, section 6 carries the inventory of queries, commands and events. **Helm gets every query, and of the commands only the acts the owner gave it: redirecting a Drone, drafting Jobs, examining a Job, showing again, runs, servers, raising a cap within Fleet's Helm bound, and asking the person to approve a Job.** `fleet::helm::may` is that line, and the agent door refuses every other command to a Helm session by name — undoing a run stays the person's. With action authority set to read-only, Helm is refused every command.

**The 8 WebSocket events are Bridge-only.** Why: an agent cannot be interrupted mid-turn.

**Helm polls instead.** `get_events_since(cursor)` runs at the start of each turn, returning a count plus one line per event kind rather than the events themselves. Helm is never more than one turn stale, and fetches detail through the other queries only when it bears on what was asked.

**Fleet never wakes the session.** Why: it would make Helm a second notification channel alongside Alerts.

**Every call is scoped to the selected Manifest, except Doctor, which Helm reads machine-wide.** Doctor's modules are machine-level by nature — the Fleet daemon, disk, Armada API reachability — and cannot be Manifest-scoped, so under strict scoping Helm could not answer whether the daemon is healthy.

Machine health is often the cause: three Jobs stalled because the disk filled is exactly the cross-Job pattern Helm exists to find, and a strictly-scoped Helm would see three stalls and no reason. It leaks nothing, because the Manifest boundary exists to stop one project's work being visible from another, and machine health is **shared context rather than another project's work**.

What Helm may *do* rather than read is a separate limit — see Action authority below.

## Action authority

Helm's write access is deliberately narrow and maps onto the Intervention Ladder — what you do about a problem, ordered by how much you take over. It does **not** map onto alert levels, which are a separate axis.

| Rung | Action | Helm directly? |
| --- | --- | --- |
| 1 | Redirect — structured instruction sent to a Drone | **Yes**, on your behalf |
| 2 | Kill & Redispatch — kill Drone, dispatch a fresh one with new context | **No** — your explicit approval |
| 3 | Break-glass Pilot — raw terminal takeover | **No, by definition** |
| — | Any Job-level dispatch | **No** — Helm may draft only |

Rung 2 routes through your approval even if Helm drafted the plan. Rung 3 is a human at a keyboard, which is what rung 3 is. A Job Helm drafts sits at the same approval gate every Job sits at: the primary autonomy control stays strict and human-gated, with no exception for Helm-originated proposals.

The line sits at rung 1 because stopping or redirecting something already approved is much lower-stakes than starting something new. Job dispatch is the system's core autonomy control (see [Fleet](fleet.md) — Scheduling and gating) and stays untouched regardless of who is proposing it.

**The one exception is a cap.** A held Drone still costs money, and a Drone that was told to report and then went quiet is spending it without converging — so that one is killed rather than held. Holding is for a Drone waiting on a person, and a Drone that went quiet is not waiting, it is burning. **A Drone still writing inside its declared plan is doing neither, and is not killed.** Its worktree survives either way, which is what the rule was protecting.

**A Helm-initiated Redirect on a healthy Drone is recorded on the Job, the same as a human one.** The rung-1 line is unchanged by that.

## Audit trail

**Every Helm-initiated action is logged as its own distinct event type**, never conflated with your manual actions or with Drone self-reports. Manual actions are already distinguished from Drone evidence, per the Debug/Pilot design. Three-way separation: Drone evidence, human manual action, Helm-initiated action.

## Session model

**One conversation per repository, not per topic.** See Placement above for how it is reached and retained. A session per topic would require deciding when a topic has ended — a judgement nobody wants to make mid-investigation — so the boundary is the repository, which already exists, rather than a question.

## Budget & cost

| Aspect | Resolution |
| --- | --- |
| Hard gating | None — Helm is outside the $ / quota-% gating on Drone and Job spend |
| Visibility | Helm surfaces its own usage and cost; nothing enforces a cap |

The exception is deliberate, against the Known Risks "cost / rate-limit blowup" mitigation elsewhere in the brief: Helm is a human-driven tool you're actively steering in real time, not autonomous background spend. Revisit if usage patterns prove this wrong.

## Voice & conduct

Helm is the only surface in Armada that speaks in first person. These are behavioural constraints, not styling. Source: `../contracts/design-system.md`.

**First person is Helm's alone, and only for what Helm itself did.** Bridge and Fleet never say "I". Helm says "I" for its own Redirects, its own reasoning and its own suggestions.

**Reporting a Fleet-originated event uses the same impersonal phrasing Bridge does** — "Drone 4 stopped reporting", never "I paused Drone 4". Why: if Helm narrates Fleet's work as its own, the three-way separation between Drone evidence, human action and Helm action stops being legible in the one place a human actually reads it.

**Answer, plus at most one observation.** Helm answers what was asked. It may add a single observation, subject to two conditions: it actually went and looked, and the observation is flagged as its own inference. No second and third observation, no throat-clearing openers.

> Job 12 failed at step 3. `pnpm test` exited 1 on 4 assertions.

> I checked the last three jobs in `api`. That suite has now failed on the same assertion twice. The test may be the problem, not the drone.

**Hedge by source.** Helm and Judge are the only things in Armada permitted to state a hypothesised cause. Fleet never guesses in its own voice. When Helm speculates, it says so.

**Not linted.** Helm replies are governed by prompt only, not by the copy lint. Why: a lint in a real-time conversational loop is visible lag. See `../contracts/agent-copy.md`.

## Configuration

The following settings (see `../contracts/configuration.md`) directly affect this concept:

| Setting | Scope |
| --- | --- |
| Helm action authority (Tier 1 Redirect enabled vs. read-only) | Machine |
| Helm budget soft-warning threshold | Machine |
| Helm session retention / expiry | Machine |
| Voice/tone | Machine — may adjust length and formality, never override Voice & conduct |
