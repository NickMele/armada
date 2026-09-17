import type { RefObject } from "react";
import { useLayoutEffect, useRef } from "react";

/**
 * A row re-sorting travels to its new place at `--duration-travel`; a new row does not travel.
 * `docs/contracts/design-system.md`, Motion.
 *
 * **Travel is a change of order among rows that were already there.** A row inserted above pushes
 * the rest down, and that is not a re-sort: only a row whose rank among the surviving rows moved
 * travels, so a poll that adds or drops a Job moves nothing, and one that changes nothing measures
 * and returns.
 *
 * Positions are `offsetTop` rather than `getBoundingClientRect`, which would read a row still
 * mid-flight from the last re-sort, and relative to the frame so a scroll between renders is not
 * read as travel.
 */
export function useTravel(frame: RefObject<HTMLElement | null>): void {
  const last = useRef<{ order: string[]; tops: Map<string, number> } | null>(null);

  useLayoutEffect(() => {
    const list = frame.current;
    if (list === null) {
      last.current = null;
      return;
    }
    const rows = Array.from(list.querySelectorAll<HTMLElement>(":scope > [data-job-id]"));
    const order = rows.map((row) => row.dataset.jobId ?? "");
    const tops = new Map(rows.map((row, at) => [order[at] ?? "", topIn(row, list)]));
    const before = last.current;
    last.current = { order, tops };
    if (before === null || sameOrder(before.order, order)) return;

    const moved = reranked(before.order, order);
    if (moved.size === 0) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const style = getComputedStyle(list);
    const duration = msOf(style.getPropertyValue("--duration-travel"));
    if (!(duration > 0)) return;
    const easing = style.getPropertyValue("--ease").trim() || "linear";

    rows.forEach((row, at) => {
      const id = order[at] ?? "";
      const from = before.tops.get(id);
      const to = tops.get(id);
      if (!moved.has(id) || from === undefined || to === undefined || from === to) return;
      row.animate([{ transform: `translateY(${from - to}px)` }, { transform: "none" }], { duration, easing });
    });
  });
}

function topIn(row: HTMLElement, list: HTMLElement): number {
  return row.offsetParent === list ? row.offsetTop : row.offsetTop - list.offsetTop;
}

function sameOrder(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((id, at) => id === b[at]);
}

/** The rows present both times whose rank among each other changed. */
function reranked(before: readonly string[], after: readonly string[]): Set<string> {
  const kept = new Set(after);
  const was = before.filter((id) => kept.has(id));
  const stayed = new Set(was);
  const now = after.filter((id) => stayed.has(id));
  return new Set(now.filter((id, at) => was[at] !== id));
}

function msOf(raw: string): number {
  const value = Number.parseFloat(raw);
  if (!Number.isFinite(value)) return 0;
  return raw.trim().endsWith("ms") ? value : value * 1000;
}
