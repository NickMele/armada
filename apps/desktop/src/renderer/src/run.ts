// The run, as a tree, from the steps `GET /jobs/:job_id` serves.
//
// **Not `rail.ts` with a different component.** The rail drew every step's gate
// rows inline; a step's gates are the phase strip's now, in the panel, so what
// this builds is what a step *produced*, *cleared* and *came to* — short facts,
// behind a chevron. The two files share the vocabulary and nothing else, and
// `rail.ts` is still what the fold-out record's own rail is built from.
//
// # What the wire does not carry, and where that shows
//
// **A per-step attempt count is not served.** `StepDetail` carries `state`,
// `last_verdict` and `judged` and nothing that says how many times the step has
// been tried, so "an attempt is a row, not a counter" cannot be built from this
// seam at all — not as rows, and not even as the counter it is meant to replace.
// The panel names it once per step rather than this file writing a row per step
// that says the same thing seven times.
//
// **What a step produced is not served either.** `job.files_changed` and
// `JobDetail.footprint` are the whole Job's, not the step's, so a `Produced`
// fact per step would be the same file list under every row. The Produced
// chapter in the panel is where the Job's own reading is drawn, once.

import type { RunTreeFact, RunTreeStep, StepActivity } from "@armada/components";

import { CHECK_ADVANCES, CHECK_OUTCOME, STEP_STATE } from "../../shared/generated/vocabulary";
import type { CheckRun, JobDetail as JobWhole, StepDetail } from "../../shared/protocol";
import { span } from "./duration";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";

/**
 * The run.
 *
 * **Nothing here is inferred.** `state` is `job_steps.state` as Fleet recorded
 * it, the current row is the one `current_step_id` names, and a step whose state
 * the registry does not spell draws `not_started`, which claims nothing.
 *
 * **A step's activity reads against its Job's status.** `job-statuses.toml`
 * freezes the step machine at every terminal status, so a step still reading
 * `running` beneath a Job that is over is frozen and draws as frozen —
 * `frozen.ts` holds that rule.
 *
 * **The selected step's facts start open and no others do.** A seven-step
 * workflow with every step expanded fits no screen; after that the tree holds
 * whatever the reader opened, which is `RunTree`'s own rule.
 */
export function runOf(whole: JobWhole, now: number, selected: string | undefined): RunTreeStep[] {
  return ordered(whole).map((step) => {
    const frozen = frozenBeneath(whole.job.status, step.state);
    const activity = frozen?.activity ?? activityOf(step.state);
    const current = step.step_id === (selected ?? whole.job.current_step_id);
    const facts = factsOfStep(step, activity);
    return {
      id: step.step_id,
      label: step.label,
      labelIsAnIdentifier: step.label === step.step_id || undefined,
      activity,
      status: frozen?.word ?? stateOf(step),
      current: current || undefined,
      factsOpen: current || undefined,
      elapsed: took(step, now, frozen !== undefined),
      facts,
      factsAbsent: NOTHING_RECORDED,
    };
  });
}

/**
 * What a step with no facts says. **Not "nothing happened"** — a step that has
 * not started has produced nothing, and that is ordinary rather than a gap.
 */
const NOTHING_RECORDED = "Nothing is recorded against this step yet.";

/**
 * The short facts beneath a step: what its Checks came to, what its Judge came
 * to, what the last gate ruled, and where the step now stands.
 *
 * **A fact is a value, never a sentence.** Anything that reads as prose is the
 * panel's, which is the division the tree exists to hold.
 */
function factsOfStep(step: StepDetail, activity: StepActivity): RunTreeFact[] {
  const facts: RunTreeFact[] = [];

  const checks = checksFact(step);
  if (checks !== undefined) facts.push(checks);

  const judge = judgeFact(step);
  if (judge !== undefined) facts.push(judge);

  if (step.last_verdict !== undefined) {
    facts.push({
      label: "Verdict",
      value: step.last_verdict.named,
      named: step.last_verdict.named,
    });
  }

  // Served as a field rather than left as a pair to notice: a step reading
  // `advanced` beside a failed verdict is one a person overruled, and a tree
  // that drew only the first would render an overruled gate as a cleared one.
  if (step.overridden) facts.push({ label: "Advanced", value: "overruled by a person" });

  const stands = standsFact(step, activity);
  if (stands !== undefined) facts.push(stands);

  return facts;
}

/**
 * What this step's Checks came to.
 *
 * **Absent and empty are two sentences.** `checks` absent is a Fleet that
 * cannot say — the Job names a workflow this Fleet does not hold — and empty is
 * a step that gates on nothing. Neither is "the Checks failed".
 */
function checksFact(step: StepDetail): RunTreeFact | undefined {
  if (step.checks === undefined) return { label: "Checks", value: "Fleet cannot say" };
  if (step.checks.length === 0) return { label: "Checks", value: "none declared" };
  const runs = step.check_runs;
  if (runs.length === 0) return { label: "Checks", value: "not run" };
  const failed = runs.filter(didNotPass);
  return failed.length === 0
    ? { label: "Checks", value: `${runs.length} of ${step.checks.length} passed`, named: "passed" }
    : {
        label: "Checks",
        // The Check's own verb, from the registry. A word chosen here would be
        // the second vocabulary the generated module exists to prevent.
        value: `${failed[0]!.name} ${CHECK_OUTCOME[failed[0]!.outcome]?.verb ?? failed[0]!.outcome}`,
        named: "failed",
      };
}

/**
 * What this step's Judge came to. A declaration until it has answered, and a
 * count once it has — the criterion text and the citation are the panel's,
 * because each of them is a sentence.
 */
function judgeFact(step: StepDetail): RunTreeFact | undefined {
  const declared = step.judge_checks;
  if (declared === undefined) return undefined;
  if (step.judged.length === 0) {
    return declared.length === 0
      ? undefined
      : { label: "Judge", value: `${declared.length} declared`, named: undefined };
  }
  const met = step.judged.filter((judged) => judged.verdict === "met").length;
  return {
    label: "Judge",
    value: `${met} of ${step.judged.length} met`,
    named: met === step.judged.length ? "passed" : "failed",
  };
}

/**
 * Where the step now stands, as the drawing's last fact row: `Waiting on you`,
 * `Held retries spent`, `Job completed_failed`.
 *
 * **Three kinds of stopped, and they never share a row.** Waiting on you is the
 * workflow working, stopped is a Drone that cannot get further, failed is over.
 */
function standsFact(step: StepDetail, activity: StepActivity): RunTreeFact | undefined {
  if (activity === "awaiting_human") return { label: "Waiting", value: "on you" };
  if (activity === "stopped") {
    return { label: "Held", value: "retries spent · waiting on you" };
  }
  if (activity === "failed") return { label: "Job", value: "ended here", named: "failed" };
  if (activity === "killed") return { label: "Killed", value: "by a person" };
  return undefined;
}

/**
 * A Check that did not pass, read off `check-outcomes.toml`'s own `advances`.
 * The status token cannot answer it: `skipped` and `never_ran` share a token
 * and only one of them is a failure.
 */
function didNotPass(run: CheckRun): boolean {
  return CHECK_ADVANCES[run.outcome] === false;
}

/**
 * The step's state in words — the mark's accessible name, and nothing visible.
 * `enum-verbs.toml` carries no `step_state` rows, so the wire spelling renders.
 */
function stateOf(step: StepDetail): string {
  return STEP_STATE[step.state]?.verb ?? step.state;
}

/**
 * How long the step took, or nothing. The rules are `rail.ts`'s and are
 * restated rather than shared because the two files draw different components
 * from the same record: a running step measures to `now`, an unstarted one
 * shows nothing, and a frozen one never measures to a clock that moves.
 */
function took(step: StepDetail, now: number, frozen: boolean): string | undefined {
  if (step.state === "running" && !frozen) return span(step.entered_at, now) ?? undefined;
  if (step.state === "not_started" || step.entered_at === step.updated_at) return undefined;
  return span(step.entered_at, step.updated_at) ?? undefined;
}

/** Every step state the registry spells. Anything else claims nothing. */
const ACTIVITIES: readonly StepActivity[] = [
  "not_started",
  "running",
  "awaiting_human",
  "retrying",
  "advanced",
  "stopped",
];

function activityOf(state: string): StepActivity {
  return ACTIVITIES.find((known) => known === state) ?? "not_started";
}
