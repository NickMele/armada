// What the Workflow tab's inspector reads for the step or group a person has
// open. `#1539`.
//
// **What it cannot answer, it says.** The cases a group owes are a draft shape
// Fleet does not serve (`draft/group.ts`, `cases_at_boundary`), so the tests
// region draws the reason rather than an empty list that reads as "none owed".

import type {
  WorkflowInspectorCheck,
  WorkflowInspectorProps,
  WorkflowInspectorTask,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { isSweepMarker, nameOf } from "./declared";
import type { GroupView } from "./draft/group";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";
import { activityOf, stateOf } from "./run";
import { groupNodeId, stepNodeId } from "./workflow-canvas";

/** What the inspector draws, less the two controls the tab wires itself. */
export type WorkflowReading = Omit<WorkflowInspectorProps, "redirect" | "stop"> & {
  /** The Drones a redirect from here could reach, in the order they are listed. */
  drones: { id: string; label: string }[];
};

/** Why there are no cases here — a shape the wire has no home for yet. */
export const NO_CASES_SERVED =
  "Fleet does not serve the cases a boundary owes yet, so none is drawn. " +
  "What runs here is the Checks above.";

/** Why a step has no tasks: either no plan was recorded, or none of it lands here. */
const NO_TASKS_RECORDED = "No plan has been recorded for this Job.";
const NO_TASKS_HERE = "No task of the plan is worked at this step.";

/** The latest run of each Check the step declares, by name. */
function checksOf(step: StepDetail, only?: readonly string[]): WorkflowInspectorCheck[] {
  const latest = new Map<string, string>();
  for (const run of step.check_runs ?? []) latest.set(run.name, run.outcome);
  // **De-duplicated.** A step can declare the same Check twice under different
  // globs — the arc's `implement` declares `test` for Rust and again for
  // Bridge — and one command is one row whatever selected it.
  const named = (step.checks ?? []).filter((check) => !isSweepMarker(check)).map((check) => nameOf(check));
  return [...new Set(named)]
    .filter((name) => only === undefined || only.includes(name))
    .map((name) => {
      const outcome = latest.get(name);
      const row: WorkflowInspectorCheck = { name };
      if (outcome !== undefined) {
        row.outcome = outcome;
        if (outcome === "passed" || outcome === "failed") row.named = outcome;
      }
      return row;
    });
}

/** One task, with what it has spent where its own agent has stopped. */
function taskOf(task: GroupView["tasks"][number]): WorkflowInspectorTask {
  const facts: string[] = [];
  if (task.scope.length > 0) facts.push(`${task.scope.length} files`);
  if (task.turns !== undefined) facts.push(`${task.turns} turns`);
  if (task.cost_micros !== undefined) facts.push(`$${(task.cost_micros / 1_000_000).toFixed(2)}`);
  const row: WorkflowInspectorTask = { id: task.id, title: task.title, said: task.state, facts };
  if (task.touched_after_done) {
    row.flag = "A later task edited a file this one had finished. It stays done.";
  }
  return row;
}

/** The Drone on a task, or the Job's own where the task names none. */
function dronesOf(whole: JobWhole, tasks: GroupView["tasks"]): { id: string; label: string }[] {
  const named = tasks
    .filter((task) => task.drone_id !== undefined)
    .map((task) => ({ id: task.drone_id!, label: `Drone on ${task.id}` }));
  if (named.length > 0) return [...new Map(named.map((one) => [one.id, one])).values()];
  const drone = whole.job.assigned_drone;
  return drone === undefined ? [] : [{ id: drone, label: "This Job's Drone" }];
}

export type WorkflowInspectorReading = {
  whole: JobWhole;
  groups: readonly GroupView[];
  /** The node id a person has open — `step:…` or `group:…`. */
  selected: string | null;
  /** The step the groups hang under, as `workflow-canvas.ts` computed it. */
  groupsUnder?: string;
};

/**
 * The step or group a person has open, read whole. `undefined` where the
 * selection names nothing this Job has — a Job switched under a held id.
 */
export function workflowReadingOf({
  whole,
  groups,
  selected,
  groupsUnder,
}: WorkflowInspectorReading): WorkflowReading | undefined {
  if (selected === null) return undefined;

  const group = groups.find((one) => groupNodeId(one.id) === selected);
  if (group !== undefined) {
    const step = ordered(whole).find((one) => one.step_id === groupsUnder);
    return {
      name: `Group ${group.ordinal}`,
      kind: "group",
      doing: doingOfGroup(group),
      tasks: group.tasks.map(taskOf),
      checks: step === undefined ? [] : checksOf(step, group.checks_selected),
      checksAbsent: "No Check runs at this group's end.",
      tests: [],
      testsAbsent: NO_CASES_SERVED,
      drones: dronesOf(whole, group.tasks),
    };
  }

  const step = ordered(whole).find((one) => stepNodeId(one.step_id) === selected);
  if (step === undefined) return undefined;
  const mine = step.step_id === groupsUnder ? groups : [];
  const tasks = mine.flatMap((one) => one.tasks);
  return {
    name: step.label,
    kind: "step",
    doing: doingOfStep(whole, step, mine.length),
    tasks: tasks.map(taskOf),
    tasksAbsent: whole.work_plan === undefined ? NO_TASKS_RECORDED : NO_TASKS_HERE,
    checks: checksOf(step),
    checksAbsent: "No Check runs at this step.",
    tests: [],
    testsAbsent: NO_CASES_SERVED,
    drones: dronesOf(whole, tasks),
  };
}

/** What the step is doing now, as a sentence. Read off the record, never guessed. */
function doingOfStep(whole: JobWhole, step: StepDetail, groups: number): string {
  const frozen = frozenBeneath(whole.job.status, step.state);
  const said = frozen?.word ?? stateOf(step);
  const activity = frozen?.activity ?? activityOf(step.state);
  const where = groups > 0 ? ` It opens into ${groups} ${groups === 1 ? "group" : "groups"}.` : "";
  if (activity === "not_started") return `Nothing has entered this step yet.${where}`;
  if (activity === "awaiting_human") return `This step is ${said} — it is waiting on you.${where}`;
  return `This step is ${said}.${where}`;
}

/** What the group is doing now. Its own word, which the step machine has none of. */
function doingOfGroup(group: GroupView): string {
  const running = group.tasks.filter((task) => task.state === "working").length;
  const beside =
    group.concurrent && group.tasks.length > 1
      ? `${group.tasks.length} tasks at the same time`
      : `${group.tasks.length} ${group.tasks.length === 1 ? "task" : "tasks"}, one after another`;
  const now = running === 0 ? "" : ` ${running} still running.`;
  return `${beside}. This group is ${group.state}.${now}`;
}
