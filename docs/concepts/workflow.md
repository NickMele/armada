# Workflow

**What it is:** The template a Job runs against: an ordered or loop-capable set of steps, each carrying its mechanical checks, Judge checks, evidence scope, advance gate and retry/iteration policy. Referenced by `workflow_id` and frozen into the Job at creation, so the yardstick cannot move under the work.

---

**Kind:** Entity.

How a workflow is structured, how evidence is verified, how Drone behaviour is policed, and what escalates. [Judge](judge.md) owns the semantic tier and [Drone](drone.md) the runtime; this document owns the definition a Job runs against.

## Core principle

**Self-report is a signal, not a source of truth — at every layer.** It covers not just "did the Drone finish the step" but "did the Drone report correctly at all", "is the evidence real", and "is the Drone even still working productively".

The system below enforces that principle mechanically wherever possible, and cheaply — via a small Judge model — everywhere else. Human judgment is reserved for the calls that actually need it.

## Workflow schema

Every workflow is a `WorkflowDef`: an ordered (or loop-capable) set of steps.

**The resolved `WorkflowDef` is frozen into the Job at creation** — the same treatment `acceptance_criteria[]` gets, and for the same reason: the yardstick must not move under the work. A Manifest may shadow a built-in workflow, so `workflow_id` alone does not identify what was run.

**The full field-by-field schema is `crates/core-model/domain/workflowdef-fields.toml`.** It carries every top-level field and every field nested under `steps[]`, with its type, whether it is required and the reasoning behind it. One row per field, nested the way the schema nests: the top-level rows are the `WorkflowDef`'s own fields, and everything under `steps[]` is a field of a step.

**Not all workflows are linear.** Coding workflows (Feature, Bug, Refactor) are a fixed sequence. Planning/exploratory workflows (Design Plan, Investigation) are draft↔feedback loops that repeat until converged or capped.

**A workflow's one outbound edge creates a Job and never dispatches one.** `on_fail` and `verdict_routing` jump between steps inside a definition; `on_complete` names a `workflow_id` to follow this one.

Reaching `completed_success` creates that Job at `awaiting_approval`, wired back through `subject` and carrying the same owning Manifest — and stops there. The ordinary one-by-one dispatch gate stands in front of it like any other Job.

**The split is the Job proposer's, reused.** Approving a proposed plan already creates Jobs without dispatching them, so a definition naming a successor is the same act with a different author.

**A cycle of workflows cannot run away, because every hop waits for a person.** The primary control on Fleet's autonomy stays intact. The fan-out cap does not reach this case: it counts Jobs *inside* a Job, and successors are peers.

**Only `completed_success` fires it.** What follows a failed Job is escalation, not more work.

**A successor is not a sub-dispatch.** It carries no `dispatched_by` — the Job that named it is finished and there is no live Drone to attribute it to — so it does not consume the fan-out cap, and [Job](job.md)'s `origin` carries `workflow_triggered` rather than `sub_dispatched`.

**It is a completion edge, not an event hook.** A Job that lands cleanly and breaks main an hour later is not a workflow completing; that trigger is a different mechanism.

**Declared File Plan (continuous drift check).** For steps with `scope_diff_check: true`, the Drone can declare `context_paths` at step *start* (`declare_plan_at: step_start`), not only at evidence-submission time. Fleet then compares live edits against the declared plan throughout the step rather than only at the end.

A live mismatch does not auto-fail, because legitimate investigation sometimes reveals the real work is in a file outside the original plan. It tags the step for a mandatory Judge look instead. This makes the mid-step Judge pass conditional on a cheap mechanical signal rather than run on a fixed schedule — see Thrashing / off-rails below.

**The declaration is kept, per step and per run of it.** It outlives the slot that held it, so a finished Job's footprint says which of the files it touched were inside what a step promised and which were outside every promise anybody made — and a Job whose steps declared nothing says that, rather than reading as a Job that went exactly where it said it would.

## The workflows

### How a Job gets one

**You do not have to pick.** A Job can be dispatched from a prompt or a link to a ticket, and the [Job proposer](job-proposer.md) reads that request and proposes which workflow the work should run under. 
Picking one by hand stays available and is the override, not the path. That document owns the call.

**What is proposed is not what is stored.** The resolved `WorkflowDef` is frozen into the Job at creation, as above. The proposal chooses which one; freezing is what stops it moving afterwards, so a workflow edited in the repo between the proposal and the dispatch reaches no Job already created against it.

**Feature is the pattern other coding workflows follow; Bug is the sample to copy from.** Design Plan is the only loop; Prototype is the only workflow with gateless steps.

**Refactor exercises the gaming-check path end to end and demonstrates the no-parsing rule.** A Manifest Check exit code is its entire mechanical tier, and every question about what the tests now cover is answered by a Judge reading the diff.

**The full workflow catalogue is `crates/core-model/domain/workflows.toml`**, one row per workflow shape — the shape, gate profile and status, including policy variants and workflows still needing a step list, so there is one list rather than three. The instantiated `WorkflowDef` sits beside it as JSON in `crates/core-model/domain/workflow-samples/`, where a sample exists.

A Job instance references `workflow_id` and carries its own `status`, `current_step`, `retry_count` and `iteration_count` per step, `acceptance_criteria[]` (frozen at Job creation), `facts[]` (append-only), and `escalations[]` (using the trigger taxonomy below). A `WorkflowDef` is the template; the Job is the running instance against it. Those counters live in `job_steps`, one row per step, on [Job](job.md).

## Evidence scope object

Attached to every step that triggers a Judge or a human gate. Prevents the Judge (or the human at the gate) from having to search the whole codebase — it is told exactly where to look.

**Definition vs. resolved object.** A `WorkflowDef` step carries the *policy* — `context_source`, `max_context_size`, `exclude_paths`, the flags. It cannot carry `context_paths`, because at definition time nobody knows them: that is precisely what `context_source: drone_declared` means.

The Drone supplies them at evidence-report time, Fleet validates them, and the **resolved** object handed to the Judge is the one on which `context_paths` is required. Where a field row says a field is required, it means required on the resolved object.

**The diff is delivered, not fetched.** The resolved object carries the **actual patch** for the files in scope, pre-loaded into the Judge's context — not a list of filenames the Judge then has to go and read. The veto-only contract already says the Judge receives the task text, the **work product** and the deterministic facts; on a coding step the diff *is* the work product.

Reasons, in order of weight:

1. **It is what makes a panel a panel.** `judge_check.panel_size > 1` requires each Judge to run "independently against identical inputs." A Judge that chooses what to read does not have identical inputs — three Judges could read three different subsets and their unanimity would mean less than it appears to.
2. **The evidence trail records what was actually judged.** A refusal has to cite the specific evidence it is refusing on. If the Judge fetched its own context, the exact bytes it saw are not in the record, and neither the Drone retrying against the refusal nor the human reading it can reconstruct what it looked at.
3. **Cost and latency.** Fetching burns turns on a call measured at ~4k input tokens.

`context_paths` is then the narrower thing: **what else** the Judge may open beyond the diff — the unchanged remainder of a touched file, a sibling module, an interface being conformed to. A hunk alone often cannot answer "is this test still meaningful"; the rest of the file can.

**A large diff can exceed `max_context_size` and fails closed there** like anything else, which is the correct behaviour and the reason that cap exists.

**On Code Review the diff is the Drone's *input* rather than its output.** "The work product" needs restating for that workflow, and it is one of the reasons the gate row is the component that bends there.

### What the Judge receives, per `evidence_type`

The veto-only contract says the Judge receives the task text, the **work product** and the deterministic facts. On a coding step the work product is the diff. On several of Bug's steps it is not, and "only the diff" would leave nothing to judge there — hence this table.

**Drone-authored evidence is admissible; the transcript is not.** These are different things, and the distinction is what keeps `facts_note` from reading as a loophole. **The transcript is the Drone's account of its own process** — why it struggled, what it tried — testimony about itself.

**A facts note is a deliverable submitted as evidence** and judged as one: the Judge asks *is this a sound root-cause analysis*, never *did the Drone work hard*. In one line: **the Judge sees what the step produced. It never sees how the step went.**

| `evidence_type` | Work product handed to the Judge | What `context_paths` is for on that step |
| --- | --- | --- |
| `diff` | The actual patch, pre-loaded | **What else** may be opened beyond the diff |
| `failing_test` | The patch adding the test, plus the run's exit code and status | As for `diff` |
| `facts_note` | The note itself — Drone-authored, judged as a deliverable | **The whole payload** |
| `test_suite_run` | The exit code as a deterministic fact, plus the diff being verified | As for `diff` |
| `bundle` | The accumulated evidence of prior steps, assembled | **The whole payload**, alongside `reference_docs` |
| Document or mockup (Design Plan) | The document | **The whole payload** |
| Absent (Bug's `merge`) | Nothing — no Judge runs | n/a |

Per row:

- `failing_test` — Bug's `repro` asks whether the test represents the reported bug, which needs the code under test. The exit code and status are handed over as deterministic facts.
- `test_suite_run` — the Judge question is about the **diff**, not the run; the run is already answered by its exit code.
- `bundle` — its `reference_docs` point at prior steps' evidence. Bug's `review` is advisory (`gates_advancement: false`) and summarises rather than refuses.

**Code Review inverts and is the exception the table cannot fold in.** The diff is the Drone's *input*; the work product is the review it wrote. `files_read` is what verifies the input was actually opened.

The `evidence_scope` fields are rows under `evidence_scope` in the field-by-field schema above — each with its type, whether it is required, and the argument behind it.

**Resolution order:** Manifest default for step type → Drone declares specifics at evidence-report time → Fleet validates against allowlist and (if enabled) the real diff → **for loop workflows, the most recent iteration's work product plus every prior verdict**.

**Only the latest work product is carried, and all of the feedback.** Appending every prior iteration against a fixed cap breaches it by construction, and since exceeding `max_context_size` escalates, an unbounded append would make every loop workflow generate a guaranteed false escalation. That is worse than failing, because it trains the engineer to ignore that escalation.

The split works because the two things are different objects: drafts, documents and mockups are large, while a `request_changes` verdict is a sentence or two. Keeping all the verdicts is what shows the same note went unaddressed three times, which is the judgement `iteration_cap` exists to force. Carrying only the latest iteration entire would make each round read as a fresh note.

Rejected: a rolling summary, which adds a model call per iteration and puts a lossy step inside the evidence path; and a larger cap or exemption for loops, which defeats the cap on the workflows most likely to need it.

## Drone compliance & policing

Prototype learning: Drones don't reliably follow instructions or use the tools they're given — they'll report status in plain text, wander off-task, or technically satisfy a check while gaming it. Each failure mode below needs its own detection mechanism.

### Silence / stalled

- **Symptom:** no activity for X time.
- **Detection:** a heartbeat/liveness timer per Job. The threshold is a config row in `../contracts/configuration.md`, distinct from Fleet's own resource-poll interval.
- **Response:** poke (nudge prompt) via `poke_limit`. N pokes with no response escalate as `stalled`.

The clock runs only while the Job is `running`. What bounds an indefinitely-open gate instead is on [Drone](drone.md), which owns the decision.

**A Drone that exits cleanly having called nothing at all is `silent` instead.** Different trigger, different human response — redispatching a silent Drone with the same brief reproduces the same silence.

`poke_limit` is a liveness check (nothing arrived at all) and is a distinct counter from [Drone](drone.md)'s evidence clarification-round cap, which is a content-sufficiency check. Both can end in the same `stalled` label.

### Claims done, no evidence artifact

- **Symptom:** "I'm done" but nothing to check.
- **Detection:** the mechanical check naturally fails — no file, diff or output exists.
- **Response:** standard gate-failure retry flow.

### Claims done in plain text, bypasses structured report

- **Symptom:** the Drone doesn't use the evidence-reporting mechanism at all.
- **Detection:** Fleet's parser finds no valid evidence submission for the turn — treated as nothing happened, not as done or failed. Largely superseded by the dedicated Evidence MCP tool — see [Drone](drone.md).
- **Response:** the same reprompt-then-poke flow as silence. Insufficient but present evidence gets a free clarification round first — capped, see [Drone](drone.md) — before this flow kicks in.

**The harness envelope cannot be trusted to tell you this.** A run denied every tool it needed still terminated reporting success with exit code 0.

### Thrashing / off-rails

- **Symptom:** actively working, but not converging — fixated on a tangent instead of reporting stuck.
- **Detection:** turn count vs. step norm, wall-clock vs. expected ceiling, and — for steps using `declare_plan_at: step_start` — a mechanical file-plan-vs-live-diff check. All mechanical, no LLM judgment needed to detect.
- **Response:** a **mid-step Judge** pass, then a force-interrupt if it returns thrashing.

An off-plan edit alone doesn't auto-fail; it tags the step for a mandatory Judge look rather than blocking outright.

**The mid-step Judge runs only when a mechanical trigger fires** — plan drift flagged, or turn count/wall-clock exceeds norm — not on a fixed schedule regardless of activity. **It does not read the Drone's turns.** It judges the *work product so far* against the declared scope and the step's intent: "is what has been produced converging on the task, is it justified drift, or is it thrashing?"

If thrashing, force-interrupt with a "stop, report current state now" directive. A Drone that then goes quiet has its step stopped on a `forced_report` row naming the report that never came. A Drone still writing inside its declared plan is not stopped for it.

**Reading the Drone's last N turns would be the stronger detector and is ruled out by the veto-only input contract.** The detector is weaker for it: it can no longer separate justified drift from thrashing by reading the Drone's stated intent, and must infer it from what was actually produced.

### Evidence gaming

- **Symptom:** evidence is technically true but substantively worthless — weakened assertion, narrowed test scope, tautological test. Mechanical *and* naive Judge checks can both pass.
- **Detection:** `judge_check.gaming_check` compares the current step's evidence against an earlier step's, named by `baseline_ref`. Not every `flag_if` pattern reaches the Judge to do it: a pattern the patch text itself settles — a test file removed whole, a skip marker added to a test that used to run, a change to the configuration a Check's command resolves through — is read straight off the diff, and costs no call, because a model call spent on a question `git diff` already answers is money for nothing. The rest need the change understood — an assertion that now asserts less, tests still running but covering less than they did, a test that would pass whatever the code under it did — so each of those is one narrow Judge question.
- **Response:** routes as its own `evidence_suspect` trigger, not gate-failure — resubmission under the same instructions likely reproduces the same gaming.

`flag_if` names the patterns: scope narrowed, assertions weakened, a tautological test, tests skipped, tests deleted, and **a change to the configuration a Check's command resolves through** — `package.json` `scripts.test`, a Makefile target.

**The config-edit pattern is there because the others are all phrased in terms of *test code*.** Editing the script a frozen `run: "pnpm test"` resolves to honours the frozen string exactly while narrowing the gate, and fell outside all of them. Freezing harder is not available: the no-parsing rule refuses to model what a tool command expands to.

### Enforcement

**The most reliable fix is forcing structured output at the harness level, not policing after the fact**, so a non-compliant response simply doesn't parse as a valid turn. A dedicated Evidence MCP tool is the one sanctioned way a Drone reports completion — see [Drone](drone.md), which owns it and the clarification-round cap.

Headless output parses reliably and denials are visible three separate ways, both closed by measurement, so this no longer rests on an unanswered spike.

**Thrashing and off-scope work are computable from the stream** — repetition and paths touched are typed events, needing no prose parsing. **Silence is the one that does not fall out for free**, since only assistant and user events carry a timestamp. What stays open is the judgement layer rather than the data — see Open questions.

## Dispatch approval — two levels

| Dispatch type | Approval needed? | Rationale |
| --- | --- | --- |
| Job-level — a Drone picks up a Job off the Board | Human, always | Primary autonomy control — you decide what gets worked on |
| Sub-dispatch — a Drone spawns a **Job** to complete its own | No — auto | Already inside an approved Job |
| Sub-dispatch, abuse pattern — fan-out beyond threshold | Escalate to Inbox | Volume itself is the signal, not any single dispatch |
| Successor — a `WorkflowDef`'s `on_complete` names the next workflow | Human, always | It is a Job-level dispatch; the parent is finished |

**Job-level dispatch is strictly one by one.** [Fleet](fleet.md) owns the rule.

**Blocking a sub-dispatch would be approval-fatigue with no real decision content.** The fan-out cap counts **Jobs only** — a Judge call is not a sub-dispatch, which is what makes "2 of 5" a legible number. [Judge](judge.md) owns why; [Job](job.md) counts `spawned_jobs` and `judge_calls` separately.

**The sub-dispatch rationale does not reach a successor**: there is no approved Job for this one to sit inside. The definition creates the Job; it does not start it, which is the same plan-versus-dispatch split the Job proposer's proposal already uses.

**Human approval applies at the Job level.** Work a Drone generates in service of its own approved Job does not require separate approval.

## Escalation taxonomy

One Inbox surface, `snake_case` triggers. **The canonical set is `crates/core-model/domain/escalation-triggers.toml`** — [Judge](judge.md), [Drone](drone.md), [Fleet](fleet.md) and [Job](job.md) all cite individual triggers, so each is a row they can point at.

Every name states the condition, not the mechanism. A **sub-kind** pauses the Job exactly as its parent trigger does and differs only in the suggested action on the payload; `silent` is the only one. Routing for `evidence_suspect` and `thrashing` is Judge's, and `stalled` is Drone's.

**The escalation trigger enum, with its In code migration status, lives in `crates/core-model/domain/escalation-triggers.toml`.** What is marked *not yet* is not yet a variant in `core-model`'s Rust enum, which is the only home — `Job::transition` is the sole mutator, and the TypeScript side is generated from `ipc` rather than authored. Nothing is removed, and one migration window covers all of them.

**Escalation payload:** Job + step where it tripped, trigger type (and sub-kind where one applies), full attempt/dispatch history (all prior Judge summaries, not just the latest), suggested actions (kill / rephrase & redispatch / manual override). **The suggested action is where a sub-kind earns its keep** — `stalled` and `stalled(silent)` pause the Job identically and differ only in what the payload recommends.

**A repeat failure escalates identically to a first one.** It differs only in the payload and its suggested action, which is the same shape as a sub-kind. Escalation already halts autonomous action, so a first failure has already reached the engineer and there is nothing for a repeat to escalate harder into.

What a repeat actually signals is not about the Drone. Retries inside a Job are bounded by the retry limit, so a repeat is across redispatches, which means a person intervened in between — and a second failure is evidence the intervention did not take. The right response to that is a better-worded escalation rather than a faster one.

Rejected: escalating sooner by reducing the retry budget, which spends real retries on a judgement that may be wrong; and a distinct trigger value, which adds to a `core-model` enum for something that is not a different kind of failure.

**A repeat is counted across redispatches, along the `redispatched_from` lineage** on [Job](job.md). A redispatch is always a new Job, so a job id would read every second failure as a first one.

**Same failure means the same step, any reason.** A step that stalls and then fails a gate is a second time at that step rather than two first-time failures, because the note it earns is true either way: the last redirect did not change the outcome. Under a same-reason rule that case would get no note at all, which is exactly where one is most warranted.

The count answers whether this step is defeating the engineer repeatedly, not whether it is defeating them identically. The payload's full attempt history keeps the individual reasons visible where the count does not distinguish them.

**Thresholds are configurable and their values are not stated here.** Every dial the machinery above reads — gate-failure retry limit, sub-dispatch fan-out cap and rate limit, loop iteration cap, the `default_gate_policy` aggregate attempt cap and Judge call-count ceiling, poke limit, heartbeat interval — is a row in the Configuration Settings registry, which carries the value, the tier it resolves at and the reasoning behind it. `../contracts/configuration.md` owns the tiering rule.

**One threshold is deliberately not configurable:** once a Job has escalated, no further auto-action is taken until a human responds. That is a safety default with no row and no override.

## What is not a workflow

**Onboarding/Setup is not a workflow**, and is deliberately absent from the catalogue. Setting up a project is configuration, and it already has a journey — Locate, Scan, Proposal, Write, Verify (see the Set Up a Project (Manifest) user journey).

The workflow system exists because you are *not* watching: gates, retry budgets, escalation to Alerts and a Judge that never sees the transcript are all machinery for unattended work. During setup you are sitting there reading the proposal and editing it, so a retry budget buys nothing and an escalation surface makes no sense for a screen you are already looking at.

The Scan is agent work and could be dispatched, but that does not make the setup a workflow.

## Open questions

- **[workflow-thrashing-threshold-judgement]** What threshold of repetition counts as thrashing, and what counts as off-scope work? Headless output parses reliably and denials are visible, so this no longer rests on an unanswered spike about the data — what stays open is the judgement layer, not the data.
