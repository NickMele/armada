// The workflow rail, from the steps `GET /jobs/:job_id` serves.
//
// The step's name and a Check's command are served now, so the rail reads
// `Plan the change · cargo build --workspace` rather than `plan · build`. One
// thing the drawing shows is still not on the wire — a verb per step state —
// and it is named where it is worked around below.

import type {
  CriterionVerdict,
  StepActivity,
  WorkflowRailGate,
  WorkflowRailStep,
} from "@armada/components";

import {
  CHECK_OUTCOME,
  CRITERION_VERDICT_CHECK,
  CRITERION_VERDICT_JUDGE,
  ESCALATION_REASON,
  STEP_STATE,
} from "../../shared/generated/vocabulary";
import type {
  CheckRun,
  Criterion,
  DeclaredCheck,
  JobDetail as JobWhole,
  Judged,
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
 * **The label is Fleet's, and the id is Fleet's fallback.** `StepDetail.label`
 * is never absent and never blank: where the workflow declares no label, or
 * Fleet cannot say which workflow this is, Fleet substitutes the id. So the
 * row draws what arrived and never composes a name — and it renders in mono
 * only when what arrived *is* the id, which is the one case a reader needs
 * told apart.
 */
export function railOf(whole: JobWhole, now: number): WorkflowRailStep[] {
  return ordered(whole).map((step) => ({
    id: step.step_id,
    label: step.label,
    labelIsAnIdentifier: step.label === step.step_id || undefined,
    activity: activityOf(step.state),
    status: stateOf(step),
    current: step.step_id === whole.job.current_step_id || undefined,
    elapsed: took(step, now),
    verdict: step.last_verdict === undefined ? undefined : verdictOf(step),
    verdictNamed: step.last_verdict?.named,
    gates: gatesOf(step),
    verdicts: verdictsOf(step, whole.acceptance_criteria),
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
      // Name first, then the command the Job's frozen workflow resolved it to.
      // `build` says nothing about what gated the step; the command says what
      // to run to reproduce it. `diff_nonempty` runs nothing and carries none.
      command: check.run === undefined ? nameOf(check) : `${nameOf(check)} · ${check.run}`,
      result: run === undefined ? (reading?.verb ?? undefined) : resultOf(run),
      icon: reading?.icon ?? undefined,
      iconLabel: reading?.verb ?? undefined,
      // Where the Check wrote its stdout and stderr. **The path, never the
      // contents** — Bridge does not read the filesystem, so naming the file
      // is the whole of what it can do, and without it a failed Check is
      // unreadable without going and finding it.
      outputPath: run?.output_path,
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

/**
 * What the Judge answered on one step, as criterion rows beneath it.
 *
 * **A refusal is not a failed Check and does not render as one.** The verb and
 * the glyph come from `criterion_verdict_judge`, whose `circle-*` family the
 * icon registry reserves to the Judge — a Check takes `shield-*` — so the
 * silhouette says which source produced the verdict before the word is read.
 *
 * **The number is the criterion's frozen position, not the row's.** A citation
 * names "criterion 4" against `acceptance_criteria[]`, which is frozen at Job
 * creation and only ever appended to, so the ordinal is looked up there rather
 * than counted off this list. A verdict citing an id the Job does not carry
 * keeps its id and loses its number, which is visible; guessing a position
 * would silently break the one reference the Drone retries against.
 */
function verdictsOf(step: StepDetail, criteria: Criterion[]): CriterionVerdict[] {
  return step.judged.map((judged) => {
    const at = criteria.findIndex((c) => c.criterion_id === judged.criterion_id);
    const reading = CRITERION_VERDICT_JUDGE[judged.verdict];
    return {
      ordinal: at < 0 ? undefined : at + 1,
      criterionId: judged.criterion_id,
      text: at < 0 ? undefined : criteria[at]?.text,
      named: judged.verdict,
      // The wire spelling where the registry has no verb, which is recoverable.
      // A word chosen here would be the second vocabulary.
      verdict: reading?.verb ?? judged.verdict,
      icon: reading?.icon ?? undefined,
      ...cited(judged),
    };
  });
}

/**
 * The three fields a refusal owes. Passed through, never joined into a
 * sentence: they arrive named from the Judge record, and composing prose out of
 * them here would be writing copy the Judge did not.
 */
function cited(judged: Judged): Pick<Judged, "expected" | "produced" | "consequence"> {
  return {
    expected: judged.expected,
    produced: judged.produced,
    consequence: judged.consequence,
  };
}
