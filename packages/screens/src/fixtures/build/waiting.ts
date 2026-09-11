// Five statuses a person or a queue is holding this Job at, light depth.
//
// **Nothing here is full-depth**, per the roster: no turns, no journal, no
// Check outputs — just enough of `JobDetail` for `renderFor` to land on the
// right render and for the panel to draw without a step to say anything about.

import type { JobResources, StepDetail } from "@armada/protocol";
import type { JobFixture } from "../fixture";
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
  spend,
  stuck,
  watchedRead,
  workflow,
} from "./base";

/** No worktree yet — every step still ahead. Used by both gate statuses. */
function beforeDispatch(): { steps: StepDetail[]; resources: JobResources } {
  return {
    steps: [
      freshStep("repro", "Reproduction", 1),
      freshStep("root_cause", "Root cause", 2),
      freshStep("fix", "Fix", 3, [BUILD_CHECK]),
      freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
      consumersStep(),
      landStep(),
    ],
    resources: { job_id: JOB_ID, read_at: "2026-09-10T14:12:00.000Z", held: "none", processes: [] },
  };
}

export function queued(): JobFixture {
  const before = beforeDispatch();
  const theJob = job("queued", {
    current_step_id: "repro",
    queued_reason: "waiting_on_resources",
    branch: undefined,
    assigned_drone: undefined,
  });
  const whole = detail(theJob, before.steps, { branch: undefined, spend: spend({ cost_micros: 0, turns: 0, ran_ms: 0, drones: 0 }) });
  return {
    name: "queued — approved, waiting on a free drone",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(before.resources),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function awaitingApproval(): JobFixture {
  const before = beforeDispatch();
  const theJob = job("awaiting_approval", {
    current_step_id: "repro",
    branch: undefined,
    assigned_drone: undefined,
  });
  const whole = detail(theJob, before.steps, { branch: undefined, spend: spend({ cost_micros: 0, turns: 0, ran_ms: 0, drones: 0 }) });
  return {
    name: "awaiting_approval — a person must approve the dispatch",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead(before.resources),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

function regressionSpentRetries(): StepDetail {
  return {
    ...freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    state: "stopped",
    check_runs: [
      { attempt: 1, name: "cargo_nextest", outcome: "failed", produced: "exit 101" },
      { attempt: 2, name: "cargo_nextest", outcome: "failed", produced: "exit 101" },
    ],
    attempts: [
      { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: "2026-09-10T14:22:18Z", ended_at: "2026-09-10T14:24:00Z" },
      { attempt: 2, outcome: "stopped", why: "gate_failure", started_at: "2026-09-10T14:24:00Z", ended_at: "2026-09-10T14:25:40Z" },
    ],
    verdicts: [{ attempt: 2, named: "failed", trigger: "gate_failure" }],
    last_verdict: { attempt: 2, named: "failed", trigger: "gate_failure" },
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:25:40Z",
  };
}

export function awaitingRepair(): JobFixture {
  const theJob = job("awaiting_repair", { current_step_id: "regression_verify", assigned_drone: undefined });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    regressionSpentRetries(),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps, {
    stuck: stuck({
      step_id: "regression_verify",
      recourse: ["restart_step", "redispatch_job"],
      worktree_on_disk: true,
      drone_unheard: false,
    }),
  });
  return {
    name: "awaiting_repair — the retry budget is spent and the work is unfinished",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:26:00.000Z",
      held: "none",
      processes: [],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_310_720_000 },
    }),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function awaitingAttestation(): JobFixture {
  const theJob = job("awaiting_attestation", {
    current_step_id: "land",
    reason: { criteria_owed: ["c2"] },
  });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    advancedStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    advancedStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]),
    advancedStep("land", "Land", 6),
  ];
  const whole = detail(theJob, steps);
  return {
    name: "awaiting_attestation — the work landed, a criterion needs a person's action outside Armada",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:31:00.000Z",
      held: "none",
      processes: [],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_476_395_008 },
    }),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function piloted(): JobFixture {
  const theJob = job("piloted", {
    current_step_id: "fix",
    reason: { named: "take_over" },
    assigned_drone: undefined,
  });
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
    name: "piloted — a person is working it now, and the drone is gone",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:31:00.000Z",
      held: "none",
      processes: [],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_310_720_000 },
    }),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
