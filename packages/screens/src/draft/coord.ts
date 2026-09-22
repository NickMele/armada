// Where something happened inside a Job. Draft, for `crates/ipc/src/work_plan.rs`.
//
// Source of truth today: `StepDetail.step_id` and `StepAttempt.attempt`
// (`crates/ipc/src/detail/step.rs`) and `PlanTask.id`
// (`crates/ipc/src/work_plan.rs`). Groups and group attempts are the part
// nothing serves.
//
// **One coordinate, not four fields spread across every row.** A Record row, a
// case run and a task all have to say where in the Job they sit, and the run
// tree has to join them. Three spellings of the same tuple is how two surfaces
// come to disagree about which attempt a row belonged to.

import type { JobDetail, PlanTask, StepDetail } from "@armada/protocol";

/**
 * Where in a Job something happened.
 *
 * **`group` is a stable id and never an ordinal.** A group that is retried, or
 * one a person splits, keeps its id; its position in the step can move. A row
 * keyed on the ordinal would follow the position rather than the group.
 */
export type RunCoord = {
  step: string;
  /** Which run of the step, counted from one. Joins to `StepDetail.attempts`. */
  step_attempt: number;
  /** The group within that run of the step. Absent where the fact is the step's. */
  group?: string;
  /** Which run of that group, counted from one. Absent with `group`. */
  group_attempt?: number;
  /** The task within that group. Absent where the fact is the group's. */
  task?: string;
};

/**
 * The id a derived group carries while the wire has no groups.
 *
 * **Derived from the task id, which is stable for the life of the plan** —
 * `PlanTask.id` is documented as never renumbered, so a group minted from it is
 * stable too. The prefix keeps a group id from ever being mistaken for a task
 * id in a join.
 */
export function derivedGroupId(taskId: string): string {
  return `g-${taskId}`;
}

/** The step a Job is on right now, or its last step, or nothing. */
function currentStep(detail: JobDetail): StepDetail | undefined {
  const named = detail.steps.find(
    (step) => step.step_id === detail.job.current_step_id,
  );
  return named ?? detail.steps[detail.steps.length - 1];
}

/**
 * Which run of a step is the live one. **Counted from one, and one on a step
 * nothing has entered** — `attempts` is empty until a Drone arrives, and a
 * coordinate naming attempt zero would join to no row at all.
 */
export function stepAttemptOf(step: StepDetail | undefined): number {
  if (!step || step.attempts.length === 0) {
    return 1;
  }
  return step.attempts.length;
}

/**
 * The coordinate a task sits at, from today's wire.
 *
 * With no groups served, one task is one group — `derivedGroupId` mints the id
 * and the attempt is the step's own, because nothing retries a group on its
 * own yet.
 */
export function coordOfTask(detail: JobDetail, task: PlanTask): RunCoord {
  const step = currentStep(detail);
  const attempt = stepAttemptOf(step);
  return {
    step: step?.step_id ?? "",
    step_attempt: attempt,
    group: derivedGroupId(task.id),
    group_attempt: attempt,
    task: task.id,
  };
}

/** The coordinate one step sits at, naming no group and no task. */
export function coordOfStep(step: StepDetail): RunCoord {
  return { step: step.step_id, step_attempt: stepAttemptOf(step) };
}
