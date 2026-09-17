// Work plans for Job detail's Plan region: a Job partway through its plan, one
// with a task dropped, and one awaiting approval with its plan pending. Here
// rather than beside one story file, since the mock's browser tests read it too.

import type { DeclaredCheck, WorkPlan } from "@armada/protocol";
import type { JobFixture } from "./fixture";
import { awaitingApproval, running } from "./build/index";
import { PLAN_MID_TASK } from "./build/working-a-plan";
import { watchedRead } from "./build/base";

/**
 * The same plan with its windows dropped — a Fleet before 14.5, or a Drone that
 * never marked a task. The plan itself lives beside the fixture that carries
 * it, so a turn time moves in one place. #1185.
 */
export const PLAN_PARTWAY: WorkPlan = {
  ...PLAN_MID_TASK,
  tasks: PLAN_MID_TASK.tasks.map(({ working_windows: _windows, ...task }) => task),
};

export { PLAN_MID_TASK };

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

/** The Check `awaitingApprovalPlanPending`'s recast lead step declares. */
const PLAN_RECORDED: DeclaredCheck = { kind: "plan_recorded" };

/**
 * `awaitingApproval()`, with its lead step recast as the one that will
 * record the plan — a Bug Job at the approval gate, before that step has
 * run. `#1007`.
 */
export function awaitingApprovalPlanPending(): JobFixture {
  const fixture = awaitingApproval();
  if (fixture.watched.state !== "read") return fixture;
  const whole = fixture.watched.detail;
  const steps = whole.steps.map((step) =>
    step.step_id === "repro"
      ? { ...step, step_id: "plan", label: "Plan the change", checks: [PLAN_RECORDED] }
      : step,
  );
  return {
    ...fixture,
    job: { ...fixture.job, current_step_id: "plan" },
    watched: watchedRead({
      ...whole,
      job: { ...whole.job, current_step_id: "plan" },
      steps,
    }),
  };
}
