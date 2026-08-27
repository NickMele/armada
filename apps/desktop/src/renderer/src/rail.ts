// The workflow rail, from the steps `GET /jobs/:job_id` serves.
//
// Three things the M1 drawing shows are not on the wire, and each renders as
// what is rather than as a guess: the step's `label`, a Check's command, and a
// verb per step state. Each is named where it is worked around below.

import type { StepActivity, WorkflowRailGate, WorkflowRailStep } from "@armada/components";

import {
  CHECK_OUTCOME,
  CRITERION_VERDICT_CHECK,
  ESCALATION_REASON,
  STEP_STATE,
} from "../../shared/generated/vocabulary";
import type {
  CheckRun,
  DeclaredCheck,
  JobDetail as JobWhole,
  StepDetail,
} from "../../shared/protocol";
import { span } from "./duration";
import { ordered } from "./facts";

/**
 * The rail, from the served steps.
 *
 * **Nothing here is inferred.** `state` is `job_steps.state` as Fleet recorded
 * it, and the row that is current is the one the Job's `current_step_id` names
 * rather than the one a status implied. A step whose state the registry does
 * not spell renders as `not_started`, which is the mark that draws its own
 * ordinal and claims nothing.
 *
 * **A workflow serves ids and no names.** `WorkflowDef` declares a `label` per
 * step and `core-model` has a reader for it, but neither `StepDetail` nor
 * `WorkflowStep` carries one on the wire — so every row is the `step_id`, in
 * mono, saying it is an identifier. Nothing composes a name out of the id.
 * Reported.
 */
export function railOf(whole: JobWhole, now: number): WorkflowRailStep[] {
  return ordered(whole).map((step) => ({
    id: step.step_id,
    label: step.step_id,
    labelIsAnIdentifier: true,
    activity: activityOf(step.state),
    status: stateOf(step),
    current: step.step_id === whole.job.current_step_id || undefined,
    elapsed: took(step, now),
    verdict: step.last_verdict === undefined ? undefined : verdictOf(step),
    verdictNamed: step.last_verdict?.named,
    gates: gatesOf(step),
    ungatedLabel: ungatedOf(step),
    evidence: { label: "" },
  }));
}

/**
 * The step's state in words, at the row's trailing edge.
 *
 * `enum-verbs.toml` carries no `step_state` rows, so `STEP_STATE` is empty and
 * the wire spelling renders — the same fallback a Check outcome took until its
 * rows landed. A word chosen here would be the second vocabulary the generated
 * module exists to prevent. Reported.
 */
function stateOf(step: StepDetail): string {
  return STEP_STATE[step.state]?.verb ?? step.state;
}

/**
 * The Check rows beneath a step, or none.
 *
 * **`undefined` and `[]` are two different answers and the rail draws them
 * apart.** Absent means Fleet could not say — the Job names a workflow this
 * Fleet does not hold — and empty means the step gates on nothing, which is
 * true of every step of the `bug` workflow. Returning `[]` for both would tell
 * a reader a step is ungated when what is true is that nobody could answer.
 */
function gatesOf(step: StepDetail): WorkflowRailGate[] | undefined {
  if (step.checks === undefined || step.checks.length === 0) return undefined;
  return step.checks.map((check) => {
    const run = step.check_runs.find((ran) => ran.name === nameOf(check));
    const reading = run === undefined ? NOT_REACHED : CHECK_OUTCOME[run.outcome];
    return {
      // The Check's name, and never its command: the `run` string lives in the
      // Manifest and `GET /manifests` serves check names only. The design draws
      // `build · cargo build --workspace`; the wire carries `build`. Reported.
      command: nameOf(check),
      result: run === undefined ? (reading?.verb ?? undefined) : resultOf(run),
      icon: reading?.icon ?? undefined,
      iconLabel: reading?.verb ?? undefined,
    };
  });
}

/**
 * A declared Check the gate has not run.
 *
 * `check_outcome` has no such variant — its five are what a Check that *ran*
 * did — so the word and the glyph come from the criterion Check vocabulary,
 * whose `not_reached` row carries the `shield-minus` that `icons.toml` reserves
 * to Check results. Borrowed from the registry, not written here. Reported: the
 * variant belongs under `check_outcome` too.
 */
const NOT_REACHED = CRITERION_VERDICT_CHECK["not_reached"];

/**
 * The Check's name, or the built-in's kind where it names none.
 * `diff_nonempty` is an assertion rather than a Manifest Check, so it carries
 * the kind and nothing invents a name for it.
 */
function nameOf(check: DeclaredCheck): string {
  return check.name ?? check.kind;
}

/**
 * What one Check did, or nothing where the gate has not run it.
 *
 * **The five outcomes stay five.** A pass carries no `produced` because a pass
 * measured nothing; the other four say different things — an answer that was
 * not what the step declared, a signal, a budget that expired, a command that
 * never started — and folding them into "failed" would hide the one difference
 * a reader acts on. The verb comes from the registry where the registry has
 * one; today it has no `check_outcome` rows at all, so the wire spelling
 * renders. Reported.
 */
function resultOf(run: CheckRun): string | undefined {
  const outcome = CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
  // What it was measured against, and what it did. Absent on a pass, where the
  // outcome is the whole sentence.
  const measured = [run.expected, run.produced].filter((part) => part !== undefined);
  return measured.length === 0 ? outcome : `${outcome} · ${measured.join(" → ")}`;
}

/**
 * What the rail says where a Check would be. Two sentences, because a step that
 * gates on nothing and a Fleet that cannot answer are not the same fact — and
 * the running screen's design says an ungated step must say so in words rather
 * than leaving a gap.
 */
function ungatedOf(step: StepDetail): string | undefined {
  // A step that declares none takes the rail's own default — "no check on this
  // step" — which is the contract's sentence and not one written here.
  return step.checks === undefined ? "Fleet cannot say what this step checks" : undefined;
}

/**
 * How long the step took, or nothing.
 *
 * **A step that has not started shows no duration.** `entered_at` is stamped at
 * Job creation for every step of the frozen workflow, so an unstarted step has
 * two instants and a span between them that measures how long the Job has been
 * alive rather than how long the step ran. `0s` on four rows reads as four
 * steps that ran instantly — which is also why two identical instants show
 * nothing whatever the state says.
 *
 * **A running step measures to now.** `updated_at` moved when the step entered
 * `running` and does not move again while it works, so the served pair would
 * freeze at the moment work started. `now` is injected, which is the same end
 * the whole-Job elapsed already takes.
 */
function took(step: StepDetail, now: number): string | undefined {
  if (step.state === "running") return span(step.entered_at, now) ?? undefined;
  if (step.state === "not_started" || step.entered_at === step.updated_at) return undefined;
  return span(step.entered_at, step.updated_at) ?? undefined;
}

/**
 * The last ruling, and the trigger a failure carried. The trigger's verb comes
 * from the escalation vocabulary where it has one; where it does not, the wire
 * spelling renders, which is recoverable and never invented.
 */
function verdictOf(step: StepDetail): string | undefined {
  const verdict = step.last_verdict;
  if (verdict === undefined) return undefined;
  if (verdict.trigger === undefined) return verdict.named;
  const named = ESCALATION_REASON[verdict.trigger]?.verb ?? verdict.trigger;
  return `${verdict.named} · ${named}`;
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
