# Change a Job's Scope

**What it is:** The flow for widening or narrowing a dispatched Job's scope, and what each direction costs.

Design fidelity: drawn. Analysis: Complete. UI/UX design: Complete.

---

**Trigger:** A dispatched Job needs a Workspace nobody listed, or needs less than was approved. Without this the only lever is killing it and dispatching again.

**Concepts touched:** Job, Convoy, Drone, Workspace, Judge.

**Milestone:** Recovery.

Everything the flow needs beyond what the record already states is listed under [What is invented](#what-is-invented), so nothing below reads as settled that is not.

## Scope is judged, not approved

**A declaration was never binding on writes.** `write_targets` binds nothing — a Drone's worktree is a whole-repo checkout — and the reason a Job states its scope at all is so that [Fleet](../concepts/fleet.md) and the [Judge](../concepts/judge.md) can see it drifting. So a Drone asking for more scope is not asking for permission, and the question it raises is *does this make sense for the step this Drone was given*, which is a Judge's question rather than a person's.

**A Judge call ends nothing.** A step with a call in flight is still `running`, so the Drone stays alive, keeps its session, and carries on the moment the call answers. That is what makes this different from every earlier reading of a widening, which sent the Job back to the dispatch approval gate — and the gate holds no process, so asking spent the session before anybody answered.

**A person is met by the exception.** A Judge that refuses escalates the Job, with the reason on the record and the Drone alive and idle in front of somebody.

## Two lists are called a Job's scope

They answer different questions, and neither is authoritative over the other.

| | The Job's write targets | A step's declared paths |
| --- | --- | --- |
| What it is | What a person mentioned when asking | What a Drone found on reading the code |
| The field | `write_targets`, one list per Job | `DeclaredPaths`, one per step per run |
| Set | Once, at creation, from the request | Every step, through `declare_scope` |
| Measured against | Nothing. It meets no diff | The real diff at the gate, and live edits each turn |
| Nothing there | Null — scope not yet determined | The step declared nothing |
| Who reads it | The overlap warning, and `request_scope` | The drift check, and the Judge look it triggers |

**A stated list is a seed, not a fence.** A person asking for work names the files worth naming and will not name every file the work turns out to need, so the list is partial even where it exists.

**Drift is measured against the declaration alone.** A changed file no declared path covers is what a Judge is given; the Job's write targets weigh nothing in that comparison.

**A Job that stated no scope cannot be widened.** `request_scope` grows the write targets, so a null list leaves the paths nothing to be an addition to and the request is refused with *scope not yet determined*.
Why: a request that determined the list would let a Drone write the whole answer to a question the Job never asked.

**That is the common case rather than the edge.** Every Job the [Job proposer](../concepts/job-proposer.md) drafted reaches its first step with the list null. What such a Drone has instead is `declare_scope`, which costs no call, refuses nothing but the step's own denylist, and is what the drift check reads.

## Flow

Two entrances and two directions. A Drone asking is the common one; a person changing it before anybody asks is rare and is the only route to a narrowing.

| Step | What happens |
| --- | --- |
| 1. A Drone finds it needs a path the Job's scope does not cover | It asks, with the paths and a reason in its own words |
| 2. Fleet answers what it can without spending anything | A path the step's denylist excludes, a path already in scope, a Job whose scope was never stated, or a step that has already asked — each is refused there and then, and none of them costs a call |
| 3. One Judge call goes out | The Job stays `running` and the Drone stays on its step. A person watching sees the wait, because the call is marked like every other |
| 4a. It is consistent with the step | The paths join the Job's scope, the revision is recorded as taken, and the Drone carries on — declaring them again through `declare_scope`, which is what the drift check measures against |
| 4b. It is not | The revision is recorded as not taken, the step stops carrying `scope_refused`, and the Job escalates with the Judge's reason |
| 5. On a refusal, you take it from there | The Drone is alive and idle, so redirect, restart the step or widen the scope yourself — the acts an escalated Job already has |

**A widening is judged; a narrowing is not.** A narrowing asks for nothing — it hands back scope already held — so there is nothing for a Judge to be consistent about and no call is made. It is a person's act and it stays one: nothing in a Drone's toolset can reach it, and the tool has no field for a path to remove.

## What each direction costs

**Neither costs a respawn any more, and that is the whole of the change.** A Drone asking to widen keeps its session and its place under the concurrency bound.

| | Widening a Drone asks for | Narrowing |
| --- | --- | --- |
| Who answers | A Judge, in one call | Nobody — the person doing it has decided |
| Approval | None. A refusal escalates, and that is where a person comes in | None — the Job stays running |
| The Drone | Alive throughout. Idle only if the request is refused | Respawned on the press |
| The step | Carries on. On a refusal it stops, carrying the verdict | Restarts from the top under a new drone id |
| The worktree and branch | Kept | Kept |
| Configuration | **Not resolved again** | Resolved again |
| A gating Workspace that leaves scope | — | Its Checks record `did not run`, reason `scope_narrowed`, disposition `Suspect` |

**The running Drone keeps the toolset it was spawned with.** Configuration is resolved at spawn, and nothing here respawns — so a Drone whose Job widened mid-step holds the permissions the narrower Manifest set gave it, and the intersection rule below governs the *next* Drone rather than this one. That is the price of not ending the session, and it is the right way round: permissions intersect, so the running Drone is never holding more than the wider set would have granted.

**Redirect is the contrast, and it is what makes the remaining cost legible.** A redirect is a turn into the session the Drone is already holding. A judged widening is now the same shape — nothing restarts, nothing is discarded — and what separates them is that one is a person's words and the other is a decision about the task's own shape.

## The act sits on the scope block, not the header

**The header holds acts on the run; this edits a field.** Changing scope means reading the current scope, which the header cannot show, so level decides — the same test that placed Observe.

**A checklist over declared Workspaces, never a text field.** A write target resolves to a Workspace under one root, so the legal set is enumerable and the root bound is enforced at the choice rather than at the gate.

**Both directions are one gesture.** The record holds `paths_added` and `paths_removed` on one entry. Any addition makes the whole revision a widening; removals alone do not.

**Offered on `queued` and `running` only**, and absent elsewhere rather than disabled.

## What the Judge is given

All of it is in hand when the request arrives, and none of it is anything a Drone chose except the last:

| | |
| --- | --- |
| The step's intent | Its label and the file it was asked to write, off the frozen workflow |
| The request the whole Job answers | Its title, its Facts and its acceptance criteria, labelled as the standard rather than as something under judgment |
| The Job's write targets | Whole, so an addition can be told from what was already there |
| The paths asked for | Only the ones outside those write targets |
| Why | The Drone's own words, labelled as an argument rather than as a fact |

**It is not shown the diff.** The question is about a plan rather than about work, and a call handed the diff would be answering the mid-step convergence look's question instead.

**It answers consistency, never desirability.** Whether the widening is wise is a person's; this look exists precisely so that a person is not asked about every request.

**One call and no panel**, and it is outside `judge_call_cap` — that dial bounds `criteria × panel_size` over what a step *declared*, and no declaration mentions this look any more than one mentions drift or convergence. What bounds it is **one ask per step**, counted off the Job's own scope history rather than held on the slot, so it survives a Fleet that restarts. A request refused before a call is made spends nothing and is not counted.

## The Board still carries two asks under one badge

The gate is no longer one of them for a Drone-asked widening, but `awaiting_approval` still stores no reason, so a Job asking to *run* and a person's own widening awaiting approval show the same badge. The field run separates them: a step, an elapsed and a spend against a workflow name, with the scope delta taking the write-target field's place because the delta is what is being decided.

**No second badge.** A bordered pill is a Job state and nothing else.

## A person's widening still has a gate

The dispatch card in a second mode, on the route a *person* takes. What is approved the second time is scope, and only scope.

**Not a Job editor.** Title and workflow are a read-only line, and nothing offers the scope approved at dispatch for editing — the addition is on its own surface.

**Criteria append and are never edited, reordered or removed.** Position is identity, so a criterion a Judge cited before a widening is the same criterion after it. A criterion added at the gate is the person's, and this is the only route by which criteria are appended at all — **a Drone does not raise its own bar, and its tool has no field with which to.**

**An overlap with another Job is a fact, not a verdict.** Where the added Workspace is one another Job is writing into, approving with a wait re-queues behind it.

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

**A Drone-asked widening changes neither.** It adds paths and touches `atomic` not at all, and the Drone that asked keeps the configuration it was spawned with — so the table above bites on the next Drone rather than on the one holding the session.

## The record

**One entry per revision, and entry zero is the scope as approved**, so replaying the list reconstructs the Job's shape at any moment. An entry carries both path lists, the rationale, and its outcome — `took` or `not_taken`.

**A judged widening needs no field the record did not already have.** `at_step` names the step that asked, `paths_added` what it asked for, `rationale` its own words, `outcome` what the Judge decided. `approved_by` is `fleet`, because a Judge call is a call Fleet makes and authenticates as, and the actor vocabulary names no Judge — what separates a judged widening from a mechanical one is the outcome beside it and the Job's own log.

**A revision that did not take is dimmed, not hidden.** *Was this ever asked* is a question people ask later, and a refused request is the entry a person reads beside the escalation it caused.

**An evidence row links back to the revision, not the other way.** Scope history is what a person decided; evidence is what happened. `Suspect` is the record's own disposition and separates a Check skipped because the diff touched none of its paths — expected — from a Workspace that left scope with work in it.

## What is invented

Everything below is drawn without a source, and each is a decision somebody has to make before this can be built.

| Invented | What it rests on |
| --- | --- |
| The Board's field run distinguishing the two asks | The status cannot, so the fields shown are a judgement |
| That a Drone does not ask to narrow, and does not append criteria | Argued from a Drone gaining nothing by either, and now built that way: the tool has a field for neither |
| That the act lives on the scope block | Argued from level |
| A checklist rather than a text field | Follows from the root bound; the surface is a judgement |
| That any addition makes a mixed revision a widening | The record holds both path lists on one entry, so one act is right. The classifying rule is invented and nothing tests it |
| That the rationale is required | Built that way — a blank one is refused where the Drone can still fix it — because it is what a person reads beside a refusal. The field's own registry row still says nothing about it |
| That the narrowing is a confirmation rather than a silent toggle | Unchallenged does not mean silent |
| That narrowed-away edits stay in the diff and reach the Judge | Follows from write targets not binding writes |
| That a respawn consumes no retry attempt | A reading. `retry_count` counts failures and a scope change is not one |
| That one ask per step is the right bound | Argued rather than measured. A refusal escalates, so a second ask only arises after a clear, and a step whose plan was wrong twice is a step a person should see. Nothing measures how often that is wrong |
| That a refused widening escalates at all | The owner's, and the reason a person is met by the exception rather than by every request. It carries `scope_refused`, a row of its own rather than `blocked_by_policy` — that one is the allowlist denying a tool, where the fix is a Manifest; here the fix is to look at what the Drone was trying to write |

## Open questions

- **[drone-cannot-request-a-widening]** How does a Drone that needs a path outside the Job's scope ask for one? Filed as #299. **Three of the four things this had to settle are settled**: the Drone does not end on the call, it may ask once per step rather than twice for the same path, and it may not ask to narrow. What is left is the first half — how it asks. The tool takes free paths and free prose, which is what a Drone can produce and what a person can read, and nothing has measured whether a structured reason would be answered better.
- **[declined-widening-has-no-edge]** What does a Judge refusal leave a person to do? The Job escalates and the Drone is alive and idle, so redirect, restart the step and Pilot all reach it — but **none of them widens the scope**, which is the thing the Drone asked for. A person who agrees with the Drone and disagrees with the Judge has no act that says so: the override lifts `gate_failure` and `evidence_suspect` and this is neither, and the person's own widening route goes through the dispatch gate, which an escalated Job has no edge to. A candidate is making a person's widening legal from `escalated`; the pair it would form with the override is the argument for it.
- **[widening-and-the-judge-context]** Should a person's widening at the gate warn that a wider diff may exceed the Judge's context? A Convoy is where `max_context_size` bites first.
- **[scope-revision-on-a-stopped-job]** Is a scope revision legal on an escalated or piloted Job? No edge exists, so it is not offered, and whether that is deliberate is unknown. A Drone's own request cannot arise there — it is refused unless the Job is `running` — so what is open is the person's route, which is also the second half of the question above.

## Related

- [`../concepts/judge.md`](../concepts/judge.md) — the tier that answers a widening, and what a Judge call is and is not.
- [`../concepts/job.md`](../concepts/job.md) — the scope history field, the transitions, and the recovery acts this is measured against.
- [`../concepts/convoy.md`](../concepts/convoy.md) — the permission rules a widening resolves again.
- [`respond-to-a-push-alert.md`](respond-to-a-push-alert.md) — Redirect, the act that puts a person back on a Job and costs no respawn.
- [`take-over-a-job.md`](take-over-a-job.md) — the level test this reuses to place the act.
