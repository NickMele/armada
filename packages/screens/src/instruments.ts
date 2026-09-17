// What the two instruments on a Job page draw, as numbers and a sentence.
// `docs/contracts/design-system.md`, under Instruments.
//
// Out of `chapters.tsx` so the arithmetic is tested in node: a window boundary
// and a file with no count are cases, and a hundred cost what one costs.
import type { JobFootprint, Turn } from "@armada/protocol";
import type { FootprintColumn } from "@armada/components";

import { instant } from "./duration";

/** One bar's width. The contract's, not a tuning knob. */
export const ACTIVITY_WINDOW_MS = 30_000;
/** How far back the bars reach: twelve minutes, twenty-four windows. */
export const ACTIVITY_WINDOWS = 24;

/** What Activity measures, and its two edges, in the words the drawing carries. */
export const ACTIVITY_SAID = { label: "Tool calls per 30 seconds", from: "12m ago", to: "now" } as const;

/** What Footprint measures. */
export const FOOTPRINT_LABEL = "Lines changed per file";

/** Tool calls per window, oldest first, and what they say in words. */
export type Activity = {
  windows: number[];
  /** Calls inside the twelve minutes, which the bars add up to. */
  calls: number;
  description: string;
};

/**
 * Tool calls per 30s window over the twelve minutes ending at `now`.
 *
 * **A call stamped after `now` counts in the newest window.** Fleet's clock
 * and this one are two clocks, and a call a second ahead of Bridge still
 * happened in the window being drawn. A `ts` that will not parse is dropped
 * rather than placed.
 */
export function activityOf(turns: readonly Turn[], stepId: string | undefined, now: number): Activity {
  const windows = new Array<number>(ACTIVITY_WINDOWS).fill(0);
  const from = now - ACTIVITY_WINDOWS * ACTIVITY_WINDOW_MS;
  for (const turn of turns) {
    if (turn.saw.event !== "called") continue;
    // `workingOf`'s rule: a row with no step predates the field, so it stays.
    if (stepId !== undefined && turn.step !== undefined && turn.step !== stepId) continue;
    const at = instant(turn.ts);
    if (at === null || at <= from) continue;
    const index = Math.min(ACTIVITY_WINDOWS - 1, Math.floor((at - from) / ACTIVITY_WINDOW_MS));
    windows[index] = (windows[index] ?? 0) + 1;
  }
  const calls = windows.reduce((sum, count) => sum + count, 0);
  return { windows, calls, description: activitySaid(windows, calls) };
}

/** `No tool calls for the last 12 minutes`, or the count and how quiet the tail is. */
function activitySaid(windows: readonly number[], calls: number): string {
  const whole = minutesSaid(ACTIVITY_WINDOWS);
  if (calls === 0) return `No tool calls for the last ${whole}`;
  let quiet = 0;
  for (let i = windows.length - 1; i >= 0 && windows[i] === 0; i -= 1) quiet += 1;
  const counted = `${calls} tool ${calls === 1 ? "call" : "calls"} in the last ${whole}`;
  return quiet === 0
    ? `${counted}, some in the last 30 seconds`
    : `${counted}, none for the last ${minutesSaid(quiet)}`;
}

/** A count of windows in words: `30 seconds`, `1 minute`, `3.5 minutes`. */
function minutesSaid(windows: number): string {
  const seconds = (windows * ACTIVITY_WINDOW_MS) / 1000;
  if (seconds < 60) return `${seconds} seconds`;
  const minutes = seconds / 60;
  return `${minutes} ${minutes === 1 ? "minute" : "minutes"}`;
}

/** A footprint as columns, what was left out, and what it says in words. */
export type FootprintDrawing = {
  columns: FootprintColumn[];
  /** Files with no line count: in the key, never in the drawing. */
  uncounted: number;
  description: string;
};

/**
 * A finished Job's footprint as columns, widest first.
 *
 * **Widest first rather than the reading's order**, because the question the
 * drawing answers is which file took the work, and a sorted edge answers it
 * before a path is read. A file counted at nothing has no width to draw and is
 * left out without being called uncounted, since it was counted.
 */
export function footprintOf(kept: JobFootprint): FootprintDrawing {
  const columns: FootprintColumn[] = [];
  let uncounted = 0;
  for (const file of kept.files) {
    if (file.lines === undefined) {
      uncounted += 1;
      continue;
    }
    if (file.lines.added + file.lines.deleted === 0) continue;
    columns.push({
      path: file.path,
      added: file.lines.added,
      deleted: file.lines.deleted,
      // Present and empty is the drift; absent is nothing measured.
      outsidePlan: file.planned_by !== undefined && file.planned_by.length === 0,
    });
  }
  columns.sort((a, b) => b.added + b.deleted - (a.added + a.deleted));
  return { columns, uncounted, description: footprintSaid(columns, uncounted) };
}

function footprintSaid(columns: readonly FootprintColumn[], uncounted: number): string {
  const parts: string[] = [];
  const total = columns.reduce((sum, column) => sum + column.added + column.deleted, 0);
  const widest = columns[0];
  if (widest === undefined) {
    parts.push("No changed lines were counted");
  } else {
    const added = columns.reduce((sum, column) => sum + column.added, 0);
    parts.push(`${files(columns.length)} changed ${total} lines, +${added} −${total - added}`);
    if (columns.length > 1) {
      const share = Math.round(((widest.added + widest.deleted) / total) * 100);
      parts.push(`${widest.path} took ${share}% of them`);
    }
    const outside = columns.filter((column) => column.outsidePlan).length;
    if (outside > 0) {
      parts.push(`${files(outside)} outside every declared plan`);
    }
  }
  if (uncounted > 0) parts.push(`${files(uncounted)} not counted`);
  return parts.join("; ");
}

function files(count: number): string {
  return `${count} ${count === 1 ? "file" : "files"}`;
}
