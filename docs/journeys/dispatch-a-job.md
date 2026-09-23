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

## What the dispatch screen holds

`DispatchRequest` in `packages/components`, assembled by `DispatchJob` in
`packages/screens`. One card, in the order a person meets it:

| On the card | What it is |
| --- | --- |
| Repository | A fact, answered before the card opens. Bridge dispatches into the workspace it is pointed at |
| From, Lands in | Where the work starts and where it lands, as two fields over the repository's branches. They differ when you start from an unmerged branch or land in a long-lived one |
| Write, Sketch | Which of the two ways of saying it is open. Words or a picture, never both at once — the card is narrow, and the switch swaps the one block under it |
| Request | Prose, or a link to a ticket. `@` opens the file mention popup |
| Sketch | Boxes and the lines between them, on the canvas a Studio's whiteboard draws on. Beside it, what the picture is meant to show, and the Studio node it was made from where there was one |
| Attach, Add a link | A staged file, an address, or the Studio node the request came off. Each is a chip that says its kind and can be taken back. A link goes out with the request, one to a line |
| Settings | Optional, and closed to start. Its head says how many are set |
| What happens next | Armada reads the request and names the Job; you adjust and approve it; a planning Drone splits the work and a Judge reads what comes back |

**There is no hand-entry form.** `Enter by hand` and the Job it built are gone
— the owner's call of 23 September 2026, once Settings carried every decision
the form did. [Job proposer](../concepts/job-proposer.md) has the rule.

### The two branch fields

Both open on the Manifest's base and both offer the repository's branches, each
row saying what it is: the base, or the Job whose worktree is sitting on it.

**Only where it lands makes one.** Typing a name no branch carries offers it as
a new branch, and taking that row is the whole of creating it — there is no
second control. Where the work starts offers none, because you cannot begin on
a branch that does not exist.

**Both still take a name typed by hand**, because the list is a floor rather
than the repository's own. The shape is draft —
`packages/screens/src/draft/branches.ts` — and nothing on the wire lists a
repository's refs, so against a real Fleet the list is absent and the two draw
as plain fields. A branch Armada has never met is not a branch that is not
there, and the field says which of the two it is looking at.

### Settings, and what absent means

Four, each of them a decision somebody else takes when it is left alone:

| Setting | Left alone |
| --- | --- |
| Workflow | The proposer picks it from the request. Where a definition Fleet holds was left out, it is named under this field — #425 |
| Models by tier — difficult, medium, easy | Auto: the harness chooses, and a planning Drone's tier for a task is what selects the model |
| Drones at once | The machine's own cap holds. The two figures are drawn apart: how many this Job may run, and how many run here across every Job |
| How it lands | The workflow's delivering step decides. Set it to land without asking, or to stop for you at review |

The four are **draft** — `packages/screens/src/draft/dispatch.ts`, which names
the `crates/ipc` module it is meant for. Nothing on the wire carries them yet,
so a Job created today is classified as it always was and the schema lock is
where they reach Fleet.

### What else is running

Beside the request at a wide window, under it at a narrow one: the Jobs already
writing where this work would, each with the paths the two of them share and
where that Job is.

**It is a fact and never a verdict.** Nothing on the surface is greyed by it and
nothing is refused; dispatching into a file another Job holds is the ordinary
case, and the panel exists so it is done knowingly.

**Nobody looked and nobody was found are two sentences.** Today's overlap read
hangs off an existing Job and answers nothing until paths are claimed, so at
dispatch nothing has been compared and the panel says so. The shape is draft —
`packages/screens/src/draft/peers.ts`.

### The sketch, and what of it reaches Fleet

A sketch attaches to the prompt as a chip reading *From a Studio · sketch 1*,
and the words typed under Write stay where they are while it is drawn. The chip
takes no removal control: a picture is taken back on the pad, where the boxes
going are visible.

**The picture is draft and the wire is unchanged** —
`packages/screens/src/draft/sketch.ts`, which names the `crates/ipc` module it
is meant for. An attachment on the wire carries a staged path, a filename and a
type and no provenance, so where a sketch was made is a draft field. Nothing
stages the pad yet either: pressing Dispatch sends the words, and staging the
PNG belongs with the schema lock.

## Approval Rules

| Aspect | Resolution |
| --- | --- |
| Default | Strictly one-by-one, no batch-approve, no auto-approve-by-rule. This is the primary autonomy control on Fleet and stays strict. |
| Pre-authorized batch (away-from-desk) | Before stepping away, mark specific queued Jobs as pre-approved to dispatch in your absence. Indefinite until run or manually revoked (configurable later). Any Job eligible regardless of risk — your call at approval time. |
| Approval-card design | Not a gating change, but a mitigation for approval fatigue: the card should force a glance at diff size / Job type / cost estimate before the tap registers, rather than a bare "Approve?" button. |

## Pattern Learning

Repeated identical approvals (e.g. the same command tripping the allowlist every time) don't just sit as isolated events. After N identical trips, Armada proactively flags the pattern and suggests an allowlist/Manifest change — you confirm or decline, never auto-applied.

## Where the approval gate's shape stands

A Job proposal is approved or overridden inside the dispatch flow itself, not at a gate of its own. What forced the merge: scope revisions route back through the dispatch approval gate, so a proposal with a gate of its own would have put a scope widening and a mid-flight revision through two different gates for no reason. Merging also collapses three things once called "approval" down to two, which matters because three is how a builder implements one and silently drops another.

What the proposer proposes is the workflow, the title, and the split where the work is several Jobs. Scope is not among them — [Job proposer](../concepts/job-proposer.md) owns why, and none of that argument is restated here.

Where the Job proposer emits several Jobs, all of them stand at the gate and each takes its own one-by-one dispatch approval when its turn comes, so the strictly-one-by-one rule and the no-batch-approve rule both stay intact. There is no plan-level approval, and [Job proposer](../concepts/job-proposer.md) owns why. The head of a proposal is approved on the proposal itself, so a Job whose proposal is obvious costs one tap — the trip to detail is for reading it, not for agreeing with it.

What the structural decision deliberately leaves open is what the surface looks like. No UI/UX design had started on any journey when this was decided, and this journey is design order 1 precisely because everything else reuses its approval pattern — forcing a surface before that pass would have designed the convention backwards. What has since taken its place is the Job proposer's own order: a person describes the work or links a ticket, the wait says what the call is doing and offers the stop, the Jobs arrive whole, and approval is the act that starts the work — on the proposal itself.

**Approving is not the only act at the gate, and what locks, locks at approval.** A proposal is a reading a person corrects: the title, the gate on each step, which model each tier runs, how many Drones this Job may take, how it lands, and what the Job is held to. The workflow and the gates were once frozen when the Job was created, which made every correction a rejection and a retyped request; the decision of 22 Sep 2026 moves that freeze to the approval press. Bridge draws it — the proposal while it is yours to change, and the same values with the instant they froze at. **Fleet still freezes at creation and carries no per-Job gate, no tier map and no criterion origin**, so what Bridge holds lives in the draft schema under `packages/screens/src/draft/` and reaches no operation; the schema lock (#1545) is where each shape moves onto the wire.

Two adjacent Job Board gaps sit in the same area and are still open surface questions for that pass, not blockers on the structural decision — see Open questions below.

## What is already decided and landed

A handful of questions this journey once carried are now settled, recorded here so their reasoning isn't lost even though the open item itself is gone from the decision record:

- **The lexicon entry for the frozen acceptance criteria field** is plain lowercase "acceptance criteria" — no proper noun. Metaphor is confined to proper nouns and the lexicon already carries fourteen; countable things are lowercased by the casing rule, so "criterion" and "criteria" follow. The field is `Job.acceptance_criteria[]`, designed for 1–6 entries, written into Job's Other Fields with type, cardinality and freeze point stated, plus the Evidence linkage — Evidence carries a per-criterion row `{criterion_id, verdict, citation, source}`, where `source` there is the verification source and a different vocabulary from the criterion's own. **The freeze point is the approval press** (22 Sep 2026), not Job creation: the criteria are read out of the linked issue where there is one, each is editable until you approve, and each says where its words came from — the issue, your prompt, or you. After the press they are what the Judge marks against, and the issue moving afterwards changes nothing except that the Job says it has moved. A criterion may then be appended at an approved widening, but never edited, reordered or removed, so a frozen-position guarantee holds and a Judge citation to "criterion 4" still resolves after a widening. **What is not built is the second half**: Fleet freezes at creation, `source` still means how a criterion is verified rather than where it came from, and nothing re-reads an issue to notice it changed — Bridge draws all three against the draft schema.
- **Model selection per Job** is yes, and this journey is where it is set. `DroneSpawnConfig` carries a `model` field because the adapter must pass a model string to the harness on every spawn; dispatch names one per tier rather than one for the Job, because a plan's tasks are not all the same size — the hand form that named a model directly is gone. The scope note that said M1 would build no picker was written before any of these surfaces existed and the owner decided otherwise. A model named nowhere still resolves to the configured default at the Fleet boundary. Per-Job selection has two other homes to reconcile with: `judge_check.model`, already a per-step dial, and `policy.model`, which sits in the deferred half of the `armada.yml` schema.
- **The field naming which WorkflowDef a Job follows** is `workflow_id` on both sides, not `task_type`. `task_type` named a category that does not exist as an entity, while every other reference on the Job record — `owner_manifest_id`, `gate_manifest_ids[]`, `dispatched_by` — names the thing it points at. `task` is also a banned synonym for Job under the lexicon, which made `task_type` doubly wrong: it was the field name most likely to propagate the wrong word into code and UI. The rename costs nothing structural, since Job is not yet built.
- **Every Job is a peer node in dependencies.** It carries and is the target of `depends_on` / `blocks`, whatever it writes and however it lands. The links sequence peers rather than children, so a Job whose own members are Jobs is sequenced against a peer like any other — see [Landing](../concepts/landing.md). How the work lands is not part of what the proposer proposes.
- **A not-started Job's approval state and its readiness state are two separate fields, not one four-value field with a precedence rule.** Approval state (`needs_approval` / `pre_approved`) and readiness state (`blocked_by_dependency` / `waiting_on_resources` / ready) are two axes that were jammed into one, and a Job genuinely carries both at once. The existing symptom was the diagnosis: a sub-dispatched Job is always approved, inheriting its parent's approval, so one out of headroom used to compute as `pre_approved_queued` and, by the Board's own visibility rule, never rendered at all.

## Open questions

None. Both this journey carried were answered by the Board drawing on
2026-08-31 and now live as facts on [Job Board](../concepts/job-board.md):
origin is display-only rather than a filter axis, and the default sort is
Critical first. The Board's graph view is tracked at `[job-board-graph-view]`
there, which owns the Board's layout, and is not duplicated here.

## Related

Job Board (full board mechanics) · Kit (allowlist defaults, formerly filed as Guild) — see Guild Setup & Configuration.
