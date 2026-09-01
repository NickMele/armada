# Take Over a Job

**What it is:** The flow for taking a Job away from its Drone and finishing it by hand.

Design fidelity: drawn. Analysis: Complete. UI/UX design: Complete.

---

**Trigger:** A Drone is stuck, going the wrong way, or working on something you would rather do yourself.

**Concepts touched:** Pilot, Drone, Job.

**Milestone:** Recovery.

**Design file:** `Journey 6 - Take over a job.dc.html` in the Armada Mockups project. Frames `6a` the control, `6b` the modal, `6c` the bundle, `6d` the Convoy case, `6e` a piloted job and the way back, `6f` `hatch_unbidden`.

The mechanism lives on the Pilot concept page; this document is the flow. Assist stays deferred, so the hand-back leg is undrawn and this is worth revisiting when it ships.

Pilot is the citable source for the mechanism: the `escape_hatch` tool, the handoff bundle, the toolset and secrets rules, and the Evidence position. This document covers only what the engineer does.

## Flow

Open the Job in Bridge, hit Pilot, confirm an outcome in the modal, then work in the Claude Code session that opens on the Drone's worktree.

| Step | What happens |
| --- | --- |
| 1. Hit Pilot | The job header's action group, left of Kill, on every status that offers it |
| 2. Read the modal | States what is about to happen, then offers three outcomes plus Cancel |
| 3. Choose an outcome | Take Over, Assist (disabled), or Restart Step |
| 4. Session opens | Claude Code on the Drone's worktree, context preloaded, unrestricted toolset |
| 5. Work | Ordinary manual development. Fleet is not scheduling against this Job |
| 6. Resolve | Depends on the outcome chosen. Evidence gates are unchanged |

**Pilot lives in the job header's action group, left of Kill.** It is a job-level act — it hands over one worktree and stops the scheduler, neither of which is a property of the selected step — so it sits at job level and never moves between statuses. What changes is the fill: secondary on a running job, primary on an escalated one, at the same height in both. Emphasis comes from fill rather than size or position, so the target learned on a healthy job is the target under pressure.

**Observe is a step control and Pilot is a header control**, so the two are never adjacent. Observe is a ghost and changes nothing; Pilot is filled and ends a Drone. The modal is the second gate.

**Assist's disabled copy reads: "Not available. A parked Drone would resume against a worktree you changed, and the re-brief for that is not built."** It names the missing mechanism rather than a schedule, so nothing is promised and nothing is implied about when. "Coming soon" was rejected: it commits to a date without giving one. The row keeps its geometry and loses its affordances — dashed edge, no fill, `--fg-subtle` label, empty radio, never reduced opacity — so the outcome set does not change shape when Assist ships.

Cancel is always available and leaves the Drone untouched.

## The confirmation modal

One button, three outcomes. The modal is the only place the choice is made, and it names the consequence rather than the mechanism.

| Outcome | What the engineer gets | Status |
| --- | --- | --- |
| **Take Over** | The Drone is gone. The worktree is yours and stays yours | Live |
| **Assist** | Unblock the Drone, then hand it back mid-step | Disabled, coming soon |
| **Restart Step** | Unblock, then a new Drone picks up the step with your work in place | Live |

Assist renders disabled rather than hidden. Copy has to say why it is off without promising a date — see Open questions.

## The other direction

The same flow starts without the button. A Drone that calls `escape_hatch` on its own puts the Job in front of the engineer with its stuck narrative attached, and the engineer picks an outcome from the same three. Step 1 is the only difference.

## What the engineer sees in the session

Context arrives preloaded, so the session opens knowing the Job rather than needing to be told. Contents are listed on Pilot.

The session runs at a Guild-level unrestricted toolset. The narrow Drone toolset is the thing being escaped, so inheriting it would defeat the flow.

## Settled in design, 2026-08-31

- **One slot, two fills.** Pilot never moves between a running job and an escalated one. Secondary on a running job, primary on an escalated one, same position and same height.
- **Observe sits on the step panel and Pilot in the job header.** Level is what keeps the two apart, rather than warning copy. On an escalated job Observe is absent rather than disabled: there is no Drone to read.
- **The modal states the two things a choice changes**, as two columns per outcome — the Drone, and the worktree. Everything else is identical across the three, so it is stated once above them.
- **The bundle has two shapes and one order.** The Fleet-assembled half is identical on both paths and renders as mono facts. The last block differs: a Drone's account renders as prose with its source above it and its standing below, and an escalation payload renders as measured and judged lines.
- **A piloted job's rail is frozen and nothing pulses.** The held step takes the `terminal` mark in `--fg-default` and no surface — no step-activity value describes a person at a terminal, and borrowing `running` would pulse a step nothing is working.
- **A piloted row is not dimmed.** `Not scheduled` is a field on the row. A job you are working is not a dormant one, and `piloted` renders amber for the same reason.

## Open questions

- **[pilot-refused-call-argument]** Does a refused `escape_hatch` call's argument reach the person? A Drone reaching for the hatch unbidden writes `trying_to`, `blocked_by` and `tried` before Fleet refuses the call, and on the `hatch_unbidden` path that is the only stuck narrative there is. Fleet must tell the Drone nothing, and that constraint says nothing about what the escalation shows a person. Drawn as shown, with its source stated — deferred pending a decision on whether the argument is kept at all.

## Related

Pilot — the concept page carrying the mechanism this journey's flow only names: the `escape_hatch` tool, the handoff bundle, the toolset and secrets rules, and the Evidence position.

This journey has no number because the design project has not drawn it. A number in a filename here means a `Journey N` drawing exists to match it; inventing one would assert a correspondence that does not.
