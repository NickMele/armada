# Journey 3 — Triage Queue

**What it is:** "What's waiting for me?" — a proactive check-in across everything that needs a decision, not triggered by a notification.

Design fidelity: not set. Analysis: Complete. UI/UX design: In progress.

---

**Trigger:** You're starting a work session and want to clear the decks before diving in.

**Concepts touched:** Bridge (Alerts, Reviews, Activity Feed).

**Milestone:** Surface.

**Design file:** `Journey 3 - Triage queue.dc.html` in the Armada Mockups project. One file per journey; this document is the record, the file is the drawing.

## Flow

Covers three Bridge surfaces together, since a genuine "what's waiting" check touches all of them:

| Surface | Purpose | Structure |
| --- | --- | --- |
| Reviews | Calm, evaluative — Job Review flow, human-gated advance per workflow settings | Simple oldest-first list |
| Alerts | Interventional — escalations needing a decision, browsed proactively rather than pushed | Alert Level (Blocked on top, Waiting below) → sub-grouped by trigger type → oldest-first. State (in-progress/waiting/resolved) is a filter toggle, not primary structure. Noted items never appear here — they go to the Activity Feed. |
| Activity Feed | Passive visibility — every completed Job streams in as it finishes, success or failure | Chronological stream, no actions |

## Alert Levels and this surface

**Updated Aug 2026.** The old "urgency tier" ordering was push-worthy over passive, which described interruption rather than importance. It is now Alert Levels: Blocked, Waiting, Noted, tested by whether work stopped and whether anything is waiting on you. See Respond to a Push Alert for the full definition and mapping table.

Two consequences for this journey:

- **Gate failures, dispatch approvals and Job Reviews are all Waiting.** They were three surfaces holding the same kind of item — something halted or finished, with a decision queued. **Settled 2026-08-21:** Reviews stays its own surface. Every item in it is Waiting, so it could have been a filter, but evaluating a diff and rescuing a stalled Job are different modes of attention, and a queue that mixes them makes the evaluative work feel urgent. Drawing it proved the point — the Reviews field run carries no step bar and no spend, because a Job at review is at its last step with its cost settled.
- **~~Doctor module failures now reach Alerts at Waiting.~~ Reversed 2026-08-21.** A health check is a standing condition, not a queued decision: a failing module has no workspace, no spend, no step and no Job id, and nothing about it is *waiting* in the sense every Alerts row is. Drawn first as a queue row with three fields faked, which is what made the mismatch obvious. **Doctor never enters Alerts.** It renders as a one-line neutral strip above the tabs, naming which modules are failing and whether work is stopped, with a way into Doctor. Alerts stays a list of Jobs.

## Distinction from Respond to a Push Alert

This journey is self-initiated — you chose to look. Respond to a Push Alert is reactive — a notification pulled you in to one specific item. Same underlying surfaces, different entry point and mental mode (calm scan vs. urgent single-item response).

## What is already decided and landed

Several questions this journey once carried are now settled:

- **Citation shape for per-judge records on a panel.** One Judge record per step, containing every judge's verdict. Citations name the criterion only — per-judge detail lives inside the record rather than in the link. This keeps the citation stable whether `panel_size` is 1 or 3, so the reference a Drone retries against does not change shape when rigor is dialled up. Consistent with unanimity: the step failed because a criterion drew a refusal, and which judge raised it is detail, not headline.
- **Glyphs for criterion rows and Judge verdicts.** Three families, one per verification source — `shield-*` for Checks, `file-*` for evidence artifacts, `circle-*` for Judge verdicts — so the silhouette carries hedge-by-source before the label is read. Judge gets `circle-check` / `circle-x`, in `--verdict-met` / `--verdict-not-met`. `circle-minus`, previously Check's "not reached," moved to `shield-minus` to complete the Check family. Bare `check`/`x` are out of criterion rows entirely, since `check` already means "advanced" in step activity.
- **Whose `iteration_count` a `request_changes` verdict increments.** The emitting step's — Design Plan's `present`, not the step it routes back to. `iteration_cap` and `verdict_routing` already live on the emitting step, and the cap and the count it bounds must not be split or `loop_cap` never fires; the loop is a property of the gate, not of the work; and it generalises to a workflow with two loops sharing a target step. Cost accepted: the rail renders "iteration 3 of 5" against the gate row rather than the draft row.
- **How a step with no mechanical check advances.** Once its evidence arrives — an absent tier is not a failed tier. A step with no `mechanical_check` and no `judge_check` is a single node, and Fleet advances it once evidence of the declared `evidence_type` arrives. The veto-only rule constrains the Judge, not the step: it says a model may only ever make advancement stricter, never looser, and where there is no Judge it does not apply at all. Design Plan, Investigation and Prototype are legitimately ungated on purpose.
- **What `auto_if_judge_high_confidence` means.** The confidence notion is dropped; the gate is renamed `auto_if_judge_passes`. A veto-only Judge either refuses or it does not, so there was no confidence scale to threshold and never was one. The full `advance_gate` value list is now stated in one place on the Workflow concept page: `auto`, `auto_if_judge_passes`, `human_always`, and `manifest_rule:<key>` reading a named Manifest-level policy, with `auto_merge` and `review_gate` the two keys in use today.

## Open questions

- **[judge-glyph-family-icons]** Which lucide icons make up the Judge glyph family?
  Judge has its own glyph family alongside `shield-*` (gates and checks) and `file-*` (evidence) — decided, per "What is already decided and landed" above — but the specific lucide icons within that family are not chosen. The family needs at least a positive and a negative variant to carry "no objection" / "refused." Output is actual rows on the Iconography page, not a note.

## Related

Respond to a Push Alert (the reactive version of checking Alerts).
