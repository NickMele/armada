// The Plan region's own fixture data, beside `JobDetail.stories.tsx` rather
// than inside it — moved out when the Plan stories pushed that file over the
// gate's 900-line rule. **Not a `.stories.tsx` file on purpose**: the gate's
// one-story-per-directory rule reads every file with that suffix, and this is
// data rather than a story.

import type { WorkPlan } from "@armada/protocol";
import type { JobFixture } from "../../../fixtures/fixture";
import { running } from "../../../fixtures/build/index";
import { watchedRead } from "../../../fixtures/build/base";

const PLAN_APPROACH =
  "Split the selectors module out of the reducer so the memoised selector can " +
  "be tested without constructing the whole store. Extract selectColumnOrder " +
  "first, then re-point the reducer's own import at it.";

export const PLAN_PARTWAY: WorkPlan = {
  approach: PLAN_APPROACH,
  recorded_by: { by: "step", step_id: "fix", attempt: 1 },
  recorded_at: "2026-09-10T14:16:07Z",
  tasks: [
    { id: "T1", title: "Extract selectColumnOrder into its own module", state: "done" },
    { id: "T2", title: "Re-point the reducer's own import at it", state: "working" },
    { id: "T3", title: "Add a unit test that does not construct the store", state: "open" },
  ],
};

export const PLAN_WITH_A_DROPPED_TASK: WorkPlan = {
  ...PLAN_PARTWAY,
  tasks: [
    ...PLAN_PARTWAY.tasks.slice(0, 2),
    {
      id: "T3",
      title: "Add a unit test that does not construct the store",
      state: "dropped",
      reason: "The existing integration test already exercises this path.",
    },
    { id: "T4", title: "Update the settings package's README", state: "open" },
  ],
};

/** `running()`, with a `work_plan` merged onto its detail. `#896`. */
export function withPlan(work_plan: WorkPlan): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return { ...fixture, watched: watchedRead({ ...fixture.watched.detail, work_plan }) };
}
