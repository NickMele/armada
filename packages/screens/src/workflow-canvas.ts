// The run, as the workflow the Job froze — placed for the canvas and ordered
// for the stacked column. `#1539`.
//
// **Placement is computed here, from step order.** The canvas holds none, so
// the numbers below are the layout and they are unit-tested in this package.
//
// **The steps are the ones the Job's own workflow file declares** — no
// `verify-and-ship` is added (#1530, 22 Sep).
//
// **The steps, and one Plan node** (owner, 25 Sep 2026). The plan used to hang
// off the step that recorded it, a node per group and a node per task, with a
// second edge in from the step that worked each group. It is one node now, and
// pressing it opens the Plan tab where the whole plan is drawn —
// `plan-canvas.ts`. The second edge died with the group nodes: nothing on this
// canvas is left for it to arrive at, and Plan has no step node to leave.

import { ADVANCE_GATE } from "@armada/components";
import type {
  WorkflowCanvasEdge,
  WorkflowCanvasNode,
  WorkflowStackedRow,
  WorkflowStepCardProps,
  WorkflowStepFact,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { isSweepMarker } from "./declared";
import type { GroupView } from "./draft/group";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";
import { planActivityOf, plural } from "./plan-canvas";
import { activityOf, stateOf } from "./run";

/**
 * The layout, in the canvas's own coordinates.
 *
 * `STEP_APART` is `--w-workflow-node` plus `--space-12` plus a little, which
 * leaves room for an arrowhead. The Plan node sits `PLAN_BELOW` under the step
 * that recorded the plan, indented by `PLAN_INDENT` so it reads as inside that
 * step rather than beside the next one. Numbers rather than tokens because
 * React Flow places by number and a `var()` cannot reach it.
 */
const STEP_APART = 320;
const PLAN_INDENT = 16;
const PLAN_BELOW = 150;

/** `step:`, so a node id is never mistaken for another kind in a join. */
export const stepNodeId = (stepId: string): string => `step:${stepId}`;

/**
 * The one node the plan takes on this canvas. **Not `plan:<id>`** — a Job has
 * one plan, so there is nothing for an id to tell apart.
 */
export const PLAN_NODE_ID = "plan";

/** The run, in both arrangements, off one reading. */
export type WorkflowRun = {
  nodes: WorkflowCanvasNode[];
  rows: WorkflowStackedRow[];
  edges: WorkflowCanvasEdge[];
  /** The node the Job is on, for *Stay on the running step*. */
  running: string | null;
  /**
   * What the canvas opens on when the whole run will not read, widest first —
   * the step a person is on with its neighbours, then that step alone, each
   * carrying the Plan node where one of those steps made or works it.
   */
  opensOn: string[][];
};

/** The gate's own word. Absent on `auto`, which says nothing worth a row. */
function gateOf(gate: string | undefined): string | undefined {
  if (gate === undefined || gate === "auto") return undefined;
  return ADVANCE_GATE[gate]?.verb ?? gate;
}

/** The Checks a step declares, less the markers that name no command. */
const checksOf = (step: StepDetail): number => (step.checks ?? []).filter((check) => !isSweepMarker(check)).length;

/** What the Judge is asked here, counted. */
const criteriaOf = (step: StepDetail): number =>
  (step.judge_checks ?? []).reduce((sum, judge) => sum + judge.criteria, 0);

/** One step's card. Facts are values; the gate is the one sentence on it. */
function stepCard(
  whole: JobWhole,
  step: StepDetail,
  groups: number,
  onOpen: (() => void) | undefined,
): WorkflowStepCardProps {
  const frozen = frozenBeneath(whole.job.status, step.state);
  const activity = frozen?.activity ?? activityOf(step.state);
  const facts: WorkflowStepFact[] = [];
  if (groups > 0) facts.push({ value: plural(groups, "group") });
  const checks = checksOf(step);
  if (checks > 0) facts.push({ value: plural(checks, "check") });
  const criteria = criteriaOf(step);
  if (criteria > 0) facts.push({ value: plural(criteria, "criterion", "criteria") });
  if (step.attempts.length > 1) facts.push({ value: `attempt ${step.attempts.length}` });
  if (step.delivers === true) facts.push({ value: "delivers" });
  const gate = gateOf(step.advance_gate);
  return {
    kind: "step",
    name: step.label,
    ...(step.label === step.step_id ? { nameIsAnIdentifier: true } : {}),
    activity,
    said: frozen?.word ?? stateOf(step),
    ordinal: step.ordinal,
    facts,
    current: step.step_id === whole.job.current_step_id && frozen === undefined,
    ...(gate === undefined ? {} : { gate }),
    ...(onOpen === undefined ? {} : { onOpen }),
  };
}

/**
 * The step that made the groups: the one the plan was recorded at.
 *
 * Absent where no plan was recorded and where a person wrote it — a plan no
 * step produced has no node to come off, so its groups are left undrawn rather
 * than hung somewhere they were not made.
 */
export function stepTheGroupsWereMadeAt(whole: JobWhole): string | undefined {
  const at = whole.work_plan?.recorded_by;
  if (at === undefined || at.by !== "step") return undefined;
  return ordered(whole).some((step) => step.step_id === at.step_id) ? at.step_id : undefined;
}

/**
 * The step that works the groups: the one after the step the plan was recorded
 * at, since a plan is written at one step and worked at the next.
 *
 * Absent where the recording step is the last. **Nothing on this canvas comes
 * off it any more** — it is read for what a step's own card counts, and for
 * the board under the run.
 */
export function stepThatWorksTheGroups(whole: JobWhole): string | undefined {
  const made = stepTheGroupsWereMadeAt(whole);
  if (made === undefined) return undefined;
  const steps = ordered(whole);
  return steps[steps.findIndex((step) => step.step_id === made) + 1]?.step_id;
}

/**
 * The plan's one card: what it is made of, and where its groups have got to.
 *
 * **The group count and the task count, and nothing else** — the whole plan is
 * one press away, so a second line here would be the reading this node exists
 * to stop drawing twice.
 */
function planCard(groups: readonly GroupView[], onOpen: (() => void) | undefined): WorkflowStepCardProps {
  const { activity, said } = planActivityOf(groups);
  const tasks = groups.reduce((sum, group) => sum + group.tasks.length, 0);
  return {
    kind: "plan",
    name: "Plan",
    activity,
    said,
    facts: [{ value: plural(groups.length, "group") }, { value: plural(tasks, "task") }],
    ...(onOpen === undefined ? {} : { onOpen }),
  };
}

export type WorkflowRunReading = {
  whole: JobWhole;
  /** The plan's groups, for the Plan node's summary. Empty draws steps and nothing else. */
  groups: readonly GroupView[];
  /**
   * Opens a step in the inspector, or — on `PLAN_NODE_ID` — the Plan tab.
   * Absent draws cards that are not controls.
   */
  onOpen?: (nodeId: string) => void;
  /** The node a person has open, so the card being read says which one it is. */
  selected?: string | null;
};

/**
 * The whole run, placed. **One derivation for both arrangements**, so the
 * toggle changes the shape of the page and never what a step says.
 */
export function workflowRunOf({ whole, groups, onOpen, selected }: WorkflowRunReading): WorkflowRun {
  const steps = ordered(whole);
  const madeAt = stepTheGroupsWereMadeAt(whole);
  const worksAt = stepThatWorksTheGroups(whole);
  const mine = madeAt === undefined ? [] : groups;
  const opener = (id: string) => (onOpen === undefined ? undefined : () => onOpen(id));
  const read = (id: string, card: WorkflowStepCardProps): WorkflowStepCardProps =>
    id === selected ? { ...card, selected: true } : card;

  const nodes: WorkflowCanvasNode[] = [];
  const rows: WorkflowStackedRow[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  steps.forEach((step, at) => {
    const id = stepNodeId(step.step_id);
    // The step that works the groups counts them too — it made none, and a
    // card saying nothing about them would read as a step with no work in it.
    const counts = step.step_id === madeAt || step.step_id === worksAt ? mine.length : 0;
    const card = read(id, stepCard(whole, step, counts, opener(id)));
    nodes.push({ id, position: { x: at * STEP_APART, y: 0 }, card });

    const loop =
      step.verdict_routing_target === undefined || step.pass === undefined
        ? undefined
        : { to: step.verdict_routing_target, label: `up to ${step.pass.of} passes` };
    const back = loop === undefined ? undefined : steps.find((one) => one.step_id === loop.to);
    rows.push({
      id,
      card,
      ...(back === undefined || loop === undefined
        ? {}
        : { returns: { toName: back.label, label: loop.label } }),
    });
    if (loop !== undefined && back !== undefined) {
      edges.push({
        id: `${step.step_id}>${loop.to}`,
        source: id,
        target: stepNodeId(loop.to),
        kind: "returns",
        label: loop.label,
      });
    }

    const next = steps[at + 1];
    if (next !== undefined) {
      edges.push({
        id: `${step.step_id}>${next.step_id}`,
        source: id,
        target: stepNodeId(next.step_id),
        kind: "leads",
      });
    }

    // The plan this step recorded, as one node. It hangs off the step that
    // made it and off nothing else: the step that works it says how many
    // groups it has on its own card, which is the whole of what this canvas
    // now says about the relation.
    if (step.step_id === madeAt && mine.length > 0) {
      const card = read(PLAN_NODE_ID, planCard(mine, opener(PLAN_NODE_ID)));
      nodes.push({
        id: PLAN_NODE_ID,
        position: { x: at * STEP_APART + PLAN_INDENT, y: PLAN_BELOW },
        card,
      });
      rows.push({ id: PLAN_NODE_ID, card, under: id, depth: 1 });
      edges.push({ id: `${id}>${PLAN_NODE_ID}`, source: id, target: PLAN_NODE_ID, kind: "made" });
    }
  });

  const at = whole.job.current_step_id;
  const running = at !== undefined && steps.some((step) => step.step_id === at) ? stepNodeId(at) : null;
  const drawsAPlan = madeAt !== undefined && mine.length > 0;
  return { nodes, rows, edges, running, opensOn: opensOn(steps, at, madeAt, worksAt, drawsAPlan) };
}

/**
 * What to open on, widest first: the step a person is reading with its
 * neighbours, then that step alone, each carrying the Plan node where one of
 * those steps made or works it.
 */
function opensOn(
  steps: readonly StepDetail[],
  at: string | undefined,
  madeAt: string | undefined,
  worksAt: string | undefined,
  drawsAPlan: boolean,
): string[][] {
  // The plan belongs to both the step that wrote it and the step that works
  // it, **worked or not**. A fit is a choice of what to look at, and from the
  // working step what a person is looking for is the plan.
  const holds = (named: readonly string[]): boolean =>
    drawsAPlan &&
    ((madeAt !== undefined && named.includes(madeAt)) || (worksAt !== undefined && named.includes(worksAt)));
  const widening = (named: readonly string[]): string[] => [
    ...named.map(stepNodeId),
    ...(holds(named) ? [PLAN_NODE_ID] : []),
  ];

  const where = steps.findIndex((step) => step.step_id === at);
  if (where === -1) {
    const every = steps.map((step) => step.step_id);
    return [widening(every), every.map(stepNodeId)];
  }
  const here = steps[where]!.step_id;
  const near = steps.slice(Math.max(0, where - 1), where + 2).map((step) => step.step_id);
  return [widening(near), widening([here])];
}
