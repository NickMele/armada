// A Job's plan and its tasks. `crates/ipc/src/work_plan.rs`. Since 13.21.
//
// **`work_plan`, not `plan`**: `DeclaredPlan` in `work.ts` is where a step said
// its work would be, and this is what the work is. The header rules in
// `protocol.ts` hold: hand-written, and every closed set left as `string`.

/** An approach and its tasks, in plan order, with the history folded away. */
export type WorkPlan = {
  approach: string;
  /** Who recorded the plan as it stands. A later whole recording replaces it. */
  recorded_by: ChangedBy;
  recorded_at: string;
  tasks: PlanTask[];
};

/** A run of a step, or a person. */
export type ChangedBy =
  | { by: "step"; step_id: string; attempt: number }
  | { by: "person" };

/** One line of the plan. */
export type PlanTask = {
  /** `T1`, `T2`, … — stable for the life of the plan, never renumbered. */
  id: string;
  title: string;
  /** Absent where the task has none. */
  detail?: string;
  /** `open`, `working`, `done` or `dropped`. A claim, and it gates nothing. */
  state: string;
  /** Present on a dropped task and on nothing else. */
  reason?: string;
  /**
   * Each stretch the task was marked `working`, oldest first. Since 14.5.
   * Absent on a task nobody marked working — and on a Fleet before 14.5.
   */
  working_windows?: WorkingWindow[];
};

/**
 * From the change that marked a task `working` to the change that moved it
 * out. **A turn belongs to the task whose window holds its instant**, never to
 * one whose words match its path. `left` is absent while it is still working.
 */
export type WorkingWindow = {
  entered: string;
  left?: string;
};

/**
 * How many tasks stand where. `done` over `done + working + open` is the figure
 * a person reads; a dropped task is not counted against it.
 */
export type TaskCounts = {
  done: number;
  working: number;
  open: number;
  dropped: number;
};

/**
 * A Job's plan changed. **The counts ride along and the plan does not**: a row
 * redraws from these, and an open Job reads `get_job` for `work_plan`.
 */
export type JobPlanChanged = {
  job_id: string;
  tasks: TaskCounts;
  actor: string;
  at: string;
};

/**
 * A person adds a task to a Job's plan — `add_task`. Since 13.30.
 *
 * `detail` and `after` may both be `""`; `after` is the id it comes after, or
 * `""` for the end.
 */
export type AddTask = {
  title: string;
  detail: string;
  after: string;
};

/**
 * A person drops a task from a Job's plan, with a reason — `drop_task`.
 * Since 13.30. `reason` is never blank.
 */
export type DropTask = {
  task: string;
  reason: string;
};
