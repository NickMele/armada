import type { KeyboardEvent as ReactKeyboardEvent, FocusEvent, ReactNode } from "react";
import { Children, useCallback, useRef, useState } from "react";
import { JOB_ROW_LIST, RovingOption } from "../JobRowStacked/JobRowStacked";

/**
 * Active jobs list — the framed list of Job rows, and the header above it.
 *
 * **Ordering carries the trigger, not a control.** Rows that need a person sort
 * above rows that do not, and within each group the oldest first, because the
 * thing waiting longest is the thing most likely to have gone wrong. The list
 * renders the order it is handed: sorting is Fleet's, and a component that
 * re-sorted would be a second definition of the rule.
 *
 * The header states the count and how many need you. That sentence is written
 * where the counts are known; this composition places it and never composes it.
 *
 * **The empty state is `Board empty state`**, and it is not built here. The
 * `empty` slot is where it mounts — Fleet running with no jobs, and Fleet not
 * running, read differently and the difference is that component's to carry.
 *
 * **A listbox roves; it does not hand out one tab stop per row.** Tab reaches
 * the list once, Up and Down move within it, and Home and End jump to its ends.
 * Fourteen tab stops to cross a list is what a `listitem` with an `onClick`
 * produces, and it is why the role was wrong before the keys were.
 */
export type ActiveJobsListProps = {
  /** The surface's name. Lowercase anything countable: "Active jobs". */
  heading?: ReactNode;
  /** The count sentence: "6 jobs. 1 awaiting approval." */
  summary?: ReactNode;
  /**
   * The surface's one primary action, beside the heading and outside the
   * frame. A list row never takes one; the surface may.
   */
  action?: ReactNode;
  /** `Job row (stacked)` rows, in the order Fleet supplied. */
  children?: ReactNode;
  /** Where `Board empty state` mounts when there are no rows. */
  empty?: ReactNode;
  /**
   * True where every row opens something. The frame becomes a listbox and its
   * rows options, which is what makes "which one is open" a thing a screen
   * reader can say — a list of listitems has no such state.
   */
  selectable?: boolean;
  /** The listbox's name, where it is one. */
  label?: string;
};

/** Which key moves the cursor where. Nothing else in the list is bound. */
const ROVES = new Set(["ArrowDown", "ArrowUp", "Home", "End"]);

export function ActiveJobsList({
  heading,
  summary,
  action,
  children,
  empty,
  selectable = false,
  label,
}: ActiveJobsListProps) {
  const rows = Array.isArray(children) ? children.filter(Boolean) : children;
  const isEmpty = rows === undefined || rows === null || (Array.isArray(rows) && rows.length === 0);
  const frame = useRef<HTMLDivElement>(null);
  // Where the one tab stop is. Zero is the first row, which is where a list
  // that has never been touched should put it.
  const [active, setActive] = useState(0);

  /** The options as the DOM has them, which is the only place their order is. */
  const options = useCallback(
    (): HTMLElement[] =>
      Array.from(frame.current?.querySelectorAll<HTMLElement>(':scope > [role="option"]') ?? []),
    [],
  );

  const rove = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      if (!ROVES.has(event.key)) return;
      const found = options();
      if (found.length === 0) return;
      // Clamped rather than wrapped: a list that jumps from the last row to the
      // first loses the reader's place, and a Board is scanned rather than
      // cycled.
      const from = Math.min(active, found.length - 1);
      const to =
        event.key === "Home"
          ? 0
          : event.key === "End"
            ? found.length - 1
            : Math.min(Math.max(from + (event.key === "ArrowDown" ? 1 : -1), 0), found.length - 1);
      // Stopped whatever happens, so the arrows never also scroll the pane the
      // row just moved inside.
      event.preventDefault();
      setActive(to);
      found[to]?.focus();
    },
    [active, options],
  );

  // A row reached by mouse or by Tab becomes the cursor, so the next arrow
  // press continues from where the eye is rather than from where it was.
  const followed = useCallback(
    (event: FocusEvent<HTMLDivElement>) => {
      const option = (event.target as HTMLElement).closest<HTMLElement>('[role="option"]');
      if (option === null) return;
      const at = options().indexOf(option);
      if (at >= 0) setActive(at);
    },
    [options],
  );

  const roving = selectable && !isEmpty;

  return (
    <section className="armada-active-jobs">
      {heading || summary || action ? (
        <header className="armada-active-jobs__header">
          <div className="armada-active-jobs__titles">
            {heading ? <h2 className="armada-active-jobs__heading">{heading}</h2> : null}
            {summary ? <p className="armada-active-jobs__summary">{summary}</p> : null}
          </div>
          {action ? <div className="armada-active-jobs__action">{action}</div> : null}
        </header>
      ) : null}
      {/* The frame carries the row tracks, so a field column sizes to the
          widest value in the whole list rather than in one row. An empty
          listbox has no options to select from, so it stays a plain list. */}
      <div
        ref={frame}
        className={`armada-active-jobs__frame ${JOB_ROW_LIST}`}
        role={roving ? "listbox" : "list"}
        aria-label={label}
        onKeyDown={roving ? rove : undefined}
        onFocus={roving ? followed : undefined}
      >
        {isEmpty
          ? empty
          : roving
            ? // A provider renders no element, so the options stay direct
              // children of the listbox and `:scope >` still finds them.
              Children.map(rows, (row, index) => (
                <RovingOption.Provider value={{ index, active }}>{row}</RovingOption.Provider>
              ))
            : rows}
      </div>
    </section>
  );
}
