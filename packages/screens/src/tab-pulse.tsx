// Pulse — what this Job is costing the machine right now.
//
// Drones, Checks and Judges running; spend and turns against their caps; the
// processes, the checkouts and the logs. `#1538`, on `draft/pulse.ts`'s
// `PulseView` — which is derived from today's wire, so this board renders
// against the real Fleet as well as a mock.

import { useEffect } from "react";

import { JobResources, type JobResourcesProps } from "@armada/components";

import { TAB_LABEL } from "./detail-tabs";

export type PulseTabProps = {
  /** The board, and the act that goes and looks. */
  holds: JobResourcesProps;
  /** The Job the board is reading, for the poll below. */
  jobId: string;
  /** Hold the reading live while this tab is open. `usePulseWatch`. */
  onNeedPulse: (jobId: string | null) => void;
};

export function PulseTab({ holds, jobId, onNeedPulse }: PulseTabProps) {
  usePulseWatch(jobId, onNeedPulse);
  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.pulse}>
      <JobResources {...holds} />
    </div>
  );
}

/**
 * Ask main to keep this Job's reading live, and stop when the board goes.
 *
 * **The board's lifetime and not the Job's**, which is what bounds the cost:
 * the tick walks a process table, and a Job is open behind three other tabs
 * that never draw it. `null` on unmount is the whole of the stopping — main
 * holds one Job at a time, so leaving for another board replaces it rather
 * than stacking a second timer. `resources-poll.ts`, `#1571`.
 */
export function usePulseWatch(jobId: string | null, onNeedPulse: (jobId: string | null) => void): void {
  useEffect(() => {
    onNeedPulse(jobId);
    return () => onNeedPulse(null);
  }, [jobId, onNeedPulse]);
}
