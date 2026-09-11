// `awaiting_review` — every Check passed, the Judge met every criterion,
// waiting on a person.
//
// **Full depth.** Regression check's own gate is a human one mid-workflow —
// `advance_gate: "human_always"` on a step that is not the workflow's last —
// so no pull request is open yet. `Decide`'s merge answer only draws where
// `whole.delivery.pull_request` is present, which this fixture leaves absent
// on purpose: the roster's escalated fixture is the one that reaches delivery.

import type { JobFixture } from "../fixture";
import {
  answered,
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
  observedEnded,
  producedTurn,
  reproStep,
  resources,
  rootCauseStep,
  said,
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

function regressionStep() {
  const step = freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]);
  return {
    ...step,
    state: "awaiting_human",
    check_runs: [nextestPassed()],
    judge_checks: [{ criteria: 2, gaming_check: true }],
    judged: [
      {
        attempt: 1,
        criterion_id: "c1",
        verdict: "met",
        expected: "packages/settings/src/selectors.ts imports no store type",
        produced: "The module imports only RootState's field types, never the store itself.",
      },
      {
        attempt: 1,
        criterion_id: "c2",
        verdict: "met",
        expected: "cargo nextest run --workspace exits 0",
        produced: "2034 of 2034 tests passed, including the settings suite.",
      },
    ],
    attempts: [{ attempt: 1, outcome: "awaiting_human", started_at: "2026-09-10T14:22:18Z" }],
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:29:40Z",
  };
}

function nextestPassed() {
  return {
    attempt: 1,
    name: "cargo_nextest",
    outcome: "passed",
    output_path: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
  };
}

const FILES = [
  { path: "packages/settings/src/selectors.ts", change: "modified" },
  { path: "packages/settings/src/reducer.ts", change: "modified" },
  { path: "packages/settings/src/index.ts", change: "added" },
];

export function review(): JobFixture {
  const theJob = job("awaiting_review", { current_step_id: "regression_verify" });
  const steps = [reproStep(), rootCauseStep(), fixStep(), regressionStep(), consumersStep(), landStep()];
  const whole = detail(theJob, steps);

  const rows = [
    instructed("repro", "2026-09-10T14:11:15Z", 1, "the reproduction fails on main", "Reproduction"),
    droneEnded("repro", "2026-09-10T14:12:27Z", 4, 120_000),
    instructed("root_cause", "2026-09-10T14:12:27Z", 2, "the root cause is written down", "Root cause"),
    droneEnded("root_cause", "2026-09-10T14:16:07Z", 9, 300_000),
    instructed("fix", "2026-09-10T14:16:07Z", 3, "the selectors module has no import of the store", "Fix"),
    called("fix", "2026-09-10T14:17:20Z", "call_edit_1", "Edit", "packages/settings/src/selectors.ts"),
    answered("fix", "2026-09-10T14:17:21Z", "call_edit_1"),
    checked("fix", "2026-09-10T14:21:40Z", buildRun({ attempt: 1 })),
    producedTurn("fix", "2026-09-10T14:22:00Z", FILES),
    droneEnded("fix", "2026-09-10T14:22:18Z", 21, 1_100_000),
    instructed(
      "regression_verify",
      "2026-09-10T14:22:18Z",
      4,
      "cargo nextest run --workspace exits 0",
      "Regression check",
    ),
    called(
      "regression_verify",
      "2026-09-10T14:23:02Z",
      "call_nextest_1",
      "Bash",
      "cargo nextest run --workspace",
    ),
    answered("regression_verify", "2026-09-10T14:29:24Z", "call_nextest_1"),
    checked("regression_verify", "2026-09-10T14:29:24Z", nextestPassed()),
    said("regression_verify", "2026-09-10T14:29:30Z", "Every existing settings test still passes."),
    droneEnded("regression_verify", "2026-09-10T14:29:40Z", 6, 380_000),
  ];

  return {
    name: "awaiting_review — every Check passed, the Judge met every criterion",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedEnded(rows, "drone_ended"),
    journalled: journalledWatching([
      note("2026-09-10T14:11:02Z", "A worktree was cut for this job."),
      note("2026-09-10T14:29:40Z", "Regression check's gate passed. Waiting on a person to advance it."),
    ]),
    resources: holdsRead(
      resources("none", { processes: [], wrote_last_at: "2026-09-10T14:29:40.000Z" }),
    ),
    recorded: foldedReads({
      evidence: evidenceRead([
        {
          step_id: "repro",
          evidence_type: "failing_test",
          claimed: "useColumnSelectors re-runs the whole selector graph on any settings write.",
          shown_by: "packages/settings/test/useColumnSelectors.test.ts",
        },
        {
          step_id: "fix",
          evidence_type: "diff",
          claimed: "The selector module now imports no store type.",
          shown_by: "packages/settings/src/selectors.ts",
        },
        {
          step_id: "regression_verify",
          evidence_type: "test_suite_run",
          claimed: "2034 of 2034 tests pass.",
          shown_by: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
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
      call_nextest_1: {
        ok: true,
        call: {
          tool: "Bash",
          call: "call_nextest_1",
          arguments: "cargo nextest run --workspace",
          whole: true,
        },
      },
    },
    checkOutputs: {
      "regression_verify.1.cargo_nextest.log": {
        ok: true,
        output: {
          attempt: 1,
          name: "cargo_nextest",
          path: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
          lines: ["Summary [ 41.203s] 2034 tests run: 2034 passed, 0 skipped"],
          from_line: 1,
          total_lines: 1,
          bytes: 58,
          whole: true,
        },
      },
    },
    frames: {},
    now: NOW,
  };
}
