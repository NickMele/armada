// The last moment: handed off, the whole test set run again, the pull request
// merged.
//
// **The whole set runs again before the pull request is offered** (owner,
// 22 Sep 2026), so every case here carries a run at `handoff` whatever ran at
// a group's boundary — and Land is where that run is read.
//
// **A case with no spec is `not_run`, which reads as not covered.** `c-board`
// never had one, and a handoff run cannot invent it: the run says the case did
// not run and why, and no verdict of any kind sits beside it.

import type { StepDetail } from "@armada/protocol";
import type { CaseRunView, CaseView, GroupView, LedgerRow } from "../../draft";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import {
  ARC_APPROVED_AT,
  ARC_BRANCH,
  ARC_JOB_ID,
  ARC_NOW,
  arcAdvanced,
  arcCriterionViews,
  arcDetail,
  arcJob,
  arcManifests,
  arcResources,
  arcStep,
  arcWatched,
  arcWorkPlan,
  BRIDGE_CHECKS,
  featureWorkflow,
  RUST_CHECKS,
  tasksOf,
} from "./arc-base";
import { ARC_LANDING } from "./arc-dispatch";
import { arcCases, ARC_DRONES, finished, withGroup, withTask } from "./arc-plan";
import { doneTouched } from "./arc-executing";

const HANDED_AT = "2026-09-22T11:12:00Z";
const ENDED_AT = "2026-09-22T11:16:00Z";
const PULL_REQUEST = "https://github.com/NickMele/armada/pull/1604";

/** Every group passed, the last two with the commits they left. */
function landedGroups(): GroupView[] {
  let groups = doneTouched().draft.groups!;
  groups = withTask(groups, "T7", {
    ...finished(19, 880_000, "The panel says what the hold is while something is queued"),
    drone_id: ARC_DRONES.T7,
    touched_after_done: false,
  });
  groups = withTask(groups, "T8", {
    ...finished(11, 420_000, "Four stories, each drawing without a Fleet behind it"),
    drone_id: ARC_DRONES.T8,
  });
  return withGroup(groups, "g4", {
    state: "landed",
    verdict: "passed",
    commit: "e0d47a1",
  });
}

/**
 * The handoff run of one case. **`tree: "branch"`** — the run is against what
 * is about to be offered, not against a merge nobody has made yet.
 */
function handoffRun(id: string, at: string, frames: number): CaseRunView {
  return {
    id: `run-handoff-${id}`,
    case: id,
    coord: { step: "handoff", step_attempt: 1 },
    actor: "fleet",
    purpose: "handoff",
    tree: "branch",
    outcome: "ran",
    frames,
    ran_at: at,
  };
}

/** The set as it stands at handoff: three ran again, one never had a spec. */
function landedCases(): CaseView[] {
  const runs: Record<string, CaseRunView> = {
    "c-api": handoffRun("c-api", "2026-09-22T11:09:20Z", 0),
    "c-overview": handoffRun("c-overview", "2026-09-22T11:09:44Z", 2),
    "c-panel": handoffRun("c-panel", "2026-09-22T11:10:31Z", 6),
    "c-board": {
      id: "run-handoff-c-board",
      case: "c-board",
      coord: { step: "handoff", step_attempt: 1 },
      actor: "fleet",
      purpose: "handoff",
      tree: "branch",
      outcome: "not_run",
      not_run_reason: "no spec covers Board.tsx",
      frames: 0,
      ran_at: "2026-09-22T11:10:40Z",
    },
  };
  return arcCases().map((one) => ({ ...one, last_run: runs[one.id]! }));
}

/** The one run a person made from the Job screen, before they approved it. */
function reviewRun(): CaseRunView {
  return {
    id: "run-show-again-1",
    case: "c-panel",
    coord: { step: "handoff", step_attempt: 1 },
    actor: "person",
    who: "you",
    purpose: "show_again",
    tree: "branch",
    outcome: "ran",
    frames: 6,
    ran_at: "2026-09-22T11:11:20Z",
  };
}

/** `implement` and `tests`, both behind the handoff. */
function behind(): StepDetail[] {
  return [
    {
      ...arcAdvanced(
        arcStep("implement", "Implement", 2, [...RUST_CHECKS, ...BRIDGE_CHECKS]),
        "2026-09-22T09:22:00Z",
        "2026-09-22T11:06:00Z",
      ),
      judge_checks: [{ criteria: 2, gaming_check: false }],
      judged: [
        { attempt: 1, criterion_id: "a1", verdict: "met" },
        { attempt: 1, criterion_id: "a2", verdict: "met" },
      ],
    },
    {
      ...arcAdvanced(
        arcStep("tests", "Write tests", 3, BRIDGE_CHECKS),
        "2026-09-22T11:06:00Z",
        "2026-09-22T11:09:00Z",
      ),
      judge_checks: [{ criteria: 1, gaming_check: true }],
      judged: [{ attempt: 1, criterion_id: "a2", verdict: "met" }],
    },
  ];
}

function landedFixture(): JobFixture {
  const groups = landedGroups();
  const job = arcJob("completed_success", {
    current_step_id: "handoff",
    started_at: ARC_APPROVED_AT,
    ended_at: ENDED_AT,
    landed: "merged",
  });
  const handoff = arcAdvanced(arcStep("handoff", "Review the change", 4), HANDED_AT, ENDED_AT);
  const whole = arcDetail(job, [...behindPlan(), ...behind(), handoff], {
    work_plan: arcWorkPlan(tasksOf(groups)),
    delivery: {
      commit: "e0d47a1",
      pushed: `origin/${ARC_BRANCH}`,
      pull_request: PULL_REQUEST,
      landed: "merged",
    },
  });
  return {
    name: "completed_success — the set ran again at handoff and the pull request merged",
    job,
    watched: arcWatched(whole),
    workflows: [featureWorkflow()],
    manifests: arcManifests(),
    observed: { state: "none" },
    journalled: { state: "none" },
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

/** The plan step, which every later moment carries unchanged. */
function behindPlan(): StepDetail[] {
  return [
    {
      ...arcAdvanced(arcStep("plan", "Plan the change", 1), "2026-09-22T09:15:00Z", "2026-09-22T09:21:00Z"),
      judge_checks: [{ criteria: 2, gaming_check: false }],
      judged: [
        { attempt: 1, criterion_id: "a1", verdict: "met" },
        { attempt: 1, criterion_id: "a2", verdict: "met" },
      ],
    },
  ];
}

function landedRecord(): LedgerRow[] {
  return [
    {
      at: "2026-09-22T11:09:00Z",
      coord: { step: "handoff", step_attempt: 1 },
      actor: "check",
      kind: "cases_rerun",
      what: "the whole test set, run again before the pull request was offered",
      outcome: "three ran, one has no spec",
      cursor: 13,
    },
    {
      at: "2026-09-22T11:11:20Z",
      coord: { step: "handoff", step_attempt: 1 },
      actor: "person",
      kind: "shown_again",
      what: "you ran the panel's case yourself",
      outcome: "six frames kept",
      cursor: 14,
    },
    {
      at: ENDED_AT,
      coord: null,
      actor: "person",
      kind: "status_completed_success",
      what: "the pull request merged",
      outcome: "the Job is done",
      cursor: 15,
    },
  ];
}

export function landed(): ArcMoment {
  const groups = landedGroups();
  return {
    name: "landed",
    says: "Land — the whole set ran again, and the pull request merged",
    fixtures: [landedFixture()],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: landedCases(),
      runs: [...landedCases().map((one) => one.last_run!), reviewRun()],
      criteria: arcCriterionViews(),
      landing: { ...ARC_LANDING, complete_when: "pr_merged" },
      record: landedRecord(),
    },
  };
}
