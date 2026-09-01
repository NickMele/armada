// What the panel head says, and what it offers.
//
// Split out of `App.tsx` when that file grew past the gate's 500-line warning,
// the same way `Acts.tsx` and `copy.ts` came out of `JobDetail.tsx`. It is one
// subject — which of the views is up, what it is called, and the one control
// that leaves it — and the window's state machine beside it is another.
//
// **`Back to the list` and `Cancel` live here rather than in the body.** A
// control that leaves a view belongs beside the view's name, not scrolled into
// it.
//
// **The count sentence is not here any more.** It used to say how many Jobs
// there were and how many were at the gate, on every view this serves. Both its
// numbers move with the Board's filter now, so it went to the Board — beside
// the control that changes it, rather than in a head that also serves the
// composer, the reports and one Job read whole. The composer keeps a summary of
// its own, which is a standing sentence rather than a count.

import type { ReactNode } from "react";
import { Button, Kbd } from "@armada/components";

/** The four views one head serves, and everything each needs to draw it. */
export type HeadProps = {
  /** One Job's turns, as a screen of their own. Leaves to the Job. */
  /** One Job, read whole. Leaves to the list. */
  reading: boolean;
  /** The composer. Leaves to the list. */
  composing: boolean;
  /**
   * What has been reported against the Judge, and the counts over it. Leaves
   * to the list.
   *
   * **Its own view rather than a panel on a Job.** A report is filed about one
   * Job and the rate is read across all of them, so a listing reached through a
   * Job would lose exactly the reports that most need reading together.
   */
  auditing: boolean;
  /** A live connection. What stops a new Job being proposed into nothing. */
  live: boolean;
  /** A re-read in flight, so a second press does not send a second one. */
  refreshing: boolean;
  onCloseJob: () => void;
  onCloseComposer: () => void;
  onCompose: () => void;
  onCloseReports: () => void;
  onReadReports: () => void;
  onRefresh: () => void;
};

/** What the head is called, what it says beneath, and what sits at its edge. */
export type Head = { title: string; summary?: string; actions: ReactNode };

export function headOf({
  reading,
  composing,
  auditing,
  live,
  refreshing,
  onCloseJob,
  onCloseComposer,
  onCompose,
  onCloseReports,
  onReadReports,
  onRefresh,
}: HeadProps): Head {
  if (auditing) {
    return {
      title: "Reported in error",
      // No summary. The counts are the page's own first region and repeating
      // one of them here would be a second place to keep them right.
      actions: (
        <>
          <Button variant="ghost" size="sm" onClick={onCloseReports}>
            Back to the list
          </Button>
          <Kbd>Esc</Kbd>
        </>
      ),
    };
  }
  if (reading) {
    return {
      title: "Active jobs",
      actions: (
        <>
          <Button variant="ghost" size="sm" onClick={onCloseJob}>
            Back to the list
          </Button>
          <Kbd>Esc</Kbd>
        </>
      ),
    };
  }
  if (composing) {
    return {
      title: "New job",
      summary: "It lands at the approval gate. Nothing runs until you release it.",
      actions: (
        <Button variant="ghost" size="sm" onClick={onCloseComposer}>
          Cancel
        </Button>
      ),
    };
  }
  return {
    title: "Active jobs",
    actions: (
      <>
        {/* Re-reads over the connection Bridge already holds. It does not
            reconnect: dropping a working socket does not fix one that is
            broken, and the runtime-file path already retries on its own. */}
        <Button variant="ghost" size="sm" onClick={onRefresh} disabled={refreshing}>
          {refreshing ? "Refreshing" : "Refresh"}
        </Button>
        {/* Ghost, beside Refresh and never beside the accent fill. Reading what
            the Judge got wrong is not a decision queued on anybody — it is read
            deliberately, which is why it is here and not on a row. */}
        <Button variant="ghost" size="sm" onClick={onReadReports}>
          Reported
        </Button>
        {/* The one accent fill on the surface. */}
        <Button variant="primary" onClick={onCompose} disabled={!live}>
          New job
        </Button>
      </>
    ),
  };
}
