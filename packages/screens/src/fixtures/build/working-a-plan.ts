// `running()`, with the plan it is working — the Job the Working area was drawn
// against. The windows are set against that fixture's own Fix turns, so the
// first edit falls inside T1 and the second inside T2, and the narration groups
// under the task that was marked working at the time. #1185.

import type { TaskCounts, WorkPlan } from "@armada/protocol";
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

/** Every fixture's clock, and the moment `NOW` stands for in it. */
const FIXTURE_NOW = Date.parse("2026-09-10T14:31:00Z");

/** An ISO instant anywhere in a fixture, which is the only shape a time takes here. */
const AN_INSTANT = /"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z)"/g;

/**
 * The same fixture with every instant in it moved so its `NOW` is the moment
 * this is read.
 *
 * **The mock reads the real clock**, so a Job dated last week draws an elapsed
 * time in days and an empty Activity instrument — twelve minutes of windows
 * with nothing in them. That is a fixture too old to measure, drawn correctly,
 * and it reads as a Job asleep on the one screen built to say it is not.
 * Every other fixture keeps its fixed times, which tests read.
 */
function asOfNow(fixture: JobFixture): JobFixture {
  const moved = Date.now() - FIXTURE_NOW;
  return JSON.parse(
    JSON.stringify(fixture).replace(AN_INSTANT, (_whole, at: string) =>
      JSON.stringify(new Date(Date.parse(at) + moved).toISOString()),
    ),
  ) as JobFixture;
}

/**
 * The plan's tasks as a Board row counts them, read off the plan itself.
 *
 * **Counted rather than written down.** `JobSummary.tasks` and `WorkPlan.tasks`
 * are the same three tasks said twice — once for a row that redraws from the
 * counts and once for a screen that reads the plan — and a fixture that typed
 * the numbers in would be the one place they could disagree.
 */
const TASKS_MID_PLAN: TaskCounts = {
  done: PLAN_MID_TASK.tasks.filter((task) => task.state === "done").length,
  working: PLAN_MID_TASK.tasks.filter((task) => task.state === "working").length,
  open: PLAN_MID_TASK.tasks.filter((task) => task.state === "open").length,
  dropped: PLAN_MID_TASK.tasks.filter((task) => task.state === "dropped").length,
};

/**
 * `running()` carrying that plan, so the mock can open the Working area whole.
 *
 * **Its row carries the counts, which is what puts a Tasks column on the
 * Board.** `columnsFor` draws that column where any Job on the board has a
 * plan, and no fixture set one — so the widest field run the Board can
 * actually produce existed on nobody's screen, and the row overflowed into
 * its own action for a week before the owner hit it on a real Job.
 */
export function workingAPlan(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  const job = { ...fixture.job, tasks: TASKS_MID_PLAN };
  return asOfNow({
    ...fixture,
    job,
    name: "running — working the plan's second task of three",
    watched: watchedRead({ ...fixture.watched.detail, job, work_plan: PLAN_MID_TASK }),
  });
}
