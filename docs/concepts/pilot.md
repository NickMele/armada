# Pilot

**What it is:** Escape hatch from a running Job into a human-driven Claude Code session. Two of three outcomes ship in v2.0.

---

**Kind:** Process, Surface.

Formalises Pilot, the escape hatch from a running Job into a human-driven Claude Code session. The mechanism was raised in passing and never designed. This document is the citable source for the mechanism. The flow lives on the Take Over a Job user journey.

The problem it solves: a Drone runs on a deliberately narrow toolset to stay focused. An engineer taking over needs the opposite. Without a handoff, the engineer abandons the Job and opens a fresh session by hand, losing the Drone's context along with it.

## What it is

Pilot moves a Job from Drone execution to human execution. A single MCP tool, `escape_hatch`, is injected into every Drone's toolset. Calling it stops autonomous progress and hands the Job to a person.

- The tool is the exit path for a **running** Drone. Both the human and the Drone route through it, so one code path covers both. A Job with no live Drone takes a third path — see Trigger paths.
- Fleet does the work. The Drone contributes a narrative of what it is stuck on, and nothing else.
- The result is a Claude Code session on the Drone's worktree with the Drone's context loaded, at an unrestricted toolset.

## Trigger paths

**Three callers, one mechanism.** The third was added Aug 21 2026 for Jobs with no live Drone.

| Caller | Path | What it means |
| --- | --- | --- |
| Human, live Drone | Pilot button on the Job in Bridge, then confirm in the modal. Fleet instructs the Drone to call `escape_hatch` | The engineer decided to take over. The Drone may be fine |
| Drone | Drone calls `escape_hatch` on its own | Stuck signal. The Drone raises a hand instead of thrashing or self-reporting completion |
| Human, **no Drone** | Same button on an escalated Job. **Fleet assembles the bundle and opens the session directly** — no `escape_hatch` call, because there is nothing to call it | Autonomous progress has demonstrably ended. The state an engineer is most likely to take over from |

**The third path does not lose what the tool was buying, and improves on it.** Routing both original callers through `escape_hatch` guaranteed the stuck narrative got filled. An escalated Job has a better source: its escalation payload carries the trigger, the step, full attempt history and every prior Judge summary — and a Judge refusal must cite the specific evidence it refuses on. That is a more concrete account of what went wrong than a stuck Drone's self-description, which is by this system's own principle the least trustworthy field in the bundle.

Rejected: making takeover of a stopped Job a **separate action**, which produces two buttons doing the same thing — hand me this worktree with context — differing only in whether a process happens to be alive.

The Drone-initiated path matters more than it looks. v1 proved Drones will report success they did not achieve rather than admit a dead end. Giving the Drone a sanctioned way out is the counterweight to that.

Any path lands the engineer in a confirmation modal that names the three outcomes below. The modal itself, its copy and its placement belong to the Take Over a Job user journey.

## The three outcomes

| Outcome | Drone | Worktree | Job resumes | v2.0 |
| --- | --- | --- | --- | --- |
| **Take Over** | Terminated | Yours, permanently | Manually, or re-dispatched fresh | Ships |
| **Assist** | Parked, holding context | Yours temporarily, then returned | Same Drone, mid-step, after a re-brief | Deferred |
| **Restart Step** | Terminated | Yours, then handed to a new Drone | New Drone at the step that failed | Ships |

The choice changes two things only: the fate of the Drone process, and who owns the worktree afterwards. The tool call, the handoff bundle and the Evidence rules are identical across all three.

## The handoff bundle

Fleet assembles this. The Drone supplies one field of it.

| Contents | Source |
| --- | --- |
| Job record and the Workflow step it stopped on | Job Board |
| Drone transcript | Structured JSON logs |
| Declared file plan against the actual diff | Fleet, from the drift detection already in place |
| Worktree path and branch | Fleet |
| Evidence collected so far, and the gate that failed | Job record |
| Narrative of what it is stuck on — three named fields: `trying_to`, what the step was meant to produce. `blocked_by`, the specific thing preventing it, such as a denied command or a missing credential. `tried`, what was attempted and what each attempt produced. **Not Evidence** — Evidence is proof tied to an advance gate, and this states that no proof is coming | Drone, as the argument to `escape_hatch` |

Fleet then adds what it knows and the Drone does not: sibling workspaces, Manifest, Kit and Machine config in force, and Convoy membership if the Job is atomic.

**Piloting a [Convoy](convoy.md) pilots all of it.** A Convoy is one Job, one Drone, one worktree spanning its Workspaces, so there is no per-workspace unit to take over — one Drone calls `escape_hatch`, one worktree changes hands. Worth stating because the blast radius is wider than a single-workspace pilot: an unrestricted terminal on a worktree spanning several Workspaces at once.

## The piloted session

A new Claude Code session, on the Drone's worktree, with the bundle loaded as context.

- **Kit-level toolset, unrestricted.** The Manifest allowlist governs Drones, not people. A human at a terminal is outside its scope.
- **Secrets brokering does not apply.** Drones never touch secrets directly, and the broker exists to enforce that. A piloted session is the engineer's own shell with the engineer's own credentials.
- **Fleet stops scheduling against the Job** for as long as it is piloted.
- **Worktree ownership transfers.** Fleet must not reclaim, clean or garbage-collect a piloted worktree.

## Job state

A piloted Job has its own status, `piloted`: on the Board, not scheduled, not finished, with the worktree belonging to the engineer rather than to Fleet. Its reason records which of the three outcomes was chosen.

**A Job that owes a human an attestation is a different status.** `awaiting_attestation` is waited on — a person owes an action outside Armada and must return to report it. `piloted` is worked — a person is at an unrestricted toolset right now. Who is acting, against what is being waited on, is the axis the whole status set is built on, and one reason on one status cannot carry both. See [Job](job.md). **Decided Aug 21 2026 — the ninth Job status**, carrying a Pilot-specific reason; the same status also holds a Job that is done but owes a human an attestation. The name and the reason set are still open — see Open questions.

Neither is `stalled`, which describes a Drone still assigned and making no progress, and neither is `rejected`, which is reserved for hard stops.

## Resolved Aug 21 2026 — starting Pilot from a Job with no Drone

**A third trigger path.** Fleet assembles the bundle and opens the session directly — see Trigger paths above.

**This unblocks the placement question** on the Take Over a Job journey: Pilot appears on escalated Jobs as well as running ones, so it appears on the screen where the wireframe drew it.

The problem as originally stated, kept as the record:

Raised 2026-08-21 by the job detail wireframe (journey 4 in the Armada Journeys mockups), where Pilot is the primary action on an escalated Job.

Both trigger paths above assume a live Drone. The human path has Fleet instruct the Drone to call `escape_hatch`; the Drone path is the Drone calling it unprompted. The table says the button is available on the Job while a Drone is running.

**An escalated Job has no Drone.** Its retries are spent and it stopped — and that is the state an engineer is most likely to want to take over from, since it is the state where autonomous progress has demonstrably ended. As written, the mechanism cannot serve it.

Two ways out, neither chosen:

- **A third trigger path.** Fleet assembles the handoff bundle and opens the session with no `escape_hatch` call. Everything the bundle needs already exists on the Job record — the stuck narrative is the only field the Drone supplies, and a Judge refusal arguably substitutes for it. Costs the "one code path covers both" property.
- **A different action for a stopped Job.** Pilot stays a live-Drone mechanism and taking over a stopped Job is something else, which means two names for one act.

The third trigger path is what was chosen, resolving the placement question above.

## Evidence

**Pilot is a change of operator. Whether it is also a change of standard is the engineer's explicit call, recorded — amended Aug 21 2026.**

This document previously read: *Evidence unchanged. Human-touched work clears the same gates as Drone work.* That holds for one of the three exits and not the other two, and stating it absolutely was hiding a decision rather than enforcing one.

**Three exits from a piloted Job, chosen when the engineer is done** — mirroring the three-outcome modal that starts Pilot.

| Exit | What happens to the gates | When it is right |
| --- | --- | --- |
| **Submit for verification** | Unchanged. The engineer signals done from Bridge and [Fleet](fleet.md) runs that step's gates against the worktree exactly as it would for a Drone — same Checks, same Judge, same advance gate. Most of the machinery never needed a Drone: Fleet invokes Checks itself and the Judge reads the diff. Only the Evidence submission did | The Job was basically fine and needed a wider toolset for a step. The work still fits the plan, so the gates still mean something |
| **Attest complete** | The engineer marks criteria satisfied using the **human-attested verification source**. Recorded as attested rather than verified — human-only, and rendered distinctly, so a Job closed by hand never looks like one that passed its gates | The plan was wrong and the outcome is right. Re-running a Judge against a step definition the engineer has just invalidated tells nobody anything |
| **Close as superseded** | None run. A distinct terminal state meaning **superseded by human work** — not `completed_failed`, not `killed`, both of which read as the work being lost. **A Job depending on this one unblocks and surfaces**, with its dependency marked unsatisfied and a warning that the upstream never landed as planned — superseding means the work landed outside the Job, so blocking on the record rather than the outcome is the wrong test | The Job was a bad idea. The work has landed and the Job record has nothing left to say |

**Most takeovers are expected to end in attest or supersede**, because an engineer takes over precisely when a Job has gone wrong.

**What v1's lesson actually constrains.** Any path around verification becomes the default path — which is why the relaxation is a **named, recorded act by a person** rather than an absence. Attesting is visible as attesting; superseding is visible as superseding. What is forbidden is work that clears no gate and reads as though it did.

## Assist, and why it is deferred

Assist parks the Drone with its context intact, then resumes it after the engineer is done. The cost sits in what happens to that context.

The moment the engineer touches the worktree, the parked Drone's context goes stale. Its declared file plan is wrong. Its transcript describes a tree that no longer exists. Resuming it as-is produces a Drone that fights the engineer's changes or redoes them.

The fix is a re-brief on resume: Fleet injects a diff summary of what changed plus a fresh file plan declaration before the Drone continues. That mechanism is real work and does not block the other two outcomes, so it sits out of v2.0.

Assist stays visible in the modal rather than hidden, so the outcome set does not change shape when it ships.

## What is already resolved

| Question | Resolution |
| --- | --- |
| How does a Job exit to a human? | **One MCP tool, `escape_hatch`,** injected into every Drone. Resolved Aug 2026 |
| Can the Drone trigger it, or only the engineer? | **Both.** Same tool. Drone-initiated is the sanctioned stuck signal |
| Does the Drone assemble the handoff? | **No.** Fleet assembles it. The Drone supplies its stuck narrative and nothing else |
| Does the piloted session inherit the Drone's toolset? | **No.** Kit-level and unrestricted. The narrow toolset is the thing being escaped |
| Does secrets brokering apply to a piloted session? | **No.** It is the engineer's own shell |
| One outcome or several? | **Three,** chosen in a confirmation modal: Take Over, Assist, Restart Step |
| Which ship in v2.0? | **Take Over and Restart Step.** Assist renders disabled with a coming-soon state. Resolved Aug 2026 |
| Do Evidence gates change for human work? | **The engineer chooses, and it is recorded.** Three exits: submit for verification (gates unchanged), attest complete (human-attested source, rendered distinctly), or close as superseded. Amended Aug 2026 — this previously read "no, identical across all three outcomes" |
| Does Fleet keep scheduling a piloted Job? | **No.** The Job stays on the Board and leaves the scheduler |

## Open questions

- **[pilot-ninth-status-name]** What is the name of the ninth Job status, and what is its full reason set? Decided Aug 21 2026 that a ninth status carries a Pilot-specific reason and also holds a Job that is done but owes a human an attestation — but the status's name and its reason set are not yet decided.
