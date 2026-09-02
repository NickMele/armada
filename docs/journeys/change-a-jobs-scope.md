# Change a Job's Scope

**What it is:** The flow for widening or narrowing a dispatched Job's scope, and what each direction costs.

Design fidelity: drawn. Analysis: Complete. UI/UX design: Complete.

---

**Trigger:** A dispatched Job needs a Workspace nobody listed, or needs less than was approved. Without this the only lever is killing it and dispatching again.

**Concepts touched:** Job, Convoy, Drone, Workspace.

**Milestone:** Recovery.

The mechanism a widening rests on does not exist — see [Open questions](#open-questions). Everything the flow needs beyond what the record already states is listed under [What is invented](#what-is-invented), so nothing below reads as settled that is not.

## Flow

Two entrances and two directions. A Drone asking is the common one; a person changing it before anybody asks is rare and is the only route to a narrowing.

| Step | What happens |
| --- | --- |
| 1. A Drone finds it needs a path its scope excludes | It asks, with the paths and a reason in its own words |
| 2. The Job returns to the gate | `running → awaiting_approval`, with the revision recorded `pending` |
| 3. The Drone ends | The gate holds no process, so asking spends the session before anybody answers |
| 4. The Board shows it needs approval | A second approval on a Job that has already run |
| 5. You approve, wait, or decline | Approving re-queues the Job; the third answer has nowhere to go |
| 6. A fresh Drone restarts the step | On the same worktree and branch, with configuration resolved again |

**A widening returns to the gate; a narrowing does not.** The asymmetry is specified rather than an omission: a widening asks for permission never given, and a narrowing hands back permission already held, so the Job stays `running`.

## What each direction costs

**Both cost a respawn.** The Drone is terminated and a fresh one spawned from its brief either way, so neither is free, and the cost strip states the same events on both surfaces.

| | Widening | Narrowing |
| --- | --- | --- |
| Approval | Returns to the gate and waits | None — the Job stays running |
| The Drone | Ends when it asks, before an answer | Respawned on the press |
| The step | Restarts from the top under a new drone id | Restarts from the top under a new drone id |
| The worktree and branch | Kept | Kept |
| Configuration | Resolved again | Resolved again |
| A gating Workspace that leaves scope | — | Its Checks record `did not run`, reason `scope_narrowed`, disposition `Suspect` |

**A scope revision is the only act that re-resolves configuration.** Every other act reads the snapshot taken when the Job was created.

**Redirect is the contrast, and it is what makes the cost legible.** Both put a person back on a running Job; a redirect is a turn into the session the Drone is already holding, keeps everything it has read, restarts nothing and consumes no attempt. A change of scope that kept the session would be a redirect that lied about what it takes away.

**When the cost lands is the sharpest fact.** Because the gate holds no process, the Drone ends when the widening is asked for — so a card saying that approving *will* restart the step would describe a bill settled before a person arrived.

## The act sits on the scope block, not the header

**The header holds acts on the run; this edits a field.** Changing scope means reading the current scope, which the header cannot show, so level decides — the same test that placed Observe.

**A checklist over declared Workspaces, never a text field.** A write target resolves to a Workspace under one root, so the legal set is enumerable and the root bound is enforced at the choice rather than at the gate.

**Both directions are one gesture.** The record holds `paths_added` and `paths_removed` on one entry. Any addition makes the whole revision a widening; removals alone do not.

**Offered on `queued` and `running` only** — the two statuses with an edge to the gate — and absent elsewhere rather than disabled.

## The Board carries two asks under one badge

`awaiting_approval` stores no reason, so a Job asking to *run* and a Job asking to *carry on* show the same badge. The field run separates them: a step, an elapsed and a spend against a workflow name, with the scope delta taking the write-target field's place because the delta is what is being decided.

**No second badge.** A bordered pill is a Job state and nothing else.

## The gate, second time

The dispatch card in a second mode. What is approved the second time is scope, and only scope.

**Not a Job editor.** Title and workflow are a read-only line, and nothing offers the scope approved at dispatch for editing — the addition is on its own surface.

**Criteria append and are never edited, reordered or removed.** Position is identity, so a criterion a Judge cited before a widening is the same criterion after it. A criterion added at the gate is the person's; a Drone does not raise its own bar.

**An overlap with another Job is a fact, not a verdict.** Where the added Workspace is one another Job is writing into, approving with a wait re-queues behind it.

**The rationale is the Drone's own words, attributed.** It is an argument rather than a fact, and does not render in the same voice as the check counts above it.

## Widening a Convoy makes the Drone weaker

The one case where more scope means less capability, and the only thing on a Convoy's gate a person could not work out alone. **Permissions intersect, and intersection is monotone**, so every Workspace added can only remove capability.

| Resolved across the Convoy | Rule |
| --- | --- |
| Allowlist | Intersection — a tool no added Workspace grants is dropped |
| Secrets | Intersection |
| MCP servers, sub-agents | Only what every gating Manifest defines |
| Commands | Union, namespaced, so two commands never collapse into one name |
| Skills, agent file | Union — they are instructions, not permissions |

**A revision can end the shape.** Narrowing to one target, or clearing atomic, stops the Job being a Convoy, so the card states both atomic lines whether or not they differ.

## The record

**One entry per revision, and entry zero is the scope as approved**, so replaying the list reconstructs the Job's shape at any moment. An entry carries both path lists, the rationale, and its outcome — taken, taken without approval, or not taken.

**A revision that did not take is dimmed, not hidden.** *Was this ever asked* is a question people ask later.

**An evidence row links back to the revision, not the other way.** Scope history is what a person decided; evidence is what happened. `Suspect` is the record's own disposition and separates a Check skipped because the diff touched none of its paths — expected — from a Workspace that left scope with work in it.

## What is invented

Everything below is drawn without a source, and each is a decision somebody has to make before this can be built.

| Invented | What it rests on |
| --- | --- |
| That a Drone can ask for a widening at all | Nothing. `declare_scope` refuses a path the resolved scope excludes, and no escalation trigger covers the refusal |
| That the Drone ends when it asks | Follows from the gate holding no process, and is the most expensive consequence here |
| The Board's field run distinguishing the two asks | The status cannot, so the fields shown are a judgement |
| That a Drone does not ask to narrow, and does not append criteria | Argued from a Drone gaining nothing by either. Nothing rules either out |
| That the act lives on the scope block | Argued from level |
| A checklist rather than a text field | Follows from the root bound; the surface is a judgement |
| That any addition makes a mixed revision a widening | The record holds both path lists on one entry, so one act is right. The classifying rule is invented and nothing tests it |
| That the rationale is required | The field exists; nothing says it is required or who writes it |
| That the narrowing is a confirmation rather than a silent toggle | Unchallenged does not mean silent |
| That narrowed-away edits stay in the diff and reach the Judge | Follows from write targets not binding writes |
| That a respawn consumes no retry attempt | A reading. `retry_count` counts failures and a scope change is not one |

## Open questions

- **[drone-cannot-request-a-widening]** How does a Drone that needs a path outside the Job's scope ask for one? Filed as #299. The `running → awaiting_approval` edge exists and nothing in the Drone's toolset can take it. The answer has to settle four things: whether the Drone ends on the call, whether it may ask twice for the same path, whether the reason is its own words or a structured field, and whether it may ask to narrow.
- **[declined-widening-has-no-edge]** Where does a Job go when a person declines a widening? `awaiting_approval` leaves only to `queued`, `rejected` or `killed`, so declining ends a Job that has passed two steps — while the record states that its entries include revisions that did not take. A candidate is `awaiting_approval → queued`, the revision recorded as not taken and the Job resuming at the scope it already had; it re-queues rather than resumes because the gate ended the Drone.
- **[widening-and-the-judge-context]** Should the gate warn that a wider diff may exceed the Judge's context? A Convoy is where `max_context_size` bites first.
- **[scope-revision-on-a-stopped-job]** Is a scope revision legal on an escalated or piloted Job? No edge exists, so it is not offered, and whether that is deliberate is unknown.

## Related

- [`../concepts/job.md`](../concepts/job.md) — the scope history field, the transitions, and the recovery acts this is measured against.
- [`../concepts/convoy.md`](../concepts/convoy.md) — the permission rules a widening resolves again.
- [`respond-to-a-push-alert.md`](respond-to-a-push-alert.md) — Redirect, the act that puts a person back on a Job and costs no respawn.
- [`take-over-a-job.md`](take-over-a-job.md) — the level test this reuses to place the act.
