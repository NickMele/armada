import type { ReactNode } from "react";
import { Alert } from "../../primitives/Alert/Alert";
import { DroneTurns, type DroneTurn } from "../../compositions/DroneTurns/DroneTurns";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import type { JobDetailHeading } from "../detail";

/**
 * Watching a drone work — one Job's turns, while the Drone keeps working.
 *
 * **Nothing on this screen offers to intervene, and that is the design.** Pilot
 * takes over: it ends the Drone, moves the Job to `piloted` and writes a
 * transition. Observing does none of those things and is reversible by closing
 * the view, so a control here would offer an act this surface cannot perform —
 * and would make the two look alike, which is the confusion
 * `docs/concepts/observe.md` exists to prevent. The header's `actions` slot is
 * for leaving the view, never for acting on the Drone.
 *
 * **Three things can be missing and each says which.** A Job that was never
 * dispatched has no rows and that is ordinary; a bounded backfill left older
 * rows out; a viewer that fell behind lost rows it will not get back. All three
 * are stated, because a transcript with a silent gap reads as a Drone that went
 * quiet — the one thing this record exists to tell apart.
 *
 * **`live` reaches the transcript as well as the caption.** A run of the Drone
 * thinking collapses to one line, and whether that line moves is the same fact
 * the caption states — a finished history must not show a working mark.
 */
export type WatchingADroneWorkProps = {
  heading: JobDetailHeading;
  /** The rows, in order. Called and Answered arrive joined. */
  turns: DroneTurn[];
  /** The sentence for a Job with no rows at all. */
  emptyNote: string;
  /** The label over the transcript. */
  turnsLabel?: ReactNode;
  /** What the screen says about itself: read-only, and nothing is being taken over. */
  readOnlyNote?: ReactNode;
  /** Whether a Drone was writing when the socket opened. */
  live?: boolean;
  /** What the header says beside the label in each case. */
  liveNote?: ReactNode;
  quietNote?: ReactNode;
  /** Older rows the bounded backfill left out. Stated, never silently dropped. */
  skipped?: number;
  /** Rows this viewer fell behind and lost. **Not the sink's losses.** */
  missed?: number;
  /**
   * Why nothing more is coming, where the socket has closed. The wire's own
   * spelling: `Silence` is an `ipc` enum with no `enum-verbs.toml` rows, so
   * there is no sanctioned verb for it and none is written here. Reported.
   */
  closedBecause?: string;
  /** Why the pane could not be opened at all. A failure, not an empty history. */
  failure?: ReactNode;
};

export function WatchingADroneWork({
  heading,
  turns,
  emptyNote,
  turnsLabel = "The drone's turns",
  readOnlyNote = "Watching only. The drone is not told, nothing about the job changes, and closing this view ends nothing.",
  live = false,
  liveNote = "A drone is writing now.",
  quietNote = "Nothing is writing. This is the whole history.",
  skipped = 0,
  missed = 0,
  closedBecause,
  failure,
}: WatchingADroneWorkProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} />

      <div className="armada-screen__col">
        <div className="armada-screen__head-row">
          <span className="armada-screen__eyebrow">{turnsLabel}</span>
          <span className="armada-screen__caption">{live ? liveNote : quietNote}</span>
        </div>
        <p className="armada-screen__caption" data-note>
          {readOnlyNote}
        </p>

        {/* A slow viewer is told what it lost, in rows, rather than being shown
            a history with a hole in it. */}
        {missed > 0 ? (
          <Alert tone="escalated" title="Rows were dropped before this window saw them">
            {`${missed} turns will never arrive. What follows is everything else, in order.`}
          </Alert>
        ) : null}

        {/* The backfill is bounded, and a viewer that is not told reads a
            truncated history as the whole one. */}
        {skipped > 0 ? (
          <Alert tone="neutral" title="Older turns are not shown">
            {`${skipped} earlier turns are on disk and were left out of this history.`}
          </Alert>
        ) : null}

        {failure === undefined ? (
          <DroneTurns turns={turns} emptyNote={emptyNote} live={live} />
        ) : (
          <Alert tone="escalated" title="This job's turns could not be read">
            {failure}
          </Alert>
        )}

        {closedBecause === undefined ? null : (
          <p className="armada-screen__caption" data-note>
            {"Nothing more is coming: "}
            <span className="armada-screen__mono">{closedBecause}</span>
          </p>
        )}
      </div>
    </div>
  );
}
