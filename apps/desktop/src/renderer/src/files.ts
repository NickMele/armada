// What a Drone has changed in its worktree — the live readings main holds from
// `job.files_changed`, and the record Fleet wrote down when the Job stopped.
//
// **They are two answers and this module keeps them apart.** A live reading is
// what a working Drone has touched so far and carries a drift mark; a record is
// what the Job finally touched and carries none. The pair below each other's
// names — `filesOf`/`footprintNote` for the first, `touchedOf`/`RECORD_NOTE`
// for the second — rather than one function taking a flag, because the sentences
// they produce have nothing in common.
//
// **Nothing here reads a worktree.** The paths arrive as a named event like
// everything else, and Bridge does not open a directory to check them. Nor is
// anything counted that Fleet did not send: there are no line counts here,
// because the patch is the expensive read and it is not on this seam.
//
// **The change kind renders as the wire spelled it.** `enum-verbs.toml` carries
// no `change_kind` rows, so there is no verb, glyph or hue for one — the same
// fallback a step state takes, and for the same reason. A word chosen here
// would be the second vocabulary the generated module exists to prevent.
// Reported.

import type { ChangedFile } from "@armada/components";

import type {
  JobFilesChanged,
  JobFootprint,
  ChangedFile as WireFile,
  TouchedFile,
} from "../../shared/protocol";
import type { Footprint } from "../../shared/bridge";

/** The rows, in the order the reading found them. Never re-sorted. */
export function filesOf(reading: JobFilesChanged): ChangedFile[] {
  return reading.files.map((file: WireFile) => ({
    path: file.path,
    change: file.change,
    // Absent and `false` are the same fact here — the wire omits it rather than
    // sending null — so the row is marked only where it is true.
    outsidePlan: file.outside_plan === true || undefined,
  }));
}

/**
 * What the list says about itself: where the reading came from, and what the
 * drift mark does or does not mean on it.
 *
 * **`plan_declared` false is "there is no plan", not "nothing drifted."** A
 * note that stayed silent about it would let an unscoped step read as a step
 * perfectly on plan, which is the one thing this field exists to prevent.
 */
export function footprintNote(reading: JobFilesChanged, live: boolean): string {
  const read = live
    ? "Read from the worktree while the drone is working."
    : "The last reading taken while the drone was working, and not the record. The record is what fleet wrote down when the job stopped.";
  if (!reading.plan_declared) {
    return `${read} This step declared no plan, so no row is marked.`;
  }
  const outside = reading.files.filter((file) => file.outside_plan === true).length;
  const total = reading.files.length;
  return outside === 0
    ? `${read} Every path is inside the plan this step declared.`
    : `${read} ${outside} of ${total} paths are outside the plan this step declared.`;
}

/**
 * The one-line answer for the outcome row: how many files, and how many of them
 * drifted. **Counts of what arrived, never of what was measured** — no line
 * count appears here, because nothing serves one.
 */
export function footprintSummary(reading: JobFilesChanged): { value: string; meta?: string } {
  const total = reading.files.length;
  const value = total === 1 ? "1 file" : `${total} files`;
  if (!reading.plan_declared) return { value };
  const outside = reading.files.filter((file) => file.outside_plan === true).length;
  return outside === 0 ? { value } : { value, meta: `${outside} outside plan` };
}

/** The reading for this Job, or nothing. A footprint belongs to one Job. */
export function readingFor(footprint: Footprint, jobId: string): JobFilesChanged | undefined {
  return footprint.state === "read" && footprint.jobId === jobId ? footprint.reading : undefined;
}

/**
 * Why there is no footprint, which is never the same sentence twice.
 *
 * **A Drone that has not reported yet and a Job with no Drone on it are two
 * different facts.** `job.files_changed` is published by a working Drone, so
 * the first says wait and the second says nothing is coming — and one sentence
 * for both would leave a Job at the approval gate looking like it had stalled.
 * The third case, a Job that is over, is `NOT_SERVED_WHEN_FINISHED`.
 */
export function whyNoFootprint(hasDrone: boolean): string {
  return hasDrone
    ? "This drone has not reported its changed files yet. The first reading arrives as it works."
    : "No drone is on this job, so nothing is reading its worktree.";
}

/**
 * Named once, because the outcome row and the record section both say it.
 *
 * **It is the older-job sentence now, not the shape of the seam.** Fleet writes
 * a footprint down at the terminal transition and serves it on `JobDetail`, so
 * a job with none either stopped before that existed or had a worktree that
 * would not open when it did — and the job's own log says which.
 */
export const NOT_SERVED_WHEN_FINISHED =
  "No footprint was recorded when this job stopped. It ended before fleet kept one, or its " +
  "worktree could not be read at the time — the job's log says which.";

// ------------------------------------------- the record, not the last reading

/** The rows, in the order the reading found them. Never re-sorted. */
export function touchedOf(footprint: JobFootprint): ChangedFile[] {
  return footprint.files.map((file: TouchedFile) => ({
    path: file.path,
    change: file.change,
    // No `outsidePlan`, and it is absent rather than false: the wire type
    // carries no mark, because a plan belongs to the step that declared it and
    // this is the job's whole work. Nothing here may invent one.
  }));
}

/**
 * What the record says about itself: that it is a record, and why nothing on it
 * is marked.
 *
 * **The second sentence is the point.** A reader deciding whether work stayed in
 * scope has to be told the marks are not here, rather than left to read their
 * absence as every path being inside a plan.
 */
export const RECORD_NOTE =
  "Read from this job's worktree when the job stopped, and kept — so it says the same thing " +
  "whether or not anyone was watching. No path is marked: a declared plan belongs to one step, " +
  "and this is the job's whole work since the branch was cut.";

/**
 * The one-line answer for the outcome row: how many files. **No drift count**,
 * for `recordNote`'s reason — the record carries no mark, and a `0 outside
 * plan` here would be a measurement nobody made.
 */
export function recordSummary(footprint: JobFootprint): { value: string; meta?: string } {
  const total = footprint.files.length;
  return { value: total === 1 ? "1 file" : `${total} files` };
}

/** What a record with no rows in it says. Ordinary, and never an error. */
export const TOUCHED_NOTHING =
  "This job's worktree was read when it stopped and held no change against the branch it was " +
  "cut from. That is what a diff_nonempty check refuses, and it is not the same as a job whose " +
  "footprint was never recorded.";
