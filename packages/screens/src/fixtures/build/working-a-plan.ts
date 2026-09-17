// `running()`, with the plan it is working — the Job the Working area was drawn
// against. The windows are set against that fixture's own Fix turns, so the
// first edit falls inside T1 and the second inside T2, and the narration groups
// under the task that was marked working at the time. #1185.

import type { WorkPlan } from "@armada/protocol";
import type { JobFixture } from "../fixture";
import { running } from "./running";
import { watchedRead } from "./base";

/**
 * A plan partway through, with the windows Fleet sends since 14.5. **The one
 * place these times are written down**: `fixtures/plans.ts` takes its plans
 * from here rather than restating them, so a turn time moving in `running()`
 * moves one plan, not two.
 */
export const PLAN_MID_TASK: WorkPlan = {
  approach:
    "Split the selectors module out of the reducer so the memoised selector can " +
    "be tested without constructing the whole store. Extract selectColumnOrder " +
    "first, then re-point the reducer's own import at it.",
  recorded_by: { by: "step", step_id: "fix", attempt: 1 },
  recorded_at: "2026-09-10T14:16:07Z",
  tasks: [
    {
      id: "T1",
      title: "Extract selectColumnOrder into its own module",
      state: "done",
      working_windows: [{ entered: "2026-09-10T14:16:30Z", left: "2026-09-10T14:20:00Z" }],
    },
    {
      id: "T2",
      title: "Re-point the reducer's own import at it",
      state: "working",
      working_windows: [{ entered: "2026-09-10T14:20:00Z" }],
    },
    { id: "T3", title: "Add a unit test that does not construct the store", state: "open" },
  ],
};

/** `running()` carrying that plan, so the mock can open the Working area whole. */
export function workingAPlan(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    name: "running — working the plan's second task of three",
    watched: watchedRead({ ...fixture.watched.detail, work_plan: PLAN_MID_TASK }),
  };
}
