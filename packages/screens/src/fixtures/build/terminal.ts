// Five ways this Job could have ended, light depth.

import type { JobFixture } from "../fixture";
import type { StepDetail } from "@armada/protocol";
import {
  advancedStep,
  BUILD_CHECK,
  consumersStep,
  detail,
  foldedReads,
  freshStep,
  holdsRead,
  job,
  JOB_ID,
  landStep,
  manifest,
  NEXTEST_CHECK,
  NO_JOURNALLED,
  NO_OBSERVED,
  NOW,
  reproStep,
  rootCauseStep,
  watchedRead,
  workflow,
} from "./base";

const RECLAIMED = {
  job_id: JOB_ID,
  read_at: "2026-09-10T15:00:00.000Z",
  held: "none" as const,
  processes: [],
};

export function completedSuccess(): JobFixture {
  const theJob = job("completed_success", { current_step_id: "land" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    advancedStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    advancedStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]),
    advancedStep("land", "Land", 6),
  ];
  const whole = detail(theJob, steps, {
    delivery: {
      commit: "a1b2c3d",
      pushed: "origin/fix/settings-split-selectors",
      pull_request: "https://forge.example/armada/settings/pull/501",
      landed: "merged",
    },
  });
  return {
    name: "completed_success — every step advanced, every criterion verified",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(RECLAIMED),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

function regressionFailedForGood(): StepDetail {
  return {
    ...freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    state: "stopped",
    check_runs: [{ attempt: 1, name: "cargo_nextest", outcome: "failed", produced: "exit 101" }],
    attempts: [{ attempt: 1, outcome: "stopped", why: "gate_failure", started_at: "2026-09-10T14:22:18Z", ended_at: "2026-09-10T14:24:00Z" }],
    verdicts: [{ attempt: 1, named: "failed", trigger: "gate_failure" }],
    last_verdict: { attempt: 1, named: "failed", trigger: "gate_failure" },
  };
}

export function completedFailed(): JobFixture {
  const theJob = job("completed_failed", { current_step_id: "regression_verify" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    regressionFailedForGood(),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps);
  return {
    name: "completed_failed — a person accepted the failure as the outcome",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(RECLAIMED),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

function regressionAwaitingHuman(): StepDetail {
  return {
    ...freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    state: "awaiting_human",
    check_runs: [{ attempt: 1, name: "cargo_nextest", outcome: "passed" }],
    attempts: [{ attempt: 1, outcome: "awaiting_human", started_at: "2026-09-10T14:22:18Z" }],
  };
}

export function rejected(): JobFixture {
  const theJob = job("rejected", { current_step_id: "regression_verify" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    regressionAwaitingHuman(),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps);
  return {
    name: "rejected — a person declined the work at the human gate",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(RECLAIMED),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function killed(): JobFixture {
  const theJob = job("killed", { current_step_id: "fix", assigned_drone: undefined });
  const steps = [
    reproStep(),
    rootCauseStep(),
    { ...freshStep("fix", "Fix", 3, [BUILD_CHECK]), state: "running", attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:16:07Z" }] },
    freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps);
  return {
    name: "killed — cleared from the Board, carrying no verdict",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(RECLAIMED),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function superseded(): JobFixture {
  const theJob = job("superseded", {
    current_step_id: "repro",
    branch: undefined,
    assigned_drone: undefined,
  });
  const steps = [
    freshStep("repro", "Reproduction", 1),
    freshStep("root_cause", "Root cause", 2),
    freshStep("fix", "Fix", 3, [BUILD_CHECK]),
    freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps, {
    branch: undefined,
    dependencies: [{ direction: "after", peer: "01M2C1TJ8G0016YK5APEERJOB" }],
  });
  return {
    name: "superseded — the work landed outside this Job, and there is nothing left for it to do",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({ job_id: JOB_ID, read_at: "2026-09-10T14:12:00.000Z", held: "none", processes: [] }),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
