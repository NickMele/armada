// Pulse — what this Job is costing the machine right now.
//
// **Today's resources region, given a destination.** `JobResources` is a
// verdict, a look, a process table, a disk figure and four absence states, and
// none of them changes here: what moved is that it has a tab of its own rather
// than being a sheet opened from a five-line summary. Overview keeps that
// summary, which is the reading a person glances at without leaving the run.
//
// `#1538` replaces this with the full board — stat tiles, processes with their
// owner, worktrees and the Job's own logs.

import { JobResources, type JobResourcesProps } from "@armada/components";

import { TAB_LABEL } from "./detail-tabs";

export type PulseTabProps = {
  /** The reading and its act, exactly as the Pulse sheet already takes them. */
  holds: JobResourcesProps;
};

export function PulseTab({ holds }: PulseTabProps) {
  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.pulse}>
      <JobResources {...holds} />
    </div>
  );
}
