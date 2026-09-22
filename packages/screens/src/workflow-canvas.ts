// The run, as the workflow the Job froze — placed for the canvas and ordered
// for the stacked column. `#1539`.
//
// **Placement is computed here, from step order.** The canvas holds none: that
// is the whole difference between it and a Studio's whiteboard, where a person
// puts a node down and it stays. So the numbers below are the layout, and they
// are here rather than in the component because arithmetic is unit-tested in
// this package.
//
// **The steps are the ones the Job's own workflow file declares.** There is no
// `verify-and-ship` and none is added (#1530, 22 Sep) — every step drawn comes
// off `JobDetail.steps`, which is the frozen workflow.

import { ADVANCE_GATE } from "@armada/components";
import type {
  WorkflowCanvasEdge,
  WorkflowCanvasNode,
  WorkflowStackedRow,
  WorkflowStepCardProps,
  WorkflowStepFact,
  StepActivity,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { isSweepMarker } from "./declared";
import type { GroupState, GroupView } from "./draft/group";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";
import { activityOf, stateOf } from "./run";

/**
 * The layout, in the canvas's own coordinates.
 *
 * `STEP_APART` is `--w-workflow-node` plus `--space-12` plus a little, which is
 * what leaves room for an arrowhead between two steps; `GROUP_APART` is a
 * group's card plus its gap. Numbers rather than tokens because React Flow
 * places by number and a `var()` cannot reach it.
 */
const STEP_APART = 320;
const GROUP_INDENT = 16;
const FIRST_GROUP = 150;
const GROUP_APART = 104;

/** `step:` and `group:`, so a node id is never mistaken for the other kind in a join. */
export const stepNodeId = (stepId: string): string => `step:${stepId}`;
export const groupNodeId = (groupId: string): string => `group:${groupId}`;

/** The run, in both arrangements, off one reading. */
export type WorkflowRun = {
  nodes: WorkflowCanvasNode[];
  rows: WorkflowStackedRow[];
  edges: WorkflowCanvasEdge[];
  /** The node the Job is on, for *Stay on the running step*. */
  running: string | null;
  /**
   * What a narrow window opens on: the step a person is on and its two
   * neighbours. Fitting nine cards into 768px draws nine cards nobody can read.
   */
  opensOn: string[];
};

/**
 * A group's state mapped to the nearest step activity, for the mark alone.
 *
 * **The group's own word is kept and printed beside it**, so nothing is lost:
 * `joining` and `checking` are not step states and the step machine has no
 * mark for either. This is a drawing decision, which is why it is here and not
 * in the card.
 */
const GROUP_ACTIVITY: Record<GroupState, StepActivity> = {
  pending: "not_started",
  running: "running",
  joining: "running",
  checking: "running",
  passed: "advanced",
  failed: "failed",
  retrying: "retrying",
  landed: "advanced",
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

/** `1 group`, `4 groups`, `2 criteria` — the plural is given where it is not the `s`. */
function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

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

/** One group's card. Its own word, with the nearest step mark behind it. */
function groupCard(group: GroupView, onOpen: (() => void) | undefined): WorkflowStepCardProps {
  const facts: WorkflowStepFact[] = [{ value: plural(group.tasks.length, "task") }];
  if (group.checks_selected.length > 0) facts.push({ value: plural(group.checks_selected.length, "check") });
  if (group.concurrent) facts.push({ value: "at the same time" });
  if (group.retry_count > 0) facts.push({ value: `run again ${plural(group.retry_count, "time")}` });
  return {
    kind: "group",
    name: `Group ${group.ordinal}`,
    activity: GROUP_ACTIVITY[group.state],
    said: group.state,
    facts,
    ...(onOpen === undefined ? {} : { onOpen }),
  };
}

/**
 * Which step the groups hang under: the one after the step the plan was
 * recorded at, since a plan is written at one step and worked at the next.
 *
 * Absent where no plan was recorded, or where the recording step is the last —
 * a plan with nowhere to be worked draws no groups rather than hanging them
 * off the wrong step.
 */
export function stepTheGroupsHangUnder(whole: JobWhole): string | undefined {
  const at = whole.work_plan?.recorded_by;
  if (at === undefined || at.by !== "step") return undefined;
  const steps = ordered(whole);
  const where = steps.findIndex((step) => step.step_id === at.step_id);
  return where === -1 ? undefined : steps[where + 1]?.step_id;
}

/** The step a person is reading and its two neighbours, as node ids. */
function neighbours(steps: StepDetail[], at: string | undefined): string[] {
  const where = steps.findIndex((step) => step.step_id === at);
  if (where === -1) return steps.map((step) => stepNodeId(step.step_id));
  return steps
    .slice(Math.max(0, where - 1), where + 2)
    .map((step) => stepNodeId(step.step_id));
}

export type WorkflowRunReading = {
  whole: JobWhole;
  /** The groups under the implement step. Empty draws steps and nothing else. */
  groups: readonly GroupView[];
  /** Opens a step or a group in the inspector. Absent draws cards that are not controls. */
  onOpen?: (nodeId: string) => void;
  /**
   * The step's groups are opened under the run rather than hung on it — the
   * implement board, `#1536`. The step still says how many it holds; the cards
   * would be the same groups drawn twice, and they are what makes the run too
   * tall to fit once the canvas is drawn small.
   */
  opened?: boolean;
};

/**
 * The whole run, placed. **One derivation for both arrangements**, so the
 * toggle changes the shape of the page and never what a step says.
 */
export function workflowRunOf({ whole, groups, onOpen, opened }: WorkflowRunReading): WorkflowRun {
  const steps = ordered(whole);
  const under = stepTheGroupsHangUnder(whole);
  const opener = (id: string) => (onOpen === undefined ? undefined : () => onOpen(id));

  const nodes: WorkflowCanvasNode[] = [];
  const rows: WorkflowStackedRow[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  steps.forEach((step, at) => {
    const mine = step.step_id === under ? groups : [];
    const id = stepNodeId(step.step_id);
    const card = stepCard(whole, step, mine.length, opener(id));
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
        returning: true,
        label: loop.label,
      });
    }

    const next = steps[at + 1];
    if (next !== undefined) {
      edges.push({ id: `${step.step_id}>${next.step_id}`, source: id, target: stepNodeId(next.step_id) });
    }

    if (opened === true) return;
    mine.forEach((group, j) => {
      const groupId = groupNodeId(group.id);
      const groupsCard = groupCard(group, opener(groupId));
      nodes.push({
        id: groupId,
        position: { x: at * STEP_APART + GROUP_INDENT, y: FIRST_GROUP + j * GROUP_APART },
        card: groupsCard,
      });
      rows.push({ id: groupId, card: groupsCard, under: id });
      const from = j === 0 ? id : groupNodeId(mine[j - 1]!.id);
      edges.push({ id: `${from}>${groupId}`, source: from, target: groupId });
    });
  });

  const at = whole.job.current_step_id;
  const running = at !== undefined && steps.some((step) => step.step_id === at) ? stepNodeId(at) : null;
  return { nodes, rows, edges, running, opensOn: neighbours(steps, at) };
}
