// Two `running` moments the roster's own `running` fixture does not reach:
// a step whose Check has already failed once and is being retried, and a step
// whose gates have not been asked anything yet.
//
// **Both stay `running`.** Neither has spent its retry budget or stopped for a
// person — `awaiting_repair` and `escalated` are `escalated.ts`'s and
// `waiting.ts`'s, and drawing either state as one of those would tell a
// person the Job had stopped when the Drone is still on it.

import type { JobFixture } from "../fixture";
import type { StepDetail, WorkflowSummary } from "@armada/protocol";
import {
  advancedStep,
  answered,
  BUILD_CHECK,
  called,
  checked,
  consumersStep,
  detail,
  foldedReads,
  holdsRead,
  instructed,
  job,
  journalledWatching,
  landStep,
  manifest,
  NEXTEST_CHECK,
  nextestRun,
  note,
  NOW,
  observedWatching,
  reproStep,
  resources,
  rootCauseStep,
  said,
  watchedRead,
  workflow,
} from "./base";

const REGRESSION_LOG =
  ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log";

const NEXTEST_FAILURE =
  "FAIL settings::selectors::visible_manifests_memoises\n  expected the same reference on " +
  "repeat calls, got a new object";

function nextestFailed(attempt: number) {
  return {
    attempt,
    name: "cargo_nextest",
    outcome: "failed",
    expected: "cargo nextest run --workspace exits 0",
    produced: "exit 101 — 3 of 2034 tests failed",
    output_path: `.armada/checks/77-split-the-settings-reducer/regression_verify.${attempt}.cargo_nextest.log`,
  };
}

/**
 * `regression_verify`, attempt 1 failed and attempt 2 in progress — the
 * moment `escalated.ts`'s `escalatedGateFailure` reaches on its third and
 * last try. The Drone is fixing what the Check named, so nothing here asks
 * anything of a person.
 */
function retryingStep(): StepDetail {
  return {
    step_id: "regression_verify",
    label: "Regression check",
    ordinal: 4,
    state: "retrying",
    checks: [NEXTEST_CHECK],
    check_runs: [nextestFailed(1)],
    judge_checks: [{ criteria: 2, gaming_check: true }],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [
      { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: "2026-09-10T14:22:18Z", ended_at: "2026-09-10T14:24:40Z" },
      { attempt: 2, outcome: "running", started_at: "2026-09-10T14:24:40Z" },
    ],
    verdicts: [{ attempt: 1, named: "failed", trigger: "gate_failure" }],
    last_verdict: { attempt: 1, named: "failed", trigger: "gate_failure" },
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:25:10Z",
  };
}

export function retryingCheckFailure(): JobFixture {
  const theJob = job("running", { current_step_id: "regression_verify" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    retryingStep(),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps);

  const rows = [
    instructed("regression_verify", "2026-09-10T14:22:18Z", 4, "cargo nextest run --workspace exits 0", "Regression check"),
    called("regression_verify", "2026-09-10T14:23:02Z", "call_nextest_1", "Bash", "cargo nextest run --workspace"),
    checked("regression_verify", "2026-09-10T14:24:40Z", nextestFailed(1)),
    said("regression_verify", "2026-09-10T14:24:45Z", "Attempt 2: the memo key is keeping the old reference. Adjusting it."),
    called("regression_verify", "2026-09-10T14:25:10Z", "call_nextest_2", "Bash", "cargo nextest run --workspace"),
  ];

  return {
    name: "running — a Check failed and the Drone is retrying, attempt 2 of 3",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedWatching(rows, true),
    journalled: journalledWatching([
      note("2026-09-10T14:11:02Z", "A worktree was cut for this job."),
      note("2026-09-10T14:24:40Z", "Regression check failed on attempt 1 of 3. The step was handed back.", {
        level: "warn",
        step: "regression_verify",
      }),
    ]),
    resources: holdsRead(
      resources("running", {
        processes: [
          { pid: 41233, command: "node", cpu_percent: 6.4, memory_bytes: 411_041_792, running_for: "03:12", recorded: true },
        ],
        wrote_last_at: "2026-09-10T14:25:10.000Z",
      }),
    ),
    recorded: foldedReads(),
    calls: {
      call_nextest_2: {
        ok: true,
        call: { tool: "Bash", call: "call_nextest_2", arguments: "cargo nextest run --workspace", whole: true },
      },
    },
    checkOutputs: {
      "regression_verify.1.cargo_nextest.log": {
        ok: true,
        output: {
          attempt: 1,
          name: "cargo_nextest",
          path: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
          lines: NEXTEST_FAILURE.split("\n"),
          from_line: 1,
          total_lines: 2,
          bytes: NEXTEST_FAILURE.length,
          whole: true,
        },
      },
    },
    frames: {},
    now: NOW,
  };
}

/**
 * A second, later version of the shared Bug workflow — `regression_verify`
 * gained a build Check alongside nextest. **Not a change to `workflow()`
 * itself.** Every other fixture in this roster writes `regression_verify`'s
 * `StepDetail.checks` by hand rather than reading it off `WorkflowSummary`, so
 * mutating the shared function would leave five other fixtures whose frozen
 * workflow claimed two Checks and whose step still carried one — the
 * mismatch a real `WorkflowDef` version bump exists to avoid. `version` moves
 * and `id` does not, the same distinction a real redispatch onto a newer
 * Manifest would carry.
 */
function workflowWithBuildCheckOnRegression(): WorkflowSummary {
  const base = workflow();
  return {
    ...base,
    version: base.version + 1,
    steps: base.steps.map((step) =>
      step.step_id === "regression_verify" ? { ...step, checks: [NEXTEST_CHECK, BUILD_CHECK] } : step,
    ),
  };
}

/**
 * `regression_verify`, mid-attempt with two declared Checks: nextest has
 * already reported and passed, and the build Check is still queued behind
 * it. The Judge is declared too and has been asked nothing, so it queues
 * behind both.
 *
 * **There is no wire value for a Check that is still running.**
 * `check-outcomes.toml` names six things a Check can come to once the gate
 * has run it, and none of them is "in progress" — a `CheckRun` is the record
 * of what a Check *did*. `checksStage` in `phases.tsx` reads the tier as
 * `current` off `ran.length` sitting between zero and the declared count,
 * which is the honest wire equivalent of "one done, one still coming": what
 * is actually true is that nextest's own reading has arrived and the build
 * Check's has not.
 */
function twoChecksOneReported(): StepDetail {
  return {
    step_id: "regression_verify",
    label: "Regression check",
    ordinal: 4,
    state: "running",
    checks: [NEXTEST_CHECK, BUILD_CHECK],
    check_runs: [nextestRun({ attempt: 1, output_path: REGRESSION_LOG })],
    judge_checks: [{ criteria: 2, panel_size: 3, gaming_check: true }],
    judged: [],
    flagged: [],
    overridden: false,
    attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:22:18Z" }],
    verdicts: [],
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:23:40Z",
  };
}

export function runningAtGate(): JobFixture {
  const theJob = job("running", { current_step_id: "regression_verify" });
  const steps = [
    reproStep(),
    rootCauseStep(),
    advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    twoChecksOneReported(),
    consumersStep(),
    landStep(),
  ];
  const whole = detail(theJob, steps);

  const rows = [
    instructed("regression_verify", "2026-09-10T14:22:18Z", 4, "cargo nextest run --workspace exits 0", "Regression check"),
    called("regression_verify", "2026-09-10T14:22:40Z", "call_nextest_1", "Bash", "cargo nextest run --workspace"),
    answered("regression_verify", "2026-09-10T14:23:22Z", "call_nextest_1"),
    checked("regression_verify", "2026-09-10T14:23:22Z", nextestRun({ attempt: 1, output_path: REGRESSION_LOG })),
    said("regression_verify", "2026-09-10T14:23:28Z", "Nextest passed. Running the build Check next."),
    called("regression_verify", "2026-09-10T14:23:40Z", "call_build_1", "Bash", "cargo build --workspace --locked"),
  ];

  return {
    name: "running — Regression check: nextest passed, the build Check is queued behind it",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflowWithBuildCheckOnRegression()],
    manifests: [manifest()],
    observed: observedWatching(rows, true),
    journalled: journalledWatching([note("2026-09-10T14:11:02Z", "A worktree was cut for this job.")]),
    resources: holdsRead(
      resources("running", {
        processes: [
          { pid: 41233, command: "node", cpu_percent: 5.6, memory_bytes: 405_112_832, running_for: "01:22", recorded: true },
        ],
        wrote_last_at: "2026-09-10T14:23:40.000Z",
      }),
    ),
    recorded: foldedReads(),
    calls: {
      call_nextest_1: {
        ok: true,
        call: { tool: "Bash", call: "call_nextest_1", arguments: "cargo nextest run --workspace", whole: true },
      },
      call_build_1: {
        ok: true,
        call: { tool: "Bash", call: "call_build_1", arguments: "cargo build --workspace --locked", whole: true },
      },
    },
    checkOutputs: {
      "regression_verify.1.cargo_nextest.log": {
        ok: true,
        output: {
          attempt: 1,
          name: "cargo_nextest",
          path: REGRESSION_LOG,
          lines: ["Summary [ 41.203s] 2034 tests run: 2034 passed"],
          from_line: 1,
          total_lines: 1,
          bytes: 45,
          whole: true,
        },
      },
    },
    frames: {},
    now: NOW,
  };
}
