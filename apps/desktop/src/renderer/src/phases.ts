// Where a step is — its phases and its gate tiers, from the one `StepDetail`
// the panel is showing.
//
// **The phases are derived and the tiers are served.** `job_steps.state` says
// whether the step is instructed, working or submitted; `checks`, `check_runs`,
// `judge_checks`, `judged` and `advance_gate` say what looks at it afterwards.
// Nothing here invents a tier: a step declaring no Check and no Judge draws
// three stages and a sentence saying what does advance it, which is the whole
// of "an absent tier is not a failed tier".

import type { PhaseStage, PhaseStageRow, PhaseStripProps } from "@armada/components";

import { ADVANCE_GATE, CHECK_ADVANCES, CHECK_OUTCOME } from "../../shared/generated/vocabulary";
import type { CheckRun, Criterion, StepDetail } from "../../shared/protocol";
import { commandOf, nameOf } from "./declared";

/** The gate value whose whole meaning is "the Checks are the whole gate". */
const AUTO = "auto";
/** The gate value that holds the Job for a person whatever the machines said. */
const HUMAN = "human_always";

/**
 * The strip for one step.
 *
 * **The three phases are one progression with the tiers, not a marker beside
 * them.** A step that has been submitted and is waiting on a Check is not in
 * two places, and drawing them apart made a reader hold two readings of one
 * fact.
 */
export function phasesOf(step: StepDetail, criteria: Criterion[]): PhaseStripProps {
  const submitted = HAS_SUBMITTED.has(step.state);
  const working = step.state === "running" || step.state === "retrying";
  const stages: PhaseStage[] = [
    {
      id: "instructed",
      label: "Instructed",
      state: step.state === "not_started" ? "ahead" : "cleared",
      stands: step.state === "not_started" ? "not reached" : undefined,
      detail: "Armada wrote the step's brief and the Drone opened with it.",
    },
    {
      id: "working",
      label: "Working",
      state: working ? "current" : submitted ? "cleared" : "ahead",
      detail: "The Drone has the step and is taking turns against it.",
    },
    {
      id: "submitted",
      label: "Submitted",
      state: submitted ? "cleared" : "ahead",
      stands: submitted ? undefined : "nothing submitted",
      detail:
        "The Drone reported the step complete through the Evidence tool. What it claimed is a " +
        "signal; everything that gates is computed on Fleet's side.",
    },
  ];

  const checks = checksStage(step);
  if (checks !== undefined) stages.push(checks);

  const judge = judgeStage(step, criteria);
  if (judge !== undefined) stages.push(judge);

  const you = youStage(step);
  if (you !== undefined) stages.push(you);

  return { stages, note: noteOf(step, checks !== undefined, judge !== undefined, you !== undefined) };
}

/** The step states that mean the Drone has handed the work over. */
const HAS_SUBMITTED: ReadonlySet<string> = new Set([
  "awaiting_human",
  "advanced",
  "stopped",
]);

/**
 * The Checks tier, or none.
 *
 * **The tier names its commands rather than counting them while two fit.** Past
 * three it counts, because six commands on one control is a paragraph in a
 * strip.
 */
function checksStage(step: StepDetail): PhaseStage | undefined {
  const declared = step.checks;
  if (declared === undefined || declared.length === 0) return undefined;

  const rows: PhaseStageRow[] = declared.map((check) => {
    const run = step.check_runs.find((ran) => ran.name === nameOf(check));
    return {
      label: commandOf(check),
      mono: true,
      result: run === undefined ? "not run" : resultOf(run),
      named: run === undefined ? undefined : didNotPass(run) ? "failed" : "passed",
    };
  });

  const runs = step.check_runs;
  const failed = runs.filter(didNotPass);
  const label =
    declared.length > 2
      ? `${declared.length} Checks`
      : declared.map((check) => nameOf(check)).join(", ");

  return {
    id: "checks",
    label,
    kind: "checks",
    state: failed.length > 0 ? "failed" : runs.length === declared.length ? "cleared" : runs.length > 0 ? "current" : "ahead",
    stands:
      runs.length === 0
        ? "not run"
        : failed.length > 0
          ? `${failed.length} of ${declared.length} did not pass`
          : `${runs.length} of ${declared.length} passed`,
    rows,
  };
}

/**
 * The Judge tier, or none.
 *
 * **A cleared tier reports the criteria it met**, because that is the reason to
 * trust it, and a declared one says how many it will answer — which is what it
 * will report against.
 */
function judgeStage(step: StepDetail, criteria: Criterion[]): PhaseStage | undefined {
  const declared = step.judge_checks;
  if (declared === undefined || declared.length === 0) return undefined;

  const asked = declared.reduce((sum, judge) => sum + judge.criteria, 0);
  const rows: PhaseStageRow[] = step.judged.map((judged) => {
    const criterion = criteria.find((held) => held.criterion_id === judged.criterion_id);
    return {
      // The criterion's own text where the Job carries it, and its id where it
      // does not. A criterion is a sentence somebody wrote, so it is not mono;
      // an id that could not be joined is machine-derived, so it is.
      label: criterion?.text ?? judged.criterion_id,
      mono: criterion === undefined || undefined,
      result: judged.verdict,
      named: judged.verdict,
    };
  });

  if (step.judged.length === 0) {
    return {
      id: "judge",
      label: asked === 0 ? "Judge" : `Judge · ${asked} ${asked === 1 ? "criterion" : "criteria"}`,
      kind: "judge",
      state: "ahead",
      stands: step.judging === undefined ? "not reached" : `asking · ${step.judging.look}`,
      rows,
    };
  }

  const met = step.judged.filter((judged) => judged.verdict === "met").length;
  const refused = step.judged.length - met;
  return {
    id: "judge",
    label:
      refused === 0
        ? `Judge · ${met} of ${step.judged.length} met`
        : `Judge · ${refused} of ${step.judged.length} refused`,
    kind: "judge",
    state: refused === 0 ? "cleared" : "failed",
    stands: refused === 0 ? `${met} of ${step.judged.length} met` : `${refused} refused`,
    rows,
  };
}

/**
 * The human tier, where the step declares one.
 *
 * **`auto` draws no stage**, because the Checks above are the whole gate and a
 * stage on every step of every workflow would bury the two values that matter.
 * A gate the registry does not spell draws as itself rather than being dropped:
 * a tier nobody can name is still a tier that will halt the Job.
 */
function youStage(step: StepDetail): PhaseStage | undefined {
  const gate = step.advance_gate;
  if (gate === undefined || gate === AUTO || gate === "auto_if_judge_passes") return undefined;
  const waiting = step.state === "awaiting_human";
  return {
    id: "you",
    label: gate === HUMAN ? "You" : `You · ${ADVANCE_GATE[gate]?.verb ?? gate}`,
    kind: "human",
    state: waiting ? "waiting" : step.state === "advanced" ? "cleared" : "ahead",
    stands: waiting ? "waiting on you" : undefined,
    // Where `advance_gate` is a manifest rule, the tier resolved at dispatch
    // from the Manifest's own policy — so two Jobs on one workflow can show
    // different gates. Naming the value is what says why.
    detail:
      gate === HUMAN
        ? undefined
        : `This step's gate is ${gate}, resolved when the Job was dispatched.`,
  };
}

/**
 * The sentence beneath the strip. **This is what an ungated step says instead
 * of an empty gate**, so it is not decoration — a greyed-out tier reads as a
 * gate that failed to render.
 */
function noteOf(step: StepDetail, checks: boolean, judge: boolean, you: boolean): string {
  if (!checks && !judge && !you) {
    return step.checks === undefined
      ? "Fleet cannot say what gates this step, because it does not hold the workflow this Job named."
      : "This step declares no Check and asks no Judge. Its evidence advances it, and nothing else.";
  }
  if (step.state === "awaiting_human") {
    return "Everything mechanical has cleared. Nothing is wrong; the workflow asks for a person here.";
  }
  if (step.state === "running" || step.state === "retrying") {
    return step.check_runs.length === 0
      ? "The Drone is working. Nothing has been submitted, so no gate has been asked anything yet."
      : "The gate has run and the Drone has the step back. The tiers behind it are still ahead, not cancelled.";
  }
  if (step.state === "not_started") return "Nothing has reached this step yet.";
  return "";
}

/** What one Check did, in the registry's own verb. */
function resultOf(run: CheckRun): string {
  const outcome = CHECK_OUTCOME[run.outcome]?.verb ?? run.outcome;
  const measured = [run.expected, run.produced].filter((part) => part !== undefined);
  return measured.length === 0 ? outcome : `${outcome} · ${measured.join(" → ")}`;
}

/** A Check that did not pass, read off the registry's own `advances`. */
function didNotPass(run: CheckRun): boolean {
  return CHECK_ADVANCES[run.outcome] === false;
}
