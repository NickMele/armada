// The run, as the workflow the Job froze — placed for the canvas and ordered
// for the stacked column. `#1539`.
//
// **Placement is computed here, from step order.** The canvas holds none, so
// the numbers below are the layout and they are unit-tested in this package.
//
// **The steps are the ones the Job's own workflow file declares** — no
// `verify-and-ship` is added (#1530, 22 Sep).
//
// **One node, two edges into it** (owner, 23 Sep 2026). A group is drawn once,
// where the step that recorded the plan made it, and the step that works it
// draws a second edge in. Two edges is planned and worked; one is planned. The
// cost — crossing edges on a long plan — was taken knowingly, so nothing here
// draws a group twice to avoid it.

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
import type { TaskState, TaskView } from "./draft/task";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";
import { activityOf, stateOf } from "./run";

/**
 * The layout, in the canvas's own coordinates.
 *
 * `STEP_APART` is `--w-workflow-node` plus `--space-12` plus a little, which
 * leaves room for an arrowhead. `TASK_ACROSS` is the group indent plus
 * `--w-workflow-group-node` plus that same room, so a task column clears its
 * group's card. Numbers rather than tokens because React Flow places by number
 * and a `var()` cannot reach it.
 */
const STEP_APART = 320;
const GROUP_INDENT = 16;
const FIRST_GROUP = 150;
const TASK_ACROSS = 272;
/** One task's pitch down its group's column, and the gap to the next group. */
const TASK_APART = 104;
const AFTER_GROUP = 24;
/** A group holding no task still takes a row of its own. */
const GROUP_APART = 104;

/** `step:`, `group:` and `task:`, so a node id is never mistaken for another kind in a join. */
export const stepNodeId = (stepId: string): string => `step:${stepId}`;
export const groupNodeId = (groupId: string): string => `group:${groupId}`;
export const taskNodeId = (taskId: string): string => `task:${taskId}`;

/** The task a node id names, or nothing where it names a step or a group. */
export function taskOfNodeId(nodeId: string): string | undefined {
  return nodeId.startsWith("task:") ? nodeId.slice("task:".length) : undefined;
}

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
   * carrying the groups its steps made or worked. A task is left out: it is
   * the finest grain on the graph, and what a person pans or presses to once
   * they have found its group.
   */
  opensOn: string[][];
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

/**
 * A task's state mapped the same way. `dropped` takes `stopped`, the step
 * machine's word for work that ended without advancing; the task's own word is
 * printed beside the mark either way.
 */
const TASK_ACTIVITY: Record<TaskState, StepActivity> = {
  open: "not_started",
  working: "running",
  done: "advanced",
  failed: "failed",
  dropped: "stopped",
};

/**
 * Whether a group has been worked — anything but `pending`. **This is what the
 * second edge says**, so it is one predicate rather than a condition spelled
 * at each caller.
 */
export const worked = (group: GroupView): boolean => group.state !== "pending";

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
 * One task's card. **The id is the first fact and never the name** — `T5`
 * alone would be a graph of identifiers.
 *
 * **Two facts, because a third wraps and a wrapped card overlaps the one under
 * it.** Turns displace the file count once something has run: the scope is
 * what there is to say before, and what it has taken is what there is to say
 * after.
 */
function taskCard(task: TaskView, onOpen: (() => void) | undefined): WorkflowStepCardProps {
  const facts: WorkflowStepFact[] = [{ value: task.id }];
  if (task.turns !== undefined) facts.push({ value: plural(task.turns, "turn") });
  else if (task.scope.length > 0) facts.push({ value: plural(task.scope.length, "file") });
  return {
    kind: "task",
    name: task.title,
    activity: TASK_ACTIVITY[task.state],
    said: task.state,
    facts,
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
 * Absent where the recording step is the last. The groups are still drawn
 * there — what is missing is the second edge, not the node.
 */
export function stepThatWorksTheGroups(whole: JobWhole): string | undefined {
  const made = stepTheGroupsWereMadeAt(whole);
  if (made === undefined) return undefined;
  const steps = ordered(whole);
  return steps[steps.findIndex((step) => step.step_id === made) + 1]?.step_id;
}

export type WorkflowRunReading = {
  whole: JobWhole;
  /** The plan's groups. Empty draws steps and nothing else. */
  groups: readonly GroupView[];
  /** Opens a step, a group or a task in the inspector. Absent draws cards that are not controls. */
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
  /** The working step's label, for what a group's row says in the stacked run. */
  const workedAt = steps.find((step) => step.step_id === worksAt)?.label;

  const nodes: WorkflowCanvasNode[] = [];
  const rows: WorkflowStackedRow[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  steps.forEach((step, at) => {
    const here = step.step_id === madeAt ? mine : [];
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

    // The groups this step made, with their tasks beside them. The step that
    // works them reaches in with a second edge rather than taking a copy.
    let down = FIRST_GROUP;
    here.forEach((group) => {
      const groupId = groupNodeId(group.id);
      const groupsCard = read(groupId, groupCard(group, opener(groupId)));
      nodes.push({ id: groupId, position: { x: at * STEP_APART + GROUP_INDENT, y: down }, card: groupsCard });
      rows.push({
        id: groupId,
        card: groupsCard,
        under: id,
        depth: 1,
        ...(worked(group) && workedAt !== undefined ? { worked: workedAt } : {}),
      });
      edges.push({ id: `${id}>${groupId}`, source: id, target: groupId, kind: "made" });
      if (worked(group) && worksAt !== undefined) {
        const from = stepNodeId(worksAt);
        edges.push({ id: `${from}>${groupId}`, source: from, target: groupId, kind: "worked" });
      }

      group.tasks.forEach((task, k) => {
        const taskId = taskNodeId(task.id);
        const tasksCard = read(taskId, taskCard(task, opener(taskId)));
        nodes.push({
          id: taskId,
          position: { x: at * STEP_APART + TASK_ACROSS, y: down + k * TASK_APART },
          card: tasksCard,
        });
        rows.push({ id: taskId, card: tasksCard, under: groupId, depth: 2 });
        edges.push({ id: `${groupId}>${taskId}`, source: groupId, target: taskId, kind: "holds" });
      });
      down += Math.max(GROUP_APART, group.tasks.length * TASK_APART + AFTER_GROUP);
    });
  });

  const at = whole.job.current_step_id;
  const running = at !== undefined && steps.some((step) => step.step_id === at) ? stepNodeId(at) : null;
  return { nodes, rows, edges, running, opensOn: opensOn(steps, at, madeAt, worksAt, mine) };
}

/**
 * What to open on, widest first: the step a person is reading with its
 * neighbours, then that step alone, each carrying the groups its steps made or
 * worked.
 *
 * **The last entry keeps its groups even where they will not fit.** A frame
 * that cannot hold one step and its plan is better spent on a clipped picture
 * of that plan than on one card centred in an empty pane.
 */
function opensOn(
  steps: readonly StepDetail[],
  at: string | undefined,
  madeAt: string | undefined,
  worksAt: string | undefined,
  groups: readonly GroupView[],
): string[][] {
  // The plan belongs to both the step that wrote it and the step that works
  // it, **worked or not** — which is why this is not the edge rule. A fit is a
  // choice of what to look at, and from the working step what a person is
  // looking for is the plan, including the groups their turn has not come to.
  const holds = (named: readonly string[]): boolean =>
    (madeAt !== undefined && named.includes(madeAt)) || (worksAt !== undefined && named.includes(worksAt));
  const widening = (named: readonly string[]): string[] => [
    ...named.map(stepNodeId),
    ...(holds(named) ? groups.map((group) => groupNodeId(group.id)) : []),
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
