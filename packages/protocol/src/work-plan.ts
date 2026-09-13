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
