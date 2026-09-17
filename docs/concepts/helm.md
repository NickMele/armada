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

**Helm drives a [Studio](studio.md).** It may add proposed nodes and propose edges unasked, and starts a [Scout](scout.md), a Run or a dispatch only on your ask. The Studio page owns which is which.

## Placement

**Helm is a dock, not a surface.** It sits beside the content on every Bridge screen at 1100px and wider, folding to an edge strip that opens as a sheet below that; `⌘J` toggles it from anywhere, including from inside a field. It carries no rail digit and does not appear in the sidebar. See `../contracts/design-system.md`, Two tiers, for the layout and `../concepts/job-board.md` for how a question on the Board relates to a question on the dock.

The dock's upper zone lists every question waiting on a person, from every repository Fleet serves — a Drone question, a held command, a Judge refusal — each card naming its own repository. Answering one there settles it on that Job's own detail too. The lower zone holds Helm's conversation.

**One running conversation per repository, with Start fresh.** Which repository Helm answers for is set by the most recent explicit act — picking a repository, "Discuss with Helm" on a card, or the dock's own switch — never by the picker moving on its own. Helm is hosted by Fleet and resumed from the stored session when a message needs it; no process idles between messages. A conversation clears after 30 quiet days (`settings.helm-session-retention-expiry`, Machine-scoped).

**Every ask names where the person is.** Bridge sends the screen, the picked repository, the open Job chipped above the message box, and the Board or Overview cursor row with each message; Fleet hands the session one line naming it ahead of what was typed, and the thread keeps only what the person wrote. `#1075`.

**Fleet-wide reasoning is not a Helm capability.** Why: a conversation belongs to the repository it is pointed at, so a Fleet-wide Helm would be the only thing in the product ignoring that.

Anything genuinely cross-Manifest stays a Bridge job.

**Built to change.** Conversations are looked up by a key that is a repository today, and Helm's host sits behind one interface — a key per topic, or a host inside Bridge instead of Fleet, is a switch on either, not a rebuild.

## Tools

**Helm holds what you hold in a terminal, and the Fleet MCP besides.** A session opens in the repository's own checkout and resolves what your Claude configuration resolves there — your MCP servers, your plugins, your skills, your agent files, and the ordinary built-in tools that read, search, edit, run a command and fetch. Nothing is withheld by a list Armada wrote. What Helm is told at session start — the selected Manifest, its resolved authority and Voice — is specified on `../contracts/agent-prompt.md`, section 2.

**Kit is what should resolve that set, and Kit is not built (#1275).** Until it is, the set is whatever Claude Code itself resolves for this repository on this machine: your user-scoped configuration and the repository's own. Kit takes it over, and a Manifest tier arrives with it.

**A Drone's confinement is not Helm's, and widening one must never widen the other.** `--strict-mcp-config` exists because a Drone came up holding every server an operator had connected — `../scope.md`. A Drone is unattended; Helm is a person talking, in their own checkout, to a session they are watching. The two launches are rendered separately and a test holds them apart.

Helm's advantage is not more access than you have. It is reasoning over results and chaining calls you would otherwise make one at a time — "why is Job 12 failing", "what needs my attention, one by one", "what is open that we could dispatch".

**Helm gets every query, and of the commands, every one but a person's own.** `../contracts/system-architecture.md`, section 6 carries the inventory of queries, commands and events. `fleet::helm::may` is the line: every read is Helm's, and a command is Helm's when the machine's authority setting allows acting at all and the act is not `undo_run`, the one command reserved to a person regardless of the ask. With action authority set to read-only, Helm is refused every command.

**On a Studio, a few commands are Helm's without an ask**: adding a node that starts proposed, proposing an edge, and naming an untitled Studio — `fleet::helm::reach::UNASKED`, named in the brief. The door cannot tell an ask from its absence, so that line is drawn in the brief and not at the door. See [Studio](studio.md), Helm on a Studio.

**The 8 WebSocket events are Bridge-only.** Why: an agent cannot be interrupted mid-turn.

**Helm polls instead.** `get_events_since(cursor)` runs at the start of each turn, returning a count plus one line per event kind rather than the events themselves. Helm is never more than one turn stale, and fetches detail through the other queries only when it bears on what was asked.

**Fleet never wakes the session.** Why: it would make Helm a second notification channel alongside Alerts.

**Every call is scoped to the selected Manifest, except Doctor, which Helm reads machine-wide.** Doctor's modules are machine-level by nature — the Fleet daemon, disk, Armada API reachability — and cannot be Manifest-scoped, so under strict scoping Helm could not answer whether the daemon is healthy.

Machine health is often the cause: three Jobs stalled because the disk filled is exactly the cross-Job pattern Helm exists to find, and a strictly-scoped Helm would see three stalls and no reason. It leaks nothing, because the Manifest boundary exists to stop one project's work being visible from another, and machine health is **shared context rather than another project's work**.

What Helm may *do* rather than read is a separate limit — see Action authority below.

## Action authority

Helm may call any command it is offered once you ask it to, in this conversation — the ask is the human gate itself, not a step before one. `undo_run` is the sole exception, a person's regardless of what is asked. This replaced a narrower rule keyed to the Intervention Ladder — what you do about a problem, ordered by how much you take over — which is now a description of the acts rather than a boundary on Helm.

**Helm edits your checkout when you ask it to, with no worktree and no gate between the ask and the file.** A deliberate exception to "work happens in a Job", taken with the owner on 17 Sep 2026 because a terminal session does it and the app should. There is no Judge on the change, no branch, and no approval — the ask was the approval. Every write is `helm.changed_checkout`, below.

**Your own settings decide, and what they do not cover is put to you.** Helm runs in the mode a terminal session runs — `default` — with the door's own permission tool named, so a call your `allow` rules cover runs silently, a call your `deny` rules refuse is refused, and everything else waits for you in the dock while the session sits inside its own tool call. `../spikes/019-is-auto-mode-reachable-for-a-helm-session.md` measured the path, and `../spikes/018-what-can-a-helm-session-do-in-each-permission-mode.md` measured what the modes do without it.

**Auto mode was asked for and is not reachable.** The CLI accepts `--permission-mode auto` for a spawned session, reports `default` on its `init` line and makes no classifier call: auto mode is the harness's rather than a session's. So the ask reaching a person is not a substitute for it — it is what a spawned session has instead, and it is the same answer arrived at by a person rather than by a classifier.

**Helm asks before it edits, now.** Under `acceptEdits` a write to the checkout was silent, because nothing could be asked; under your own settings it is a call like any other. What the ask is worth is the *Action authority* paragraph above: the file still changes with no branch, no review and no undo once you allow it.

**A card in the dock carries the tool, the one-line argument and the rule that would have to allow it.** Three answers: allow once, which writes nothing; allow and remember, which writes that one rule into this repository's own `settings.local.json`, under `.claude/` — yours, not committed, and the only settings file Armada ever writes; or refuse, with your words carried to the session. Nobody answering is a refusal at five minutes and never an allow: silence is not consent. See [Kit](kit.md) (#1275) for where the rules should eventually come from.

| Rung | Action | Helm directly? |
| --- | --- | --- |
| 1 | Redirect — structured instruction sent to a Drone | **Yes**, on your ask |
| 2 | Kill & Redispatch — kill Drone, dispatch a fresh one with new context | **Yes**, on your ask |
| 3 | Break-glass Pilot — raw terminal takeover | **No, by definition** |
| — | Approve dispatch, on a Job Helm drafted or any other | **Yes**, on your ask, never as a silent follow-on to drafting |

Rung 3 carries no MCP operation for Helm to reach for, ladder or no. A Job Helm drafts still sits at the same approval gate every Job sits at; Helm may press it too, but only where you asked it to by name, exactly as it may take any other act.

**The one exception is a cap.** A held Drone still costs money, and a Drone that was told to report and then went quiet is spending it without converging — so that one is killed rather than held. Holding is for a Drone waiting on a person, and a Drone that went quiet is not waiting, it is burning. **A Drone still writing inside its declared plan is doing neither, and is not killed.** Its worktree survives either way, which is what the rule was protecting.

**A Helm-initiated Redirect on a healthy Drone is recorded on the Job, the same as a human one.** The record is unchanged by that.

## Audit trail

**Every Helm-initiated action is logged as its own distinct event type**, never conflated with your manual actions or with Drone self-reports. Manual actions are already distinguished from Drone evidence, per the Debug/Pilot design. Three-way separation: Drone evidence, human manual action, Helm-initiated action.

**A Helm write to your checkout is `helm.changed_checkout`**, naming the repository, the tool and the path, published as the session's stream says it happened. A person who finds a file changed and did not change it reads this to see that Helm did. It names writes Armada can name: a shell line Helm ran may have written something too and nothing says whether it did, so those calls are on the conversation's own thread and produce no event.

**Every ask and every answer is `helm.asking_to_run` and `helm.call_answered`**, naming the repository, the tool, the one-line argument and the rule. The second says which of the six ends it was — allowed once, allowed and remembered, allowed where the rule would not write, refused, unanswered, or the session gone — so a person who finds a command was run can see that they allowed it, and a person who finds one was not can see that the hold ran out.

**On a Studio, Helm's act is `studio.helm_acted`**, published beside the `studio.changed` every write publishes, naming the Studio, the act and what it added. A person's act on a Studio publishes `studio.changed` alone, so the two are told apart by kind rather than by a field someone has to remember to read. **The record keeps it too**: each node and edge carries `added_by` and a Studio its `named_by`, a person or Helm, so a client that was not connected when Helm acted reads who did what off `get_studio`.

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

## Open questions

- **[helm-target-per-window]** Should each Bridge window keep its own Helm target, or does the most recent explicit act in any window set it for all? Today there is one conversation target for the whole Bridge: a pick or a "Discuss with Helm" in one window moves Helm in every open window, because since #1022 picks are per window while `HelmConnection` in `apps/desktop/src/main/helm.ts` holds one target. Two windows open at once is rare, and the rule as built is consistent with "the most recent explicit act wins". A per-window target would need one Helm socket per window in main, and the dock's switch and chip to read their own window's target.
