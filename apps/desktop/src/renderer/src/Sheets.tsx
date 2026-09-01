// The two readings the panel cannot hold, on the layer that can — #286, and
// Journey 4's frames 4i-4m.
//
// The activity log holds 1676 entries on a real Job and the diff is the Job's
// whole patch. Neither is a longer version of something a chapter can hold: an
// expander pushes every chapter under it off the screen, and a patch in a 602px
// column is a decision taken on a line that wrapped.
//
// **One sheet at a time, and `Esc` returns to the panel** rather than to the
// previous sheet. Which one is open is `JobDetail`'s state; what closes one is
// `Sheet` itself, which catches `Esc` in the capture phase so the other clause
// of the same registry row — *returns to the list from a detail route* — does
// not answer the same press.
//
// **Two exits and no third.** The labelled control and `Esc`. A click on the
// ground behind does not close a sheet.

import { ActivityLogSheet, JobDiffSheet, type JobDiffFile } from "@armada/components";
import type { ReactNode } from "react";

import type { Diff, Footprint, Observed } from "../../shared/bridge";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "../../shared/protocol";
import type { JobFootprint } from "../../shared/footprint";
import type { Calls } from "./calls";
import { DecidedDiff } from "./Decide";
import { clock } from "./duration";
import { readingFor } from "./files";
import { Log } from "./Log";
import { producedIn } from "./produced";
import { recourseOf } from "./recovery";
import { NOTHING_YET_ON_THIS_STEP, type LogRow } from "./story";

/** Which sheet is open, or none. Two cannot be. */
export type OpenSheet = "log" | "diff" | null;

/**
 * Where the log's reading was held, and how much it had then.
 *
 * **The tail is not followed while the sheet is open.** A stream that scrolls
 * itself cannot be read, so the reading stays where it was put and what arrives
 * is counted instead — `rows` is the count at the moment it was held, and the
 * difference is what *Jump to now* carries.
 */
export type HeldAt = { at: string; rows: number };

export type DetailSheetProps = {
  which: OpenSheet;
  job: JobSummary;
  whole: JobWhole | null;
  /** The step the panel is showing, restated here: the tree is under the layer. */
  step: StepDetail;
  /** The step's rows, in the order they arrived. */
  rows: LogRow[];
  observed: Observed;
  footprint: Footprint;
  diff: Diff;
  calls: Calls;
  /** What one log takes, by name, so the sheet's rows are not the preview's. */
  log: { region: string; openId: string | null; onOpen: (rowId: string | null) => void };
  held: HeldAt | null;
  onHold: (held: HeldAt) => void;
  /** Now, injected, because holding the reading records a wall clock. */
  now: number;
  /** The window is at `--window-floor`. */
  floor: boolean;
  onClose: () => void;
};

export function DetailSheet({
  which,
  job,
  whole,
  step,
  rows,
  observed,
  footprint,
  diff,
  calls,
  log,
  held,
  onHold,
  now,
  floor,
  onClose,
}: DetailSheetProps) {
  if (which === "log") {
    return (
      <ActivityLogSheet
        open
        floor={floor}
        step={step.label}
        jobId={job.id}
        total={rows.length}
        live={observed.state === "watching"}
        // When the stream stopped. Nothing on the wire carries a Job's end, so
        // this is the open step's own `updated_at` — the instant the panel's
        // own `Took` is measured to, rather than a second reading of it.
        endedAt={clock(step.updated_at)}
        heldAt={held?.at}
        arrived={held === null ? 0 : Math.max(rows.length - held.rows, 0)}
        onJumpToNow={() => onHold(holdOf(now, rows.length))}
        escalation={escalationOf(job, whole, step, onClose)}
        onClose={onClose}
      >
        <Log rows={rows} emptyNote={NOTHING_YET_ON_THIS_STEP} calls={calls} {...log} />
      </ActivityLogSheet>
    );
  }
  if (which === "diff") {
    return (
      <JobDiffSheet
        open
        floor={floor}
        branch={job.branch ?? job.id}
        files={railOf(footprint, whole?.footprint, job.id)}
        note={WHICH_STEP_WROTE_IT}
        onClose={onClose}
      >
        <DecidedDiff diff={diff} jobId={job.id} />
      </JobDiffSheet>
    );
  }
  return null;
}

/** The reading, held where it is now. One spelling, used opening and jumping. */
export function holdOf(now: number, rows: number): HeldAt {
  return { at: clock(new Date(now).toISOString()), rows };
}

/**
 * The file rail beside the patch — the paths, and what each gained and lost.
 *
 * **No step against a file.** The drawing names the step that wrote each one
 * and nothing served says which step that was: the footprint carries
 * `planned_by`, which is the step that *promised* a path, and a file no step
 * declared would then read as a file no step wrote. The rail draws the counts
 * alone and says why underneath rather than guessing. Reported.
 */
function railOf(footprint: Footprint, kept: JobFootprint | undefined, jobId: string): JobDiffFile[] {
  const produced = producedIn(kept, readingFor(footprint, jobId));
  if (produced === undefined) return [];
  return produced.files.map((file) => ({
    path: file.path,
    added: file.added ?? 0,
    removed: file.deleted ?? 0,
  }));
}

/** What the rail says instead of naming a step, because nothing serves one. */
const WHICH_STEP_WROTE_IT =
  "Fleet commits once at the end, so the patch is the Job's. Nothing served says which step " +
  "wrote each file.";

/**
 * What the Job did while the sheet was open, where it did something.
 *
 * **The sheet says the Job moved and no more.** The failed step and its hued
 * cross are on the rail behind the layer, which is where that reading lives.
 * The act does not grow either: *Show me* closes the sheet, which is what `Esc`
 * already does, so it is a labelled second face of one act rather than a second
 * binding — and Pilot keeps the accent in the Job header, behind the layer.
 */
function escalationOf(
  job: JobSummary,
  whole: JobWhole | null,
  step: StepDetail,
  onShowMe: () => void,
): { at: string; because: ReactNode; onShowMe: () => void } | undefined {
  if (job.status !== "escalated") return undefined;
  return { at: clock(step.updated_at), because: recourseOf(job, whole).stands, onShowMe };
}
