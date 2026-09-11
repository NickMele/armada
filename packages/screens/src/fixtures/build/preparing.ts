// `running` — before the first Drone turn: the worktree is cut and the
// repository's own preparation commands are still running.
//
// **`crates/fleet/src/dispatch.rs` fixes the order this freezes.** A Job moves
// to `running` before `create_worktree` is even called, and `prepared()` runs
// the repository's setup commands in that worktree before `branded()` sets the
// Job's own `branch` field or `put_a_drone_on` assigns a Drone — so a Job
// frozen here carries `running`, an unset branch, no assigned Drone, and every
// step still `not_started`. `preparing.rs`'s own log lines are what
// `journalled` carries, word for word: "the worktree is being prepared before
// any Drone is put on it" and "a preparation command is starting".
//
// **The one state where `running` and `not started` are both true and neither
// is wrong.** The badge reads `running` because the Job's status is; the run
// reads every step `not_started` because none has been entered — and that gap
// is what a wedged Job also looks like, which is why the machine panel's own
// two lines are the thing that tells the two apart, not a status this Job
// does not carry.

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
