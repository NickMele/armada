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

import type { ReactNode } from "react";
import { Button, Kbd } from "@armada/components";

/** The three views one head serves, and everything each needs to draw it. */
export type HeadProps = {
  /** One Job's turns, as a screen of their own. Leaves to the Job. */
  watching: boolean;
  /** One Job, read whole. Leaves to the list. */
  reading: boolean;
  /** The composer. Leaves to the list. */
  composing: boolean;
  /** How many Jobs, and how many at the gate. The list's own sentence. */
  summary: string;
  /** A live connection. What stops a new Job being proposed into nothing. */
  live: boolean;
  /** A re-read in flight, so a second press does not send a second one. */
  refreshing: boolean;
  onCloseTurns: () => void;
  onCloseJob: () => void;
  onCloseComposer: () => void;
  onCompose: () => void;
  onRefresh: () => void;
};

/** What the head is called, what it says beneath, and what sits at its edge. */
export type Head = { title: string; summary?: string; actions: ReactNode };

export function headOf({
  watching,
  reading,
  composing,
  summary,
  live,
  refreshing,
  onCloseTurns,
  onCloseJob,
  onCloseComposer,
  onCompose,
  onRefresh,
}: HeadProps): Head {
  if (watching) {
    return {
      title: "Active jobs",
      summary,
      actions: (
        <>
          {/* Leaves the pane. **Never an act on the Drone** — closing this ends
              the watching and nothing else, which is the whole difference
              between observing and taking over. */}
          <Button variant="ghost" size="sm" onClick={onCloseTurns}>
            Back to the job
          </Button>
          <Kbd>Esc</Kbd>
        </>
      ),
    };
  }
  if (reading) {
    return {
      title: "Active jobs",
      summary,
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
    summary,
    actions: (
      <>
        {/* Re-reads over the connection Bridge already holds. It does not
            reconnect: dropping a working socket does not fix one that is
            broken, and the runtime-file path already retries on its own. */}
        <Button variant="ghost" size="sm" onClick={onRefresh} disabled={refreshing}>
          {refreshing ? "Refreshing" : "Refresh"}
        </Button>
        {/* The one accent fill on the surface. */}
        <Button variant="primary" onClick={onCompose} disabled={!live}>
          New job
        </Button>
      </>
    ),
  };
}
