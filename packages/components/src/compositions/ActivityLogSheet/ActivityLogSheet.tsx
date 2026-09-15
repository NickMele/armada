import { useEffect, useLayoutEffect, useRef, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
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
 * **The log follows the tail at the bottom, and holds the moment you scroll
 * away from it.** #1155. Watching a live drone wants each entry to arrive the
 * way a chat's does; someone who has scrolled up into 1676 entries looking for
 * one of them is the reader a followed tail would pull out from under —
 * `onFollowingChange` is how this sheet tells its caller which of the two just
 * became true, off the scroll itself rather than off a row landing, so a row
 * arriving mid-scroll cannot flip the read. Held, the strip says so and *Jump
 * to now* carries the count of what arrived; the same count is repeated under
 * the last entry, because the strip is at the top and the reader is at the
 * bottom. Scrolling back down, or pressing *Jump to now*, resumes following.
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
 * primary per view, and this view is the log.
 *
 * **The notice's control carries no key**, though it did. *Show me* falls back
 * to closing the sheet, which is what `Esc` does, and it was captioned `Esc` on
 * that reasoning — one act with two faces. On screen it reads as two controls
 * bound to one key, next to a *Close* captioned `Esc` and under a *Back to the
 * list* captioned `Esc`: three of them, and a person cannot tell which one the
 * key will reach. A caption is a promise about what a key does, so the key is
 * captioned once, on the control whose whole job it is.
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
 * How close to the bottom counts as *at* it. A couple of rows, not a pixel —
 * an exact match misses on a fractional scroll position a browser rounds
 * differently than the layout that produced it, which reads as "never quite
 * at the bottom" to a person who plainly is.
 */
const NEAR_THE_BOTTOM_PX = 48;

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
  /**
   * The message box, fixed under the stream rather than scrolling with it —
   * `Sheet`'s own `footer` slot. #1154. Absent draws the sheet as it was.
   */
  footer?: ReactNode;
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
   * is drawn — there is nothing to jump back to. **Also what says whether this
   * sheet is following**: present is held, absent is following, and the two
   * effects below key off exactly this rather than a second flag the caller
   * could disagree with it about.
   */
  heldAt?: string;
  /** How many arrived while the reader was reading. */
  arrived?: number;
  onJumpToNow?: () => void;
  /**
   * The reader crossed the near-bottom line, by an actual scroll — never by a
   * row landing while `heldAt` is already set, which would read as a press
   * nobody made. #1155. The caller owns what happens next: `true` is what
   * *Jump to now* already does, `false` is what scrolling up now also does.
   */
  onFollowingChange?: (following: boolean) => void;
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
  footer,
  total,
  live = false,
  endedAt,
  filter = "all",
  onFilter,
  heldAt,
  arrived = 0,
  onJumpToNow,
  onFollowingChange,
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

  const body = useRef<HTMLDivElement>(null);
  const following = heldAt === undefined;

  // **A real scroll, not a computed one.** Re-subscribed on every `following`
  // change so the comparison inside always reads the caller's current answer
  // rather than one closed over at mount — the mismatch is what says a person
  // crossed the line, in either direction.
  useEffect(() => {
    const el = body.current;
    if (el === null || onFollowingChange === undefined) return;
    function onScroll(): void {
      const distance = el!.scrollHeight - el!.scrollTop - el!.clientHeight;
      const atBottom = distance <= NEAR_THE_BOTTOM_PX;
      if (atBottom !== following) onFollowingChange!(atBottom);
    }
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [following, onFollowingChange]);

  // Each new entry scrolls into view — while following. Runs on mount too, so
  // opening the sheet on a live step starts at the newest entry rather than at
  // the top of a stream it has not read yet. Holding leaves the scroll exactly
  // where the reader put it; new rows still land, off the bottom of the view.
  useLayoutEffect(() => {
    if (!following) return;
    const el = body.current;
    if (el === null) return;
    el.scrollTop = el.scrollHeight;
  }, [following, total]);

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
      bodyRef={body}
      footer={footer}
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
