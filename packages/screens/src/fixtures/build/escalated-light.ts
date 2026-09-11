// Five more reasons a Job can escalate, light depth. `escalated.ts` carries
// the roster's full-depth trigger, `gate_failure`; these are the ones the
// roster asks for at lighter depth — each just enough to prove `renderFor`
// lands on `stopped` and `ESCALATION_REASON` has a row this build can draw.

import type { JobFixture } from "../fixture";
import type { Refusal, StepDetail, Stuck } from "@armada/protocol";
import {
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
  stuck,
  watchedRead,
  workflow,
} from "./base";

const REFUSED_POLICY: Refusal = {
  tool: "Bash",
  call: "call_npm_1",
  detail: "npm publish --access public",
  truncated: false,
  because: "not on the allowlist",
  // All three: the Job stopped at `blocked_by_policy` and its Drone is still
  // there to be told no, which is what Fleet offers Reject on.
  offers: ["allow_for_job", "always_allow", "reject"],
};

/** `fix`, frozen `running` — every one of these five stops mid-step. */
function frozenFix(): StepDetail {
  return {
    ...freshStep("fix", "Fix", 3, [BUILD_CHECK]),
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:16:07Z" }],
  };
}

function escalatedFixture(name: string, trigger: string, extra: Partial<Stuck>): JobFixture {
  const theJob = job("escalated", { current_step_id: "fix", reason: { named: trigger } });
  const steps = [
    reproStep(),
    rootCauseStep(),
    frozenFix(),
    freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps, {
    stuck: stuck({
      stopped_by: trigger,
      step_id: "fix",
      recourse: ["redirect_drone", "redispatch_job"],
      worktree_on_disk: true,
      drone_unheard: false,
      ...extra,
    }),
  });
  return {
    name,
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: holdsRead({
      job_id: JOB_ID,
      read_at: "2026-09-10T14:31:00.000Z",
      held: "running",
      processes: [
        { pid: 41233, command: "node", cpu_percent: 0.1, memory_bytes: 402_653_184, running_for: "15:12", recorded: true },
      ],
      worktree: { path: ".armada/worktrees/77-split-the-settings-reducer", branch: "fix/settings-split-selectors", bytes: 1_310_720_000 },
    }),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}

export function escalatedBlockedByPolicy(): JobFixture {
  return escalatedFixture("escalated · blocked_by_policy — the drone reached for a command not on the allowlist", "blocked_by_policy", {
    recourse: ["redirect_drone", "redispatch_job"],
    refused: [REFUSED_POLICY],
    refusals: 1,
  });
}

export function escalatedInterrupted(): JobFixture {
  return escalatedFixture("escalated · interrupted — fleet restarted mid-run and lost track of the drone", "interrupted", {
    recourse: ["restart_step", "redispatch_job"],
    drone_unheard: true,
  });
}

export function escalatedSilent(): JobFixture {
  return escalatedFixture("escalated · silent — the drone stopped writing and nothing explains why", "silent", {
    recourse: ["restart_step", "redispatch_job"],
    drone_unheard: true,
  });
}

export function escalatedLoopCap(): JobFixture {
  return escalatedFixture("escalated · loop_cap — the step hit its iteration cap", "loop_cap", {
    recourse: ["redispatch_job"],
  });
}

export function escalatedNoReport(): JobFixture {
  return escalatedFixture("escalated · no_report — the drone's run ended and it never submitted", "no_report", {
    recourse: ["restart_step", "redispatch_job"],
  });
}
