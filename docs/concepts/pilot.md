# Pilot

**What it is:** Escape hatch from a running Job into a human-driven Claude Code session. Two of three outcomes ship in v2.0.

---

**Kind:** Process, Surface.

The citable source for the mechanism. The flow lives on the Take Over a Job user journey.

The problem it solves: a Drone runs on a deliberately narrow toolset to stay focused. An engineer taking over needs the opposite. Without a handoff, the engineer abandons the Job and opens a fresh session by hand, losing the Drone's context along with it.

**Pilot is not [Observe](observe.md).** Pilot ends autonomous execution and gives the worktree to a person; Observe reads a running Drone's turns and changes nothing about the Job. Wanting to see what a Drone is doing is Observe's question, and answering it with Pilot costs the Job its Drone.

## What it is

Pilot moves a Job from Drone execution to human execution. A single MCP tool, `escape_hatch`, is injected into every Drone's toolset. Calling it stops autonomous progress and hands the Job to a person.

- The tool is the exit path for a **running** Drone. Both the human and the Drone route through it, so one code path covers both.
- A Job with no live Drone takes a third path — see Trigger paths.
- Fleet does the work. The Drone contributes a narrative of what it is stuck on, and nothing else.
- The result is a Claude Code session on the Drone's worktree with the Drone's context loaded, at an unrestricted toolset.

## Trigger paths

**Three callers, one mechanism.**

| Caller | Path | What it means |
| --- | --- | --- |
| Human, live Drone | Pilot button in Bridge, then the modal; Fleet instructs the Drone to call `escape_hatch` | The engineer decided to take over. The Drone may be fine |
| Drone | Drone calls `escape_hatch` on its own | Stuck signal, in place of thrashing or self-reporting completion |
| Human, **no Drone** | Same button on an escalated Job; Fleet assembles the bundle and opens the session directly | Autonomous progress has demonstrably ended |

**Pilot appears on escalated Jobs as well as running ones**, and is the primary action on an escalated Job. An escalated Job has no Drone — its retries are spent and it stopped — so there is no `escape_hatch` call to make. That is the state an engineer is most likely to take over from.

**One button, not two.** Taking over a stopped Job is the same act as taking over a running one — hand me this worktree with context — differing only in whether a process happens to be alive. The one-code-path property covers the two live-Drone callers only.

**On the third path the escalation payload arguably substitutes for the stuck narrative.** It carries the trigger, the step, full attempt history and every prior Judge summary, and a Judge refusal must cite the specific evidence it refuses on. That is a more concrete account of what went wrong than a stuck Drone's self-description, which is by this system's own principle the least trustworthy field in the bundle.

**The Drone-initiated path is the counterweight to v1's lesson.** v1 proved Drones will report success they did not achieve rather than admit a dead end. Giving the Drone a sanctioned way out is what balances that.

Any path lands the engineer in a confirmation modal that names the three outcomes below. The modal itself, its copy and its placement belong to the Take Over a Job user journey.

## The mark

**A pull succeeds only on a Job Fleet has marked for the handoff.** A person indicates the hatch should be pulled, Fleet marks the Job, and Fleet then prompts the Drone to pull it and wrap up its work.

| Pull | The Job | What happens |
| --- | --- | --- |
| Fleet prompted it | Marked | Allowed. The Job goes to `piloted` |
| The Drone reached for it | Not marked | Refused. The Job escalates as `hatch_unbidden` |

**The mark enforces the distinction the trigger paths table already draws.** Why: a Drone reaching for the hatch unbidden is a Drone struggling, and a struggling Drone belongs in front of a person rather than out through a clean exit.

**Both routes end autonomous execution and pass the Job to a person.** A marked pull does it by clean exit to `piloted`, an unmarked one by escalation. The baseline clause in the [Agent Prompt Contract](../contracts/agent-prompt.md) holds on either.

**The Drone is never told which route it will get.** Why: a refusal that explains itself teaches the Drone to look for the mark. Fleet answers an unmarked pull that the hatch is unavailable, and says nothing further.

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
| Narrative of what it is stuck on | Drone, as the argument to `escape_hatch` |

The narrative carries three named fields: `trying_to`, what the step was meant to produce; `blocked_by`, the specific thing preventing it, such as a denied command or a missing credential; `tried`, what was attempted and what each attempt produced.

**The narrative is not Evidence.** Why: Evidence is proof tied to an advance gate, and this states that no proof is coming.

Fleet then adds what it knows and the Drone does not: sibling workspaces, Manifest, Kit and Machine config in force, and Convoy membership if the Job is atomic.

**Piloting a [Convoy](convoy.md) pilots all of it.** A Convoy is one Job and one worktree spanning its Workspaces, worked by one Drone at a time, so there is no per-workspace unit to take over — the Drone on the step calls `escape_hatch`, and the one worktree changes hands. The blast radius is wider than a single-workspace pilot: an unrestricted terminal on a worktree spanning several Workspaces at once.

## The piloted session

A new Claude Code session, on the Drone's worktree, with the bundle loaded as context.

- **Kit-level toolset, unrestricted.** The narrow toolset is the thing being escaped. The Manifest allowlist governs Drones, not people, and a human at a terminal is outside its scope.
- **Secrets brokering does not apply.** Drones never touch secrets directly, and the broker exists to enforce that. A piloted session is the engineer's own shell with the engineer's own credentials.
- **Fleet stops scheduling against the Job** for as long as it is piloted. The Job stays on the Board and leaves the scheduler.
- **Worktree ownership transfers.** Fleet must not reclaim, clean or garbage-collect a piloted worktree.

## Job state

A piloted Job has its own status, `piloted` — **the ninth Job status**, carrying a Pilot-specific reason. On the Board, not scheduled, not finished, with the worktree belonging to the engineer rather than to Fleet. Its reason records which of the three outcomes was chosen.

**A Job that owes a human an attestation is a different status.** `awaiting_attestation` is waited on — a person owes an action outside Armada and must return to report it. `piloted` is worked — a person is at an unrestricted toolset right now.

Who is acting, against what is being waited on, is the axis the whole status set is built on, and one reason on one status cannot carry both. See [Job](job.md). The ninth status also holds a Job that is done but owes a human an attestation; its name and its reason set are still open — see Open questions.

Neither is `stalled`, which describes a Drone still assigned and making no progress, and neither is `rejected`, which is reserved for hard stops.

## Evidence

**Pilot is a change of operator. Whether it is also a change of standard is the engineer's explicit call, recorded.**

**Three exits from a piloted Job, chosen when the engineer is done** — mirroring the three-outcome modal that starts Pilot.

| Exit | What happens to the gates | When it is right |
| --- | --- | --- |
| **Submit for verification** | Unchanged | The work still fits the plan |
| **Attest complete** | Replaced by a human attestation | The plan was wrong and the outcome is right |
| **Close as superseded** | None run | The Job was a bad idea |

**Submit for verification.** The engineer signals done from Bridge and [Fleet](fleet.md) runs that step's gates against the worktree exactly as it would for a Drone — same Checks, same Judge, same advance gate. Most of the machinery never needed a Drone: Fleet invokes Checks itself and the Judge reads the diff, and only the Evidence submission did. It is right when the Job was basically fine and needed a wider toolset for a step, so the gates still mean something.

**Attest complete.** The engineer marks criteria satisfied using the **human-attested verification source**. Recorded as attested rather than verified — human-only, and rendered distinctly, so a Job closed by hand never looks like one that passed its gates. Why: re-running a Judge against a step definition the engineer has just invalidated tells nobody anything.

**Close as superseded.** A distinct terminal state meaning **superseded by human work** — not `completed_failed`, not `killed`, both of which read as the work being lost. It is right when the work has landed and the Job record has nothing left to say.

**A Job depending on a superseded one unblocks and surfaces**, with its dependency marked unsatisfied and a warning that the upstream never landed as planned. Why: superseding means the work landed outside the Job, so blocking on the record rather than the outcome is the wrong test.

**Most takeovers are expected to end in attest or supersede**, because an engineer takes over precisely when a Job has gone wrong.

**Any path around verification becomes the default path**, which is what v1's lesson constrains. The relaxation is therefore a **named, recorded act by a person** rather than an absence: attesting is visible as attesting, superseding is visible as superseding. What is forbidden is work that clears no gate and reads as though it did.

## Assist, and why it is deferred

Assist parks the Drone with its context intact, then resumes it after the engineer is done. The cost sits in what happens to that context.

The moment the engineer touches the worktree, the parked Drone's context goes stale. Its declared file plan is wrong. Its transcript describes a tree that no longer exists.

Resuming it as-is produces a Drone that fights the engineer's changes or redoes them. The fix is a re-brief on resume: a diff summary of what changed plus a fresh file plan declaration before the work continues. That mechanism was real work when this was written and does not block the other two outcomes, so it sits out of v2.0.

**It is no longer work with no other reason to exist.** A Drone belongs to a workflow step ([Drone](drone.md)), so every step boundary already hands a fresh Drone a worktree it did not make and tells it what it inherited. Assist is the same brief with a person's edits in it rather than a previous step's. What it still needs beyond that is a decision about whether the parked Drone resumes at all or is replaced, which is the third outcome wearing Assist's name.

**Assist renders disabled in the modal with a coming-soon state**, rather than hidden, so the outcome set does not change shape when it ships.

## Open questions

- **[pilot-ninth-status-name]** What is the name of the ninth Job status, and what is its full reason set? Decided Aug 21 2026 that a ninth status carries a Pilot-specific reason and also holds a Job that is done but owes a human an attestation — but the status's name and its reason set are not yet decided.
