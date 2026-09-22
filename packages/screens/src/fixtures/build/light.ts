// One Job at light depth, for the moments that are several Jobs at once.
//
// **The roster in `base.ts` is one Job at many moments; these are many Jobs at
// one moment.** A dispatch draws what else is running, a landing order draws
// three Jobs waiting on each other, and a wave draws a parent and its
// children — none of which can be told with one Job's id.
//
// Light means what it means in `waiting.ts`: enough of `JobDetail` to land on
// the right render, with no turns, no journal and no Check output. The depth
// belongs to the Job a moment is about, and these are the Jobs around it.

import type {
  Held,
  JobDetail,
  JobSummary,
  StepDetail,
  WorkflowSummary,
} from "@armada/protocol";
import type { JobFixture } from "../fixture";
import { manifest, MANIFEST_ID } from "./base";

/** What a light Job needs said about it. Everything else follows. */
export type LightJob = {
  id: string;
  handle: string;
  title: string;
  status: string;
  workflow: WorkflowSummary;
  /** The step it stands on, by id. */
  at: string;
  steps: StepDetail[];
  /** The state, as a sentence — the picker lists it. */
  says: string;
  created_at: string;
  started_at?: string;
  ended_at?: string;
  branch?: string;
  /** Anything else the Job row carries: `dispatched_by`, `landed`, `asking`. */
  row?: Partial<JobSummary>;
  /** Anything else the detail carries: a delivery, a plan, a write scope. */
  detail?: Partial<JobDetail>;
};

/**
 * The workflow's own steps as a Job stands in them: everything before `at`
 * advanced, `at` in the state given, everything after not started.
 *
 * **The steps come from the workflow, so a Job draws what its own file
 * declares** — three for `bug`, two for `revert`, four for `feature`.
 */
export function stepsOf(
  workflow: WorkflowSummary,
  at: string,
  state: string,
  entered: string,
): StepDetail[] {
  const index = workflow.steps.findIndex((one) => one.step_id === at);
  return workflow.steps.map((one, ordinal) => {
    const before = index >= 0 && ordinal < index;
    const here = ordinal === index;
    return {
      step_id: one.step_id,
      label: one.label,
      ordinal: ordinal + 1,
      state: before ? "advanced" : here ? state : "not_started",
      checks: one.checks,
      check_runs: before
        ? (one.checks ?? []).map((check) => ({
            attempt: 1,
            name: check.name ?? check.kind,
            outcome: "passed",
          }))
        : [],
      judge_checks: one.judge_checks,
      advance_gate: one.advance_gate,
      delivers: one.delivers,
      judged: [],
      flagged: [],
      overridden: false,
      attempts: before || here ? [{ attempt: 1, outcome: before ? "advanced" : state, started_at: entered }] : [],
      verdicts: before ? [{ attempt: 1, named: "passed" }] : [],
      entered_at: entered,
      updated_at: entered,
    };
  });
}

/** The Board row. */
export function lightRow(one: LightJob): JobSummary {
  const row: JobSummary = {
    id: one.id,
    handle: one.handle,
    title: one.title,
    status: one.status,
    workflow_id: one.workflow.id,
    owner_manifest_id: MANIFEST_ID,
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: one.created_at,
    current_step_id: one.at,
    ...one.row,
  };
  if (one.started_at !== undefined) row.started_at = one.started_at;
  if (one.ended_at !== undefined) row.ended_at = one.ended_at;
  if (one.branch !== undefined) row.branch = one.branch;
  return row;
}

/** The whole fixture: the row, its detail, and empty reads for the rest. */
export function lightFixture(one: LightJob, now: number): JobFixture {
  const job = lightRow(one);
  // `escalated` holds its Drone too — "alive and idle where the step stopped …
  // the worktree and port span are held as-is", `job-statuses.toml`.
  const held: Held =
    one.status === "running" || one.status === "escalated" ? "running" : "none";
  const whole: JobDetail = {
    job,
    created_at: one.created_at,
    steps: one.steps,
    acceptance_criteria: [],
    dependencies: [],
    when_blocked: "refuse_and_hold",
    ...(one.branch === undefined ? {} : { branch: one.branch }),
    ...one.detail,
  };
  return {
    name: one.says,
    job,
    watched: { state: "read", jobId: one.id, detail: whole },
    workflows: [one.workflow],
    manifests: [manifest()],
    observed: { state: "none" },
    journalled: { state: "none" },
    resources: {
      state: "read",
      jobId: one.id,
      resources: {
        job_id: one.id,
        read_at: "2026-09-22T11:18:00.000Z",
        held,
        processes: [],
        ...(one.branch === undefined
          ? {}
          : { worktree: { path: `.armada/worktrees/${one.handle}`, branch: one.branch } }),
      },
    },
    recorded: {
      footprint: { state: "none" },
      handed: { state: "none" },
      evidence: { state: "none" },
      diff: { state: "none" },
      remarks: { state: "none" },
    },
    calls: {},
    checkOutputs: {},
    frames: {},
    now,
  };
}
