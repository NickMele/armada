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
// **A running Job's counts are Fleet's live reading**, taken once the Drone's
// calls settle and at most every ten seconds, since protocol 14.9 — #1187. A
// finished Job's are the count taken as it stopped.
//
// `files.ts` is the live half and says the same thing from the other side.

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
 * How often an open diff takes its reading again while a Drone is writing.
 *
 * **Five seconds, and it is the resolution of a live patch rather than a
 * setting.** `FOOTPRINT_INTERVAL` in `crates/fleet/src/footprint.rs` is two,
 * for a reading that costs a directory walk; this one costs the patch, which
 * `docs/journeys/monitor-active-work.md` measured at 25ms over a hundred files
 * and 90ms over four hundred. Slower than a person re-reads a hunk, faster
 * than they can wonder whether it is stuck.
 */
const WHILE_OPEN = 5_000;

/**
 * Take the patch again for the sheet somebody is looking at: on the press that
 * opens it, whenever the file list moves under it, and on a clock while a
 * Drone is still writing.
 *
 * **Only while it is open, and the clock only while a Drone holds the pen.**
 * The patch is the megabyte the split in `crates/ipc/src/work.rs` exists to
 * save. Fleet republishes a footprint only where the *list* changed, so the
 * event alone left a Drone editing the same seven files drawing the hunks as
 * of whenever the list last moved — the reading a person opened the sheet to
 * watch, frozen with nothing saying so. A worktree nobody is writing to cannot
 * go stale, so a Job with no Drone on it pays nothing.
 *
 * `apps/desktop/src/main/review.ts` holds why taking it again does not blank
 * the layer being read.
 */
export function useDiffAgain(
  read: (jobId: string | null) => void,
  jobId: string,
  sheet: string | null,
  turns: readonly Turn[],
  working: boolean,
): void {
  const wrote = wroteSoFar(turns);
  useEffect(() => {
    if (sheet === "diff") read(jobId);
  }, [sheet, wrote, jobId]);
  useEffect(() => {
    if (sheet !== "diff" || !working) return;
    const ticking = setInterval(() => read(jobId), WHILE_OPEN);
    return () => clearInterval(ticking);
  }, [sheet, working, jobId]);
}
