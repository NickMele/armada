import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tabs } from "../../primitives/Tabs/Tabs";

/**
 * A step's activity log, on the layer that can hold it — Journey 4, frames
 * `4i`, `4l` and `4m`.
 *
 * **It is a sheet because the reading has no end.** 1676 entries on a real Job
 * is not a longer version of the five the panel previews: opened in place it
 * pushes every chapter under it off the screen, and the chapter line a reader
 * came back to goes with them. So the log leaves the panel and the panel stays
 * exactly as it was, which is the way back.
 *
 * **The log holds position and does not follow the tail.** A stream that
 * scrolls itself cannot be read, and a stream that silently stops arriving
 * cannot be trusted — so it does both: the reading is held where you left it,
 * the strip says so, and *Jump to now* carries the count of what arrived while
 * you were reading. The same count is repeated under the last entry, because
 * the strip is at the top and the reader is at the bottom.
 *
 * **The stream itself is a slot.** Two log renderings exist in this package —
 * `ActivityLog` and `LogEntry` — and Bridge draws the second. A sheet that
 * imported one of them would be the sheet for one of the two surfaces, so it
 * takes the rows as children and the caller brings whichever log it already
 * draws in the panel. Which one a step's story should use is a question for
 * those two components, not for this layer. Reported.
 *
 * **The notice carries no glyph.** The drawing draws `triangle-alert` on it and
 * the icon registry reserves that glyph to Doctor — the reservation was
 * withdrawn once and reinstated — so the contract wins and the band says what
 * it is with the escalated hue and surface, as the panel's own notice band
 * already does. Reported.
 *
 * **An escalation states itself in a notice inside the sheet and does not grow
 * the act.** Pilot keeps the accent in the Job header, behind the layer — one
 * primary per view, and this view is the log. *Show me* closes the sheet,
 * which is what `Esc` already does, so it is a labelled second face of one act
 * rather than a second binding.
 */

/** One filter over the stream. The set is closed by who can write into a log. */
export type ActivityFilter = "all" | "drone" | "fleet" | "armada";

const FILTERS: { id: ActivityFilter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "drone", label: "Drone" },
  { id: "fleet", label: "Fleet" },
  { id: "armada", label: "Armada" },
];

/**
 * What the Job did while the sheet was open. **Stated in the sheet and nowhere
 * else on this layer** — the rail behind it carries the failed step and the
 * hued cross, and the sheet only says the Job moved.
 */
export type ActivityEscalation = {
  /** When it escalated, as the log recorded it. */
  at: string;
  /** Why, in one sentence. The Judge's own words, never a paraphrase. */
  because: ReactNode;
  /** Closes the sheet and lands focus on the failed step in the rail. */
  onShowMe?: () => void;
};

export type ActivityLogSheetProps = {
  open: boolean;
  /** The step this log belongs to. Restated here: the tree is under the layer. */
  step: ReactNode;
  /** The Job, in mono. Absent at the floor, where the width is not there. */
  jobId?: ReactNode;
  /** The stream, drawn by whichever log the caller's surface already uses. */
  children: ReactNode;
  /** How many the stream holds, which is not how many are drawn. */
  total: number;
  /** Whether rows are still arriving. The live mark, and the pulse with it. */
  live?: boolean;
  /** When it stopped, on a Job that is over. Drawn instead of the live mark. */
  endedAt?: ReactNode;
  filter?: ActivityFilter;
  onFilter?: (filter: ActivityFilter) => void;
  /**
   * When the reading was held. Absent means the log is at the tail and no strip
   * is drawn — there is nothing to jump back to.
   */
  heldAt?: string;
  /** How many arrived while the reader was reading. */
  arrived?: number;
  onJumpToNow?: () => void;
  /** What the Job did while the sheet was open, where it did something. */
  escalation?: ActivityEscalation;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function ActivityLogSheet({
  open,
  step,
  jobId,
  children,
  total,
  live = false,
  endedAt,
  filter = "all",
  onFilter,
  heldAt,
  arrived = 0,
  onJumpToNow,
  escalation,
  floor = false,
  onClose,
}: ActivityLogSheetProps) {
  const tabs = (
    <Tabs
      items={FILTERS.map((held) => ({ id: held.id, label: held.label }))}
      value={filter}
      onChange={(id) => onFilter?.(id as ActivityFilter)}
    />
  );

  // The tail control. `Jump to now` above the floor; `Now` at it, where the
  // strip is also carrying the four filters and the sentence has gone.
  const toTheTail =
    heldAt === undefined ? null : (
      <Button variant="secondary" size="sm" ground="sunken" onClick={onJumpToNow}>
        {floor ? "Now" : "Jump to now"}
        <span className="armada-log-sheet__count">{`+${arrived}`}</span>
      </Button>
    );

  return (
    <Sheet
      open={open}
      contained
      size="wide"
      floor={floor}
      title="Activity log"
      subtitle={
        <>
          {step}
          {jobId === undefined || floor ? null : (
            <>
              {" · "}
              <span className="armada-log-sheet__mono">{jobId}</span>
            </>
          )}
          {" · "}
          <span className="armada-log-sheet__mono">{total}</span>
          {` ${total === 1 ? "entry" : "entries"} · `}
          {live ? (
            <>
              <span className="armada-log-sheet__live" aria-hidden />
              live
            </>
          ) : (
            <>
              {"ended "}
              <span className="armada-log-sheet__mono">{endedAt}</span>
            </>
          )}
        </>
      }
      // At the floor the filters drop into the strip: the header is carrying a
      // title, a subtitle and a close in 768px, and four tabs beside them is
      // the line that breaks.
      controls={floor ? undefined : tabs}
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      bands={
        <>
          {escalation === undefined ? null : (
            <div className="armada-log-sheet__notice" role="status">
              <span className="armada-log-sheet__said">
                <strong>{`The Job escalated at ${escalation.at}.`}</strong>{" "}
                {escalation.because}
              </span>
              <Button
                variant="secondary"
                size="sm"
                ground="sunken"
                onClick={escalation.onShowMe ?? onClose}
              >
                Show me
                <Kbd>Esc</Kbd>
              </Button>
            </div>
          )}
          {heldAt === undefined && !floor ? null : (
            <div className="armada-log-sheet__strip">
              {floor ? (
                tabs
              ) : (
                <span className="armada-log-sheet__held">
                  {"Held at "}
                  <span className="armada-log-sheet__mono">{heldAt}</span>
                  {" — the tail is not followed while you are reading"}
                </span>
              )}
              {toTheTail}
            </div>
          )}
        </>
      }
      onClose={onClose}
    >
      <div className="armada-log-sheet__body">
        {children}
        {arrived === 0 ? null : (
          <p className="armada-log-sheet__arrived" role="status">
            {`${arrived} ${arrived === 1 ? "entry" : "entries"} arrived while you were reading`}
          </p>
        )}
      </div>
    </Sheet>
  );
}
