// A reading is the open Job's, or it is not.
//
// Five readings reach the detail screen — the Job whole, the Drone's turns, the
// Job's own log, what it holds on this machine, and what the last look found —
// and every one of them carries the id it was taken for. **Every one of them
// has to be checked against the Job on screen before a pixel is drawn from
// it**, because all five lag a selection by a round trip: open one Job, open
// the next before the first answers, and the answer that lands is about a Job
// nobody is looking at.
//
// `apps/desktop/src/main/reader.ts` states the same rule on the other side of
// the seam and drops a read whose id moved while it was in flight. This is the
// renderer's half, and it is needed as well rather than instead: a socket that
// is still open on the previous Job is publishing correctly and about the wrong
// Job, which no request-side guard can catch.
//
// **Its own file so the sixth reading has somewhere to be written.** Four of
// the five were added one at a time, each by copying the ternary above it into
// `JobDetail.tsx`; a fifth copy is how one of them ends up without the check.

import type {
  Diff,
  Evidence,
  Examination,
  Footprint,
  Holds,
  JobDetail as JobWhole,
  JobLog,
  JobResources,
  Journalled,
  Observed,
  Remarks,
  Turns,
  Watched,
} from "@armada/protocol";

/**
 * The reads the panel's own chapters draw from. `JobDetail.tsx` re-exports it,
 * which is where every caller already imports it from; it moved here, beside
 * the other readings of one Job, when that file reached the gate's line count.
 */
export type FoldedReads = {
  footprint: Footprint;
  evidence: Evidence;
  diff: Diff;
  /**
   * What people wrote on the Job's pull request, where the decision block asked
   * for it. **The one read in this set that costs a forge**, so nothing takes
   * it on a timer and no event refreshes it.
   */
  remarks: Remarks;
};

/**
 * The look a person pressed for, once it is known to be this Job's. The `none`
 * arm is gone, because a look nobody asked for and a look about another Job are
 * the same nothing to a panel.
 */
export type Looked = Exclude<Examination, { state: "none" }>;

/**
 * The Job read whole. **A stale one from the Job that was open a moment ago
 * would draw another Job's steps under this Job's title** — which is the whole
 * of why this file exists, in the one place it is most visible.
 */
export function detailOf(watched: Watched, jobId: string): JobWhole | null {
  return watched.state === "read" && watched.jobId === jobId ? watched.detail : null;
}

/**
 * The rows this Job's observe socket has carried, or none. Kept through
 * `ended` and `failed`, which is `Observed`'s own rule: a closed transcript is
 * still a record.
 */
export function turnsOf(observed: Observed, jobId: string): Turns | null {
  return "turns" in observed && observed.jobId === jobId ? observed.turns : null;
}

/** What Fleet has done to the Job itself, on the same terms. */
export function logOf(journalled: Journalled, jobId: string): JobLog | null {
  return "log" in journalled && journalled.jobId === jobId ? journalled.log : null;
}

/** What the open Job holds on this machine, where the reading is its own. */
export function holdingOf(resources: Holds, jobId: string): JobResources | null {
  return resources.state === "read" && resources.jobId === jobId ? resources.resources : null;
}

/**
 * The last look, where somebody pressed for one on this Job. **`looking` is
 * kept** — the press in flight is what stops a second press, and it is as much
 * this Job's as an answer is.
 */
export function lookOf(examination: Examination, jobId: string): Looked | null {
  return examination.state !== "none" && examination.jobId === jobId ? examination : null;
}
