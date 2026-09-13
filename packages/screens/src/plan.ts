// The Plan region's own read of a Job — pure, so the arithmetic is tested
// without a browser. `docs/concepts/plan.md`; the wire is `work-plan.ts`.

import type { JobDetail } from "@armada/protocol";
import type { PlanRegionData, PlanTaskRow } from "./InsideAJob";
import type { TaskMarkState } from "@armada/components";

/** The four states a task's own wire string may be. Anything else is `open`. */
const STATES: readonly TaskMarkState[] = ["open", "working", "done", "dropped"];

function markStateOf(state: string): TaskMarkState {
  return (STATES as readonly string[]).includes(state) ? (state as TaskMarkState) : "open";
}

/**
 * The Plan region's props, or `undefined` for a Job whose workflow has no
 * plan step — nothing is drawn empty on this screen.
 */
export function planOf(whole: JobDetail | null): PlanRegionData | undefined {
  const plan = whole?.work_plan;
  if (plan === undefined) return undefined;
  const tasks: PlanTaskRow[] = plan.tasks.map((task) => ({
    id: task.id,
    title: task.title,
    state: markStateOf(task.state),
    reason: task.reason,
  }));
  return { approach: plan.approach, tasks };
}
