// Pulse — what this Job is costing the machine right now.
//
// Drones, Checks and Judges running; spend and turns against their caps; the
// processes, the checkouts and the logs. `#1538`, on `draft/pulse.ts`'s
// `PulseView` — which is derived from today's wire, so this board renders
// against the real Fleet as well as a mock.

import { JobResources, type JobResourcesProps } from "@armada/components";

import { TAB_LABEL } from "./detail-tabs";

export type PulseTabProps = {
  /** The board, and the act that goes and looks. */
  holds: JobResourcesProps;
};

export function PulseTab({ holds }: PulseTabProps) {
  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.pulse}>
      <JobResources {...holds} />
    </div>
  );
}
