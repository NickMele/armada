// `running` — before the first Drone turn: the worktree is cut and the
// repository's own preparation commands are still running.
//
// **`crates/fleet/src/dispatch.rs` fixes the order this freezes** — a Job
// moves to `running` before `create_worktree`, and `prepared()` runs setup
// commands before `branded()` sets `branch` or a Drone is assigned. So a
// Job frozen here carries `running`, no branch, no Drone, every step
// `not_started`. `preparing.rs`'s log lines are what `journalled` carries,
// word for word: "the worktree is being prepared before any Drone is put
// on it" and "a preparation command is starting".
//
// **The one state where `running` and `not started` are both true, neither
// wrong** — badge reads `running` from status; run reads every step
// `not_started` since none entered — the same gap a wedged Job looks like,
// told apart by the machine panel's two lines, not a status this Job carries.

import type { JobFixture } from "../fixture";
import {
  BUILD_CHECK,
  detail,
  foldedReads,
  freshStep,
  holdsRead,
  job,
  JOB_ID,
  journalledWatching,
  landStep,
  manifest,
  NEXTEST_CHECK,
  NO_OBSERVED,
  note,
  NOW,
  watchedRead,
  workflow,
} from "./base";

export function preparing(): JobFixture {
  const theJob = job("running", {
    current_step_id: "repro",
    branch: undefined,
    assigned_drone: undefined,
  });
  const steps = [
    freshStep("repro", "Reproduction", 1),
    freshStep("root_cause", "Root cause", 2),
    freshStep("fix", "Fix", 3, [BUILD_CHECK]),
    freshStep("regression_verify", "Regression check", 4, [NEXTEST_CHECK]),
    freshStep("consumers", "Check the consumers still compile", 5, [BUILD_CHECK]),
    landStep(),
  ];
  const whole = detail(theJob, steps, { branch: undefined });
  return {
    name: "running — before the first Drone turn, the worktree is being prepared",
    job: theJob,
    watched: watchedRead(whole),
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: journalledWatching([
      note("2026-09-10T14:10:58Z", "the worktree is being prepared before any Drone is put on it"),
      note("2026-09-10T14:11:01Z", "a preparation command is starting"),
    ]),
    resources: holdsRead({ job_id: JOB_ID, read_at: "2026-09-10T14:11:03.000Z", held: "none", processes: [] }),
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
