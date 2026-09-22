// One task of a plan, as the new boards draw it. Draft, for
// `crates/ipc/src/work_plan.rs`.
//
// Source of truth today: `PlanTask` on `JobDetail.work_plan`, whose served
// fields are carried here unchanged. What is added is everything a Drone per
// task needs and nothing on the wire says: which group holds it, what it runs
// beside, which Drone is on it, what it has cost, and why it failed.
//
// **`TaskView`, not `PlanTask` and not anything with "Plan" in it.**
// `WorkPlan`, `DeclaredPlan` and `ProposedPlan` are three different things on
// the wire already, and a fourth spelling of the word would make the set
// unreadable.

import type { JobDetail, PlanTask } from "@armada/protocol";

import { coordOfTask, type RunCoord } from "./coord";

/**
 * What a task is doing. **Today's wire has four of these**; `failed` is the
 * draft's own — a task whose own agent stopped without finishing, which is not
 * the same as a person dropping it.
 */
export type TaskState = "open" | "working" | "done" | "failed" | "dropped";

/** How a task is run. A step's Drone, a Drone of its own, or a whole Job. */
export type TaskTreatment = "step_drone" | "own_drone" | "job";

/** How hard the planner thought the task was. The model follows from the Job's map. */
export type TaskTier = "difficult" | "medium" | "easy";

/** One task of a plan, with everything the boards draw. */
export type TaskView = {
  /** `T1`, `T2`, … Stable for the life of the plan, as on the wire. */
  id: string;
  title: string;
  /** The wire's `note`. Absent where the task has none. */
  note?: string;
  /** The repository-relative paths the planner said this task touches. */
  scope: string[];
  /** What the planner said should prove it. Absent where none was named. */
  expects?: string;
  /** What the work said proved it. Read beside `expects`, never instead of it. */
  shown?: string;
  state: TaskState;
  /** Present on a dropped task and on nothing else, as on the wire. */
  reason?: string;
  /**
   * A later task edited a file this one had already finished.
   *
   * **The task stays `done` and is flagged** (#1530, 21 Sep). Moving it back to
   * `open` would lose the fact that it was finished once, and re-running it is
   * not what a person is being asked to decide.
   */
  touched_after_done: boolean;
  /** The group holding it. A stable id — see `RunCoord`. */
  group: string;
  /** The other tasks of that group running at the same time, by id. */
  concurrent_with: string[];
  /**
   * The tier the planner gave it. **The planner picks the tier and the model
   * follows from the Job's map** (#1530, 22 Sep), so nothing here names a model
   * the person did not choose a tier for.
   */
  tier: TaskTier;
  /** The model the Job's tier map resolved that to. */
  model: string;
  treatment: TaskTreatment;
  /** The Drone on it, where one is. Absent on a task nothing has run. */
  drone_id?: string;
  /** Turns taken so far. Drawn while it runs. */
  turns?: number;
  /**
   * What it cost, in millionths of a dollar.
   *
   * **Set when this task's own agent stops, not when its group passes**
   * (#1530, 22 Sep), and absent while it runs: cost reaches Armada on a
   * session's terminating line (`docs/concepts/machine.md`), so a live figure
   * would be invented.
   */
  cost_micros?: number;
  /** Why the task failed. Present on `failed` and on nothing else. */
  failed_reason?: string;
  /**
   * The cases this task owes, by id. **Resolved by Fleet from the planner's
   * scope and the Drone's declaration combined, and never chosen by a Drone**
   * (#1530, 21 Sep).
   */
  cases: string[];
  /** Where it sits, for joining to the Record and to a case run. */
  coord: RunCoord;
};

/**
 * Today's wire holds one Drone per Job and no groups, so every task is
 * `step_drone`, the Job's model, and its own group of one.
 */
export function taskViewOf(detail: JobDetail, task: PlanTask): TaskView {
  const coord = coordOfTask(detail, task);
  const view: TaskView = {
    id: task.id,
    title: task.title,
    scope: task.scope ?? [],
    state: stateOf(task),
    touched_after_done: false,
    group: coord.group ?? "",
    concurrent_with: [],
    tier: "medium",
    model: detail.job.model,
    treatment: "step_drone",
    cases: [],
    coord,
  };
  if (task.note !== undefined) view.note = task.note;
  if (task.expects !== undefined) view.expects = task.expects;
  if (task.shown !== undefined) view.shown = task.shown;
  if (task.reason !== undefined) view.reason = task.reason;
  if (task.state === "working" && detail.job.assigned_drone !== undefined) {
    view.drone_id = detail.job.assigned_drone;
  }
  return view;
}

/** Every task of a Job's plan, in plan order. Empty where no plan was recorded. */
export function taskViewsOf(detail: JobDetail): TaskView[] {
  return (detail.work_plan?.tasks ?? []).map((task) => taskViewOf(detail, task));
}

// `failed` is the draft's own value and today's wire cannot produce it: a task
// whose Drone stopped is still `working` on the record, and the failure is the
// step's. So a state arriving from the wire is carried through as itself, and
// anything unrecognised reads as `open` rather than as a guess.
function stateOf(task: PlanTask): TaskState {
  switch (task.state) {
    case "working":
    case "done":
    case "dropped":
    case "failed":
      return task.state;
    default:
      return "open";
  }
}
