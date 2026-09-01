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
- $ cost cap per Job
- Turn cap per Job
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

## Budget

**A budget is Machine's and nothing else has one.** No Manifest, no Kit, no
second tier — which is what the Kit/Machine split is for, and it is why there is
no precedence rule to state. A repo has no opinion about how much this
installation will spend on it.

**It is per Job, and two numbers rather than one.** A Drone belongs to a
workflow step, so a four-step Job is four Drones; a per-Drone ceiling on that
Job is four times the number anybody thought they set, and the Job is what a
person approved. The dollar cap is deliberately wide and the turn cap is what
catches what a wide ceiling misses — three identical, identically successful
runs of one Job were measured at $0.063, $0.087 and $0.146, a 2.31× spread on
cache warmth alone, while their turn counts held at 7, 7 and 4.

**It refuses the next dispatch. It does not stop a Drone that is spending.**
What a run cost arrives on the final line of a Drone's session and nowhere else,
so there is no mid-session figure to interrupt on. A Job over its budget waits
at `queued`, reading `over_budget`, until somebody raises the cap. The case this
catches is the one that matters: a runaway is a sequence of Jobs rather than one
Drone.

**Verification spend counts against it.** A Judge call bills against the same
cap as the work it verifies — a cap that excluded it would understate what the
Job cost. Whether verification carries a sub-limit of its own is
[Judge](judge.md)'s to say, and is open there.

**A budget refusal is a Job state and never a gate verdict.** Fleet decides it
at admission, from what the record says the Job's Drones have spent;
verification never reads the cap and a budget failure never counts against a
step's retry limit.

**The dollars are notional.** The figure is what a run would have cost at API
list price, and a subscription account is not billed per token. It is exact
arithmetic in a currency this installation does not spend, which makes it a
runaway detector rather than an invoice.

## Quota — measured, and not gated on

**Armada does not read quota.** The gate below was designed and is not built,
and the reason is a measurement rather than an omission: a headless Drone's
stream reports *whether* it is allowed and *when* the window resets, and carries
no quantity at all. The percentage is real and live on every request, and the
channel that carries it does not run headless.

| Machine type | What was intended | What is true |
| --- | --- | --- |
| Work machine, API-billed | **$ cost cap** per Job | Built. See Budget above |
| Personal machine, subscription plan | Gate on % of the 5-hour or weekly window remaining | **Not built.** No quantity reaches Armada |

**What that costs, said plainly.** On a subscription the five-hour window is
what will actually stop a fleet — dollars stop nothing, because nothing is
billed per token. So the fleet meets its real limit as a hard failure with no
approach warning: Jobs running normally, then work dying mid-flight. That shape
is expected rather than designed against.

## Init

The installation half of first-run setup: resources, budget mode, notification routing, interface defaults. The tool half belongs to [Kit](kit.md). Both chain together in the broader First-Run Onboarding journey.

What this combined surface is called, and whether it is its own journey and its own gated step, is tracked in [Kit](kit.md).

## Configuration

All rows in the Configuration Settings registry tagged `Machine`. A Machine row carries no `Merge strategy` and no `Peer polarity` — both read `n/a — single layer` by construction.

## Open questions

- **[machine-voice-owning-tier]** Does Voice belong to Machine or to Kit? Voice is a Machine setting by the group mapping above, but `../contracts/agent-prompt.md` places it in prompt layer 2 beside Skills, the Agent file and Sub agent definitions — all of which are Kit. The two readings disagree about which concept owns it.
