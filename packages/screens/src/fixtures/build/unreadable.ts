// A Job the Board already knows, whose own detail Fleet would not answer for.
//
// **`watched.state === "failed"` is the whole of it.** The Board's own list
// read the Job's `JobSummary` — its status, its branch, its assigned Drone —
// so the badge and the header draw as they would for any running Job. It is
// `GET /jobs/:job_id` that came back a refusal: no steps, no brief, no work,
// no machine reading. `whyNoSteps`, `whyNoBrief` and `workOf` in
// `JobDetail.tsx` are what turn that one failed read into every "cannot be
// read" sentence on the panel, so nothing here writes those sentences itself.

import type { JobFixture } from "../fixture";
import type { Outcome } from "@armada/protocol";
import { foldedReads, job, JOB_ID, manifest, NO_JOURNALLED, NO_OBSERVED, NOW, workflow } from "./base";

/** Fleet did not answer this Job's own detail route inside the wait. */
const FLEET_DID_NOT_ANSWER: Outcome = {
  ok: false,
  why: "transport",
  detail: "The operation was aborted due to timeout",
  fault: { method: "GET", path: `/jobs/${JOB_ID}`, why: "unreachable" },
};

export function unreadable(): JobFixture {
  const theJob = job("running", { current_step_id: "fix" });
  return {
    name: "running — Fleet would not answer for this Job's own detail",
    job: theJob,
    watched: { state: "failed", jobId: JOB_ID, outcome: FLEET_DID_NOT_ANSWER },
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: { state: "failed", jobId: JOB_ID, outcome: FLEET_DID_NOT_ANSWER },
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
