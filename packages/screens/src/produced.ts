// What the Produced chapter draws, and which of two readings it draws from.
//
// **The record wins wherever there is one.** Fleet serves
// `JobDetail.footprint` only on a stopped job — a running job has the
// live event and nothing else; a finished one has a reading taken when
// it stopped, not last watched. Preferring live on a finished job draws
// whatever the socket last carried, nothing on a job opened after it ended.
//
// **This is where the line counts appear, and only here** — counting is
// the same walk that renders the patch: 25ms over a hundred files, 90ms
// over four hundred, against under a microsecond for the paths. Fleet
// takes it once, at the transition that ends the job. A running chapter
// reads `3 files · all inside the plan`; a finished one reads `3 files ·
// +94 −31 · all inside the plan` — a measurement, not a forgotten field.
// `files.ts` is the live half, saying the same thing from the other side.

import { useEffect } from "react";

import type { ChangedFile } from "@armada/components";

import type { JobFilesChanged, Turn } from "@armada/protocol";
import type { ChangedFile as WireFile, JobFootprint, TouchedFile } from "@armada/protocol";
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
  /** Where the list drifted from a plan, under it. Nothing where it did not. */
  note?: string;
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
export function keptNote(kept: JobFootprint): string | undefined {
  if ((kept.plans ?? []).length === 0) return undefined;
  const outside = kept.files.filter(
    (file) => file.planned_by !== undefined && file.planned_by.length === 0,
  ).length;
  return outside === 0
    ? undefined
    : `${outside} of ${kept.files.length} paths are outside the plans the steps declared.`;
}

/**
 * What the live footprint last said this Job has written, as one value an
 * effect can depend on.
 *
 * **The signal the diff sheet re-reads on.** The chapter and the sheet read one
 * worktree at two different times, and the sheet's reading was taken when the
 * Job was opened — so a Drone that wrote afterwards left `0 files` beside a
 * list naming seven. This is what says the list moved.
 *
 * The last reading wins, `timeline.tsx`'s rule for the same event: Fleet
 * republishes the footprint only where it changed, so a Drone editing the same
 * files for an hour produces one value and no re-read.
 */
export function wroteSoFar(turns: readonly Turn[]): string {
  let files: readonly WireFile[] = [];
  for (const turn of turns) if (turn.saw.event === "produced") files = turn.saw.files;
  return files.map((file) => `${file.change} ${file.path}`).join("\n");
}

/**
 * Whose work the list is, said under it.
 *
 * **The chapter is the Job's and it is drawn inside one step's row.** Both
 * readings behind it are whole-worktree — the live event and the record Fleet
 * keeps at the stopping instant — and since the story became the step
 * timeline, they are drawn under the phase one Drone worked. A reader takes
 * that for the step's own output, which is the reading the row's own count
 * contradicts. The sheet says the same sentence over the same files.
 */
export const THE_JOBS_WORK =
  "Fleet commits once at the end, so this is the job's work and not this step's.";

/** The note under the list: whose work it is, and where it went outside a plan. */
export function noteUnder(produced: Produced): string {
  return produced.note === undefined ? THE_JOBS_WORK : `${THE_JOBS_WORK} ${produced.note}`;
}

/**
 * Take the patch again for the sheet somebody is looking at: on the press that
 * opens it, and whenever the file list moves under it.
 *
 * **Only while it is open.** The patch is the megabyte the split in
 * `crates/ipc/src/work.rs` exists to save, and Fleet republishes a footprint
 * only where the list changed — so a Drone editing the same files all night
 * costs one read. `apps/desktop/src/main/review.ts` holds why taking it again
 * does not blank the layer being read.
 */
export function useDiffAgain(
  read: (jobId: string | null) => void,
  jobId: string,
  sheet: string | null,
  turns: readonly Turn[],
): void {
  const wrote = wroteSoFar(turns);
  useEffect(() => {
    if (sheet === "diff") read(jobId);
  }, [sheet, wrote, jobId]);
}
