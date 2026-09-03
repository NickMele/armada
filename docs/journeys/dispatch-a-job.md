# Journey 1 — Dispatch a Job

**What it is:** "I want to start something" — browse available work, pick one, approve it.

Design fidelity: Full now — design this one completely, ahead of everything else, because its approval UI reappears in Triage, Alerts, Helm and the pre-approved queue, and a journey designed after the surfaces that copy from it gets retrofitted into them.

---

**Trigger:** You have work ready to hand off, or you're clearing the Job Board.

**Concepts touched:** Job Board, Fleet.

**Milestone:** M1 — Dogfood.

**Drawn against** `docs/contracts/design-system.md` and `docs/contracts/workflow-design-system.md`.

**From the milestone design pass.** The full approval flow and the M1 subset are drawn side by side there, with the subset marked by dimming everything M1 does not render — the Job Board, pre-approval, pattern learning and the criteria editor. M1's reduced card keeps the glance strip with the two values it can measure before dispatch, workflow length and which Checks gate it, because a card whose whole design is a forced glance cannot ship with nothing to glance at.

Analysis: Complete. UI/UX design: In progress.

## Flow

Open Job Board (scoped to a Manifest) → browse the flat list (or opt into the DAG graph view) → select a ready Job → Job detail view opens → explicit approval action, separate from selection.

See Job Board for the full board mechanics — layout, status states, origin tags.

## Approval Rules

| Aspect | Resolution |
| --- | --- |
| Default | Strictly one-by-one, no batch-approve, no auto-approve-by-rule. This is the primary autonomy control on Fleet and stays strict. |
| Pre-authorized batch (away-from-desk) | Before stepping away, mark specific queued Jobs as pre-approved to dispatch in your absence. Indefinite until run or manually revoked (configurable later). Any Job eligible regardless of risk — your call at approval time. |
| Approval-card design | Not a gating change, but a mitigation for approval fatigue: the card should force a glance at diff size / Job type / cost estimate before the tap registers, rather than a bare "Approve?" button. |

## Pattern Learning

Repeated identical approvals (e.g. the same command tripping the allowlist every time) don't just sit as isolated events. After N identical trips, Armada proactively flags the pattern and suggests an allowlist/Manifest change — you confirm or decline, never auto-applied.

## Where the approval gate's shape stands

The classifier's proposal is approved or overridden inside the dispatch flow itself, decided 21 Aug 2026 — not as a separate gate. Two things forced the merge: the Job proposer no longer proposes a "shape," it proposes a Job's write targets and its atomic flag, which is scope; and scope revisions were decided the same day to route back through the dispatch approval gate. A scope decision at dispatch and a scope decision mid-flight going through two different gates would have been arbitrary. Merging also collapses three things once called "approval" down to two, which matters because three is how a builder implements one and silently drops another.

Where the Job proposer emits several Jobs, all of them stand at the gate and each takes its own one-by-one dispatch approval when its turn comes, so the strictly-one-by-one rule and the no-batch-approve rule both stay intact. There is no plan-level approval, and [Job proposer](../concepts/job-proposer.md) owns why. The accepted cost is two taps for a single-workspace Job whose proposal is obvious, mitigated by such a proposal being trivially acceptable.

What the structural decision deliberately leaves open is what the surface looks like. No UI/UX design had started on any journey when this was decided, and this journey is design order 1 precisely because everything else reuses its approval pattern — forcing a surface before that pass would have designed the convention backwards. What has since taken its place is the Job proposer's own order: a person describes the work or links a ticket, the proposal is visible filling in as it is worked out rather than appearing complete at the end, and approval is the act that starts the work.

Two adjacent Job Board gaps sit in the same area and are still open surface questions for that pass, not blockers on the structural decision — see Open questions below.

## What is already decided and landed

A handful of questions this journey once carried are now settled, recorded here so their reasoning isn't lost even though the open item itself is gone from the decision record:

- **The lexicon entry for the frozen acceptance criteria field** is plain lowercase "acceptance criteria" — no proper noun. Metaphor is confined to proper nouns and the lexicon already carries fourteen; countable things are lowercased by the casing rule, so "criterion" and "criteria" follow. The field is `Job.acceptance_criteria[]`, frozen at Job creation, designed for 1–6 entries, written into Job's Other Fields with type, cardinality and freeze point stated, plus the Evidence linkage — Evidence carries a per-criterion row `{criterion_id, verdict, citation, source}`, where `source` there is the verification source and a different vocabulary from the criterion's own. A criterion may be appended at an approved widening, but never edited, reordered or removed, so a frozen-position guarantee holds and a Judge citation to "criterion 4" still resolves after a widening.
- **Model selection per Job** is yes in principle — `DroneSpawnConfig` carries a `model` field because the adapter must pass a model string to the harness on every spawn — but M1 builds no plumbing to vary it: no create-form field, no Job column a person sets, no picker in Bridge. The value comes from configuration. A picker arrives with the first milestone that wants two models, and per-Job selection will then have two other homes to reconcile with: `judge_check.model`, already a per-step dial, and `policy.model`, which sits in the deferred half of the `armada.yml` schema.
- **The field naming which WorkflowDef a Job follows** is `workflow_id` on both sides, not `task_type`. `task_type` named a category that does not exist as an entity, while every other reference on the Job record — `owner_manifest_id`, `gate_manifest_ids[]`, `dispatched_by` — names the thing it points at. `task` is also a banned synonym for Job under the lexicon, which made `task_type` doubly wrong: it was the field name most likely to propagate the wrong word into code and UI. The rename costs nothing structural, since Job is not yet built.
- **A Convoy can be a peer node in dependencies.** It carries and is the target of `depends_on` / `blocks` like any other Job; only decomposition is excluded, and both the Convoy and Job pages were reworded from "uses none of" to "does not decompose." The links sequence peers, and peers are not children, so having no children never ruled out sequencing a Convoy against a peer. The Job proposer may also emit a mixed decomposition — a graph with a Convoy as one node, atomicity set per group — decided the same day.
- **A not-started Job's approval state and its readiness state are two separate fields, not one four-value field with a precedence rule.** Approval state (`needs_approval` / `pre_approved`) and readiness state (`blocked_by_dependency` / `waiting_on_resources` / ready) are two axes that were jammed into one, and a Job genuinely carries both at once. The existing symptom was the diagnosis: a sub-dispatched Job is always approved, inheriting its parent's approval, so one out of headroom used to compute as `pre_approved_queued` and, by the Board's own visibility rule, never rendered at all.

## Open questions

None. Both this journey carried were answered by the Board drawing on
2026-08-31 and now live as facts on [Job Board](../concepts/job-board.md):
origin is display-only rather than a filter axis, and the default sort is
Critical first. The Board's graph view is tracked at `[job-board-graph-view]`
there, which owns the Board's layout, and is not duplicated here.

## Related

Job Board (full board mechanics) · Kit (allowlist defaults, formerly filed as Guild) — see Guild Setup & Configuration.
