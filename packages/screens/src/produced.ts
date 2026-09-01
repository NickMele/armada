// What the Produced chapter draws, and which of two readings it draws from.
//
// **The record wins wherever there is one.** Fleet serves `JobDetail.footprint`
// only on a job that has stopped, so its presence is the whole test: a running
// job has the live event and nothing else, and a finished one has a reading
// taken at the moment it stopped rather than whenever the last watcher happened
// to be looking. Preferring the live reading on a finished job would draw
// whatever the socket last carried, which on a job opened after it ended is
// nothing at all.
//
// **This is where the line counts appear, and why they appear only here.**
// Counting is the same walk that renders the patch — 25ms over a hundred files,
// 90ms over four hundred, against under a microsecond for the paths — so Fleet
// takes it once, at the transition that ends the job. A running job's chapter
// reads `3 files · all inside the plan` and a finished one reads `3 files · +94
// −31 · all inside the plan`, and the difference is a measurement rather than a
// field somebody forgot to send.
//
// `files.ts` is the live half and says the same thing from the other side.

import type { ChangedFile } from "@armada/components";

import type { JobFilesChanged } from "@armada/protocol";
import type { JobFootprint, TouchedFile } from "@armada/protocol";
import { filesOf, footprintNote } from "./files";

/** One reading, shaped for the chapter that draws it. */
export type Produced = {
  /** The rows, in the order the reading found them. Never re-sorted. */
  files: ChangedFile[];
  /**
   * Whether there is a plan for a row's mark to mean anything.
   *
   * **False is "nothing declared one", not "nothing drifted."** The summary
   * omits the clause rather than claiming everything is inside a plan that does
   * not exist.
   */
  planDeclared: boolean;
  /** What the list says about itself, under it. */
  note: string;
};

/**
 * The reading to draw: the record where the job kept one, the live event
 * otherwise, and nothing where neither has arrived.
 */
export function producedIn(
  kept: JobFootprint | undefined,
  live: JobFilesChanged | undefined,
): Produced | undefined {
  if (kept !== undefined) {
    return {
      files: kept.files.map(keptRow),
      planDeclared: (kept.plans ?? []).length > 0,
      note: keptNote(kept),
    };
  }
  if (live !== undefined) {
    return { files: filesOf(live), planDeclared: live.plan_declared, note: footprintNote(live) };
  }
  return undefined;
}

/**
 * One kept file as a row.
 *
 * **`planned_by` has three readings and only one of them is a mark.** Absent is
 * a job where no step declared anything, so nothing was measured and nothing is
 * marked. Present and empty is a path outside every plan that was declared,
 * which is the drift. Present with steps in it is a path one of them promised.
 *
 * A file with no `lines` carries no counts rather than zeroes, so a file nobody
 * could count and a file that gained and lost nothing stay apart.
 */
function keptRow(file: TouchedFile): ChangedFile {
  return {
    path: file.path,
    change: file.change,
    outsidePlan: file.planned_by !== undefined && file.planned_by.length === 0 ? true : undefined,
    ...(file.lines === undefined ? {} : { added: file.lines.added, deleted: file.lines.deleted }),
  };
}

/**
 * What the record says about itself.
 *
 * **It says when it was taken.** A finished job's list is not the live reading
 * gone quiet — it is what the worktree held at the instant the job stopped, and
 * it is still there after `armada clean` has given the worktree back.
 *
 * The drift clause counts against every plan every step declared, not against
 * one. A job's work is the whole branch and a plan belongs to a step, so
 * `implement` scoping three files says nothing about what `handoff` wrote.
 */
export function keptNote(kept: JobFootprint): string {
  const taken = "Read when the job stopped.";
  if ((kept.plans ?? []).length === 0) {
    return `${taken} No step declared a plan, so nothing here is measured against one.`;
  }
  const outside = kept.files.filter(
    (file) => file.planned_by !== undefined && file.planned_by.length === 0,
  ).length;
  return outside === 0
    ? `${taken} Every path is inside the plans the steps declared.`
    : `${taken} ${outside} of ${kept.files.length} paths are outside the plans the steps declared.`;
}
