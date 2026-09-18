// The Plan region's own read of a Job — pure, so the arithmetic is tested
// without a browser. `docs/concepts/plan.md`; the wire is `work-plan.ts`.

import type { JobDetail } from "@armada/protocol";
import type { PlanRegionRead, PlanTaskRow } from "./InsideAJob";
import type { TaskMarkState } from "@armada/components";

/** The four states a task's own wire string may be. Anything else is `open`. */
const STATES: readonly TaskMarkState[] = ["open", "working", "done", "dropped"];

function markStateOf(state: string): TaskMarkState {
  return (STATES as readonly string[]).includes(state) ? (state as TaskMarkState) : "open";
}

/** The `DeclaredCheck.kind` `crates/core-model/src/job/workflow.rs` names `PLAN_RECORDED`. */
const PLAN_RECORDED = "plan_recorded";

/**
 * The Plan region's props, or `undefined` for a Job whose workflow declares
 * no step recording a plan — nothing is drawn empty on this screen.
 *
 * **Read off the declared checks, never `workflow_id`.** A step's own
 * `checks` say whether it records the plan; the workflow's name does not,
 * and `#1006` lets any step declare `plan_recorded` rather than only the one
 * a workflow happens to call `plan`.
 */
export function planOf(whole: JobDetail | null): PlanRegionRead | undefined {
  if (whole === null) return undefined;
  const plan = whole.work_plan;
  if (plan !== undefined) {
    const tasks: PlanTaskRow[] = plan.tasks.map((task) => ({
      id: task.id,
      title: task.title,
      state: markStateOf(task.state),
      reason: task.reason,
      note: task.note,
      scope: task.scope,
      expects: task.expects,
      shown: task.shown,
    }));
    return { recorded: true, approach: plan.approach, tasks };
  }
  const pending = whole.steps.find((step) =>
    (step.checks ?? []).some((check) => check.kind === PLAN_RECORDED),
  );
  return pending === undefined ? undefined : { recorded: false, stepLabel: pending.label };
}
