// The plan as a graph — its groups, and the tasks inside each one. The Plan
// tab's Graph view, beside the List it already had. `#1539`.
//
// **It came off the Workflow canvas whole** (owner, 25 Sep 2026): the layout,
// the cards and the placement are the ones that used to hang under the step
// that recorded the plan. What was left behind is the step row above them, and
// with it the second edge — there are no step nodes here, so nothing can draw
// *and this step worked it*. That cost was stated and taken; Workflow draws one
// Plan node now and pressing it lands here.
//
// **Placement is computed here, from plan order.** The canvas holds none, so
// the numbers below are the layout and they are unit-tested in this package.

import type {
  WorkflowCanvasEdge,
  WorkflowCanvasNode,
  WorkflowStepCardProps,
  StepActivity,
} from "@armada/components";

import type { GroupState, GroupView } from "./draft/group";
import type { TaskState, TaskView } from "./draft/task";

/**
 * The layout, in the canvas's own coordinates.
 *
 * `TASK_ACROSS` is `--w-workflow-group-node` plus room for an arrowhead, so a
 * task column clears its group's card. `TASK_APART` is one task's pitch down
 * that column and `AFTER_GROUP` the gap to the group below. Numbers rather
 * than tokens because React Flow places by number and a `var()` cannot reach
 * it.
 */
const TASK_ACROSS = 256;
const TASK_APART = 104;
const AFTER_GROUP = 24;
/** A group holding no task still takes a row of its own. */
const GROUP_APART = 104;

/** `group:` and `task:`, so a node id is never mistaken for another kind in a join. */
export const groupNodeId = (groupId: string): string => `group:${groupId}`;
export const taskNodeId = (taskId: string): string => `task:${taskId}`;

/** The task a node id names, or nothing where it names a group. */
export function taskOfNodeId(nodeId: string): string | undefined {
  return nodeId.startsWith("task:") ? nodeId.slice("task:".length) : undefined;
}

/** `1 group`, `4 groups`, `2 criteria` — the plural is given where it is not the `s`. */
export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

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
 * The order a plan's own word is chosen in: what is wrong first, then what is
 * moving, then what has not started, then what is done.
 *
 * **A plan has no machine of its own.** `GroupState` is the finest state
 * anything records about the work a plan describes, so the word on the Plan
 * node is one of its groups' words rather than a vocabulary invented for the
 * summary. `pending` outranks `passed` because a plan is not done while a
 * group of it has not started.
 */
const ROLLS_UP: readonly GroupState[] = [
  "failed",
  "retrying",
  "checking",
  "joining",
  "running",
  "pending",
  "passed",
  "landed",
];

/**
 * Where the plan is, as one mark and one word — what the Plan node on the
 * workflow's canvas says about itself.
 *
 * A plan with no group says `pending`: there is nothing to have started.
 */
export function planActivityOf(groups: readonly GroupView[]): { activity: StepActivity; said: GroupState } {
  const said = ROLLS_UP.find((state) => groups.some((group) => group.state === state)) ?? "pending";
  return { activity: GROUP_ACTIVITY[said], said };
}

/** One group's card. Its own word, with the nearest step mark behind it. */
function groupCard(group: GroupView, onOpen: (() => void) | undefined): WorkflowStepCardProps {
  const facts = [{ value: plural(group.tasks.length, "task") }];
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
  const facts = [{ value: task.id }];
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

export type PlanGraphReading = {
  /** The plan's groups, in the order they run. Empty draws nothing. */
  groups: readonly GroupView[];
  /**
   * Opens a task in the sheet the list opens it in — **one destination for one
   * task**, so the toggle changes the arrangement and never what a press does.
   * Absent draws cards that are not controls.
   */
  onOpenTask?: (taskId: string) => void;
  /** The task a person has open, so the card it came from says which one it is. */
  openTask?: string | null;
};

export type PlanGraph = {
  nodes: WorkflowCanvasNode[];
  edges: WorkflowCanvasEdge[];
  /**
   * What the canvas opens on when the whole plan will not read: the groups
   * alone. A task is left out — it is the finest grain on the graph, and what
   * a person pans or presses to once they have found its group.
   */
  opensOn: string[][];
};

/**
 * The plan, placed: a column of groups, each with its own tasks beside it.
 *
 * **A group is a root here.** On Workflow a group hung off the step that wrote
 * it; on this tab there is no step to hang from, so the plan is as many small
 * trees as it has groups.
 */
export function planGraphOf({ groups, onOpenTask, openTask }: PlanGraphReading): PlanGraph {
  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  let down = 0;
  for (const group of groups) {
    const groupId = groupNodeId(group.id);
    nodes.push({ id: groupId, position: { x: 0, y: down }, card: groupCard(group, undefined) });

    group.tasks.forEach((task, at) => {
      const open = onOpenTask === undefined ? undefined : () => onOpenTask(task.id);
      const card = taskCard(task, open);
      nodes.push({
        id: taskNodeId(task.id),
        position: { x: TASK_ACROSS, y: down + at * TASK_APART },
        card: task.id === openTask ? { ...card, selected: true } : card,
      });
      edges.push({
        id: `${groupId}>${taskNodeId(task.id)}`,
        source: groupId,
        target: taskNodeId(task.id),
        kind: "holds",
      });
    });
    down += Math.max(GROUP_APART, group.tasks.length * TASK_APART + AFTER_GROUP);
  }

  return { nodes, edges, opensOn: [groups.map((group) => groupNodeId(group.id))] };
}
