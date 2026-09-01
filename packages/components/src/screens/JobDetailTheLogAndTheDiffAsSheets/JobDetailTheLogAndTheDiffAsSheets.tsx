import type { ReactNode } from "react";
import { InsideAJob } from "../InsideAJobOneArrangementAtEveryState/InsideAJobOneArrangementAtEveryState";
import type { InsideAJobProps } from "../InsideAJobOneArrangementAtEveryState/InsideAJobOneArrangementAtEveryState";

/**
 * Job detail with a trailing sheet open — Journey 4, frames `4i`–`4m`.
 *
 * **The screen behind the layer does not move.** Opening the log or the diff
 * navigates nowhere: the Job header, the run tree and the panel stay exactly
 * where they were, on screen and under the sheet, and closing gives them back
 * with the chapter line still holding focus. That is the whole reason the sheet
 * beat a route of its own, which would have made `Esc` mean two depths of
 * *back* on one screen — and it is why this is the same `InsideAJob` the other
 * six stories draw, with one slot filled, rather than a second arrangement.
 *
 * **What it costs is stated rather than hidden.** The run tree goes behind the
 * layer, so which step you are reading is restated in the sheet header, and
 * moving to another step's log is close, select, open rather than one press.
 *
 * **The pulse moves with the reading.** The tree's current step is behind the
 * layer, so its mark stops and the sheet's live mark takes it — one animated
 * mark per screen, and it belongs on the thing being read.
 */
export type JobDetailWithASheetProps = Omit<InsideAJobProps, "pulsing"> & {
  /** The sheet itself. `ActivityLogSheet` or `JobDiffSheet`. */
  sheet: ReactNode;
};

export function JobDetailWithASheet(props: JobDetailWithASheetProps) {
  return <InsideAJob {...props} pulsing={false} />;
}
