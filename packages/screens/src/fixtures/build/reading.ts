// A Job the Board already knows, whose own detail has been asked for and has
// not come back yet.
//
// **`watched.state === "reading"` is the whole of it.** The Board's list read
// the `JobSummary`, and the workflow and Manifest holds are loaded for every
// Job, so the header, the run's step names, the open step's name and every row
// of where things are draw at once. What waits is what only `GET /jobs/:job_id`
// answers: where each step stands, the brief, and the open step's own reading.

import type { JobFixture } from "../fixture";
import { foldedReads, job, JOB_ID, manifest, NO_JOURNALLED, NO_OBSERVED, NOW, workflow } from "./base";

export function reading(): JobFixture {
  return {
    name: "running — this Job's own detail was asked for and has not come back",
    job: job("running", { current_step_id: "fix" }),
    watched: { state: "reading", jobId: JOB_ID },
    workflows: [workflow()],
    manifests: [manifest()],
    observed: NO_OBSERVED,
    journalled: NO_JOURNALLED,
    resources: { state: "reading", jobId: JOB_ID },
    recorded: foldedReads(),
    calls: {},
    checkOutputs: {},
    frames: {},
    now: NOW,
  };
}
