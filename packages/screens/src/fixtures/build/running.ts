// `running` — mid-step on Fix, a Check not yet run.
//
// **Full depth.** The activity log carries every step's turns rather than only
// the open one's, because `Observed` is a whole-Job socket and a fixture that
// narrowed it would hide the join `entriesOf` makes on `step_id`. The footprint
// and the diff agree on the same three files for the reason `chapters.tsx`'s
// own comment gives: they are one drone's work, read twice.

import type { JobFixture } from "../fixture";
import {
  answered,
  BUILD_CHECK,
  called,
  consumersStep,
  detail,
  diffRead,
  DRONE_ID,
  droneEnded,
  evidenceRead,
  foldedReads,
  freshStep,
  holdsRead,
  instructed,
  job,
  JOB_ID,
  journalledWatching,
  landStep,
  manifest,
  note,
  NOW,
  observedWatching,
  producedTurn,
  reproStep,
  resources,
  rootCauseStep,
  said,
  watchedRead,
  workflow,
} from "./base";

const FIX_ENTERED = "2026-09-10T14:16:07Z";

function fixStep() {
  const step = freshStep("fix", "Fix", 3, [BUILD_CHECK]);
  return {
    ...step,
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: FIX_ENTERED }],
    entered_at: FIX_ENTERED,
    updated_at: "2026-09-10T14:24:00Z",
  };
}

/** The workflow's six steps, midway through Fix. */
function runningSteps() {
  return [reproStep(), rootCauseStep(), fixStep(), freshStep("regression_verify", "Regression check", 4), consumersStep(), landStep()];
}

/** The call the Drone below is waiting on, which is what an answer names. */
export const WAITING_CALL = "call_pnpm_add_1";

/**
 * The same Job at Ask me first, its Drone stopped inside a call to a command it was
 * not given. **Still `running`**, as it is while a question is out, and the row
 * carries `asking` — the flag that lifts it into Needs you on the Board.
 */
export function runningWaitingOnACommand(): JobFixture {
  const was = running();
  const theJob = job("running", { current_step_id: "fix", asking: true });
  const whole = detail(theJob, runningSteps(), {
    when_blocked: "ask_me",
    command_waiting: {
      call: WAITING_CALL,
      step_id: "fix",
      asked_at: "2026-09-10T14:29:10Z",
      tool: "Bash",
      detail: "pnpm add -D reselect@5.1.1",
      truncated: false,
      length: 26,
      offers: ["allow_for_job", "always_allow", "reject"],
    },
  });
  return {
    ...was,
    name: "running — the drone is waiting for a person to allow a command",
    job: theJob,
    watched: watchedRead(whole),
  };
}

export function running(): JobFixture {
  const theJob = job("running", { current_step_id: "fix" });
  const steps = runningSteps();
  const whole = detail(theJob, steps);

  const rows = [
    instructed("repro", "2026-09-10T14:11:15Z", 1, "the reproduction fails on main", "Reproduction"),
    producedTurn("repro", "2026-09-10T14:12:20Z", [
      { path: "packages/settings/test/useColumnSelectors.test.ts", change: "added" },
    ]),
    droneEnded("repro", "2026-09-10T14:12:27Z", 4, 120_000),
    instructed("root_cause", "2026-09-10T14:12:27Z", 2, "the root cause is written down", "Root cause"),
    said(
      "root_cause",
      "2026-09-10T14:13:00Z",
      "Reading the reducer to find where the memoised selector is defined.",
    ),
    droneEnded("root_cause", "2026-09-10T14:16:07Z", 9, 300_000),
    instructed(
      "fix",
      FIX_ENTERED,
      3,
      "the selectors module has no import of the store",
      "Fix",
    ),
    said(
      "fix",
      "2026-09-10T14:16:44Z",
      "Splitting the selector block into its own module so the tests can import it without " +
        "constructing the store.",
    ),
    called("fix", "2026-09-10T14:17:20Z", "call_edit_1", "Edit", "packages/settings/src/selectors.ts"),
    answered("fix", "2026-09-10T14:17:21Z", "call_edit_1"),
    called("fix", "2026-09-10T14:20:05Z", "call_edit_2", "Edit", "packages/settings/src/reducer.ts"),
    answered("fix", "2026-09-10T14:20:06Z", "call_edit_2"),
    said("fix", "2026-09-10T14:24:00Z", "thinking"),
  ];

  const files = [
    { path: "packages/settings/src/selectors.ts", change: "modified" },
    { path: "packages/settings/src/reducer.ts", change: "modified" },
    { path: "packages/settings/src/index.ts", change: "added" },
  ];

  return {
    name: "running — mid-step on Fix, a Check not yet run",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: observedWatching(rows, true),
    journalled: journalledWatching([
      note("2026-09-10T14:11:02Z", "A worktree was cut for this job."),
      note("2026-09-10T14:11:10Z", "The repository's preparation commands succeeded."),
    ]),
    resources: holdsRead(
      resources("running", {
        processes: [
          {
            pid: 41233,
            command: "node",
            cpu_percent: 8.2,
            memory_bytes: 402_653_184,
            running_for: "07:53",
            recorded: true,
          },
        ],
        wrote_last_at: "2026-09-10T14:24:00.000Z",
      }),
    ),
    recorded: foldedReads({
      footprint: {
        state: "read",
        jobId: JOB_ID,
        reading: {
          job_id: JOB_ID,
          step_id: "fix",
          drone_id: DRONE_ID,
          plan_declared: false,
          files,
          actor: "drone",
          at: "2026-09-10T14:24:00Z",
        },
      },
      evidence: evidenceRead([
        {
          step_id: "repro",
          evidence_type: "failing_test",
          claimed: "useColumnSelectors re-runs the whole selector graph on any settings write.",
          shown_by: "packages/settings/test/useColumnSelectors.test.ts",
        },
        {
          step_id: "root_cause",
          evidence_type: "document",
          claimed: "The columns selector is memoised against the whole settings slice.",
          shown_by: ".armada/deliverables/77-split-the-settings-reducer/root_cause.1.plan.md",
        },
      ]),
      diff: diffRead(
        files,
        [
          "--- a/packages/settings/src/selectors.ts",
          "+++ b/packages/settings/src/selectors.ts",
          "@@ -14,6 +14,9 @@",
          "+import { selectColumnOrder } from './selectors/columns'",
        ].join("\n"),
      ),
    }),
    calls: {
      call_edit_1: {
        ok: true,
        call: {
          tool: "Edit",
          call: "call_edit_1",
          arguments: "packages/settings/src/selectors.ts: extract selectColumnOrder",
          whole: true,
        },
      },
    },
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
