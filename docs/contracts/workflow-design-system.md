# Workflow Design System

**Kind:** spec. **Governs:** how a workflow is structured, how evidence is
verified, how Drone behaviour is policed, and what escalates. Judge owns
the semantic tier and Drone the runtime; this document owns the definition
a Job runs against.

---

# Core Principle

Self-report is a signal, not a source of truth — at every layer. This
applies not just to "did the Drone finish the step" but to "did the Drone
report correctly at all," "is the evidence real," and "is the Drone even
still working productively." The whole system below exists to enforce that
principle mechanically wherever possible, and cheaply (via a small Judge
model) everywhere else, reserving human judgment for the calls that
actually need it.

---

# Workflow Schema

Every workflow is a `WorkflowDef`: an ordered (or loop-capable) set of
steps.

**The resolved WorkflowDef is frozen into the Job at creation** — the same
treatment `acceptance_criteria[]` gets, and for the same reason: the
yardstick must not move under the work. A Manifest may shadow a built-in
workflow, so `workflow_id` alone does not identify what was run. See the
open question on what a Job records about the workflow it runs, in Armada
Decisions.

Field-by-field detail, nested the way the schema nests — the top-level
fields are the `WorkflowDef`'s own, and everything under `steps[]` is a
field of a step — lives in the WorkflowDef Fields database, one row per
field with the reasoning behind it. That table is not reproduced here.

**Not all workflows are linear.** Coding workflows (Feature, Bug, Refactor)
are a fixed sequence. Planning/exploratory workflows (Design Plan,
Investigation) are draft↔feedback loops that repeat until converged or
capped.

**A workflow's one outbound edge creates a Job and never dispatches one.**
`on_fail` and `verdict_routing` jump between steps inside a definition;
`on_complete` names a `workflow_id` to follow this one. Reaching
`completed_success` creates that Job at `awaiting_approval`, wired back
through `subject` and carrying the same owning Manifest — and stops there.
The ordinary one-by-one dispatch gate stands in front of it like any other
Job.

**The split is the classifier's, reused.** Approving a proposed plan
already creates Jobs without dispatching them, so a definition naming a
successor is the same act with a different author. That is what keeps the
primary control on Fleet's autonomy intact, and it is also why a workflow
triggering a workflow triggering the first cannot run away: every hop
waits for a person. Nothing else would have caught it — the fan-out cap
counts Jobs *inside* a Job, and these are peers.

**Only `completed_success` fires it.** What follows a failed Job is
escalation, not more work.

**A successor is not a sub-dispatch.** It carries no `dispatched_by` — the
Job that named it is finished and there is no live Drone to attribute it
to — so it does not consume the fan-out cap, and Job's `origin` carries
`workflow_triggered` rather than `sub_dispatched`.

**It is a completion edge, not an event hook.** A Job that lands cleanly
and breaks main an hour later is not a workflow completing; that trigger
is a different mechanism and belongs to the open question on what happens
when a completed Job's merge breaks main, in Armada Decisions.

**Declared File Plan (continuous drift check):** For steps with
`scope_diff_check: true`, the Drone can declare `context_paths` at step
*start* (`declare_plan_at: step_start`), not only at evidence-submission
time. This lets Fleet mechanically compare live edits against the declared
plan throughout the step, not just at the end. A live mismatch doesn't
auto-fail — legitimate investigation sometimes reveals the real work is in
a file outside the original plan — it instead tags the step for a
mandatory Judge look. See Thrashing / off-rails below: this turns the
mid-step Judge pass from a fixed-schedule check into a conditional one,
triggered by a cheap mechanical signal rather than run regardless.

---

# The Workflows

## How a Job gets one

**You do not have to pick.** A Job can be dispatched from a prompt or a
link to a ticket, and the Job proposer reads that request and proposes
which workflow the work should run under, alongside its scope. Picking one
by hand stays available and is the override, not the path. That page owns
the call; what belongs here is what it means for a workflow.

**What is proposed is not what is stored.** The resolved `WorkflowDef` is
frozen into the Job at creation, as below. The proposal chooses which one;
freezing is what stops it moving afterwards — so a workflow edited in the
repo between the proposal and the dispatch reaches no Job already created
against it.

Per-workflow detail — the instantiated `WorkflowDef` where a sample
exists, and the shape, gate profile and status where one does not; policy
variants and workflows still needing a step list included, so there is one
list rather than three — lives in the Armada Workflows database and is not
reproduced here.

**Feature is the pattern other coding workflows follow; Bug is the sample
to copy from.** Design Plan is the only loop; Prototype is the only
workflow with gateless steps; Refactor is the one that exercises the
gaming-check path end to end and demonstrates the no-parsing rule — a
Manifest Check exit code is its entire mechanical tier, and every question
about what the tests now cover is answered by a Judge reading the diff.

A Job instance references `workflow_id` and carries its own `status`,
`current_step`, `retry_count` and `iteration_count` per step,
`acceptance_criteria[]` (frozen at Job creation), `facts[]`
(append-only), and `escalations[]` (using the trigger taxonomy below) — a
`WorkflowDef` is the template; the Job is the running instance against it.
Those counters live in `job_steps`, one row per step, on Job.

---

# Evidence Scope Object

Attached to every step that triggers a Judge or a human gate. Prevents the
Judge (or the human at the gate) from having to search the whole
codebase — it is told exactly where to look.

**Definition vs. resolved object.** A `WorkflowDef` step carries the
*policy* — `context_source`, `max_context_size`, `exclude_paths`, the
flags. It cannot carry `context_paths`, because at definition time nobody
knows them: that is precisely what `context_source: drone_declared` means.
The Drone supplies them at evidence-report time, Fleet validates them, and
the **resolved** object handed to the Judge is the one on which
`context_paths` is required. Where a field row below says a field is
required, it means required on the resolved object.

**The diff is delivered, not fetched.** The resolved object carries the
**actual patch** for the files in scope, pre-loaded into the Judge's
context — not a list of filenames the Judge then has to go and read. The
veto-only contract already says the Judge receives the task text, the
**work product** and the deterministic facts; on a coding step the diff
*is* the work product, and this states that it is handed over rather than
looked up.

Three reasons, in order of weight:

1. **It is what makes a panel a panel.** `judge_check.panel_size > 1`
   requires each Judge to run "independently against identical inputs." A
   Judge that chooses what to read does not have identical inputs — three
   Judges could read three different subsets and their unanimity would
   mean less than it appears to. Pre-loading is what makes the guarantee
   true.
2. **The evidence trail records what was actually judged.** A refusal has
   to cite the specific evidence it is refusing on. If the Judge fetched
   its own context, the exact bytes it saw are not in the record, and
   neither the Drone retrying against the refusal nor the human reading it
   can reconstruct what it looked at.
3. **Cost and latency.** Fetching burns turns on a call measured at ~4k
   input tokens.

`context_paths` is then the narrower thing: **what else** the Judge may
open beyond the diff — the unchanged remainder of a touched file, a
sibling module, an interface being conformed to. A hunk alone often cannot
answer "is this test still meaningful"; the rest of the file can.

**Two limits worth stating.** A large diff can exceed `max_context_size`
and fails closed there like anything else, which is the correct behaviour
and the reason that cap exists. And on **Code Review** the diff is the
Drone's *input* rather than its output — "the work product" needs
restating for that workflow, and it is one of the reasons the gate row is
the component that bends there.

## What the Judge receives, per `evidence_type`

The veto-only contract says the Judge receives the task text, the **work
product** and the deterministic facts. On a coding step the work product
is the diff. On several of Bug's steps it is not, and "only the diff"
would leave nothing to judge there — hence this table.

**Drone-authored evidence is admissible; the transcript is not.** These
are different things and the distinction is what keeps `facts_note` from
reading as a loophole. **The transcript is the Drone's account of its own
process** — why it struggled, what it tried — testimony about itself. **A
facts note is a deliverable submitted as evidence** and judged as one: the
Judge asks *is this a sound root-cause analysis*, never *did the Drone
work hard*. In one line: **the Judge sees what the step produced. It never
sees how the step went.**

| `evidence_type` | Work product handed to the Judge | What `context_paths` is for on that step |
| --- | --- | --- |
| `diff` | The actual patch, pre-loaded | **What else** may be opened — the unchanged remainder of a touched file, a sibling module, an interface being conformed to |
| `failing_test` | The patch adding the test, plus the run's exit code and status as deterministic facts | As for `diff`. Bug's `repro` asks whether the test represents the reported bug, which needs the code under test |
| `facts_note` | The note itself — Drone-authored, judged as a deliverable | **The whole payload.** There is no diff, so the Judge reads the note plus whatever the note is about |
| `test_suite_run` | The exit code as a deterministic fact, plus the diff being verified | As for `diff`. Note the Judge question here is about the **diff**, not the run — the run is already answered by its exit code |
| `bundle` | The accumulated evidence of prior steps, assembled. Bug's `review` is advisory (`gates_advancement: false`) and summarises rather than refuses | **The whole payload**, alongside `reference_docs` pointing at prior steps' evidence |
| Document or mockup (Design Plan) | The document | **The whole payload** |
| Absent (Bug's `merge`) | Nothing — no Judge runs | n/a |

**Code Review inverts and is the exception the table cannot fold in.** The
diff is the Drone's *input*; the work product is the review it wrote.
`files_read` is what verifies the input was actually opened.

Its six fields are entries under `evidence_scope` in the WorkflowDef
Fields database — each with its type, whether it is required, and the
argument behind it.

**Resolution order:** Manifest default for step type → Drone declares
specifics at evidence-report time → Fleet validates against allowlist and
(if enabled) the real diff → **for loop workflows, the most recent
iteration's work product plus every prior verdict**.

**Only the latest work product is carried, and all of the feedback.**
Appending every prior iteration against a fixed cap breaches it by
construction, and since exceeding `max_context_size` escalates, an
unbounded append would make every loop workflow generate a guaranteed
false escalation — worse than failing, because it trains the engineer to
ignore that escalation. The split works because the two things are
different objects: drafts, documents and mockups are large, while a
`request_changes` verdict is a sentence or two. Keeping all the verdicts
is what shows the same note went unaddressed three times, which is the
judgement `iteration_cap` exists to force; carrying only the latest
iteration entire would make each round read as a fresh note. Rejected: a
rolling summary, which adds a model call per iteration and puts a lossy
step inside the evidence path; and a larger cap or exemption for loops,
which defeats the cap on the workflows most likely to need it. See the
open question on how a loop workflow's appended prior-iteration context
stays under `max_context_size`, in Armada Decisions.

---

# Drone Compliance & Policing

Prototype learning: Drones don't reliably follow instructions or use the
tools they're given — they'll report status in plain text, wander
off-task, or technically satisfy a check while gaming it. Five distinct
failure modes, each needs its own detection mechanism:

| Failure Mode | Symptom | Detection | Response |
| --- | --- | --- | --- |
| Silence / Stalled | No activity for X time | Heartbeat/liveness timer per Job — threshold is a config row (see [Configuration](configuration.md)), distinct from Fleet's own resource-poll interval. The clock runs only while the Job is `running`, and what bounds an indefinitely-open gate instead — see Drone, which owns the decision | Poke (nudge prompt) via `poke_limit`. N pokes with no response → escalate as `stalled`. **A Drone that exits cleanly having called nothing at all is `silent` instead** — different trigger, different human response, and redispatching a silent Drone with the same brief reproduces the same silence. `poke_limit` is a liveness check (nothing arrived at all) and is a distinct counter from Drone's evidence clarification-round cap, which is a content-sufficiency check. Both can end in the same `stalled` label. |
| Claims done, no evidence artifact | "I'm done" but nothing to check | Mechanical check naturally fails (no file/diff/output exists) | Standard gate-failure retry flow |
| Claims done in plain text, bypasses structured report | Doesn't use the evidence-reporting mechanism at all | Fleet's parser finds no valid evidence submission for the turn — treated as nothing happened, not as done or failed. Largely superseded by the dedicated Evidence MCP tool — see Drone. **Note the harness envelope cannot be trusted to tell you this**: a run denied every tool it needed still terminated reporting success with exit code 0. See the open question on what Fleet accepts as proof a Drone did the work, in Armada Decisions | Same reprompt-then-poke flow as silence. Insufficient (but present) evidence gets a free clarification round first (capped, see Drone) before this flow kicks in |
| Thrashing / off-rails | Actively working, but not converging — fixated on a tangent instead of reporting stuck | Turn count vs. step norm, wall-clock vs. expected ceiling, and (for steps using `declare_plan_at: step_start`) a mechanical file-plan-vs-live-diff check — all mechanical, no LLM judgment needed to detect. An off-plan edit alone doesn't auto-fail; it tags the step for a mandatory Judge look rather than blocking outright | A **mid-step Judge** pass runs only when a mechanical trigger fires (plan drift flagged, or turn count/wall-clock exceeds norm) — not on a fixed schedule regardless of activity. **It does not read the Drone's turns.** It judges the *work product so far* against the declared scope and the step's intent: "is what has been produced converging on the task, is it justified drift, or is it thrashing?" If thrashing, force-interrupt with a "stop, report current state now" directive. If that also fails → escalate as `thrashing`. Reading the Drone's last N turns would be the stronger detector and is ruled out by the veto-only input contract. **The detector is weaker for it, and that is accepted** — it can no longer separate justified drift from thrashing by reading the Drone's stated intent, and must infer it from what was actually produced |
| Evidence gaming | Evidence is technically true but substantively worthless (weakened assertion, narrowed test scope, tautological test) — mechanical AND naive Judge checks can both pass | Judge explicitly prompted to check for gaming patterns via `judge_check.gaming_check`; baseline comparison against an earlier step's evidence, named by `baseline_ref`. `flag_if` names five patterns: scope narrowed, assertions weakened, tests skipped, tests deleted, and **a change to the configuration a Check's command resolves through** — `package.json` `scripts.test`, a Makefile target. The first four are all phrased in terms of *test code*, so editing the script a frozen `run: "pnpm test"` resolves to honours the frozen string exactly while narrowing the gate, and fell outside all of them. Freezing harder is not available: the no-parsing rule refuses to model what a tool command expands to. See the open question on what catches a Check weakened by editing the config its command resolves through, in Armada Decisions | Routes as its own `evidence_suspect` trigger, not gate-failure — resubmission under the same instructions likely reproduces the same gaming |

**Structural note on enforcement:** the most reliable fix isn't policing
after the fact — it's forcing structured output at the harness level, so a
non-compliant response simply doesn't parse as a valid turn. A dedicated
Evidence MCP tool is the one sanctioned way a Drone reports completion —
see Drone, which owns it and the clarification-round cap. Headless output
parses reliably and denials are visible three separate ways, both closed
by measurement, so this no longer rests on an unanswered spike. Of the
detectors above, **thrashing and off-scope work are computable from the
stream** — repetition and paths touched are typed events, needing no prose
parsing. **Silence is the one that does not fall out for free**, since
only assistant and user events carry a timestamp. What stays open is the
judgement layer rather than the data: what threshold of repetition counts
as thrashing, and what counts as off-scope. See the open question on what
policing signals and denial events headless output actually exposes, in
Armada Decisions.

---

# Dispatch Approval — Two Levels

| Dispatch Type | Approval Needed? | Rationale |
| --- | --- | --- |
| Job-level (Drone picks up a Job off the Board) | Human, always | Primary autonomy control — you decide what gets worked on. Strictly one by one; Fleet owns the rule |
| Sub-dispatch (a Drone spawns a **Job** needed to complete its assigned Job) | No — auto | Already inside an approved Job; blocking here is approval-fatigue with no real decision content. **The fan-out cap counts Jobs only** — a Judge call is not a sub-dispatch, which is what makes "2 of 5" a legible number. Judge owns why; Job counts `spawned_jobs` and `judge_calls` separately |
| Sub-dispatch, abuse pattern (fan-out beyond threshold) | Escalate to Inbox | Volume itself is the signal, not any single dispatch |
| Successor (a `WorkflowDef`'s `on_complete` names the next workflow) | Human, always — it is a Job-level dispatch | The parent is finished, so the sub-dispatch rationale does not reach it: there is no approved Job for this one to sit inside. The definition creates the Job; it does not start it, which is the same plan-versus-dispatch split the classifier's proposal already uses |

Refines the original "every Drone dispatch requires human approval"
decision — that rule applies at the Job level; work a Drone generates in
service of its own approved Job does not require separate approval.

---

# Escalation Taxonomy

One Inbox surface, `snake_case` triggers. **The Escalation Triggers
database is the canonical set** — Judge, Drone, Fleet and Job all cite
individual triggers, so each is a row they can point at. That table is not
reproduced here.

Every name states the condition, not the mechanism. A **sub-kind** pauses
the Job exactly as its parent trigger does and differs only in the
suggested action on the payload; `silent` is the only one. Routing for
`evidence_suspect` and `thrashing` is Judge's, and `stalled` is Drone's.
Decision history: see the open question on the complete escalation trigger
enum, in Armada Decisions.

The **In code** column is the migration list: what it marks *Not yet* is
not yet a variant in `core-model`'s Rust enum, which is the only home —
`Job::transition` is the sole mutator, and the TypeScript side is
generated from `ipc` rather than authored. Nothing is removed, and one
migration window covers all of them.

**Escalation payload:** Job + step where it tripped, trigger type (and
sub-kind where one applies), full attempt/dispatch history (all prior
Judge summaries, not just the latest), suggested actions (kill / rephrase
& redispatch / manual override). **The suggested action is where a
sub-kind earns its keep** — `stalled` and `stalled(silent)` pause the Job
identically and differ only in what the payload recommends.

**A repeat failure escalates identically to a first one.** It differs only
in the payload and its suggested action, which is the same shape as a
sub-kind. Escalation already halts autonomous action, so a first failure
has already reached the engineer and there is nothing for a repeat to
escalate harder into. What a repeat actually signals is not about the
Drone: retries inside a Job are bounded by the retry limit, so a repeat is
across redispatches, which means a person intervened in between — and a
second failure is evidence the intervention did not take. The right
response to that is a better-worded escalation rather than a faster one.
Rejected: escalating sooner by reducing the retry budget, which spends
real retries on a judgement that may be wrong; and a distinct trigger
value, which adds to a `core-model` enum for something that is not a
different kind of failure.

**A repeat is counted across redispatches, along the `redispatched_from`
lineage** on Job — a redispatch is always a new Job, so a job id would
read every second failure as a first one. **Same failure means the same
step, any reason.** A step that stalls and then fails a gate is a second
time at that step rather than two first-time failures, because the note
it earns is true either way: the last redirect did not change the
outcome. Under a same-reason rule that case would get no note at all,
which is exactly where one is most warranted. The count answers whether
this step is defeating the engineer repeatedly, not whether it is
defeating them identically — and the payload's full attempt history keeps
the individual reasons visible where the count does not distinguish them.

**Thresholds are configurable and their values are not stated here.**
Every dial the machinery above reads — gate-failure retry limit,
sub-dispatch fan-out cap and rate limit, loop iteration cap, the
`default_gate_policy` aggregate attempt cap and Judge call-count ceiling,
poke limit, heartbeat interval — is a row in Armada Configuration
Settings, which carries the value, the tier it resolves at and the
reasoning behind it. A live filtered view of the relevant rows sat at the
foot of the Notion page; it does not carry over here. [Configuration](configuration.md)
owns the tiering rule.

**One threshold is deliberately not configurable:** once a Job has
escalated, no further auto-action is taken until a human responds. That is
a safety default with no row and no override.

---

# What Is Not a Workflow

**Onboarding/Setup is not a workflow**, and is deliberately absent from
the list above. Setting up a project is configuration, and it already has
a journey — Locate, Scan, Proposal, Write, Verify (see the Set Up a
Project journey for Manifest). The workflow system exists because you are
*not* watching: gates, retry budgets, escalation to Alerts and a Judge
that never sees the transcript are all machinery for unattended work.
During setup you are sitting there reading the proposal and editing it, so
a retry budget buys nothing and an escalation surface makes no sense for a
screen you are already looking at. The Scan is agent work and could be
dispatched, but that does not make the setup a workflow.

---

# Open Items Carried Forward

Live open items for this subject are tracked in the decision record rather
than here — decided items stay in that list and carry their reasoning with
them. A second embedded list was the settings list, which is now
`crates/config/settings.toml`.
