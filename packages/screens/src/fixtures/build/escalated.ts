// `escalated` / `gate_failure` — a Check failed and ended the Job.
//
// **Full depth.** Three attempts of `cargo_nextest`, all the same failure,
// retries spent — `RUN_STOPPED` in `InsideAJob`'s
// fixtures draws the same shape as `RunTreeStep`s; this is its wire original.
// The Drone stays alive and idle, which `job-statuses.toml`'s own row for
// `escalated` states as the ordinary case where a step stopped mid-work, so
// `resources` and `observed` both show a live, quiet process rather than none.

import type { JobFixture } from "../fixture";
import {
  BUILD_CHECK,
  buildRun,
  called,
  checked,
  consumersStep,
  detail,
  diffRead,
  droneEnded,
  evidenceRead,
  foldedReads,
  freshStep,
  holdsRead,
  instructed,
  job,
  journalledWatching,
  landStep,
  manifest,
  NEXTEST_CHECK,
  note,
  NOW,
  observedWatching,
  producedTurn,
  reproStep,
  resources,
  rootCauseStep,
  said,
  stuck,
  watchedRead,
  workflow,
} from "./base";

function fixStep() {
  const step = freshStep("fix", "Fix", 3, [BUILD_CHECK]);
  return {
    ...step,
    state: "advanced",
    check_runs: [buildRun({ attempt: 1 })],
    attempts: [{ attempt: 1, outcome: "advanced", started_at: "2026-09-10T14:16:07Z", ended_at: "2026-09-10T14:22:18Z" }],
    verdicts: [{ attempt: 1, named: "passed" as const }],
    entered_at: "2026-09-10T14:16:07Z",
    updated_at: "2026-09-10T14:22:18Z",
  };
}

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

function regressionStep() {
  const step = freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]);
  return {
    ...step,
    state: "stopped",
    check_runs: [nextestFailed(1), nextestFailed(2), nextestFailed(3)],
    attempts: [
      { attempt: 1, outcome: "retrying", why: "gate_failure", started_at: "2026-09-10T14:22:18Z", ended_at: "2026-09-10T14:24:40Z" },
      { attempt: 2, outcome: "retrying", why: "gate_failure", started_at: "2026-09-10T14:24:40Z", ended_at: "2026-09-10T14:26:55Z" },
      { attempt: 3, outcome: "stopped", why: "gate_failure", started_at: "2026-09-10T14:26:55Z", ended_at: "2026-09-10T14:29:04Z" },
    ],
    verdicts: [
      { attempt: 1, named: "failed", trigger: "gate_failure" },
      { attempt: 2, named: "failed", trigger: "gate_failure" },
      { attempt: 3, named: "failed", trigger: "gate_failure" },
    ],
    last_verdict: { attempt: 3, named: "failed", trigger: "gate_failure" },
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:29:04Z",
  };
}

const FILES = [
  { path: "packages/settings/src/selectors.ts", change: "modified" },
  { path: "packages/settings/src/reducer.ts", change: "modified" },
  { path: "packages/settings/src/index.ts", change: "added" },
];

export function escalatedGateFailure(): JobFixture {
  const theJob = job("escalated", {
    current_step_id: "regression_verify",
    reason: { named: "gate_failure" },
  });
  const steps = [reproStep(), rootCauseStep(), fixStep(), regressionStep(), consumersStep(), landStep()];
  const whole = detail(theJob, steps, {
    stuck: stuck({
      stopped_by: "gate_failure",
      step_id: "regression_verify",
      recourse: ["override_verdict", "redirect_drone", "redispatch_job"],
      worktree_on_disk: true,
      drone_unheard: false,
    }),
  });

  const rows = [
    instructed("repro", "2026-09-10T14:11:15Z", 1, "the reproduction fails on main", "Reproduction"),
    droneEnded("repro", "2026-09-10T14:12:27Z", 4, 120_000),
    instructed("root_cause", "2026-09-10T14:12:27Z", 2, "the root cause is written down", "Root cause"),
    droneEnded("root_cause", "2026-09-10T14:16:07Z", 9, 300_000),
    instructed("fix", "2026-09-10T14:16:07Z", 3, "the selectors module has no import of the store", "Fix"),
    producedTurn("fix", "2026-09-10T14:22:00Z", FILES),
    droneEnded("fix", "2026-09-10T14:22:18Z", 21, 1_100_000),
    instructed(
      "regression_verify",
      "2026-09-10T14:22:18Z",
      4,
      "cargo nextest run --workspace exits 0",
      "Regression check",
    ),
    called("regression_verify", "2026-09-10T14:23:02Z", "call_nextest_1", "Bash", "cargo nextest run --workspace"),
    checked("regression_verify", "2026-09-10T14:24:40Z", nextestFailed(1)),
    said("regression_verify", "2026-09-10T14:24:45Z", "Attempt 2: adjusting the memo key."),
    called("regression_verify", "2026-09-10T14:25:10Z", "call_nextest_2", "Bash", "cargo nextest run --workspace"),
    checked("regression_verify", "2026-09-10T14:26:55Z", nextestFailed(2)),
    said("regression_verify", "2026-09-10T14:27:00Z", "Attempt 3: the same failure again."),
    called("regression_verify", "2026-09-10T14:27:30Z", "call_nextest_3", "Bash", "cargo nextest run --workspace"),
    checked("regression_verify", "2026-09-10T14:29:04Z", nextestFailed(3)),
  ];

  return {
    name: "escalated · gate_failure — a Check failed and ended the Job",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedWatching(rows, true),
    journalled: journalledWatching([
      note("2026-09-10T14:11:02Z", "A worktree was cut for this job."),
      note(
        "2026-09-10T14:29:04Z",
        "Regression check failed on attempt 3 of 3. Escalated: gate_failure.",
        { level: "warn", step: "regression_verify" },
      ),
    ]),
    resources: holdsRead(
      resources("running", {
        processes: [
          {
            pid: 41233,
            command: "node",
            cpu_percent: 0.2,
            memory_bytes: 431_144_960,
            running_for: "17:49",
            recorded: true,
          },
        ],
        wrote_last_at: "2026-09-10T14:29:04.000Z",
      }),
    ),
    recorded: foldedReads({
      evidence: evidenceRead([
        {
          step_id: "regression_verify",
          evidence_type: "test_suite_run",
          claimed: "3 of 2034 tests fail the same way on every attempt.",
          shown_by: ".armada/checks/77-split-the-settings-reducer/regression_verify.3.cargo_nextest.log",
        },
      ]),
      diff: diffRead(
        FILES,
        [
          "--- a/packages/settings/src/selectors.ts",
          "+++ b/packages/settings/src/selectors.ts",
          "@@ -14,6 +14,9 @@",
          "+import { selectColumnOrder } from './selectors/columns'",
        ].join("\n"),
      ),
    }),
    calls: {
      call_nextest_3: {
        ok: true,
        call: {
          tool: "Bash",
          call: "call_nextest_3",
          arguments: "cargo nextest run --workspace",
          whole: true,
        },
      },
    },
    checkOutputs: {
      "regression_verify.3.cargo_nextest.log": {
        ok: true,
        output: {
          attempt: 3,
          name: "cargo_nextest",
          path: ".armada/checks/77-split-the-settings-reducer/regression_verify.3.cargo_nextest.log",
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
