// The workflow overview a Job waiting for approval draws in place of the idle
// step view — one box per step, read off the same declarations `preview.ts`
// draws the rail from, so the two cannot say different things. #1149.

import type { JobDetailField, WorkflowDiagramStep } from "@armada/components";
import { ADVANCE_GATE } from "@armada/components";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";

import { commandOf, coversOf, isSweepMarker, judgeOf } from "./declared";
import { cap } from "./RaiseCap";
import { ordered } from "./facts";

const AWAITING_APPROVAL = "awaiting_approval";
const HUMAN_ALWAYS = "human_always";
const AUTO = "auto";

/** Whether the step's own gate waits for a person, or moves the Job on. */
function waits(gate: string | undefined): boolean {
  return gate === HUMAN_ALWAYS;
}

/** The gate's own word. Absent on `auto`, which draws no row — `declared.ts`'s
 * own rule for the same registry, one level up from a single declaration. */
function gateLabelOf(gate: string | undefined): string | undefined {
  if (gate === undefined || gate === AUTO) return undefined;
  return ADVANCE_GATE[gate]?.verb ?? gate;
}

/** One step's box: its Checks and its Judge, never its gate — the gate is the
 * marker the diagram draws after the box, not a row inside it. */
function boxOf(step: StepDetail): WorkflowDiagramStep {
  const checks = (step.checks ?? []).filter((check) => !isSweepMarker(check));
  const judged = step.judge_checks ?? [];
  return {
    id: step.step_id,
    label: step.label,
    labelIsAnIdentifier: step.label === step.step_id || undefined,
    checks: checks.length === 0 ? undefined : checks.map((check) => ({ command: commandOf(check), covers: coversOf(check) })),
    declarations: judged.length === 0 ? undefined : judged.map((judge) => ({ label: judgeOf(judge) })),
    gate: waits(step.advance_gate) ? "person" : "auto",
    gateLabel: gateLabelOf(step.advance_gate),
    loop:
      step.verdict_routing_target === undefined || step.pass === undefined
        ? undefined
        : { to: step.verdict_routing_target, label: `up to ${step.pass.of} passes` },
  };
}

export type ApprovalOverview = {
  diagram: WorkflowDiagramStep[];
  /** Where the Job stops, its caps and its model — beside the diagram. */
  facts: JobDetailField[];
};

/** Beside the diagram: where the Job stops, its caps and its model. Cost cap
 * is absent on a Fleet that does not count, `spendFact`'s own reason; the
 * other two are always on `JobDetail.spend` and `JobSummary.model`. */
function factsOf(job: JobSummary, whole: JobWhole, steps: StepDetail[]): JobDetailField[] {
  const stops = steps.filter((step) => waits(step.advance_gate)).map((step) => step.label);
  const facts: JobDetailField[] = [];
  if (stops.length > 0) facts.push({ label: "Stops for a person", value: stops.join(", ") });
  if (whole.spend !== undefined) {
    facts.push({ label: "Cost cap", value: cap(whole.spend.cost_cap_micros), mono: true });
    facts.push({ label: "Turn cap", value: String(whole.spend.turn_cap), mono: true });
  }
  facts.push({ label: "Model", value: job.model, mono: true });
  return facts;
}

/**
 * The overview, or `undefined` where there is none to draw — every status but
 * `awaiting_approval`, and a Job whose steps have not arrived yet.
 */
export function approvalOverviewOf(job: JobSummary, whole: JobWhole | null): ApprovalOverview | undefined {
  if (job.status !== AWAITING_APPROVAL || whole === null) return undefined;
  const steps = ordered(whole);
  if (steps.length === 0) return undefined;
  return {
    diagram: steps.map(boxOf),
    facts: factsOf(job, whole, steps),
  };
}
