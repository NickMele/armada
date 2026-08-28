# Judge

**What it is:** The semantic tier of evidence verification — veto-only, blind to the Drone, fires on mechanical triggers only. Judges whether evidence satisfies step intent, never whether something happened. Model is a per-step dial, cheap by default and stronger where the step earns it.

---

**Kind:** Policy.

Formalizes the Judge — the semantic tier of evidence verification. The Judge is referenced on nearly every concept document, and this document is the home of its constitutional rules.

## What it is

The Judge is a stateless, one-shot, cheap-model call that [Fleet](fleet.md) makes **after** a Mechanical Check has already passed. It answers one question the mechanical tier structurally cannot: not *did something happen*, but *was it the right thing*.

It is not an actor. It has no session, no memory between calls, no tools, no repository access, and no ability to transition anything. Fleet dispatches it, reads its verdict, and writes the result.

| The Judge is | The Judge is not |
| --- | --- |
| A single model call with a narrow question | A session, a reviewer agent, or a Drone |
| The second of two verification tiers | A replacement for human review |
| A veto | A vote, a score, or an approval |
| Triggered by mechanical signals | A scheduled or polling detector |
| Called Judge, always | The auditor, the reviewer, or AI review |

**"Auditor" is retired vocabulary.** See `../contracts/design-system.md` lexicon. The Judge is not a [Drone](drone.md).

## Why it exists

The core principle from [Workflow](workflow.md): self-report is a signal, not a source of truth. Without the Judge there are two tiers and a gap between them.

| Tier | Answers | Cost | Fails at |
| --- | --- | --- | --- |
| Mechanical Check | Did something happen — diff, exit code, artifact | Free, deterministic | Intent — a weakened assertion still exits 0 |
| Human review | Was it the right thing | Your attention | Volume — it does not scale to a queue |

Evidence gaming is the failure mode a Mechanical Check passes **by design**. Human attention spent on routine steps is unavailable for the calls that need it.

The Judge occupies the middle: machine cost, semantic reach. Measured in v1 at roughly 4k input / 300 output tokens per judgment — under a cent on Haiku.

**Rejected: spawning a full child Job and Drone for review.** That gives the reviewer a session, tools and the ability to act, all of which are liabilities in a verifier, at many times the cost.

## Constitutional constraints

**Loosen any one of these and the Judge becomes a second opinion drawn from the same failure distribution as the Drone it is checking.**

**1. Veto-only, never a vote.** The Judge runs only after the mechanical check holds, and may only refuse; it can never grant advancement the mechanical tier did not already earn.
Why: there is no such thing as a Judge pass — only a mechanical pass the Judge declined to refuse, and a model that can grant is a model that can be talked into granting.

**2. Blind to the Drone.** The Judge never sees the Drone's transcript or self-report, and receives only the original task text, the diff, and the deterministic facts.
Why: a verifier that reads the defendant's testimony is not independent, so the Drone's own account is removed as a contamination source.
Enforced at prompt assembly — `../contracts/agent-prompt.md` section 7.

**3. Narrow question.** Every question is yes/no, never "review this".
Why: open-ended prompts produce agreeable prose, and a narrow question has a wrong answer.

**4. A refusal must cite.** A refusal names the specific evidence it is refusing on.
Why: that citation is the artifact the Drone retries against and the human reads — no separate summary exists, so an uncited refusal is unactionable for both audiences.

**5. Unanimity, not majority.** A `panel_size` above 1 runs independent judges against identical inputs, none seeing the others' verdicts, and **any single refusal fails the step**.
Why: majority voting would let the Judge grant by consensus, breaking rule 1, so a panel stays a stricter veto rather than a vote.

**6. Cold by default.** The Judge fires on mechanical triggers only, never on a schedule, and on a well-behaved Drone it never runs at all.
Why: a detector that polls is the wrong design — the Judge costs money, and a free mechanical signal decides when it is worth spending.

> How a v1 path citation resolves — and why it currently needs a commit hash — is on `../contracts/design-system.md`, Typography of reference.

## Where it fires

| Trigger | What is asked | Gates advancement? |
| --- | --- | --- |
| Step evidence gate | Does this evidence satisfy the step's criteria | Yes — a refusal is a gate failure, standard retry flow |
| Gaming check | Baseline comparison against an earlier step's evidence | No — routes as its own `evidence_suspect` escalation |
| Declared plan drift | Did live edits fall outside the declared `context_paths` | No — tags the step for a mandatory Judge look |
| Thrashing ceiling | Converging, justified drift, or thrashing? | No — force-interrupts with a report-now directive |
| Final review | What this diff does, and what a human should look at closely | No — `gates_advancement: false`, advisory only |
| Gate-definition change | Did an edit to a Check definition weaken the gate it defines | No — makes the attempt visible without blocking |

Per row:

- **Step evidence gate** runs only after the mechanical check has passed.
- **Gaming check** names its baseline with `baseline_ref`. `flag_if` names the patterns that make work look green while gutting it, and not all of them reach the Judge. A pattern the patch text itself settles — a test file removed whole, a skip marker added to a test that used to run, a change to the configuration a Check's command resolves through — costs no call, because paying a model for an answer `git diff` already gives is money for nothing. The rest need the change understood before they can be answered — an assertion that now asserts less, tests still running but covering less than they did, a test that would pass whatever the code under it did — and each of those is one narrow Judge question. The config-edit pattern was added because editing `package.json` `scripts.test` honours a frozen `run:` string exactly while narrowing the gate, and a diff catches that without needing the change understood. [Workflow](workflow.md) carries the set and the same split.
- **Gaming check routes as `evidence_suspect`, not gate-failure.** Why: resubmission under the same instructions likely reproduces the same gaming.
- **Declared plan drift** does not fail the step, because legitimate investigation sometimes moves the work.
- **Thrashing ceiling** fires when turn count or wall-clock exceeded the step norm. Failure of the report-now directive escalates as `thrashing`.
- **Final review** pre-digests the diff for the human and never replaces them.
- **Gate-definition change** covers an edit to a Check definition in `armada.yml`. The spawn-time freeze already protects this Job's gate; the look exists to make the attempt visible and to let a legitimate edit flag rather than block.

Every trigger is mechanical. The Judge never fires on a timer, on a healthy Drone, or on its own initiative.

## Multi-criterion judging

Supersedes the single-`question` shape.

One narrow question per step forces every acceptance condition into a single yes/no, which either makes the question broad (violating rule 3) or silently drops conditions. A refusal also loses resolution: "the fix does not address the root cause" cannot say *which* of four requirements went unmet.

**`judge_check.criteria[]` replaces `question`.** Each criterion is dispatched as its own narrow yes/no call.
Why: this preserves rule 3 rather than bending it — the question gets narrower, not broader.

**A step passes only if no criterion draws a refusal.** Same unanimity semantics as `panel_size`, on a different axis. Two axes, one rule.

**`Job.acceptance_criteria[]` is new and frozen at Job creation.** Each criterion carries a source — `ticket`, `helm_drafted` or `human`.
Why: it is the load-bearing half of the change — without it the Judge grades against a list it invented, which is unfalsifiable and quietly puts self-report back in the trust path.

**A criterion may be appended at an approved widening, never edited, reordered or removed.** Why: appending is safe because citations use frozen positions.

**Evidence gains one Judge record per step, holding every judge's verdict.** Each verdict carries `{criterion_id, verdict, citation, source}`, where `source` is the verification-source vocabulary: `check`, `judge`, `attestation`.

`attestation` means **a person attested**. It is settable only by a human, renders distinctly, and carries `confirmed` or `withheld` — which is neither mechanical vocabulary.

**Citations name the criterion only.** Why: which judge raised a refusal is detail inside the record rather than part of the link, so the reference a Drone retries against keeps its shape whether `panel_size` is 1 or 3.

**The verdict stays binary.** A refusal fails the step, and there is no `unresolved` third state (see Open questions).

### Cost

`criteria × panel_size` is the call count and it multiplies. Four criteria at `panel_size: 3` is twelve calls — roughly 48k input / 3.6k output, still cents on Haiku. It bills against the [Manifest](manifest.md) budget cap like any other spend.

**`default_gate_policy` carries a stated call-count ceiling per step**, overridable per step, with `unbounded` a legal value, defaulting to today's realistic worst case.

**Money was never the driver — latency is.** The calls sit at a gate a person is waiting behind, and each one is a chance to trip `max_context_size`, which escalates.

The product also stopped being fixed at Job creation: `acceptance_criteria[]` may be appended at an approved widening, so the left factor can climb mid-Job.

### Schema sketch

```json
"judge_check": {
  "enabled": true,
  "model": "haiku",
  "panel_size": 1,
  "criteria": [
    {
      "criterion_id": "c1",
      "source_ref": "job.acceptance_criteria[0]",
      "question": "Do expired tokens refresh once rather than per request?"
    },
    {
      "criterion_id": "c2",
      "source_ref": "job.acceptance_criteria[1]",
      "question": "Does a failed refresh sign the session out?"
    }
  ],
  "gaming_check": {
    "enabled": true,
    "baseline_ref": "root_cause.evidence",
    "flag_if": ["assertion_weakened", "test_scope_narrowed",
                "test_skipped", "test_deleted", "check_config_edited"]
  }
}
```

**The** `baseline_ref` **above is contested** (see Open questions). Bug's sample sets `root_cause.evidence` while its own Compliance table still says the baseline is `repro`'s failing assertion. The two catch different things, and this sketch does not settle it.

## Outputs and where they land

| Output | Lands in |
| --- | --- |
| Verdict + citation | Job `evidence[]`, step-and-timestamp attributed |
| A refusal's content | Named fields inside the Judge record |
| Pass/fail | `workflow_status.last_step_verdict` |
| Gaming flag | `escalated` with reason `evidence_suspect` |
| Prior judgments | Escalation payload, carrying **all** prior Judge summaries |
| UI surface | `evidence_suspect` renders as **evidence disputed** |
| Summary prose | Governed by `../contracts/agent-copy.md` |

Verdicts land as one Judge record per step carrying every judge's verdict. Pass/fail is held separately from `step_activity`, so a retrying step can carry both what is happening and why. Record shape is on [Job](job.md), What an Evidence entry holds.

A refusal's content is three named fields, never a formatted string:

| Field | What it holds |
| --- | --- |
| `expected` | What should be seen, returned or recorded if the work is right — the value itself |
| `produced` | What will be seen instead |
| `consequence` | What that difference does to whoever consumes it |

`consequence` is the field a person triages on. What reaches a live Drone is `expected` and `produced` only, and a field selection is what makes that withholding safe.

**Judge renders in the verification source field, never in the headline.** Why: that keeps P5 (event-first) intact. Avoid "Judge rejected the evidence" — `rejected` is already a Job state.

Summary prose is prompt plus mechanical lint. **Substance requirement: every Judge summary must cite a file, line or assertion that could not appear in any other job's summary.** A summary that reads plausibly under a different job has failed.

## Configuration

| Field | Purpose |
| --- | --- |
| `enabled` | Per-step. Some steps have no semantic question worth asking |
| `model` | Per-step model dial. The rigor-scaling mechanism |
| `panel_size` | Default 1. Rigor as a dial — see rule 5 |
| `criteria[]` | Per-criterion narrow questions — see above |
| `prompt_key` | Reference into the compiled-in prompt set |
| `gaming_check` | `baseline_ref` plus `flag_if` patterns — see Where it fires |
| `gates_advancement` | `false` makes the pass advisory — used by final review |
| `manifest_rule:review_gate` | Repo-level: `human_always` (default) or `auto_if_judge_passes` |

**`enabled`.** Bug's `regression_verify` is purely mechanical and asks no semantic question.

**`model` is a deliberate per-step dial, not fixed fleet-wide.** Routine steps use a cheap/fast model for cost and latency; the advisory final-review gate uses a stronger one. **The fleet-wide default is the cheapest model on the roster**, decided 28 Aug 2026 and carried by the Judge model config row. It is deliberately the floor rather than the middle: the dial is only legible against a default it moves away from, and Bug raising `review` to a stronger model says nothing if every step started there. A stronger default would not let the Judge see more — it has one turn, no tools and no repository — it would only make every criterion cost more and take longer at a gate somebody is waiting behind.

**Judge prompts are compiled into the binary and are not user-editable.** No Kit library, no Manifest override, so prompt version is the binary version and no prompt hash is recorded on the verdict. Where the prompt library lives, and whether a prompt version is recorded on the verdict, is tracked in `../contracts/agent-prompt.md`; what a Judge is and is not told is specified there, sections 2 and 7.

**`review_gate` is the repo's trust decision**, same pattern as `auto_merge`. `auto_if_judge_high_confidence` is retired vocabulary — no confidence scale ever existed; see [Workflow](workflow.md).

**The Judge cost cap per check row still reads `undecided`**, and was reviewed and left open on 28 Aug 2026 rather than guessed at (see Open questions). Verification bills against the Job budget with no sub-limit of its own. What bounds Judge spend today is count-shaped rather than money-shaped: the call-count ceiling bounds `criteria × panel_size` per step and the Manifest budget cap bounds the Job.

**Verification `max_context_size` is deliberately ownerless.** Why: the cap bounds all of verification rather than the Judge Check alone.

**Nothing now stops a Manifest removing Judge triggers.** A Manifest could once add them and never remove them; that protection was the config direction rule, which has been withdrawn with nothing replacing it (see Open questions).
Why it mattered: a repo that can remove them can opt out of semantic verification entirely while every other signal still reads green, and Judge is the only tier measuring correctness.

The **Judge confidence threshold** row is superseded and awaiting disposal (see Open questions).

Evidence Scope governs what the Judge may read — `context_paths` (what changed) kept deliberately separate from `reference_docs` (the yardstick), so the Judge is never confused about which is target and which is standard. Full object on [Workflow](workflow.md).

## Dispatch and approval

**A Judge call is not a sub-dispatch.** It is a call with a verdict: no worktree, no branch, no rail, no [Job Board](job-board.md) row, no `dispatched_by`, under a cent. A **sub-dispatch** is a [Drone](drone.md) spawning a **Job**, with all of those things.

**It does not consume the fan-out cap.** The cap counts **Jobs only**, which is what makes "2 of 5" a legible number. [Job](job.md) keeps `spawned_jobs` and `judge_calls` as separate counts, neither of them stored.

`spawned_jobs` is answered by an index on `dispatched_by`; `judge_calls` sums `panel_size` across a step's Judge records. Folding Judge calls in would make the number meaningless: a step with `panel_size: 3` and four criteria makes twelve calls and would exhaust a cap of five without spawning anything.

**Auto-approval still holds, for a different reason than it used to.** A Judge call needs no human gate — not because it is a sub-dispatch, but because Fleet makes the call itself, inside an already-approved Job. Blocking there would be approval-fatigue with no decision content.

## Open questions

- **[judge-unresolved-verdict-state]** Should there be an unresolved third verdict state, alongside pass and refuse? The verdict is currently strictly binary — a refusal fails the step.
- **[judge-cost-cap-per-check]** Does a Judge call get a cost readout, and is a per-check dollar cap the right shape for the sub-limit? Reviewed 28 Aug 2026 alongside the model default and deliberately left open: nothing can read what a Judge call cost, because the call is rendered as one-shot text and emits no result envelope, so the figure a cap would compare against never reaches Armada. A number written into the row today would be enforced by nothing while reading as a limit that was set, which is worse than an absent cap because only the absent one is visible as missing. Without it verification bills against the Job budget and competes for spend with the work it verifies — bounded meanwhile by the call-count ceiling and the Manifest budget cap, both of which need no price to enforce.
- **[judge-triggers-removal-guard]** What now guards a Manifest from removing Judge triggers entirely? A Manifest could once add Judge triggers and never remove them — a repo that removes them can opt out of semantic verification while every other signal still reads green, and Judge is the only tier measuring correctness. That protection was the config direction rule, since withdrawn, and nothing has replaced it.
- **[judge-confidence-threshold-disposal]** Is the Judge confidence threshold config row deleted or repurposed? It is superseded and currently awaiting disposal.
- **[judge-gaming-baseline-ref-bug]** Is Bug's gaming-check baseline `root_cause.evidence` or `repro`'s failing assertion? Bug's sample schema sets `root_cause.evidence` while its own Compliance table still names `repro`'s failing assertion — the two catch different things.
