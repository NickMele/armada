// The mechanism a Job list's keyboard needs, shared by every screen that draws `Row`.
//
// **`keys.ts` owns the map — what a key means.** This file owns none of that; it is the window
// listener, the DOM-focus-as-cursor pattern, and `j`/`k`/arrow movement `Jobs.tsx` built first.
// `OverviewLists.tsx` draws the same rows and needed the same three things, and copying `Jobs.tsx`'s
// implementation into a second file is exactly the duplicate this repository treats as a defect —
// two copies of "the cursor is DOM focus" is two places that rule can drift apart.
//
// **The cursor is DOM focus, not state kept beside it.** A row reached by the mouse, by Tab, by a
// panel's own arrows or by `j` all set the same value, because every one of those moves focus and
// `useListCursor` only reads it back through the wrapper's own `onFocusCapture`.

import { useEffect, useRef, useState, type FocusEvent } from "react";

/** Every row on screen, in the order the DOM has them — the only place their drawn order is. */
export function rowsOnScreen(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>("[data-job-id]"));
}

export type ListCursor = {
  /** The job id under the cursor, or `null` before anything on the list has been focused. */
  cursor: string | null;
  /** Wired to the list's outer `onFocusCapture`: every row that gains focus becomes the cursor. */
  onFocusCapture: (event: FocusEvent) => void;
  /**
   * Move the cursor by one row. Clamped rather than wrapped — `Active jobs list`'s own rule for the
   * arrows: a list that jumps from the last row to the first loses the reader's place, and a list
   * here is scanned rather than cycled.
   */
  move: (by: 1 | -1) => void;
  /**
   * Put the cursor back where it was without moving it — what leaving a search field hands back to.
   * The row it was on where that row is still drawn, and the first row where it is not.
   */
  restoreCursor: () => void;
};

/**
 * The cursor mechanism `Jobs.tsx` built and this file now carries so `OverviewLists.tsx` can share
 * it rather than reimplement it.
 */
export function useListCursor(onCursor?: (jobId: string | null) => void): ListCursor {
  const [cursor, setCursor] = useState<string | null>(null);

  function onFocusCapture(event: FocusEvent): void {
    const row = (event.target as HTMLElement).closest<HTMLElement>("[data-job-id]");
    if (row?.dataset.jobId === undefined) return;
    setCursor(row.dataset.jobId);
    onCursor?.(row.dataset.jobId);
  }

  function move(by: 1 | -1): void {
    const rows = rowsOnScreen();
    if (rows.length === 0) return;
    const at = rows.findIndex((row) => row.dataset.jobId === cursor);
    const to = at < 0 ? (by === 1 ? 0 : rows.length - 1) : Math.min(Math.max(at + by, 0), rows.length - 1);
    rows[to]?.focus();
  }

  function restoreCursor(): void {
    const rows = rowsOnScreen();
    const at = rows.findIndex((row) => row.dataset.jobId === cursor);
    (at >= 0 ? rows[at] : rows[0])?.focus();
  }

  return { cursor, onFocusCapture, move, restoreCursor };
}

/**
 * The window listener every list keyboard needs: registered once, reading the latest handler out of
 * a ref so a clock-driven re-render does not churn the subscription — `Jobs.tsx`'s own reason, since
 * `now` moves once a second and the whole board re-renders with it.
 *
 * **Refuses every press while a dialog is open.** The confirmation a kill, a clear or a redispatch
 * opens is `App`'s, outside whichever list component is listening, so a window listener would
 * otherwise open a job behind the dialog a person is answering.
 */
export function useListKeydown(press: (event: KeyboardEvent) => void): void {
  const latest = useRef(press);
  useEffect(() => {
    latest.current = press;
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent): void => {
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      latest.current(event);
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);
}
