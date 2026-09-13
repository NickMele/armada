// The run, as a tree, from the steps `GET /jobs/:job_id` serves.
//
// **Not `rail.ts` with a different component.** The rail drew every step's gate
// rows inline; a step's gates are the phase strip's now, in the panel, so what
// this builds is what a step *produced*, *cleared* and *came to* — short facts,
// behind a chevron.
//
// # An attempt is a row, not a counter
//
// `StepDetail.attempts` is every run of the step, oldest first, so the panel's
// timeline draws a section per run rather than one number — and the tree names
// only the run before the live one, while a step is being worked again. It is the only place an earlier run's outcome survives: `state`
// and `last_verdict` are both the latest, so a step that passed on its third
// try and one that passed on its first were the same message on the wire.
//
// # What a step produced is per step, and comes off the socket
//
// `job.files_changed` and `JobDetail.footprint` are the whole Job's, so a
// `Produced` fact built from either would be the same file list under every
// row. The transcript's own reading — taken at the step boundary, stamped with
// its step like every other row — is the per-step answer, and it is what the
// tree draws.

import type { RunTreeFact, RunTreeSkeletonStep, RunTreeStep, StepActivity } from "@armada/components";

import type { Turn, Watched, WorkflowSummary } from "@armada/protocol";
import { CHECK_ADVANCES, CHECK_OUTCOME, CRITERION_VERDICT_CHECK, ESCALATION_REASON, EVIDENCE_TYPE, STEP_STATE } from "@armada/components";
import type {
  ChangedFile,
  CheckRun,
  Criterion,
  EvidenceSubmitted,
  JobDetail as JobWhole,
  Judged,
  StepDetail,
} from "@armada/protocol";
import { isSweepMarker } from "./declared";
import { DIFF_CHAPTER } from "./detail-keys";
import { span } from "./duration";
import { onlyCurrentAttempt, ordered } from "./facts";
import { askedOf, checksOf, checksStand, judgeAsking, panelsFrom } from "./gates";
import { frozenBeneath } from "./frozen";

/**
 * The run before this Job's own read answers: the workflow's steps by name, in
 * order. A Job runs the workflow frozen at dispatch, so where it was edited
 * since these can differ, and the read corrects them when it lands.
 */
export function stepsAhead(workflow: WorkflowSummary | undefined): RunTreeSkeletonStep[] {
  return (workflow?.steps ?? []).map((step) => ({
    id: step.step_id,
    label: step.label,
    labelIsAnIdentifier: step.label === step.step_id || undefined,
  }));
}

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
export function runOf(
  whole: JobWhole,
  now: number,
  selected: string | undefined,
  rows: readonly Turn[],
  /** The moment this Job's Drone handed in, where one has arrived. `#813`. */
  handed?: EvidenceSubmitted,
): RunTreeStep[] {
  const wrote = producedBy(rows);
  const criteria = whole.acceptance_criteria;
  const question = whole.judge_question;
  return ordered(whole).map((step) => {
    const frozen = frozenBeneath(whole.job.status, step.state);
    const activity = frozen?.activity ?? activityOf(step.state);
    const current = step.step_id === (selected ?? whole.job.current_step_id);
    // Scoped to this step, like `JobDetail.tsx`'s own read of it — a question
    // about a different step is not this step's criterion to hold open.
    const asking = question?.step_id === step.step_id ? question.criterion_id : undefined;
    const waiting = handed?.step_id === step.step_id ? handed : undefined;
    const facts = factsOfStep(
      step,
      activity,
      wrote.get(step.step_id) ?? [],
      criteria,
      asking,
      waiting,
      now,
    );
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
const NOTHING_RECORDED = "Nothing recorded yet";

/**
 * The short facts beneath a step: what its Checks came to, what its Judge came
 * to, what the last gate ruled, and where the step now stands.
 *
 * **A fact is a value, never a sentence.** Anything that reads as prose is the
 * panel's, which is the division the tree exists to hold.
 */
function factsOfStep(
  step: StepDetail,
  activity: StepActivity,
  wrote: ChangedFile[],
  criteria: readonly Criterion[],
  /** The criterion a live judge question holds open on this step, where one is. */
  asking?: string,
  /** The moment this step's Drone handed in, where one has arrived. */
  handed?: EvidenceSubmitted,
  now?: number,
): RunTreeFact[] {
  const facts: RunTreeFact[] = [];
  // **A Drone at work is not a gate that failed.** `check_runs` and `judged`
  // hold every attempt and `last_verdict` is the newest ruling, so a step
  // handed back and picked up again drew the gate that refused the *previous*
  // attempt beside a Drone that had just started — three rows in red under a
  // step whose own mark was running, and on the Job this was found on they
  // were not even the same attempt as each other: Checks from attempt 2, Judge
  // from attempt 1, Verdict from attempt 2, over a live attempt 3 whose Checks
  // were all passing. The reading is one attempt's or it is nobody's.
  const working = WORKING.has(activity);
  const live = step.attempts.at(-1)?.attempt;
  // **The attempts are the panel's now.** Each one used to draw here with its
  // Checks, Judge and Verdict nested beneath it, and the step panel draws that
  // same list a few inches to the right — one reading in two columns, which is
  // what the timeline replaced the strip to stop. The tree is the workflow's
  // steps; what happened inside one is the panel's.

  const produced = producedFact(wrote);
  if (produced !== undefined) facts.push(produced);

  // **The step's own gate, flat.** These used to nest under each attempt, and
  // drawing them here as well would have been the "which attempt is this"
  // ambiguity twice over. The panel's timeline holds the per-attempt reading
  // now, so what belongs in the tree is where this step's gate stands.
  //
  // **While the gate runs them, the fact is what is running**, in the words the
  // Checks chapter uses — one reading, two places.
  // **The window the wire used to say nothing in.** A Drone hands in, and the
  // gate starts on Fleet's next turn — until `job.checking` arrives the step
  // drew as one nothing had reached, which is what `#813` names. It goes above
  // the Checks because it is what the Checks are waiting on, and it is gone the
  // moment they start: `step.checking` is the gate's own reading of the same
  // window, one message later.
  const submitted = handedFact(handed, step, working, now);
  if (submitted !== undefined) facts.push(submitted);

  const checks =
    step.checking === undefined
      ? checksFact(step, gateOf(step.check_runs, live, working))
      : checkingFact(step);
  if (checks !== undefined) facts.push(checks);

  const judge = judgeFact(step, gateOf(step.judged, live, working), criteria, asking);
  if (judge !== undefined) facts.push(judge);

  // **The verdict is the live attempt's or it is not drawn.** `last_verdict` is
  // the newest ruling and nothing else, so on a step being worked again it is
  // the ruling that sent the Drone back — a sentence about a run that is over,
  // in the row a person reads for where the step is now.
  const ruled = step.last_verdict;
  if (ruled !== undefined && (!working || ruled.attempt === live)) facts.push(verdictFact(ruled));

  // Served as a field rather than left as a pair to notice: a step reading
  // `advanced` beside a failed verdict is one a person overruled, and a tree
  // that drew only the first would render an overruled gate as a cleared one.
  if (step.overridden) facts.push({ label: "Advanced", value: "overruled by a person" });

  // What the run before this one came to, once its rows have stopped being
  // this step's gate. Neutral, and only while a later attempt is in flight.
  const before = working ? beforeFact(step) : undefined;
  if (before !== undefined) facts.push(before);

  // **On the step that sends the work back**, which is whose pass it is. Not on
  // a step that has not started: its first pass has not begun, and a fact there
  // would hide the row's "nothing recorded yet".
  if (step.pass !== undefined && step.state !== "not_started") {
    facts.push({ label: "Pass", value: `${step.pass.number} of ${step.pass.of}` });
  }

  const stands = standsFact(step, activity);
  if (stands !== undefined) facts.push(stands);

  return facts;
}

/**
 * What the last gate ruled.
 *
 * **`failed` is the wrong word for most of what carries it.** The verdict says
 * the gate stopped; the trigger says what stopped it, and the registry has a
 * verb for every trigger — `evidence disputed`, `the gate could not decide`,
 * `stopped at the gate`. A step where every Check passed and every criterion
 * was met, reading `Verdict failed`, is a screen a person can only conclude is
 * broken, and the trigger is the word that reconciles it.
 */
function verdictFact(verdict: { named: string; trigger?: string }): RunTreeFact {
  return { label: "Verdict", value: ruledSaid(verdict), named: verdict.named };
}

/** A ruling in words: its trigger's verb where it carried one, its own name where not. */
function ruledSaid(verdict: { named: string; trigger?: string }): string {
  return verdict.trigger === undefined
    ? verdict.named
    : (ESCALATION_REASON[verdict.trigger]?.verb ?? verdict.trigger);
}

/** The step states in which a Drone is at work right now. `timeline.tsx`'s own set. */
const WORKING = new Set<StepActivity>(["running", "retrying"]);

/**
 * The gate rows a step's facts are read from.
 *
 * **While a Drone works, the live attempt's and no others** — an attempt whose
 * gate has not run yet has no Checks and no verdicts, and that is the true
 * answer rather than the last attempt's. Empty is what makes `Checks` read
 * `not reached` and `Judge` read `2 declared` on a retry, which is where the
 * step is.
 *
 * **At rest, the newest attempt that answered**, which is `gates.ts`'s rule for
 * every other surface: a step advanced by an overrule carries Checks from the
 * attempt before it, and dropping them there would lose the gate that ran.
 */
function gateOf<T extends { attempt: number }>(
  rows: T[],
  live: number | undefined,
  working: boolean,
): T[] {
  if (!working || live === undefined) return onlyCurrentAttempt(rows);
  return rows.filter((row) => row.attempt === live);
}

/**
 * What the attempt before the live one came to — `Attempt 2 · stopped at the
 * gate`.
 *
 * **Neutral, never hued.** A run that is over is history on a step that is
 * moving, and a red chip on it is the screen saying the step is failing right
 * now. It carries no `named` for that reason.
 *
 * **One row, and it is the previous attempt's.** Every attempt's own record is
 * the timeline's, phase by phase; what the tree owes a person is why this
 * Drone is on its second go, which is one line.
 */
function beforeFact(step: StepDetail): RunTreeFact | undefined {
  const before = step.attempts.at(-2);
  if (before === undefined) return undefined;
  const ruled = step.verdicts.find((one) => one.attempt === before.attempt);
  const said =
    ruled !== undefined
      ? ruledSaid(ruled)
      : before.why !== undefined
        ? (ESCALATION_REASON[before.why]?.verb ?? before.why)
        : (STEP_STATE[before.outcome]?.verb ?? before.outcome);
  return { label: `Attempt ${before.attempt}`, value: said };
}


/**
 * The Drone handed in, and the gate has not started. `#813`.
 *
 * **Only in that window.** `step.checking` present is the gate reading the same
 * moment one message later, and a step no longer being worked has a verdict to
 * read instead — either of them says more than this does, so this stands down.
 *
 * The evidence type is the frozen step's word for what was asked for, so the
 * row says what landed without anybody fetching it.
 */
function handedFact(
  handed: EvidenceSubmitted | undefined,
  step: StepDetail,
  working: boolean,
  now: number | undefined,
): RunTreeFact | undefined {
  if (handed === undefined || !working || step.checking !== undefined) return undefined;
  const ago = now === undefined ? null : span(handed.at, now);
  const kind = EVIDENCE_TYPE[handed.evidence_type]?.verb ?? handed.evidence_type;
  return { label: "Handed in", value: ago === null ? kind : `${kind} · ${ago} ago` };
}

/**
 * What this step wrote, as a count. The files themselves are the Produced
 * chapter's, where there is room for them, and pressing the count opens it.
 */
function producedFact(wrote: ChangedFile[]): RunTreeFact | undefined {
  if (wrote.length === 0) return undefined;
  return {
    label: "Produced",
    value: `${wrote.length} ${wrote.length === 1 ? "file" : "files"}`,
    chapter: DIFF_CHAPTER,
  };
}

/**
 * What each step wrote, folded out of the transcript.
 *
 * **The last reading per step wins.** Fleet takes one at every ruling, so a
 * step submitted three times has three rows and only the newest describes the
 * work as it stands. A row with no step is skipped rather than attributed:
 * nothing recovers which step an unstamped row ran under, and guessing would
 * put one step's files under another's name.
 */
function producedBy(rows: readonly Turn[]): Map<string, ChangedFile[]> {
  const wrote = new Map<string, ChangedFile[]>();
  for (const row of rows) {
    if (row.saw.event !== "produced" || row.step === undefined) continue;
    wrote.set(row.step, row.saw.files);
  }
  return wrote;
}

/**
 * What a step's Checks stand at while the gate runs them — `2 running · 3 of 9
 * passed`. **The gate's own count, from `gates.ts`**, so the rail cannot say a
 * different number from the chapter four inches away.
 */
function checkingFact(step: StepDetail): RunTreeFact {
  const reads = checksOf(step);
  const failed = reads.some((read) => read.run !== undefined && didNotPass(read.run));
  return { label: "Checks", value: checksStand(reads), named: failed ? "failed" : "running" };
}

/**
 * What a step's Checks came to, over the runs handed in.
 *
 * **Absent and empty are two sentences.** `checks` absent is a Fleet that
 * cannot say — the Job names a workflow this Fleet does not hold — and empty is
 * a step that gates on nothing. Neither is "the Checks failed".
 *
 * **`runs` is the caller's to narrow.** A step run once passes every row in
 * `check_runs`; the panel's timeline reads one attempt's rows, so
 * the same rule reads as that attempt's own Checks rather than the whole
 * step's.
 */
function checksFact(step: StepDetail, runs: CheckRun[]): RunTreeFact | undefined {
  if (step.checks === undefined) return { label: "Checks", value: "Fleet cannot say" };
  const declared = step.checks.filter((check) => !isSweepMarker(check));
  if (declared.length === 0) return { label: "Checks", value: "none declared" };
  // **`not reached`, never `not run`.** `not run` is `check_outcome.skipped`'s
  // own verb, and a Check the gate never got to is not one it skipped.
  if (runs.length === 0) {
    return { label: "Checks", value: CRITERION_VERDICT_CHECK.not_reached?.verb ?? "not_reached" };
  }
  const failed = runs.filter(didNotPass);
  return failed.length === 0
    ? { label: "Checks", value: `${runs.length} of ${declared.length} passed`, named: "passed" }
    : {
        label: "Checks",
        // The Check's own verb, from the registry. A word chosen here would be
        // the second vocabulary the generated module exists to prevent.
        value: `${failed[0]!.name} ${CHECK_OUTCOME[failed[0]!.outcome]?.verb ?? failed[0]!.outcome}`,
        named: "failed",
      };
}

/**
 * What a step's Judge came to, over the answers handed in. A declaration
 * until it has answered, and a count once it has — the criterion text and the
 * citation are the panel's, because each of them is a sentence.
 *
 * **`judged` is the caller's to narrow**, for [`checksFact`]'s reason. **The
 * count is `gates.ts`'s `panelsFrom`, the strip's own reading** — criteria,
 * never rows, and a live question held open counts as neither met nor
 * refused. Reading `judged` straight was the strip and the rail agreeing on a
 * clean gate and disagreeing the moment a question was open. #689.
 */
function judgeFact(
  step: StepDetail,
  judged: Judged[],
  criteria: readonly Criterion[],
  asking?: string,
): RunTreeFact | undefined {
  const declared = step.judge_checks;
  if (declared === undefined) return undefined;
  if (judged.length === 0 && asking === undefined) {
    // **A call still out is not a gate that has not been reached.** `judging`
    // is Fleet's own "right now", and the sentence is `gates.ts`'s so the
    // timeline's Judge row four inches away cannot word it differently.
    if (step.judging !== undefined) return { label: "Judge", value: judgeAsking(step) };
    // **Criteria, not declarations.** One Judge declaration asks two criteria
    // on the step this was found on, and `1 declared` beside the timeline's
    // `2 criteria, not asked` was two counts of one thing. `askedOf` is that one.
    const asked = askedOf(step);
    return asked === 0
      ? undefined
      : { label: "Judge", value: `${asked} ${asked === 1 ? "criterion" : "criteria"}, not asked` };
  }
  const panels = panelsFrom(judged, criteria, asking);
  const refused = panels.filter((one) => one.verdict === "not_met").length;
  const stillAsking = panels.filter((one) => one.verdict === "asking").length;
  const met = panels.length - refused - stillAsking;
  return {
    label: "Judge",
    value: `${met} of ${panels.length} met`,
    named: refused > 0 ? "failed" : stillAsking > 0 ? undefined : "passed",
  };
}

/**
 * Where the step now stands, as the drawing's last fact row: `Waiting on you`,
 * `Held the gate could not decide`, `Job completed_failed`.
 *
 * **Three kinds of stopped, and they never share a row.** Waiting on you is the
 * workflow working, stopped is a Drone that cannot get further, failed is over.
 */
function standsFact(step: StepDetail, activity: StepActivity): RunTreeFact | undefined {
  if (activity === "awaiting_human") return { label: "Waiting", value: "on you" };
  if (activity === "stopped") return { label: "Held", value: `${heldOn(step)} · waiting on you` };
  if (activity === "failed") return { label: "Job", value: "ended here", named: "failed" };
  if (activity === "killed") return { label: "Killed", value: "by a person" };
  return undefined;
}

/**
 * What held a stopped step, read off the attempt that actually stopped it.
 *
 * **"Retries spent" only where they were.** `gate_failure` is the trigger
 * `escalation-triggers.toml` documents as the retry limit run out; every other
 * step-level trigger — `gate_undecided` chief among them — fires once and is
 * never retried, so telling a person their retries are spent on attempt one is
 * false on the record in front of them. Everything that is not `gate_failure`
 * takes the registry's own verb for the trigger, the same fallback
 * `verdictFact` already takes.
 */
function heldOn(step: StepDetail): string {
  const why = step.attempts[step.attempts.length - 1]?.why;
  if (why === undefined || why === "gate_failure") return "retries spent";
  return ESCALATION_REASON[why]?.verb ?? why;
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
 * The word is `enum-verbs.toml`'s, through `STEP_STATE`; a state this build's
 * registry has no row for falls back to its own wire spelling.
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

/**
 * Why the run has no rows, which is never the same sentence twice. Beside the
 * run it explains, out of `JobDetail.tsx` at the 900-line line.
 */
export function whyNoSteps(watched: Watched, jobId: string): string | undefined {
  if (watched.state === "read" && watched.jobId === jobId) {
    return watched.detail.steps.length === 0
      ? "This Job's frozen workflow has no steps."
      : undefined;
  }
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer";
  }
  return "Reading this Job.";
}
