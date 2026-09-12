import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tabs } from "../../primitives/Tabs/Tabs";

/**
 * A step's activity log, on the layer that can hold it — Journey 4, frames
 * `4i`, `4l` and `4m`. A sheet because the reading has no end: 1676 entries
 * on a real Job would push every chapter below it off screen if opened in
 * place, so the log leaves the panel and the panel stays as it was.
 *
 * Holds position rather than following the tail — a stream that scrolls
 * itself cannot be read, one that silently stops cannot be trusted. *Jump to
 * now* carries the count of what arrived while reading, repeated under the
 * last entry since the strip sits at the top and the reader at the bottom.
 */

/**
 * The stream itself is a slot: `ActivityLog` and `LogEntry` both exist and
 * Bridge draws the second, so this sheet takes rows as children rather than
 * importing either. Which a step's story should use is a question for those
 * two components. Reported.
 */

/**
 * The notice carries no glyph — the drawing draws `triangle-alert`, which
 * the icon registry reserves to Doctor (a reservation withdrawn once and
 * reinstated), so the contract wins and the band states itself by hue and
 * surface instead, as the panel's own notice band already does. Reported.
 *
 * An escalation states itself here rather than growing the act: Pilot keeps
 * the accent in the Job header, one primary per view, and this view is the
 * log.
 */

/**
 * The notice's *Show me* carries no key caption, though it once did — it
 * falls back to closing the sheet, same as `Esc`, and three controls
 * captioned `Esc` on one screen (this one, *Close*, *Back to the list*) is a
 * promise a person cannot tell apart. A key is captioned once, on the
 * control whose job it is.
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
