// What job detail draws while this Job's own read is out.
//
// **Most of the screen is already in memory.** The run's step names, which step
// is open and the phases every step is read against come off the Board's row and
// the holds loaded for every Job, so they are drawn rather than stood in for. What
// only `GET /jobs/:job_id` carries — where each step stands, the brief, the open
// step's fields and gates — is what waits behind a placeholder.
//
// Split out of `JobDetail.tsx`, which the 900-line refusal caught at 909.
// **Not `reading.ts`**, which is how a Job's *status* reads.

import type { RunTreeSkeletonProps } from "@armada/components";
import type { JobSummary, Watched, WorkflowSummary } from "@armada/protocol";
import { EVERY_STEP_PHASE } from "./timeline";
import type { StepReading } from "./InsideAJob";
import { stepsAhead } from "./run";
import { stillReading } from "./work";

/** The two regions that draw differently while the read is out. */
export type WhileReading = {
  run: RunTreeSkeletonProps;
  step: StepReading;
};

/**
 * What to draw while this Job's own read is out, and `undefined` once it has
 * answered either way — a Job that was read draws itself, and one Fleet refused
 * draws the sentences saying so.
 */
export function whileReading(
  watched: Watched,
  job: JobSummary,
  workflow: WorkflowSummary | undefined,
  selected: string | null,
): WhileReading | undefined {
  if (!stillReading(watched, job.id)) return undefined;
  const ahead = stepsAhead(workflow);
  const open = ahead.find((step) => step.id === (selected ?? job.current_step_id));
  return {
    run: { steps: ahead, current: open?.id },
    step: {
      label: open?.label,
      labelIsAnIdentifier: open?.labelIsAnIdentifier,
      phases: EVERY_STEP_PHASE,
    },
  };
}

/** Why nothing about this Job can be drawn, where Fleet would not answer for it. */
export function whyUnreachable(watched: Watched, jobId: string): string | undefined {
  return watched.state === "failed" && watched.jobId === jobId ? "Fleet did not answer" : undefined;
}
