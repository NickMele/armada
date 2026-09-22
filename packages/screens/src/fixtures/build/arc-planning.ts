// Two moments on the plan: the plan as recorded, and a revision refused.
//
// **The plan step's own Judge is what refuses a revision** (#1552). A person
// narrowed one task's scope, the Judge answered `not_met` on the criterion
// that asks the plan to name what it will touch, and the whole plan did not go
// back — one task did.
//
// **A case falls out with the paths it covered** (#1530, 21 Sep). Narrowing
// T5 dropped the file `c-panel` was covering, so the case reads `dropped` with
// the revision that dropped it, and never as one that passed.

import type { CaseView, LedgerRow } from "../../draft";
import type { StepDetail } from "@armada/protocol";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import {
  ARC_APPROVED_AT,
  ARC_JOB_ID,
  ARC_NOW,
  arcAdvanced,
  arcCriterionViews,
  arcDetail,
  arcJob,
  arcManifests,
  arcResources,
  arcStep,
  arcSteps,
  arcWatched,
  arcWorkPlan,
  featureWorkflow,
  tasksOf,
} from "./arc-base";
import { ARC_LANDING } from "./arc-dispatch";
import { arcCases, arcGroups, ARC_DRONES, withTask } from "./arc-plan";

const PLAN_ENTERED = "2026-09-22T09:15:00Z";
const PLAN_ENDED = "2026-09-22T09:21:00Z";
const REVISED_AT = "2026-09-22T09:34:00Z";

/** The plan step, run once and passed by its Judge. */
function planAdvanced(): StepDetail {
  return {
    ...arcAdvanced(arcStep("plan", "Plan the change", 1), PLAN_ENTERED, PLAN_ENDED),
    judge_checks: [{ criteria: 2, gaming_check: false }],
    judged: [
      { attempt: 1, criterion_id: "a1", verdict: "met" },
      { attempt: 1, criterion_id: "a2", verdict: "met" },
    ],
    deliverables: [{ attempt: 1, path: `.armada/deliverables/3-show-what-s-running/plan.1.md` }],
  };
}

/** The same step after a revision its Judge would not take. */
function planRefused(): StepDetail {
  return {
    ...planAdvanced(),
    state: "awaiting_human",
    judged: [
      { attempt: 1, criterion_id: "a1", verdict: "met" },
      { attempt: 1, criterion_id: "a2", verdict: "met" },
      {
        attempt: 2,
        criterion_id: "a2",
        verdict: "not_met",
        expected: "the plan names every file the panel's rows are drawn from",
        produced:
          "The revision takes running-rows.tsx out of T5 and no other task claims it, so the " +
          "rows are drawn by nothing.",
      },
    ],
    attempts: [
      { attempt: 1, outcome: "advanced", started_at: PLAN_ENTERED, ended_at: PLAN_ENDED },
      { attempt: 2, outcome: "awaiting_human", started_at: REVISED_AT },
    ],
    verdicts: [
      { attempt: 1, named: "passed" },
      { attempt: 2, named: "failed", trigger: "judge_refused" },
    ],
    updated_at: REVISED_AT,
  };
}

/** The Job, with the plan on it and `implement` still ahead. */
function planned(status: string, plan: StepDetail): JobFixture {
  const groups = arcGroups();
  const job = arcJob(status, {
    current_step_id: status === "awaiting_review" ? "plan" : "implement",
    started_at: ARC_APPROVED_AT,
    assigned_drone: ARC_DRONES.T1,
  });
  const steps = [plan, ...arcSteps().slice(1)];
  const whole = arcDetail(job, steps, { work_plan: arcWorkPlan(tasksOf(groups)) });
  return {
    name: `${status} — the plan is recorded and the groups have not started`,
    job,
    watched: arcWatched(whole),
    workflows: [featureWorkflow()],
    manifests: arcManifests(),
    observed: { state: "none" },
    journalled: {
      state: "watching",
      jobId: ARC_JOB_ID,
      log: {
        skipped: 0,
        notes: [
          {
            at: PLAN_ENDED,
            by: "fleet",
            level: "info",
            seq: 2,
            msg: "The plan was recorded: four groups, eight tasks, a Drone on each.",
          },
        ],
      },
    },
    resources: { state: "read", jobId: ARC_JOB_ID, resources: arcResources("none") },
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
    now: ARC_NOW,
  };
}

/** The Record up to the plan, a Judge on its own row. */
function plannedRecord(): LedgerRow[] {
  return [
    {
      at: ARC_APPROVED_AT,
      coord: null,
      actor: "person",
      kind: "status_queued",
      what: "approved the dispatch",
      outcome: "the workflow and the gates are frozen",
      cursor: 2,
    },
    {
      at: PLAN_ENTERED,
      coord: { step: "plan", step_attempt: 1 },
      actor: "drone",
      kind: "drone_spawned",
      what: "a Drone opened the plan step",
      outcome: "",
      cursor: 3,
    },
    {
      at: PLAN_ENDED,
      coord: { step: "plan", step_attempt: 1 },
      actor: "judge",
      kind: "judged",
      what: "the plan names what it will touch",
      outcome: "met, both criteria",
      cursor: 4,
    },
    {
      at: PLAN_ENDED,
      coord: { step: "plan", step_attempt: 1 },
      actor: "fleet",
      kind: "plan_recorded",
      what: "four groups, eight tasks",
      outcome: "the implement step may start",
      cursor: 5,
    },
  ];
}

/** The same Record with the refused revision on the end. */
function refusedRecord(): LedgerRow[] {
  return [
    ...plannedRecord(),
    {
      at: REVISED_AT,
      coord: { step: "plan", step_attempt: 2, group: "g3", task: "T5" },
      actor: "person",
      kind: "plan_revised",
      what: "took running-rows.tsx out of T5's scope",
      outcome: "one task revised, the plan left standing",
      cursor: 6,
    },
    {
      at: REVISED_AT,
      coord: { step: "plan", step_attempt: 2, group: "g3", task: "T5" },
      actor: "judge",
      kind: "judged",
      what: "the plan names every file the panel's rows are drawn from",
      outcome: "not met — no task claims running-rows.tsx",
      cursor: 7,
    },
  ];
}

/** `c-panel`, dropped by the revision rather than by a retry. */
function casesAfterRevision(): CaseView[] {
  return arcCases().map((one) =>
    one.id === "c-panel"
      ? {
          ...one,
          state: "dropped" as const,
          dropped_by: { dropped: "scope_revision" as const, at: REVISED_AT },
        }
      : one,
  );
}

export function plannedMoment(): ArcMoment {
  const groups = arcGroups();
  return {
    name: "planned",
    says: "Plan — four groups and eight tasks, each with a tier and an agent of its own",
    fixtures: [planned("running", planAdvanced())],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: arcCases(),
      criteria: arcCriterionViews(),
      landing: ARC_LANDING,
      record: plannedRecord(),
    },
  };
}

export function planRevisionRefused(): ArcMoment {
  const groups = withTask(arcGroups(), "T5", {
    scope: ["packages/screens/src/Running.tsx"],
    cases: [],
  });
  return {
    name: "planRevisionRefused",
    says: "Plan — one task's scope was narrowed and the Judge refused that revision alone",
    fixtures: [planned("awaiting_review", planRefused())],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: casesAfterRevision(),
      criteria: arcCriterionViews(),
      landing: ARC_LANDING,
      record: refusedRecord(),
      scope_revisions: [
        {
          at: REVISED_AT,
          paths_added: [],
          paths_removed: ["packages/screens/src/running-rows.tsx"],
          cases_dropped: ["c-panel"],
          cases_added: [],
          by: "person",
          outcome: "refused — no task claims the file the rows are drawn from",
        },
      ],
    },
  };
}
