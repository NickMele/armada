# Journey 3.5 — Respond to a Push Alert

**What it is:** Reactive — a push notification pulled you in to rescue something specific.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** OS-level push fires for fan-out/rate abuse, a stalled Drone, or thrashing that's escalated.

**Concepts touched:** Bridge (Alerts), Drone, Debug/Pilot.

**Milestone:** Recovery. Alerts is the surface that stayed in the rail when Active Jobs, Reviews and the Activity Feed folded into the Board.

It shares design order with Triage Queue — 3 and 3.5 — because the two should be designed together, not sequentially.

## Settled in design, 2026-08-21

- **A push lands on one item, not a queue.** No tabs, no counts, no list — a person who has already been told what is wrong should not have to find it again. Debug is the landing view.
- **The primary action is the lowest rung that could still work** — rung 1, Redirect, on this screen. The ladder is climbed rather than chosen from. Job detail differs only because its retries were already spent, which is what makes Pilot primary there.
- **Diagnosis carries no controls.** Read-only means the panel states and the ladder acts, so a person cannot intervene before reading. The last-state snapshot, the declared-plan-versus-diff table and the heartbeat gap are all inert.
- **The header states that the Drone is paused rather than offering to stop it.** Fleet auto-kills only at a cap, so by the time this screen exists the state is held as-is — except where a cap was hit, and then the header says the Drone was stopped and why.
- **`Rephrase and redispatch` is retired.** Monitor Active Work's job detail header carried it as its secondary action; it was invented, like `Override` before it. Rung 2 is **Kill & Redispatch**, which is the same act with a real name and a defined evidence trail.
- This also answers Monitor Active Work's open item: **the Debug view and its ladder are reachable on a healthy Drone**, not gated on escalation.

## Left unresolved after design

- **Where a redispatched Drone starts.** Rung 2 does not say whether the fresh Job resumes at the failed step or restarts from the first. Pilot's Restart Step puts a new Drone at the step that failed, and a Job that re-runs two already-passed steps spends money to reach where it started. Drawn as resuming, inferred from the worktree being kept rather than stated on this document.

## Redirect — rung 1's input, 2026-08-21

One box, with the Drone's situation in plain words above it.

Drawn twice from the schema first and wrong both times — a segmented control reading *File plan / Root cause / Brief*, then the same three as selectable rows. Both were schema browsers: they asked a person to classify their correction against the Drone's internal fields before writing it.

**Structure belongs in the record, not in the form.** "Structured instruction" means the Job event is structured — which step, which Drone, whose words — not that the input is. Asking a person to file their own correction is the system's bookkeeping leaking into their hands.

What the composer shows, in order: what the Drone was doing, where it stopped, what it has not done yet, then the field. Enough to write one sentence and know it will land.

**One instruction, not a thread.** Recorded as a normal Job event; the Drone carries on from where it stopped and no retry is consumed, because nothing failed. A second redirect means the first did not work, which is a signal rather than a conversation.

## All three Debug layouts drawn, 2026-08-21

Debug is trigger-adaptive and all three layouts are now drawn. Neither of the two beyond `stalled` needed a new component — both were derivable from work already done, which is worth recording because they were first filed as undrawn on the grounds that they would be invented.

- **Stalled** — last-state snapshot: the final tool call, the heartbeat gap, and the declared file plan against what was actually touched.
- **Thrashing** — the escalation payload from Monitor Active Work, which already stacks every prior refusal with its citation. Thrashing is what that column looks like when the lines are identical, so the attempts sit in one readable column rather than showing the latest and hiding the rest.
- **Evidence suspect** — the gaming check. The workflow schema already names the baseline (`root_cause.evidence`) and the three flagged patterns, so the panel puts the Drone's own claim beside the diff and names which pattern fired. The claim being the Drone's own note from two steps earlier is what makes the comparison fair: it is held to what it said, not to what a Judge would have preferred.

What each layout answers is different, which is why the panel adapts rather than the screen: thrashing asks *is it getting anywhere*, evidence-suspect asks *is it telling the truth*, stalled asks *where did it stop*. The header, the ladder and the read-only rule are identical in all three.

## Alert Levels

What happens if you ignore it — not how bad it feels. The test is checkable, and Fleet already knows the answer: **did work stop, and is anything waiting on you.**

Resolved Aug 2026, replacing a two-row Push/Passive split. That split described interruption rather than severity, so two conditions could never be ranked against each other once both were passive.

| Level | Test | Behavior |
| --- | --- | --- |
| **Blocked** | Work stopped. Nothing resumes until you act | OS-level push, deep-links straight to the Alerts item |
| **Waiting** | A decision is queued. Nothing running is harmed by the delay | Queues in Alerts, surfaces in Triage Queue |
| **Noted** | Nothing is waiting on you | Recorded on the Job and read on job detail. Never interrupts, never queues, and has no surface of its own |

**Why not critical / warning / info.** Those have no test behind them. Someone decides evidence-suspect is "critical" and the next person decides otherwise. "Did work stop" is mechanical.

### Every condition, mapped

| Condition | Level | Why |
| --- | --- | --- |
| Stalled Drone | Blocked | Frozen until you act |
| Thrashing, once escalated | Blocked | Looping, paused, no progress |
| Fan-out / rate abuse | Blocked | Capped and held |
| Job `interrupted` after a Fleet crash | Blocked | Work stopped. The cause is already dead, but resumption still needs you. **Revisit once crash recovery is designed** — if resuming is one click this may belong at Waiting |
| Gate failure | Waiting | Job halted at the gate, decision queued |
| Evidence suspect | Waiting | Same shape |
| Job awaiting dispatch approval | Waiting | Nothing started, nothing running |
| Job Review ready | Waiting | Work finished, sign-off queued |
| ~~Doctor module fail~~ | ~~Waiting~~ | **Reversed 2026-08-21.** A health check is a standing condition, not a queued decision — a failing module has no workspace, no spend, no step and no Job id, and nothing about it is *waiting* in the sense every Alerts row is. **Doctor never enters Alerts.** It renders as a one-line neutral strip above the Triage Queue tabs, naming which modules are failing and whether work is stopped, and disappears when they all pass (the window's fixed status bar reports health continuously, so the absence is not ambiguous) |
| Denial-frequency rollup | Noted | A suggestion. Ignoring it costs nothing |
| Job completed and merged | Noted | Informational |

**Approvals, reviews and gate failures now share a level.** They were three surfaces holding the same kind of item — something finished or halted, and a decision is queued. Calling them all Waiting is the honest version. The distinction between an escalation and an approval stays visible in the item itself, not in whether it interrupts.

## Debug — Read-Only Diagnosis

Default landing view, trigger-adaptive layout:

| Trigger | Layout shows |
| --- | --- |
| Thrashing | Turn-history / loop pattern |
| Evidence-suspect | Diff-vs-claim comparison |
| Stalled | Last-state snapshot |

## Intervention Ladder

**Not severity.** This is what you do about a problem, ordered by how much of the work you take over. You climb it as lighter attempts fail. A Blocked alert and a Waiting one can both resolve at rung 1, and a single condition can need any rung — the rung describes your response, never the problem's weight.

Renamed from "Escalating Response Tiers" Aug 2026. The old name read as severity and collided with the alert levels above, which are the real severity axis.

| Rung | Action | Where | Evidence trail |
| --- | --- | --- | --- |
| 1 | Redirect — structured instruction sent to the Drone | In-place on Debug | Normal Job event |
| 2 | Kill & Redispatch — kill Drone, capture learnings, dispatch a fresh one with new context | In-place on Debug | New Job at the first step, old learnings become reference context |
| 3 | Break-glass Pilot — raw terminal takeover, you finish the work | Deliberate mode switch, entered rarely | Explicit manual-override event, never mixed into Drone evidence |

**Helm's authority maps onto this ladder**, not onto the alert levels. Rung 1 it may take directly; rung 2 always routes through your approval; rung 3 is a human at a keyboard by definition and no agent can take it.

## Escalation Resource State

Paused, unless a cap was hit. Worktree and state held as-is either way. Killing is a human action — rung 2, or directly from Alerts — and always prompts a cleanup step rather than happening silently; the exception is a Drone that was told to report and then went quiet, which Fleet stops itself, because it is spending money rather than waiting for you. A Drone still writing inside its declared plan is not that one, and Fleet leaves it working.

**Pause is therefore not a rung on an escalated Job.** There it is the condition you are responding *from*, not a response — anything escalated is already paused by the time you see it.

**On a healthy Drone it is an action.** The ladder is reachable on a healthy, non-escalated Drone, decided Aug 21 2026: all three of Redirect, Kill and Pause are available there. Kill was never in doubt — killing a healthy Drone is unambiguous and already safe. Pause only ever means something on a healthy Drone, since anything escalated is already paused, so Helm's ability to pause a healthy running Job rides on the same answer. Redirect carried the real risk — injecting context mid-step produces evidence the step did not plan for, which the Judge might then flag for a reason that is the engineer's fault rather than the Drone's — but two things decided the same day weakened that concern: Pilot became available on any Job, running or escalated, so gating the lightest intervention while leaving the heaviest one ungated inverts the ladder; and the Judge already handles unplanned drift, since `scope_diff_check` tags a step for a mandatory Judge look rather than auto-failing, precisely because legitimate investigation moves the work. The one constraint kept: a redirect on a healthy Drone is mid-step context injection and must not silently become a step restart. The Drone continues with more information, and the record says a human intervened.

## Where a redispatched Drone starts

**The first step.** A redispatch mints a new Job, and a new Job cannot resume — which is forced twice over rather than chosen.

**It has no worktree to resume onto.** The failed Job's branch is its record, and `create_worktree` refuses an existing branch, so the replacement takes a fresh one under its own id. Every step that passed did its work on a branch the replacement does not have.

**"The failed step" may not exist.** The replacement freezes the workflow as it stands now rather than copying the failed Job's frozen definition, deliberately, so that an edit made in response to the failure reaches the retry. A step index into a definition that may have changed shape is not a resume point.

**So what carries forward is context, never work.** The brief, the title and the workflow, plus what the dead Drone learned — and the failed attempt's branch, named as what it is: the previous attempt, in whatever state it was left. Whether that work is any good is the new Drone's reading to make, and the brief says it is unverified rather than pretending otherwise. **A named branch is a kept branch**, and ordinary cleanup keeps an unmerged one — #301 is when a failed attempt stops being worth holding.

**The cost is the whole Job, not the failed step, and the dialog says so.** That is what makes this rung 2. Rung 1 keeps the session; this spends every step that had already passed.

**Restart Step is the act that resumes**, and it is a different one: same Job, same worktree, same branch, a fresh Drone at the step that stopped. Reach for that where the diagnosis holds and only the execution went wrong.

## Open questions

- **[status-updates-os-notification-surface]** Do written status updates surface in-app only, or as OS notifications too?
  Voice governs how written status updates read. Where they surface is unspecified: in-app only, or OS notifications as well. Not cosmetic — the away-from-desk pre-authorized batch flow assumes a person is not looking at Bridge, and an alert that only exists inside a closed window is not an alert. The escalations-interrupt / approvals-queue rule would also need an OS-level expression if notifications leave the app.

- **[interrupted-job-alert-level]** Does a Job `interrupted` after a Fleet crash stay at Blocked once crash recovery is designed?
  Recorded inline in the Alert Levels table above, against the `interrupted` row: "Revisit once crash recovery is designed — if resuming is one click this may belong at Waiting." The cause of the interruption is already dead by the time a person sees it; whether resumption is cheap enough to demote it off the interrupt-worthy tier depends on a milestone (Recovery) that has not yet designed that path.

## Related

Triage Queue for the proactive, non-reactive version of checking Alerts. Monitor Active Work for the same Debug view on a healthy, non-escalated Drone.
