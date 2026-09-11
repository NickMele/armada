import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { JobResources, type JobResourcesProps } from "../JobResources/JobResources";

/**
 * The full reading of what a Job holds, on the layer that can hold it.
 *
 * **A new home rather than an edit.** `JobResources` is a verdict, a look, a
 * process table, a disk figure and four absence states, and every one of them
 * is unchanged here — what moved is where it is drawn. Above the run it was the
 * largest thing in the column and it answered a question nobody had asked yet;
 * `JobHoldsSummary` answers it in five lines, and this is what that opens.
 *
 * **The act comes with it.** `Look now` is an act on the reading, so it belongs
 * beside the reading — the summary carries no second copy of it.
 *
 * **Not `wide`.** The log and the patch take a fraction of the ground because a
 * wrapped patch line is a decision taken on the wrong text. This is a card that
 * was legible in a 380px column, and a sheet three times that wide would spread
 * four figures across it. `Sheets.tsx` holds the one-sheet-at-a-time rule and
 * the two exits, unchanged.
 */
export type JobHoldsSheetProps = JobResourcesProps & {
  open: boolean;
  /** The Job, in mono. Absent at the floor, where the width is not there. */
  jobId?: ReactNode;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function JobHoldsSheet({
  open,
  jobId,
  floor = false,
  onClose,
  ...reading
}: JobHoldsSheetProps) {
  return (
    <Sheet
      open={open}
      contained
      floor={floor}
      title="Pulse"
      subtitle={
        jobId === undefined || floor ? undefined : (
          <span className="armada-holds-sheet__mono">{jobId}</span>
        )
      }
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      onClose={onClose}
    >
      <div className="armada-holds-sheet__body">
        <JobResources {...reading} />
      </div>
    </Sheet>
  );
}
