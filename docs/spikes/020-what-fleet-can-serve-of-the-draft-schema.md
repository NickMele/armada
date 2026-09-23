# Spike 20 — What Fleet can serve of the draft schema, and what each field costs

**Read against `crates/ipc`, `core-model` and the Fleet runtime on 22 Sep 2026,
at protocol 18.0. Nothing here is locked and nothing here proposes a wire
change** — #1545 is split, and this is its first half.

**Most of the draft is additive and cheap. What is not is almost all one
problem: a closed set the registry owns, widened.** `Actor` is the expensive
one, because it is the log envelope's vocabulary and not only the wire's —
widening it writes spellings into `job_events` that an older Fleet's `from_wire`
refuses when `crates/store/src/fold.rs` folds the history back.

**One claim in the drafts is wrong and it is load-bearing.**
`packages/screens/src/draft/words.ts` says four of the five task states "are
rendered from the generated vocabulary as before". There is no `task_state`
vocabulary: `crates/core-model/domain/enum-verbs.toml` declares no table for it
and `packages/components/src/generated/vocabulary.ts` exports no `TASK_STATE`.
Every task state owes a registry row, not one.

A line number in this document sits outside the path it belongs to, because the
gate reads a path inside backticks and a path with a line on it resolves to
nothing.

## What was read

| | |
|---|---|
| Drafts | `packages/screens/src/draft/`, seventeen modules, 58 exported types |
| Wire | `crates/ipc/src/`, `crates/ipc/operations.toml`, `protocol-version.toml` at 18.0 |
| Domain | `crates/core-model/src/`, `crates/core-model/domain/` |
| Runtime | `crates/fleet/src/`, `crates/api/src/stream.rs`, `crates/config/settings.toml` |
| Decisions | #1530, #1545, #1581, and the twelve pull requests of 22 Sep 2026 |

## The count

| The exported draft types | How many |
|---|---|
| Never cross the wire — Bridge carriers and pad state | 9 |
| Served whole today, renamed or narrowed | 5 |
| Served in part, needing fields added | 17 |
| Nothing under them at all | 27 |

The ones that never cross are `JobDraft`, `DraftWord`, `LedgerReads`,
`LedgerFamily`, `GateReading`, `Declared`, `Drawing`, `SketchShape` and
`SketchJoin`.

## What moves the numbers

**Every change in the draft is minor except the rows in this table.** A minor
bump is additive-only, and `docs/practices/protocol.md` holds the table each row
is read off.

| The change | Why it is major |
|---|---|
| `Actor` gains `judge` and `check` | Strict closed set, and the store's own |
| `Actor::Human` spelled `person` | A renamed variant; recorded rows carry the old spelling |
| `TaskState` gains `failed` | The inner task machine grew a state |
| `JobStatus` gains `classifying` | Bridge picks a screen from this set |
| `JobResources.worktree` becomes a list | A field retyped |
| `Criterion.criterion_id` becomes optional | A required field made optional |

The first four are the same shape: a registry-owned set widened, and an arriving
spelling the registry does not have is refused rather than defaulted.
`crates/ipc/src/enums.rs` states that at the top of the file and the `wire_enum!`
deserializer implements it.

### `Actor` is the expensive one

`crates/core-model/src/envelope.rs` line 168 declares `human`, `fleet`, `drone`
and `helm`. `crates/ipc/src/history.rs` line 65 carries one on every `Recorded`
row.

**It is not the wire's set.** Line 180 of the same file says `job_events.actor`
and `scope_revisions[].approved_by` are both stored from it, and that `domain/`
has no row for it at all. So a fifth and sixth value are written into recorded
history.

| What a widening costs | Where |
|---|---|
| A stored spelling an older Fleet refuses | `Actor::from_wire`, `crates/core-model/src/envelope.rs` line 194 |
| A vocabulary the registry does not own | `crates/core-model/domain/enum-verbs.toml` has no actor table |
| A renamed variant across every stored row | `human` is in the database already |

`person` is the draft's spelling and `human` is the store's. **Rendering `human`
as "you" costs nothing and renaming it costs a migration** — the registry's verb
layer is where a spelling becomes a word, and `Actor` has nowhere to put one
yet.

`contributor` is the third addition, and `packages/screens/src/draft/ledger.ts`
already records that nothing derives one: it needs the store between Armada
instances that #1530 files separately.

### `failed` on a task

`crates/core-model/src/job/work_plan.rs` line 59 declares `open`, `working`,
`done` and `dropped`, and `crates/ipc/src/enums.rs` makes them a strict wire
set. Bridge's `stateOf` falls back to `open`, so Bridge survives a fifth; Rust
does not.

**`resumption`'s test is what decides it.** `docs/practices/protocol.md` says a
set is open where it grows because Fleet learned to read something, and strict
where it grows because a machine gained a state. A task that failed is the
second kind.

### `classifying`

`crates/core-model/domain/job-statuses.toml` declares neither `proposing` nor
`classifying`. `proposalViewOf` reads `detail.job.status`, which on a Job at its
gate is `awaiting_approval`.

**So the screens run on a status that already exists.** Whether a new one is
minted is a separate decision from whether the screen works, and the screen
works either way.

### Worktrees as a list

`crates/ipc/src/resources.rs` line 117 is `worktree: Option<WorktreeOnDisk>`.
Retyping it is major; adding `worktrees` beside it is minor and leaves one fact
with two spellings, which this repository has removed before.

### A criterion with no id

`crates/ipc/src/detail.rs` line 763 has `criterion_id` required.
`crates/ipc/src/job.rs` line 628 says why: the id is minted with the Job because
a Judge citation references a criterion by its frozen position.

`packages/screens/src/draft/criterion.ts` makes it optional for a criterion read
out of an issue before approval. **A criterion with no id is a criterion no
Judge can cite** — the id has to be minted at the moment the list freezes,
whichever moment that is.

## The drafts, one at a time

### `coord` — `RunCoord`

| | |
|---|---|
| Served | `step` from `StepDetail.step_id`, `step_attempt` from `StepAttempt.attempt` |
| New | `group`, `group_attempt`, `task` |
| Bump | Minor — three new optional fields |
| Cost | The group id, below |

`derivedGroupId` mints `g-` and the task id. `PlanTask.id` is documented as never
renumbered — `crates/ipc/src/work_plan.rs` line 34 — so the derived id is stable
**while one task is one group**.

**It stops being stable the moment a group holds two tasks.** A Record row
stored against `g-T4` then names a group that either no longer exists or no
longer means what it meant, and nothing in the row says which.

### `task` — `TaskView`

| | |
|---|---|
| Served | `id`, `title`, `note`, `scope`, `expects`, `shown`, `state`, `reason` — `crates/ipc/src/work_plan.rs` line 33 |
| New | `touched_after_done`, `group`, `concurrent_with`, `tier`, `model`, `treatment`, `drone_id`, `turns`, `cost_micros`, `failed_reason`, `cases`, `coord` |
| Bump | Minor for every field, major for `failed` |
| Cost | Per-task cost and turns need a per-task agent session |

`cost_micros` is stated as arriving when the task's own agent stops. `JobSpend`
totals a Job's Drones and a Job has one Drone at a time, so a per-task figure
exists only once a Drone per task does.

`model` is resolved from the tier map. `ResolvedStep.model`
(`crates/core-model/src/job/workflow.rs` line 344) is the nearest thing served:
a per-**step** model the workflow declares, absent meaning the Job's own stands.

### `group` — `GroupView`

| | |
|---|---|
| Served | Nothing. A group sits between a step and a task, and the wire has neither unit |
| New | Every field |
| Bump | Minor — a new DTO |
| Cost | The states, the timing, and the boundary |

`GroupState` has no registry table, and `crates/core-model/domain/step-states.toml`
declares none of `joining`, `checking` or `landed`.

**Nothing times a group and nothing can derive one.** `groupsTimedBy` reads the
first and last Record row at the group's coordinate and returns the group
untouched where they are equal. Every row inside one step attempt shares that
attempt's instant, so **today every group returns untimed**.

### `landing` — `LandingRule`

| | |
|---|---|
| Served | `base`, at `crates/ipc/src/configured.rs` line 24, mirrored at `crates/ipc/src/amending.rs` line 314 |
| New | `prs`, `branching`, `pr_mode`, `complete_when`, `land_together`, and `from_ref` apart from `target` |
| Bump | Minor — a new DTO |
| Cost | Three of four `complete_when` values answer nothing |

`COMPLETE_WHEN_SERVED` says only `delivered` is served, and that is right.
`JobDelivery.landed` says whether a pull request merged; nothing writes what the
repository's auto-merge policy decided onto the Job's record.

**`ManifestConfig` has no TypeScript mirror**, so the only `base` Bridge reads
today is the manifest editor's own. A landing control shipped before that gap
closed would draw a branch name off a read made for editing a file.

### `criterion` — `CriterionView`

| | |
|---|---|
| Served | `criterion_id`, `text`, `source` — `crates/ipc/src/detail.rs` line 762 |
| New | `origin`, `origin_moved_at`; `source` renamed `verified_by` |
| Bump | Minor for the new fields; major for the rename and the optional id |
| Cost | Freezing moves from creation to approval |

`AcceptanceCriterion` at `crates/core-model/src/job/fields.rs` line 465 reads
"Frozen at creation. `Job` offers no method that edits, reorders or removes
one." #1581 names the other documents that say it, and they are true of the code
today.

**The rename is avoidable and the freeze is not.** `verified_by` is a better
name for the same value than `source` and costs a major bump; keeping `source`
and adding `origin` beside it costs nothing.

### `ledger` — `LedgerRow`

| | |
|---|---|
| Served | `at`, `seq`, `actor` and three `Movement` shapes — `crates/ipc/src/history.rs` line 53 |
| New | `coord` replacing `step_id`, plus `kind`, `what`, `outcome`, and three actors |
| Bump | Major, on the actors alone |
| Cost | Seven families composed from four reads |

`get_job_events` serves three kinds of row and the Record draws seven. The rest
— a Check run, a Judge verdict, a file written, a case run — are composed
client-side from `get_job`, `get_evidence` and two events.

`kind` stays an opaque string and that is right: nothing branches on it, and a
surface renders the spelling where it has no word. That is
`FleetCapacity.held_by`'s argument, and it holds only while nothing branches.

### `cases` — `CaseView`, `CaseRunView`

| | |
|---|---|
| Served | `NamedSpec` at `crates/ipc/src/showing.rs` line 167, `ShownSet` at line 184 |
| New | `covers`, `tasks`, `groups`, `state`, `dropped_by`, and most of a run's fields |
| Bump | Minor — new DTOs and new fields |
| Cost | Coverage is unbuilt, and a case has no id |

`caseIdOf` composes an id from the step, the attempt and the path. **It changes
when a later attempt names a different path**, which is the derived group id's
defect one scope down.

`covers` is empty for every case: #1274's `COVERS` file is parked, and nothing
reads one.

### `proposal` — `ProposalView`, `GateView`

| | |
|---|---|
| Served | `advance_gate`, `checks`, `judge_checks`, `overridden` — `crates/ipc/src/detail/step.rs` lines 174, 142, 165, 218 |
| New | `tiers`, `drone_cap`, `fleet_always_looks`, `from_ref`, `pr_mode`, `approved_at` |
| Bump | Minor — every addition is a new field or a new DTO |
| Cost | A per-Job override reverses a stated decision |

**The four gate states are already on the wire.** `AdvanceGate`
(`crates/core-model/src/job/declared.rs` line 54) carries `auto`,
`auto_if_judge_passes`, `human_always` and two `manifest_rule:` keys, and
`gateViewOf` reads all of them with no wire change.

What has no meaning is a tick on a step that declares nothing.
`mechanical_checks[]` and `judge_checks[]` are the workflow's and freeze with
the Job, so asking for Checks on a step that declares none runs nothing — and
reads on the wire exactly like a step that does. `unmeantOf` is Bridge saying so.

**A per-Job override of `review_gate` reverses a decision with reasoning on it.**
`crates/core-model/src/job/declared.rs` line 79 freezes that variant unresolved
because both policies are `Live` in `crates/config/settings.toml`, so a Manifest
saved mid-Job moves them. Nothing says which wins if the repository's policy
moves after the press.

### `members` — `MemberView`

| | |
|---|---|
| Served | `dispatched_by`, `status`, `branch`, `tasks`, `landed` — `crates/ipc/src/job.rs` lines 209, 258, 276 |
| New | `link`, `landed_at`, `dropped`; `scope`, `pull_request` and `question` need each member's own read |
| Bump | Minor |
| Cost | No Fleet operation closes a pull request or stacks a branch |

`landed` reading the merge and not `completed_success` is served today:
`Settled` (`crates/ipc/src/detail.rs` line 959) is `merged` or
`closed_unmerged`, and `JobDelivery.landed` carries it.

**`landed_at` is absent for every member and cannot be derived.** The wire says
whether a pull request settled and never when; `ended_at` is a different
instant.

`crates/ipc/operations.toml` has `merge_pull_request` and `delete_branch` and
nothing that closes a pull request. Fleet rebases a branch onto its **base** —
`crates/fleet/src/noticing.rs` writes `Currency` from that — and never onto
another Job's branch. So `dropped` and `stacked` both name acts Fleet cannot
perform.

### `peers` — `PeerOverlapView`

| | |
|---|---|
| Served | `ScopeOverlap` at `crates/ipc/src/overlap.rs` line 24, hung off `JobDetail.write_scope_overlaps` |
| New | Nothing in the shape. **When it can be asked** is the whole change |
| Bump | Minor — a new query, or a field on the propose answer |
| Cost | Nothing declares a path at dispatch |

`propose_from_request` says plainly that it does not propose `write_targets`,
and that a Job reaches the gate with them null — the first step's Drone declares
scope after reading the code.

**So an overlap at dispatch is a comparison against paths a person typed**,
which is a different fact from the one `write_scope_overlaps` carries. The
`null`-versus-empty distinction is served, and the draft mirrors it exactly.

### `pulse` — `PulseView`

| | |
|---|---|
| Served | `JobResources` at `crates/ipc/src/resources.rs` line 104 — `read_at`, `held`, `processes`, one `worktree`, `wrote_last_at` |
| New | `logs`, `PulseProcess.owner`, `worktree` as a list |
| Bump | Minor for `owner` and `logs`; major for the list |
| Cost | Nothing measures a log or says whether one is being written |

`wrote_last_at` is when the Job's own log was last written. Neither its size nor
whether something is writing to it right now is served, and `pulseViewOf`
derives one row with `writing: false` and no size.

`owner` is honest today and not free tomorrow: `null` must mean a process Fleet
cannot place, and with several worktrees that becomes a real answer rather than
a case that never occurs.

### `sketch` — `SketchAttachment`

| | |
|---|---|
| Served | `StagedFrame` at `crates/ipc/src/capturing.rs` line 55; `said` and `produced_by` on `CaptureStudioNote`, lines 126 and 130 |
| New | Nothing that crosses. `drawn` is Bridge's half and stays there |
| Bump | Minor, and possibly none |
| Cost | Reopening a flattened picture |

**Staging a PNG at dispatch already works.** `JobRequest.attachments`
(`crates/ipc/src/job.rs` line 553) takes an `AttachmentRef` of a staged path, a
filename and a mime type, on the same shared-filesystem assumption `StagedFrame`
makes. A sketch attached to a prompt needs no new field.

What it does not carry is `produced_by` and `said` — which is what separates a
sketch from any other attached file — and `drawn`, which is what a person needs
to reopen their own boxes rather than an image of them.

### `wave` — `WaveView`

| | |
|---|---|
| Served | `dispatched_by`, and every Job's own status, delivery, held command and Judge question |
| New | `waits_on`, `round`, `WaveRoundView` |
| Bump | Minor |
| Cost | The order exists and is thrown away |

`.armada/workflows/epic.json` asks its plan Judge whether the plan draws "an
edge wherever one of them has to wait for another". That edge is written into
`plan.md` and never reaches the Board.

**`dispatched_by` cannot tell a wave from a landing order**, because it is a set
with no order in it. `waveOf` returns every `waits_on` empty and the graph draws
one column, which is honest and is not the picture.

### `held` — `JobDraft`

Not a wire shape, and it names no `crates/ipc` module. `xtask`'s
`nothing_in_the_main_process_reads_the_draft_schema` keeps it out of the process
that talks to Fleet.

### `words` — `DraftWord`

| | |
|---|---|
| Served | The vocabularies in `crates/core-model/domain/enum-verbs.toml` |
| New | The draft vocabularies in `DRAFT_VOCABULARIES` |
| Bump | None on the wire. A registry change and a codegen run |
| Cost | Larger than the file states |

The file says the wire's task states are rendered from the generated vocabulary
"as before". **They are not rendered from it at all** — there is no `task_state`
table in `enum-verbs.toml` and no `TASK_STATE` export in
`packages/components/src/generated/vocabulary.ts`.

So the registry owes a row for every task state, every group state, both case
vocabularies and the criterion reading — and each owes a glyph decision under
`docs/contracts/iconography.md`, which defaults to none.

### `dispatch` — `DispatchSettingsView`

| | |
|---|---|
| Served | `JobRequest` at `crates/ipc/src/job.rs` line 553 — `request`, `client_ref`, `attachments` |
| New | `workflow_id`, `tiers`, `drone_cap`, `lands` |
| Bump | Minor — additive, exactly as `attachments` was |
| Cost | One of the four contradicts the operation's stated purpose |

`JobRequest`'s own doc comment reads: "One field, and no `workflow_id` among
them. Naming the workflow is the act this operation exists to remove; a request
that carried one would be `propose_job` with an extra model call in front of
it."

**A person overriding the proposer's read is a different act from naming a
workflow up front**, and the field is optional. The decision is whether that
distinction survives the field existing.

### `revision` — `PlanRevisionView`

| | |
|---|---|
| Served | `StepDetail.attempts` and `StepDetail.judged`; #1575 landed the acts with nothing new on the wire |
| New | `PlanAskKind`, `PlanAnswer`, `PlanRevisionView`; `PlanRefusal` is `Judged`'s own fields |
| Bump | Minor |
| Cost | One ask per run of the step, and the record cannot hold two |

`attemptOf` pairs a revision with a later run of the plan step. **A Drone asked
twice inside one run is two asks the record cannot tell apart**, which the file
says itself.

## What this does to the event stream

**Worse, and by a stated amount.** `crates/api/src/stream.rs` sets `BACKLOG` to
256 on one drop-oldest broadcast carrying every Job, and a consumer that falls
off gets `Missed` and a full resync of every Job.

| What multiplies | By how much |
|---|---|
| `job.plan_changed` per Job | One per task transition, where today it is one per plan change |
| Producers per Job | One per task under a Drone per task, where today it is one |
| Streams per open Job | A second one, where Pulse's logs are live |

The mechanism the risk asks for is built: a drop is a message carrying the
count, not a silence. **What nobody has is the number** — how many events a
fleet of per-task Drones produces against a minimised Bridge, and whether 256
absorbs it.

`observe_job` is the shape to copy for anything at Drone speed. It is a per-Job
subscription for exactly this reason: transcript rows on `/events` would evict
the state changes the Board is drawn from.

## The questions

Each of these is written for the owner to answer in plain language. None of them
is decided here.

### 1. A group's name, after somebody moves a task

You approve a plan whose second group holds one task — *rewrite the token
loader*. Armada calls that group by the task's own name, because there is
nothing else to call it by. The Record fills up with rows about it.

Then you add a second task to that group, or split it in two. Every row already
written names a group that is now something else, and nothing in the row says
so.

**Option A**: Fleet gives every group a name of its own the moment the plan is
recorded, so the name survives anything you do to the group afterwards. It costs
a value Fleet has to mint and store.

**Option B**: the name stays the task's, and a group you change starts a new
name. The old rows stay readable and stop being reachable from the group on
screen.

### 2. Whether a Judge and a Check sign the Record in their own names

You open the Record on a Job that failed its gate. Every line about a Check
running and every line about a Judge answering says Armada did it, because those
are the only two words the record has for anything that is not you or an agent.

You decided they get their own names. **That is the most expensive single change
in this reading**, and not because of the Record.

The list of who-did-it is not only on screen. It is written into the database on
every row Armada has ever recorded, and it is shared with the note saying who
approved a change of scope. Adding two names means older copies of Armada cannot
read history that newer ones wrote — **and a Job may be mid-run when that
happens.**

**Option A**: do it, and treat the day it lands as a version everybody moves past
together, with no Job running across it.

**Option B**: leave the stored list alone and let the Record work out who ran a
line from which list it came out of. That is what Bridge does today, and it is
right every time.

### 3. Whether a criterion knows which tests cover it

You are reading a Job held to four things. One of them is *the token refresh path
is covered*. Armada can tell you which test files exist and which ran; it cannot
tell you that any of them is the one answering that line.

Nothing in the repository says which test covers which requirement. The file that
would say it is parked and unbuilt (#1274).

**Option A**: coverage stays between a test file and a source file, and the
criterion says only whether a person or the Judge answered it.

**Option B**: a criterion carries the tests that cover it, which means somebody
writes that down per criterion and keeps it current.

### 4. Whether a group has a clock

A Job spends two hours implementing. You want to know the first group took eleven
minutes and the third took ninety.

**Nothing times a group today, and nothing can work it out.** Every line Armada
records inside one run of a step carries the same instant, so the first and last
line of a group are the same moment and the group reads as taking no time at all.

**Option A**: Fleet stamps a group when it starts and when it ends. Two values,
and they are only ever true once groups exist.

**Option B**: the screen says a group is not timed rather than drawing a dash or
a zero, which is what the boards do now.

### 5. What a tick means on a step that has nothing to check

On the approval screen each step offers three ticks — run its Checks, let the
Judge look, stop for me — and a fourth state where the repository decides.

Tick "run its Checks" on a step whose workflow declares none and **nothing runs**.
The step then looks, to Armada, exactly like one that ran its Checks and passed.
The screens say so in words today.

There is a second half. "The repository decides" is deliberately left unanswered
until the moment it is read, because the repository's rule can change while your
Job is running. If you override it for one Job at the approval press and the
repository's rule moves an hour later, **nothing says which of the two wins.**

**Option A**: a step with nothing declared cannot be ticked, and an override wins
for the life of the Job.

**Option B**: the ticks stay offered and the screen keeps saying what has no
effect; the repository's rule wins whenever it moves.

### 6. A model per difficulty, when a model is free text

You want the hard task on the strong model and the three easy ones on the cheap
one. The planner marks each task hard, medium or easy, and a map on the Job says
which model each of those runs on.

**A model name is a string everywhere in Armada, on purpose** — naming the set
would put a vendor's list on the wire, and that list changes without us. So the
map is three strings, and a typo in one of them is a Drone that will not start.

Something related is already built: a workflow step can name its own model, and
absent means the Job's stands. A tier map is a third place a model is chosen.

**Option A**: three tiers, three strings, checked against the models Armada can
list at the moment you press approve.

**Option B**: the tier is what you set, and Armada resolves it to a model from
configuration, so no model name is ever typed on this screen.

### 7. How many agents one Job may run, and what the machine's number counts

The machine setting reads "Concurrency cap: how many Drones at once", between 1
and 8. **It does not count Drones. It counts Jobs** — `fleet::slots` keeps one
slot per Job and asks whether the number of Jobs being worked is under the cap.
The two have been the same number because a Job runs one agent.

A Drone per task breaks that. Set the machine to 4 today and you get four agents;
set it to 4 after, and one Job running six tasks is already over.

**Option A**: recount the machine's number in Drones. Everybody's existing
setting suddenly means something different, and a machine set to 4 runs fewer
Jobs than it did yesterday.

**Option B**: keep the machine counting Jobs and give each Job its own cap on
agents, so the two numbers multiply. Nothing then stops a person setting 4 and 8
and getting thirty-two agents.

### 8. Reading what a Job is held to out of the issue it links

You dispatch against issue 812. Armada reads its acceptance criteria out of the
issue and shows them, and you correct one before approving.

Two things follow. **The list is fixed the moment you press approve**, not the
moment the Job is created — which is the opposite of what Armada does today, and
#1581 tracks the documents that still say the old thing.

And the issue can move afterwards. Somebody edits 812 two hours in. The Job keeps
the words it froze and the screen says the issue has changed since; it does not
re-read it, because the Judge would then be grading against a list something
invented later.

**The question is what noticing costs.** Armada would have to ask the forge
whether the issue has been edited, and there is nowhere it does that today — it
asks the forge about open pull requests on a rotation, and about nothing else.

**Option A**: notice it on the same rotation that watches pull requests. One more
forge call per open Job.

**Option B**: notice it when you open the Job, and not otherwise.

### 9. Who else is writing there, before there is anything to compare

You type a request on the dispatch screen. Before you press anything, you want to
know that another Job is already writing in the files this one will touch.

**Armada does not know what this one will touch.** The whole point of the
dispatch path is that you describe the work and Armada reads it; which files it
touches is decided by the first agent, after it has read the code. A Job reaches
the approval gate with that question unanswered on purpose.

So the panel can only compare against paths **you** typed.

**Option A**: an optional field on the dispatch form — name the areas you expect
this to touch — and the panel answers against that.

**Option B**: the panel waits until the plan exists, and the dispatch screen says
nothing about overlap.

### 10. Live logs on the Pulse board

Pulse says what a Job is holding on the machine right now. You want the logs in
it: what each one is, what it weighs, and which are still being written.

Armada records one thing about a Job's log — **when it was last written to**. Not
how big it is, and not whether something has it open this second.

**Option A**: Fleet measures each log on the same reading that walks the
processes. One more `stat` per log, on a panel that is already the expensive
read.

**Option B**: the panel names each log and says its size is not measured, which
is what the board does now.

### 11. Reopening a sketch you drew

You sketch four boxes beside a prompt and dispatch. The agent gets a picture of
it, which is right.

Two days later you come back to that Job to dispatch a follow-up, and you want
your boxes back — to move one and add a fifth. **A picture cannot be edited back
into boxes.**

Attaching the picture costs nothing new: Armada already takes files staged beside
a request. Keeping the boxes is a second thing to store, and nothing but Bridge
would ever read it.

**Option A**: Bridge keeps what you drew, on this machine, beside the Job.
Nothing crosses to Fleet, and a different machine sees only the picture.

**Option B**: the sketch is a picture once it is sent, and coming back means
drawing it again.

### 12. When a Job under another one counts as done

Three Jobs land one change, in order. The parent is finished when all three have
landed — and **landed means somebody merged the pull request**, not that the Job
reached a successful status. A Job whose gate hands off to a person is successful
while its pull request sits unread.

That part is served and works. Two things around it are not.

**Armada can say whether a pull request merged and not when.** So a page showing
three members landing in order cannot draw the order they landed in.

**And Armada has no way to close a pull request, or to keep a stacked branch
current.** It can merge one and delete a branch. So dropping a member from the
set, and branching one member off another's branch, are both acts a person
performs on the forge with Armada watching.

**Option A**: Armada records the moment it notices a merge and the page draws
that. It is the instant Armada found out, not the instant it happened.

**Option B**: no instant, and the page draws the order the plan gave rather than
the order they landed.

### 13. Drawing a wave only where something says a Job is one

An Epic dispatches five Jobs. Two can start at once and three wait on the first.
You want to see that.

Armada records that each of the five was dispatched by the parent, and nothing
else about how they relate. **The waiting is written down** — the Epic's workflow
explicitly asks its planner for an edge wherever one Job has to wait for another
— and it goes into a file the Board never reads.

Without it the five draw as one column, which is not wrong and is not the
picture.

**Option A**: the planner's edges are recorded on the Jobs when the wave is
dispatched, so the graph draws what the plan said.

**Option B**: the graph draws what Armada can enforce — the dependency edges it
already holds — and a wave with none draws a column.

### 14. What a Drone per task does to the live stream

Every open Bridge window is fed by one stream carrying every Job. It holds 256
messages. A window that falls behind is told how many it missed and redraws
everything, which is correct and is not free.

One agent per task multiplies how fast one Job talks by the number of tasks it is
running. Pulse's live logs would be a second thing talking. **Nobody has ever
measured what the current fleet produces**, so nobody can say whether 256 is
generous or already tight.

**Option A**: measure it first — one real Job with six tasks against a minimised
Bridge — and size the stream from that before per-task events are built.

**Option B**: build per-task events on a stream of their own, per Job, the way a
Drone's transcript already is, and leave the shared stream carrying state changes
alone.

## What this spike does not say

It locks nothing, proposes no wire change, and names no version. The feedback
pass over the boards is expected to move some of these shapes, and a lock written
before it would be a second thing to reconcile.

**A negative result is a result.** The unserved fields above are as much the
finding as the served ones.
