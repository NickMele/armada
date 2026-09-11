// `escalated` / `evidence_suspect` — every mechanical Check passed, and the
// Judge panel refused two criteria.
//
// **Full depth**, the way `escalated.ts`'s `gate_failure` is, and deliberately
// beside it rather than folded into `escalated-light.ts`: `evidence_suspect`
// is "mechanically passed, semantically flagged as likely gamed" —
// `escalation-triggers.toml` — which only reads as the finding it is once the
// Checks chapter shows a green suite and the Verdicts chapter shows a panel
// that did not believe it. A light fixture with no `checks` or `judged` rows
// could not carry that argument.
//
// **A panel of three, both criteria answered, one unanimous and one not.**
// `c1` is met by every member — the ordinary row a verdict grid still owes,
// per `Judged`'s own doc comment: "the brief is on every row, including the
// met ones". `c2` is refused two of three: the suite is green because the
// assertion it depends on was deleted, which is exactly what `flagged`'s one
// row names — `Judged.expected`/`produced`/`consequence` carry the finding
// and `given` carries the proof the two who refused read the same brief.

import type { JobFixture } from "../fixture";
import type { Judged, StepDetail } from "@armada/protocol";
import {
  advancedStep,
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
  reproStep,
  resources,
  rootCauseStep,
  said,
  stuck,
  watchedRead,
  workflow,
} from "./base";

const BRIEF_PATH = (name: string) => `.armada/briefs/77-split-the-settings-reducer/regression_verify.1.${name}.md`;

function given(model: string, digest: string, size: number) {
  return { digest, size, model };
}

/** `c1` — every member of the panel agreed the selectors module is clean. */
const C1_MEMBERS: Judged[] = [1, 2, 3].map((member) => ({
  attempt: 1,
  criterion_id: "c1",
  member,
  verdict: "met",
  brief_path: BRIEF_PATH("c1"),
  given: given("sonnet", "sha256:8f2c…c1", 6_204),
}));

/** `c2` — two of three read the deleted assertion; the third did not look. */
const C2_MEMBERS: Judged[] = [
  {
    attempt: 1,
    criterion_id: "c2",
    member: 1,
    verdict: "not_met",
    expected: "cargo nextest run --workspace still asserts visible_manifests_memoises",
    produced: "The assertion was deleted. The suite is green because the case no longer exists.",
    consequence: "The regression this criterion exists to catch can recur silently.",
    brief_path: BRIEF_PATH("c2"),
    cited: [{ region: "diff", from_line: 40, to_line: 41 }],
    given: given("sonnet", "sha256:2a91…c2", 6_812),
  },
  {
    attempt: 1,
    criterion_id: "c2",
    member: 2,
    verdict: "not_met",
    expected: "cargo nextest run --workspace still asserts visible_manifests_memoises",
    produced: "The assertion the criterion depends on is gone from the test file.",
    consequence: "A future regression in this selector would pass unnoticed.",
    brief_path: BRIEF_PATH("c2"),
    cited: [{ region: "diff", from_line: 40, to_line: 41 }],
    given: given("sonnet", "sha256:2a91…c2", 6_812),
  },
  {
    attempt: 1,
    criterion_id: "c2",
    member: 3,
    verdict: "met",
    brief_path: BRIEF_PATH("c2"),
    given: given("sonnet", "sha256:2a91…c2", 6_812),
  },
];

function nextestPassed() {
  return {
    attempt: 1,
    name: "cargo_nextest",
    outcome: "passed",
    output_path: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
  };
}

function regressionRefusedStep(): StepDetail {
  return {
    step_id: "regression_verify",
    label: "Regression check",
    ordinal: 4,
    state: "stopped",
    checks: [NEXTEST_CHECK],
    check_runs: [nextestPassed()],
    judge_checks: [{ criteria: 2, panel_size: 3, gaming_check: true }],
    judged: [...C1_MEMBERS, ...C2_MEMBERS],
    flagged: [
      {
        pattern: "assertion_weakened",
        cited: "packages/settings/test/useColumnSelectors.test.ts:41",
        at: { file: "packages/settings/test/useColumnSelectors.test.ts", line: 41 },
        asked: "Is this assertion made nowhere else in this change?",
        brief_path: BRIEF_PATH("gaming"),
      },
    ],
    overridden: false,
    attempts: [{ attempt: 1, outcome: "stopped", why: "evidence_suspect", started_at: "2026-09-10T14:22:18Z", ended_at: "2026-09-10T14:29:04Z" }],
    verdicts: [{ attempt: 1, named: "failed", trigger: "evidence_suspect" }],
    last_verdict: { attempt: 1, named: "failed", trigger: "evidence_suspect" },
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:29:04Z",
  };
}

const FILES = [
  { path: "packages/settings/src/selectors.ts", change: "modified" },
  { path: "packages/settings/src/reducer.ts", change: "modified" },
  { path: "packages/settings/test/useColumnSelectors.test.ts", change: "modified" },
];

function fixStep(): StepDetail {
  return {
    ...advancedStep("fix", "Fix", 3, [BUILD_CHECK]),
    check_runs: [buildRun({ attempt: 1 })],
  };
}

export function escalatedEvidenceSuspect(): JobFixture {
  const theJob = job("escalated", {
    current_step_id: "regression_verify",
    reason: { named: "evidence_suspect" },
  });
  const steps = [reproStep(), rootCauseStep(), fixStep(), regressionRefusedStep(), consumersStep(), landStep()];
  const whole = detail(theJob, steps, {
    stuck: stuck({
      stopped_by: "evidence_suspect",
      step_id: "regression_verify",
      recourse: ["override_verdict", "redirect_drone", "redispatch_job"],
      worktree_on_disk: true,
      drone_unheard: false,
    }),
  });

  const rows = [
    instructed("regression_verify", "2026-09-10T14:22:18Z", 4, "cargo nextest run --workspace exits 0", "Regression check"),
    called("regression_verify", "2026-09-10T14:23:02Z", "call_nextest_1", "Bash", "cargo nextest run --workspace"),
    checked("regression_verify", "2026-09-10T14:24:11Z", nextestPassed()),
    said("regression_verify", "2026-09-10T14:24:20Z", "Every existing settings test still passes."),
    droneEnded("regression_verify", "2026-09-10T14:24:24Z", 11, 640_000),
  ];

  return {
    name: "escalated · evidence_suspect — every Check passed, and the panel refused two criteria",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedWatching(rows, true),
    journalled: journalledWatching([
      note("2026-09-10T14:11:02Z", "A worktree was cut for this job."),
      note(
        "2026-09-10T14:29:04Z",
        "Regression check's Judge refused criterion 02. Escalated: evidence_suspect.",
        { level: "warn", step: "regression_verify" },
      ),
    ]),
    resources: holdsRead(
      resources("running", {
        processes: [
          { pid: 41233, command: "node", cpu_percent: 0.1, memory_bytes: 402_653_184, running_for: "06:46", recorded: true },
        ],
        wrote_last_at: "2026-09-10T14:29:04.000Z",
      }),
    ),
    recorded: foldedReads({
      evidence: evidenceRead([
        {
          step_id: "regression_verify",
          evidence_type: "test_suite_run",
          claimed: "2031 of 2031 tests pass.",
          shown_by: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
        },
      ]),
      diff: diffRead(
        FILES,
        [
          "--- a/packages/settings/test/useColumnSelectors.test.ts",
          "+++ b/packages/settings/test/useColumnSelectors.test.ts",
          "@@ -38,9 +38,6 @@",
          "-  expect(selectVisibleColumns(state)).toBe(selectVisibleColumns(state))",
          "-})",
          "-",
        ].join("\n"),
      ),
    }),
    calls: {
      call_nextest_1: {
        ok: true,
        call: { tool: "Bash", call: "call_nextest_1", arguments: "cargo nextest run --workspace", whole: true },
      },
    },
    checkOutputs: {
      "regression_verify.1.cargo_nextest.log": {
        ok: true,
        output: {
          attempt: 1,
          name: "cargo_nextest",
          path: ".armada/checks/77-split-the-settings-reducer/regression_verify.1.cargo_nextest.log",
          lines: ["Summary [ 38.410s] 2031 tests run: 2031 passed, 0 skipped"],
          from_line: 1,
          total_lines: 1,
          bytes: 54,
          whole: true,
        },
      },
    },
    frames: {},
    now: NOW,
  };
}
