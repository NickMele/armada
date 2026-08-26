# Take Over a Job

**What it is:** The flow for taking a Job away from its Drone and finishing it by hand.

Design fidelity: not set. Analysis: Partial. UI/UX design: Not started.

---

*Numbering note: the design project has not yet drawn this journey and names no `Journey N - ...` file for it. Numbered here only to give the file set a stable order — see the note on Guild Setup & Configuration for how the sequence after Journey 9 was assigned.*

**Trigger:** A Drone is stuck, going the wrong way, or working on something you would rather do yourself.

**Concepts touched:** Pilot, Drone, Job.

**Milestone:** Recovery. Design note: flow only. The mechanism lives on the Pilot concept page. Thin by design, since Assist is deferred and the live path is one button, one modal, one session. Worth revisiting once Assist ships, which adds a hand-back leg the flow does not currently have.

Pilot is the citable source for the mechanism: the `escape_hatch` tool, the handoff bundle, the toolset and secrets rules, and the Evidence position. This document covers only what the engineer does.

## Flow

Open the Job in Bridge, hit Pilot, confirm an outcome in the modal, then work in the Claude Code session that opens on the Drone's worktree.

| Step | What happens |
| --- | --- |
| 1. Hit Pilot | Available on the Job while a Drone is running. Placement is an open item — see Open questions. |
| 2. Read the modal | States what is about to happen, then offers three outcomes plus Cancel |
| 3. Choose an outcome | Take Over, Assist (disabled), or Restart Step |
| 4. Session opens | Claude Code on the Drone's worktree, context preloaded, unrestricted toolset |
| 5. Work | Ordinary manual development. Fleet is not scheduling against this Job |
| 6. Resolve | Depends on the outcome chosen. Evidence gates are unchanged |

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

## Open questions

- **[pilot-button-placement]** Where does the Pilot button live on the Job surface?
  Not yet decided.

- **[assist-coming-soon-copy]** What does the Assist coming-soon copy say without promising a date?
  Not yet decided. Assist renders disabled rather than hidden, and per the Voice Contract the copy has to say why it is off without committing to a ship date.

## Related

Pilot — the concept page carrying the mechanism this journey's flow only names: the `escape_hatch` tool, the handoff bundle, the toolset and secrets rules, and the Evidence position.

This journey has no number because the design project has not drawn it. A number in a filename here means a `Journey N` drawing exists to match it; inventing one would assert a correspondence that does not.
