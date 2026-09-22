// A Job as today's Fleet serves it, for the derivations' tests.
//
// **Wire types only.** Every value here is a `@armada/protocol` shape, which is
// what makes a test of a derivation a test of the derivation: if a field moves
// on the wire this file stops compiling, and the test does not quietly go on
// proving something about a shape Fleet no longer sends.
//
// **Not exported from `index.ts`**, and not a fixture roster — `#1533` builds
// those, one Job per workflow kind. This is the minimum each derivation needs.

import type {
  JobDetail,
  JobSummary,
  PlanTask,
  StepDetail,
  WorkPlan,
} from "@armada/protocol";

/** A Board row with everything a draft reads, and nothing it does not. */
export function sampleJob(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "1532-draft-schema",
    title: "Give the new boards a typed home",
    status: "running",
    workflow_id: "feature",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-22T09:00:00Z",
    ...over,
  };
}

/** One line of a plan. */
export function sampleTask(over: Partial<PlanTask> = {}): PlanTask {
  return { id: "T1", title: "Draft the schema", state: "open", ...over };
}

/** One step of a frozen workflow. */
export function sampleStep(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Make the change",
    ordinal: 2,
    state: "running",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-22T09:05:00Z",
    updated_at: "2026-09-22T09:30:00Z",
    ...over,
  };
}

/** A plan, with whatever tasks the test needs. */
export function samplePlan(tasks: PlanTask[], over: Partial<WorkPlan> = {}): WorkPlan {
  return {
    approach: "One file per concept",
    recorded_by: { by: "step", step_id: "plan", attempt: 1 },
    recorded_at: "2026-09-22T09:04:00Z",
    tasks,
    ...over,
  };
}

/** One Job, whole, as `get_job` answers it. */
export function sampleDetail(over: Partial<JobDetail> = {}): JobDetail {
  return {
    job: sampleJob({ current_step_id: "implement" }),
    created_at: "2026-09-22T09:00:00Z",
    steps: [sampleStep()],
    acceptance_criteria: [],
    dependencies: [],
    ...over,
  };
}
