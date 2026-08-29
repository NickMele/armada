// What a Drone has changed in its worktree — the live readings main holds from
// `job.files_changed`, and the record Fleet wrote down when the Job stopped.
//
// **They are two answers and this module keeps them apart.** A live reading is
// what a working Drone has touched so far and is marked against the plan the
// step being watched declared; a record is the Job's whole work since the
// branch was cut, and it is marked against every plan any step declared, which
// Fleet keeps beside it. The pair below each other's names —
// `filesOf`/`footprintNote` for the first, `touchedOf`/`recordNote` for the
// second — rather than one function taking a flag, because the sentences they
// produce have nothing in common.
//
// **The record's mark is absent, not false, where nothing was declared.** A
// path on a Job whose steps never scoped anything was not measured and found
// clean; it was not measured. `planned_by` carries that as three readings and
// this module never collapses them to two.
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
    // Marked only where a measurement was made and came back with nothing:
    // `planned_by` present and empty is a path outside every plan any step
    // declared. Absent is a path nothing was declared for, and it stays
    // `undefined` rather than becoming `false`, which the row would draw the
    // same way but a later reader would take for "inside the plan".
    outsidePlan: outside(file) || undefined,
  }));
}

/** Whether this path was measured against a plan and fell outside every one. */
function outside(file: TouchedFile): boolean {
  return file.planned_by !== undefined && file.planned_by.length === 0;
}

/**
 * Every step that declared a plan on this job, in the order they declared, each
 * named once however many runs of it declared.
 *
 * **A step worked twice declares twice and is still one step to a reader.** The
 * run ordinal is on the wire and is what keeps the two promises apart in the
 * record; it is not what a sentence naming who promised is about.
 */
export function declaringSteps(footprint: JobFootprint): string[] {
  const named: string[] = [];
  for (const plan of footprint.plans ?? []) {
    if (!named.includes(plan.step_id)) named.push(plan.step_id);
  }
  return named;
}

/** How many paths were measured against a plan and fell outside every one. */
export function outsideCount(footprint: JobFootprint): number {
  return footprint.files.filter(outside).length;
}

/**
 * What the record says about itself: that it is a record, and what its marks do
 * or do not mean.
 *
 * **The second sentence is the point, and it is a different sentence now.** A
 * reader deciding whether to take the work is owed either the drift or the fact
 * that nobody measured it — and the one thing they must never be left to infer
 * is that unmarked means in scope. A job whose steps declared nothing says so
 * in words rather than by drawing no marks.
 */
export function recordNote(footprint: JobFootprint): string {
  const read =
    "Read from this job's worktree when the job stopped, and kept — so it says the same thing " +
    "whether or not anyone was watching.";
  const steps = declaringSteps(footprint);
  if (steps.length === 0) {
    return `${read} No path is marked, because no step declared a plan for this job — nothing here was measured against one.`;
  }
  const promised = `Measured against what ${steps.join(", ")} declared.`;
  const drifted = outsideCount(footprint);
  const total = footprint.files.length;
  return drifted === 0
    ? `${read} ${promised} Every path is inside one of those plans.`
    : `${read} ${promised} ${drifted} of ${total} paths are outside all of them.`;
}

/**
 * The one-line answer for the outcome row: how many files, and how many of them
 * went outside every declared plan.
 *
 * **No drift count where no plan was declared**, which is the whole of why the
 * meta is conditional: a `0 outside plan` on a job nobody scoped would be a
 * measurement nobody made.
 */
export function recordSummary(footprint: JobFootprint): { value: string; meta?: string } {
  const total = footprint.files.length;
  const value = total === 1 ? "1 file" : `${total} files`;
  if (declaringSteps(footprint).length === 0) return { value };
  const drifted = outsideCount(footprint);
  return drifted === 0 ? { value } : { value, meta: `${drifted} outside plan` };
}

/** What a record with no rows in it says. Ordinary, and never an error. */
export const TOUCHED_NOTHING =
  "This job's worktree was read when it stopped and held no change against the branch it was " +
  "cut from. That is what a diff_nonempty check refuses, and it is not the same as a job whose " +
  "footprint was never recorded.";
