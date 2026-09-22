// Four moments inside `implement`: one group at a time, two tasks at once, a
// boundary that failed, and a finished task a later one edited.
//
// **The Checks run at each group's end** (#1530, 21 Sep), so a group carries
// the verdict, the commit and the retry count, and a task carries only its own
// agent's turns and cost.
//
// **A task's cost appears when that task's agent stops** (owner, 22 Sep 2026),
// which is why a done task in a group still being checked already shows one.
//
// **A done task a later task edits stays done and is flagged** (#1530). T6
// finished in group three; T7 writes the same file in group four.

import type { JobProcess, StepDetail } from "@armada/protocol";
import type { GroupView, LedgerRow, PulseView } from "../../draft";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import {
  ARC_APPROVED_AT,
  ARC_BRANCH,
  ARC_JOB_ID,
  ARC_NOW,
  ARC_WORKTREE,
  arcAdvanced,
  arcCriterionViews,
  arcDetail,
  arcJob,
  arcManifests,
  arcResources,
  arcRunning,
  arcStep,
  arcSteps,
  arcWatched,
  arcWorkPlan,
  BRIDGE_CHECKS,
  checkNames,
  featureWorkflow,
  RUST_CHECKS,
  tasksOf,
} from "./arc-base";
import { ARC_LANDING, arcProposal } from "./arc-dispatch";
import { ARC_DRONES, arcCases, arcGroups, finished, withGroup, withTask } from "./arc-plan";

const IMPLEMENT_ENTERED = "2026-09-22T09:22:00Z";

/** The plan step, behind every moment in this file. */
function planAdvanced(): StepDetail {
  return {
    ...arcAdvanced(arcStep("plan", "Plan the change", 1), "2026-09-22T09:15:00Z", "2026-09-22T09:21:00Z"),
    judge_checks: [{ criteria: 2, gaming_check: false }],
    judged: [
      { attempt: 1, criterion_id: "a1", verdict: "met" },
      { attempt: 1, criterion_id: "a2", verdict: "met" },
    ],
  };
}

/** One Drone's process, as `ps` reports it. */
function droneProcess(pid: number, ran: string): JobProcess {
  return {
    pid,
    command: "node",
    cpu_percent: 12.4,
    memory_bytes: 486_539_264,
    running_for: ran,
    recorded: true,
  };
}

/** Everything the Job is holding, with a process per Drone that is up. */
function pulse(readAt: string, processes: JobProcess[]): PulseView {
  return {
    job: ARC_JOB_ID,
    read_at: readAt,
    held: processes.length === 0 ? "none" : "running",
    processes: processes.map((one) => ({
      pid: one.pid,
      command: one.command,
      cpu_percent: one.cpu_percent,
      memory_bytes: one.memory_bytes,
      running_for: one.running_for,
      recorded: one.recorded,
      // Concurrent tasks share one checkout, so every process here is placed
      // in the Job's own worktree. A second worktree is what a member would
      // bring, and `implement` has none.
      owner: ARC_BRANCH,
    })),
    worktrees: [{ path: ARC_WORKTREE, branch: ARC_BRANCH, bytes: 1_020_054_016 }],
    logs: [{ kind: "job", owner: null, writing: processes.length > 0 }],
  };
}

/** The Job at some instant inside `implement`. */
function executing(args: {
  says: string;
  groups: GroupView[];
  step: StepDetail;
  processes: JobProcess[];
  status?: string;
}): JobFixture {
  const job = arcJob(args.status ?? "running", {
    current_step_id: "implement",
    started_at: ARC_APPROVED_AT,
    assigned_drone: ARC_DRONES.T5,
  });
  const steps = [planAdvanced(), args.step, ...arcSteps().slice(2)];
  const whole = arcDetail(job, steps, { work_plan: arcWorkPlan(tasksOf(args.groups)) });
  return {
    name: args.says,
    job,
    watched: arcWatched(whole),
    workflows: [featureWorkflow()],
    manifests: arcManifests(),
    observed: { state: "none" },
    journalled: { state: "none" },
    resources: {
      state: "read",
      jobId: ARC_JOB_ID,
      resources: arcResources(args.processes.length === 0 ? "none" : "running", args.processes),
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
    now: ARC_NOW,
  };
}

/** `implement`, mid-run, with whatever the last boundary's Checks came to. */
function implementStep(runs: StepDetail["check_runs"], at: string): StepDetail {
  const step = arcRunning(
    arcStep("implement", "Implement", 2, [...RUST_CHECKS, ...BRIDGE_CHECKS]),
    IMPLEMENT_ENTERED,
    at,
  );
  return { ...step, check_runs: runs };
}

/** Every Check a boundary ran, all passed. */
function allPassed(names: string[], attempt = 1): StepDetail["check_runs"] {
  return names.map((name) => ({ attempt, name, outcome: "passed" }));
}

/** The first two groups, passed, with what their agents cost. */
function throughGroupTwo(): GroupView[] {
  let groups = arcGroups();
  groups = withTask(groups, "T1", {
    ...finished(34, 2_400_000, "The read answers Drones, Checks and Judge calls in one call"),
    drone_id: ARC_DRONES.T1,
  });
  groups = withTask(groups, "T2", {
    ...finished(12, 640_000, "A Drone starting moves the read without a poll"),
    drone_id: ARC_DRONES.T2,
  });
  groups = withTask(groups, "T3", {
    ...finished(8, 260_000, "The stat reads one running and two at most"),
    drone_id: ARC_DRONES.T3,
  });
  groups = withTask(groups, "T4", {
    ...finished(9, 310_000, "Neither surface spells the pair with of"),
    drone_id: ARC_DRONES.T4,
  });
  groups = withGroup(groups, "g1", {
    state: "passed",
    verdict: "passed",
    commit: "4c1b9d2",
  });
  return withGroup(groups, "g2", { state: "passed", verdict: "passed", commit: "7a2f0c5" });
}

/** The Record up to the end of group two, a Check and a Judge each on a row. */
function recordThroughGroupTwo(): LedgerRow[] {
  return [
    {
      at: "2026-09-22T09:41:00Z",
      coord: { step: "implement", step_attempt: 1, group: "g1", group_attempt: 1, task: "T1" },
      actor: "drone",
      kind: "drone_exited",
      what: "T1's agent stopped",
      outcome: "34 turns, and what it cost",
      cursor: 8,
    },
    {
      at: "2026-09-22T09:56:00Z",
      coord: { step: "implement", step_attempt: 1, group: "g1", group_attempt: 1 },
      actor: "check",
      kind: "checked",
      what: "four Checks at group one's boundary",
      outcome: "all four passed",
      cursor: 9,
    },
    {
      at: "2026-09-22T10:14:00Z",
      coord: { step: "implement", step_attempt: 1, group: "g2", group_attempt: 1 },
      actor: "check",
      kind: "checked",
      what: "seven Checks at group two's boundary",
      outcome: "all seven passed",
      cursor: 10,
    },
  ];
}

export function executingSequential(): ArcMoment {
  let groups = throughGroupTwo();
  groups = withGroup(groups, "g3", { state: "running" });
  groups = withTask(groups, "T5", { state: "working", turns: 14, drone_id: ARC_DRONES.T5 });
  return {
    name: "executingSequential",
    says: "Implement — groups one and two passed, group three is working",
    fixtures: [
      executing({
        says: "running — one group at a time, the third of four working",
        groups,
        step: implementStep(allPassed(checkNames(BRIDGE_CHECKS)), "2026-09-22T10:20:00Z"),
        processes: [droneProcess(52_118, "06:12")],
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: arcCases(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcProposal({ status: "approved" }),
      landing: ARC_LANDING,
      record: recordThroughGroupTwo(),
      pulse: pulse("2026-09-22T10:20:00.000Z", [droneProcess(52_118, "06:12")]),
    },
  };
}

export function executingConcurrent(): ArcMoment {
  let groups = throughGroupTwo();
  groups = withTask(groups, "T5", {
    ...finished(27, 1_900_000, "The panel lists Drones, Checks, Judge calls and proposer calls"),
    drone_id: ARC_DRONES.T5,
  });
  groups = withTask(groups, "T6", {
    ...finished(15, 720_000, "Pressing a Drone's row opens that Job"),
    drone_id: ARC_DRONES.T6,
  });
  // Joining, not checking: both agents have stopped and their work is being
  // brought together before the boundary's Checks run. Each task's cost is
  // already on it, because each agent stopped.
  groups = withGroup(groups, "g3", { state: "joining" });
  return {
    name: "executingConcurrent",
    says: "Implement — two tasks ran at once, and group three is joining their work",
    fixtures: [
      executing({
        says: "running — two tasks of one group ran at the same time",
        groups,
        step: implementStep(allPassed(checkNames(BRIDGE_CHECKS)), "2026-09-22T10:38:00Z"),
        processes: [],
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: arcCases(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcProposal({ status: "approved" }),
      landing: ARC_LANDING,
      record: recordThroughGroupTwo(),
      pulse: pulse("2026-09-22T10:38:00.000Z", []),
    },
  };
}

export function groupFailed(): ArcMoment {
  let groups = executingConcurrent().draft.groups!;
  groups = withTask(groups, "T6", {
    state: "failed",
    failed_reason: "The row's press opened the Board rather than the Job",
  });
  groups = withGroup(groups, "g3", {
    state: "retrying",
    verdict: "failed",
    retry_count: 1,
  });
  const runs = [
    ...allPassed(checkNames(BRIDGE_CHECKS).filter((name) => name !== "screens_test")),
    {
      attempt: 1,
      name: "screens_test",
      outcome: "failed",
      expected: "every test in the screens package passes",
      produced: "1 of 1384 failed: the Drones row opened the Board",
      output_path: ".armada/checks/3-show-what-s-running/implement.1.screens_test.log",
    },
  ];
  return {
    name: "groupFailed",
    says: "Implement — group three failed its Checks and is on its second run",
    fixtures: [
      executing({
        says: "running — a group failed at its boundary and is being run again",
        groups,
        step: implementStep(runs, "2026-09-22T10:46:00Z"),
        processes: [droneProcess(52_640, "01:40")],
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: arcCases(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcProposal({ status: "approved" }),
      landing: ARC_LANDING,
      record: [
        ...recordThroughGroupTwo(),
        {
          at: "2026-09-22T10:44:00Z",
          coord: { step: "implement", step_attempt: 1, group: "g3", group_attempt: 1 },
          actor: "check",
          kind: "checked",
          what: "seven Checks at group three's boundary",
          outcome: "screens_test failed — the Drones row opened the Board",
          cursor: 11,
        },
      ],
      pulse: pulse("2026-09-22T10:46:00.000Z", [droneProcess(52_640, "01:40")]),
    },
  };
}

export function doneTouched(): ArcMoment {
  let groups = executingConcurrent().draft.groups!;
  groups = withGroup(groups, "g3", {
    state: "passed",
    verdict: "passed",
    commit: "b81c3e4",
    retry_count: 1,
  });
  // T6 finished in group three. T7 writes the same file in group four, so T6
  // stays done and carries the flag — moving it back to `open` would lose the
  // fact that it was finished once.
  groups = withTask(groups, "T6", { touched_after_done: true });
  groups = withGroup(groups, "g4", { state: "running" });
  groups = withTask(groups, "T7", { state: "working", turns: 6, drone_id: ARC_DRONES.T7 });
  return {
    name: "doneTouched",
    says: "Implement — a finished task's file was edited by a later one, and it stays done",
    fixtures: [
      executing({
        says: "running — a later task edited a file a finished task had written",
        groups,
        step: implementStep(allPassed(checkNames(BRIDGE_CHECKS), 2), "2026-09-22T11:05:00Z"),
        processes: [droneProcess(53_402, "02:55")],
      }),
    ],
    opens: ARC_JOB_ID,
    draft: {
      groups,
      cases: arcCases(),
      criteria: arcCriterionViews(),
      // What the gate settled and froze — the Drone cap a concurrent group is
      // bounded by, among the rest. `#1550`.
      proposal: arcProposal({ status: "approved" }),
      landing: ARC_LANDING,
      record: [
        ...recordThroughGroupTwo(),
        {
          at: "2026-09-22T11:02:00Z",
          coord: { step: "implement", step_attempt: 1, group: "g4", group_attempt: 1, task: "T7" },
          actor: "fleet",
          kind: "touched_after_done",
          what: "T7 wrote running-rows.tsx, which T6 had finished",
          outcome: "T6 stays done, and is flagged",
          cursor: 12,
        },
      ],
      pulse: pulse("2026-09-22T11:05:00.000Z", [droneProcess(53_402, "02:55")]),
    },
  };
}
