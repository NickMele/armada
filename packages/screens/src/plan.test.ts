// `planOf`: a recorded plan, the quiet placeholder before one exists, and no
// region at all for a workflow that declares no step recording a plan. `#1007`.

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, StepDetail, WorkPlan } from "@armada/protocol";
import { planOf } from "./plan";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "root_cause",
    label: "Root cause",
    ordinal: 2,
    state: "not_started",
    checks: [],
    check_runs: [],
    judge_checks: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-13T09:00:00Z",
    updated_at: "2026-09-13T09:00:00Z",
    ...over,
  };
}

const JOB = {
  id: "01M22TYSAE0023MADDP5ZQEYGW",
  handle: "9-split-the-settings-reducer",
  title: "Split the settings reducer",
  status: "awaiting_approval",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-09-13T09:00:00Z",
} as const;

function whole(steps: StepDetail[], over: Partial<JobWhole> = {}): JobWhole {
  return {
    job: JOB,
    created_at: "2026-09-13T09:00:00Z",
    steps,
    acceptance_criteria: [],
    dependencies: [],
    ...over,
  };
}

const RECORDED: WorkPlan = {
  approach: "Split the reducer.",
  recorded_by: { by: "step", step_id: "plan", attempt: 1 },
  recorded_at: "2026-09-13T09:05:00Z",
  tasks: [{ id: "T1", title: "Extract the selector", state: "open" }],
};

describe("planOf", () => {
  it("reads a recorded plan's approach and tasks", () => {
    const read = planOf(whole([step({ step_id: "plan", label: "Plan the change" })], { work_plan: RECORDED }));
    expect(read).toEqual({
      recorded: true,
      approach: "Split the reducer.",
      tasks: [{ id: "T1", title: "Extract the selector", state: "open", reason: undefined }],
    });
  });

  it("names the step that will record it, off its declared checks, where none is recorded yet", () => {
    const read = planOf(
      whole([
        step({ step_id: "plan", label: "Plan the change", checks: [{ kind: "plan_recorded" }] }),
        step({ step_id: "implement", label: "Implement", ordinal: 3 }),
      ]),
    );
    expect(read).toEqual({ recorded: false, stepLabel: "Plan the change" });
  });

  it("never reads workflow_id — any step declaring plan_recorded is found, wherever it sits", () => {
    // `#1006` lets any step record the plan, not only one a workflow calls `plan`.
    const read = planOf(
      whole(
        [
          step({ step_id: "repro", label: "Reproduction" }),
          step({ step_id: "root_cause", label: "Root cause", checks: [{ kind: "plan_recorded" }] }),
        ],
        { job: { ...JOB, workflow_id: "revert" } },
      ),
    );
    expect(read).toEqual({ recorded: false, stepLabel: "Root cause" });
  });

  it("draws no region for a workflow that declares no step recording a plan", () => {
    const read = planOf(whole([step({ step_id: "repro", label: "Reproduction" })]));
    expect(read).toBeUndefined();
  });

  it("draws no region where there is no Job to read", () => {
    expect(planOf(null)).toBeUndefined();
  });
});
