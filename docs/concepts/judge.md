# Judge

**What it is:** The semantic tier of evidence verification — veto-only, blind to the Drone, fires on mechanical triggers only. Judges whether evidence satisfies step intent, never whether something happened. Model is a per-step dial, cheap by default and stronger where the step earns it.

---

**Kind:** Policy.

Formalizes the Judge — the semantic tier of evidence verification. The Judge is referenced on nearly every concept document, and its constitutional rules previously lived inside a single cell of the [Workflow](workflow.md) schema table. This document is their home. That is why a Policy has a document of its own.

## What it is

The Judge is a stateless, one-shot, cheap-model call that [Fleet](fleet.md) makes **after** a Mechanical Check has already passed. It answers one question the mechanical tier structurally cannot: not *did something happen*, but *was it the right thing*.

It is not an actor. It has no session, no memory between calls, no tools, no repository access, and no ability to transition anything. Fleet dispatches it, reads its verdict, and writes the result.

| The Judge is | The Judge is not |
| --- | --- |
| A single model call with a narrow question | A session, a reviewer agent, or a [Drone](drone.md) |
| The second of two verification tiers | A replacement for human review |
| A veto | A vote, a score, or an approval |
| Triggered by mechanical signals | A scheduled or polling detector |
| Called Judge, always | The auditor, the reviewer, or AI review — see `../contracts/design-system.md` lexicon. "Auditor" is dead terminology |

## Why it exists

The core principle from [Workflow](workflow.md): self-report is a signal, not a source of truth. Before the Judge there were two tiers, and a gap between them.

| Tier | Answers | Cost | Fails at |
| --- | --- | --- | --- |
| Mechanical Check | Did something happen — diff exists, exit code, artifact present | Free, deterministic | Intent. A weakened assertion still exits 0. Evidence gaming is the failure mode a Mechanical Check passes **by design** |
| Human review | Was it the right thing | Your attention | Volume. It does not scale to a queue, and spending it on routine steps means it is unavailable for the calls that need it |

The Judge occupies the middle: machine cost, semantic reach. Measured in v1 at roughly 4k input / 300 output tokens per judgment — under a cent on Haiku.

**The alternative v1 considered and rejected:** spawning a full child Job and Drone for review. That gave the reviewer a session, tools, and the ability to act — all of which are liabilities in a verifier. The Judge does the same job for a fraction of the cost, and its inability to act is a feature.

## Constitutional constraints

Six rules. These are the whole reason a Judge verdict is worth anything. Loosen any one and the Judge becomes a second opinion drawn from the same failure distribution as the Drone it is checking.

| Rule | What it means | Why |
| --- | --- | --- |
| **1. Veto-only, never a vote** | Runs only after the mechanical check holds, and may only refuse. It can never grant advancement the mechanical tier did not already earn | There is no such thing as a Judge pass — only a mechanical pass the Judge declined to refuse. A model that can grant is a model that can be talked into granting |
| **2. Blind to the Drone** | Never sees the Drone's transcript or self-report. Receives only the original task text, the diff, and the deterministic facts | Removes the Drone's own account as a contamination source. A verifier that reads the defendant's testimony is not independent. Enforced at prompt assembly — `../contracts/agent-prompt.md` section 7 |
| **3. Narrow question** | Yes/no, never "review this" | Open-ended prompts produce agreeable prose. A narrow question has a wrong answer |
| **4. A refusal must cite** | Names the specific evidence it is refusing on. That citation is the artifact — the thing the Drone retries against and the human reads | No separate summary exists. An uncited refusal is unactionable for both audiences |
| **5. Unanimity, not majority** | `panel_size` > 1 runs independent judges against identical inputs, none seeing the others' verdicts. **Any single refusal fails the step** | Keeps a panel a stricter veto rather than a vote. Majority voting would let the Judge grant by consensus, breaking rule 1 |
| **6. Cold by default** | Fires on mechanical triggers only, never on a schedule. On a well-behaved Drone the Judge never runs at all | A detector that polls is the wrong design. The Judge costs money; a free mechanical signal decides when it is worth spending |

> Rules 1–4 originate in `docs/reserved/036`. How a v1 path citation resolves — and why it currently needs a commit hash — is on `../contracts/design-system.md`, Typography of reference.

## Where it fires

| Trigger | What is asked | Gates advancement? |
| --- | --- | --- |
| Step evidence gate | Does this evidence satisfy the step's criteria — after the mechanical check has passed | Yes. A refusal is a gate failure, standard retry flow |
| Gaming check | Baseline comparison against an earlier step's evidence, named by `baseline_ref`. `flag_if` names the patterns that make work look green while gutting it — scope narrowed, assertions weakened, tests skipped, tests deleted, and a change to the configuration a Check's command resolves through. [Workflow](workflow.md) carries the set; the last was added because editing `package.json` `scripts.test` honours a frozen `run:` string exactly while narrowing the gate | Routes as its own `evidence_suspect` escalation, **not** gate-failure — resubmission under the same instructions likely reproduces the same gaming |
| Declared plan drift | Live edits fell outside the Drone's declared `context_paths` | No. The mismatch tags the step for a mandatory Judge look; legitimate investigation sometimes moves the work |
| Thrashing ceiling | Turn count or wall-clock exceeded the step norm. "Converging, justified drift, or thrashing?" | No. A thrashing verdict force-interrupts with a report-now directive; failure of that escalates as `thrashing` |
| Final review | Summarize what this diff does and what a human should look at closely | No — `gates_advancement: false`. Advisory only; it pre-digests for the human and never replaces them |
| Gate-definition change | Whether an edit to a Check definition in `armada.yml` weakened the gate it defines, or is ordinary work | No. The spawn-time freeze already protects this Job's gate; the look exists to make the attempt visible and to let a legitimate edit flag rather than block |

Every trigger is mechanical. The Judge never fires on a timer, on a healthy Drone, or on its own initiative.

## Multi-criterion judging

Supersedes the single-`question` shape.

**The problem it solves.** One narrow question per step forces every acceptance condition into a single yes/no, which either makes the question broad (violating rule 3) or silently drops conditions. A refusal also loses resolution: "the fix does not address the root cause" cannot say *which* of four requirements went unmet.

**The change.**

| Change | Detail |
| --- | --- |
| `judge_check.criteria[]` replaces `question` | Each criterion is dispatched as its own narrow yes/no call. This **preserves** rule 3 rather than bending it — the question gets narrower, not broader |
| Step passes only if no criterion draws a refusal | Same unanimity semantics as `panel_size`, on a different axis. Two axes, one rule |
| New `Job.acceptance_criteria[]` | Frozen at Job creation, each carrying a source (`ticket` / `helm_drafted` / `human`). **This is the load-bearing half of the change** — without it the Judge grades against a list it invented, which is unfalsifiable and quietly puts self-report back in the trust path. A criterion may be **appended at an approved widening**, never edited, reordered or removed — appending is safe because citations use frozen positions |
| Evidence gains a Judge record per step | **One record per step, holding every judge's verdict**, each carrying `{criterion_id, verdict, citation, source}` where `source` is the verification-source vocabulary — `check`, `judge`, `attestation`, the last meaning **a person attested**, settable only by a human and rendering distinctly. An `attestation` criterion carries `confirmed` or `withheld`, which is neither mechanical vocabulary. **Citations name the criterion only**: which judge raised a refusal is detail inside the record rather than part of the link, so the reference a Drone retries against keeps its shape whether `panel_size` is 1 or 3 |
| Verdict stays binary | A refusal fails the step. No `unresolved` third state — see Open questions |

**Cost.** `criteria × panel_size` is the call count and it multiplies. Four criteria at `panel_size: 3` is twelve calls — roughly 48k input / 3.6k output, still cents on Haiku, and it bills against the [Manifest](manifest.md) budget cap like any other spend.

**Capped:** `default_gate_policy` carries a stated call-count ceiling per step, overridable per step, with `unbounded` a legal value, defaulting to today's realistic worst case. **Money was never the driver** — latency is, since the calls sit at a gate a person is waiting behind, and each one is a chance to trip `max_context_size`, which escalates. The product also stopped being fixed at Job creation: `acceptance_criteria[]` may now be **appended at an approved widening**, so the left factor can climb mid-Job.

**Schema sketch:**

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

**The** `baseline_ref` **above is contested** (see Open questions). Bug's sample sets `root_cause.evidence` while its own Compliance table still says the baseline is `repro`'s failing assertion — the two catch different things, and this sketch should not be read as settling it.

## Outputs and where they land

| Output | Lands in |
| --- | --- |
| Verdict + citation | Job `evidence[]`, step-and-timestamp attributed, as one Judge record per step carrying every judge's verdict — see [Job](job.md), What an Evidence entry holds |
| A refusal's content | `expected` — what should be seen, returned or recorded if the work is right, as the value itself. `produced` — what will be seen instead. `consequence` — what that difference does to whoever consumes it, which is the field a person triages on. Named fields inside the record, never a formatted string: what reaches a live Drone is `expected` and `produced` only, and a field selection is what makes that withholding safe. Shape on [Job](job.md), What an Evidence entry holds |
| Pass/fail | `workflow_status.last_step_verdict`, held separately from `step_activity` so a retrying step can carry both what is happening and why |
| Gaming flag | `escalated` with reason `evidence_suspect` |
| Prior judgments | Escalation payload carries **all** prior Judge summaries, not just the latest |
| UI surface | `evidence_suspect` renders as **evidence disputed**, with Judge in the **verification source** field — never in the headline, which keeps P5 (event-first) intact. Avoid "Judge rejected the evidence": `rejected` is already a Job state |
| Summary prose | Governed by `../contracts/agent-copy.md` — prompt plus mechanical lint. **Substance requirement:** every Judge summary must cite a file, line or assertion that could not appear in any other job's summary. A summary that reads plausibly under a different job has failed |

## Configuration

| Field | Purpose |
| --- | --- |
| `enabled` | Per-step. Some steps have no semantic question worth asking — Bug's `regression_verify` is purely mechanical |
| `model` | **A deliberate per-step dial, not fixed fleet-wide.** Routine steps use a cheap/fast model for cost and latency; the advisory final-review gate uses a stronger one. This is the rigor-scaling mechanism. **The fleet-wide default is undecided** (see Open questions) — the Judge model config row carries no value |
| `panel_size` | Default 1. Rigor as a dial — see rule 5 |
| `criteria[]` | Per-criterion narrow questions — see above |
| `prompt_key` | Reference into the compiled-in prompt set. **Judge prompts are compiled into the binary and are not user-editable** — no Kit library, no Manifest override, so prompt version is the binary version and no prompt hash is recorded on the verdict. Where the prompt library lives, and whether a prompt version is recorded on the verdict, is tracked in `../contracts/agent-prompt.md`. What a Judge is and is not told is specified there, sections 2 and 7 |
| `gaming_check` | `baseline_ref` plus `flag_if` patterns — see Where it fires above |
| `gates_advancement` | `false` makes the pass advisory — used by final review |
| `manifest_rule:review_gate` | Repo-level: `human_always` (default) or `auto_if_judge_passes`. The trust decision is the repo's call, same pattern as `auto_merge`. Renamed from `auto_if_judge_high_confidence` — no confidence scale ever existed; see [Workflow](workflow.md) |

**Three things the table above does not show.** The **Judge model** and **Judge cost cap per check** rows both read `undecided` (see Open questions), so the rigor-scaling dial and the spend sub-limit are named but unset — without the cap, verification bills against the Job budget and competes for spend with the work it verifies. **Verification** `max_context_size` **is deliberately ownerless**, because the cap bounds all of verification rather than the Judge Check alone.

**And one guarantee has gone.** A Manifest could once add Judge triggers and never remove them, because a repo that can remove them can opt out of semantic verification entirely while every other signal still reads green — and Judge is the only tier measuring correctness. That protection was the config direction rule, since withdrawn, and nothing replaced it (see Open questions).

The **Judge confidence threshold** row is superseded and awaiting disposal (see Open questions).

Evidence Scope governs what the Judge may read — `context_paths` (what changed) kept deliberately separate from `reference_docs` (the yardstick), so the Judge is never confused about which is target and which is standard. Full object on [Workflow](workflow.md).

## Dispatch and approval

**A Judge call is not a sub-dispatch.** It is a call with a verdict: no worktree, no branch, no rail, no [Job Board](job-board.md) row, no `dispatched_by`, under a cent. A **sub-dispatch** is a [Drone](drone.md) spawning a **Job**, with all of those things.

**It does not consume the fan-out cap.** The cap counts **Jobs only**, which is what makes "2 of 5" a legible number, and [Job](job.md) keeps `spawned_jobs` and `judge_calls` as separate counts, neither of them stored: `spawned_jobs` is answered by an index on `dispatched_by`, and `judge_calls` sums `panel_size` across a step's Judge records. Folding Judge calls in would make the number meaningless: a step with `panel_size: 3` and four criteria makes twelve calls and would exhaust a cap of five without spawning anything.

**Auto-approval still holds, for a different reason than it used to.** A Judge call needs no human gate. Not because it is a sub-dispatch, but because Fleet makes the call itself, inside an already-approved Job. Blocking there would be approval-fatigue with no decision content.

## Open questions

- **[judge-unresolved-verdict-state]** Should there be an unresolved third verdict state, alongside pass and refuse? The verdict is currently strictly binary — a refusal fails the step.
- **[judge-model-default]** What is the fleet-wide default Judge model? Model is a deliberate per-step dial, but the config row carrying the default currently carries no value.
- **[judge-cost-cap-per-check]** What is the Judge cost cap per check? Without it, verification bills against the Job budget and competes for spend with the work it verifies.
- **[judge-triggers-removal-guard]** What now guards a Manifest from removing Judge triggers entirely? A Manifest could once add Judge triggers and never remove them — a repo that removes them can opt out of semantic verification while every other signal still reads green, and Judge is the only tier measuring correctness. That protection was the config direction rule, since withdrawn, and nothing has replaced it.
- **[judge-confidence-threshold-disposal]** Is the Judge confidence threshold config row deleted or repurposed? It is superseded and currently awaiting disposal.
- **[judge-gaming-baseline-ref-bug]** Is Bug's gaming-check baseline `root_cause.evidence` or `repro`'s failing assertion? Bug's sample schema sets `root_cause.evidence` while its own Compliance table still names `repro`'s failing assertion — the two catch different things.
