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

import { ActivityLogSheet, JobDiffSheet, railOfPatch, type JobDiffFile } from "@armada/components";
import { useMemo, type ReactNode } from "react";

import type { Diff, Observed } from "@armada/protocol";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { Calls } from "./calls";
import { DecidedDiff } from "./Decide";
import { clock } from "./duration";
import { Log } from "./Log";
import { recourseOf } from "./recovery";
import { drawn } from "./review";
import { NOTHING_YET_ON_THIS_STEP, whyNotWatching, type LogRow } from "./story";

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
        {/* The sheet is the whole log, so a socket that stopped says so here
            for the reason the chapter's preview does: an empty sheet reading as
            a step that has not started is the panel's defect one layer out. */}
        <Log
          rows={rows}
          emptyNote={whyNotWatching(observed) ?? NOTHING_YET_ON_THIS_STEP}
          calls={calls}
          {...log}
        />
      </ActivityLogSheet>
    );
  }
  if (which === "diff") {
    return <DiffSheet job={job} diff={diff} floor={floor} onClose={onClose} />;
  }
  return null;
}

/**
 * The patch, the rail beside it and the count over both, from one reading.
 *
 * **Its own component so the split is not paid for by the log.** The parse is
 * held across renders, and the panel above ticks `now` every second: a
 * 2,000-line patch re-split on every tick is the freeze the v1 failure log
 * recorded nine times. A hook in `DetailSheet` would have to run before its
 * early return and would run on every log render too.
 */
function DiffSheet({
  job,
  diff,
  floor,
  onClose,
}: {
  job: JobSummary;
  diff: Diff;
  floor: boolean;
  onClose: () => void;
}) {
  const files = useMemo(() => railOf(diff, job.id), [diff, job.id]);
  return (
    <JobDiffSheet
      open
      floor={floor}
      branch={job.branch ?? job.id}
      files={files}
      note={WHICH_STEP_WROTE_IT}
      onClose={onClose}
    >
      <DecidedDiff diff={diff} jobId={job.id} />
    </JobDiffSheet>
  );
}

/** The reading, held where it is now. One spelling, used opening and jumping. */
export function holdOf(now: number, rows: number): HeldAt {
  return { at: clock(new Date(now).toISOString()), rows };
}

/**
 * The file rail beside the patch — the paths, and what each gained and lost.
 *
 * **From the patch, which is the answer the body is drawn from.** It used to
 * come from the footprint, and a footprint is a step's read-back written when
 * the step submits: mid-step nothing has submitted, so the rail was empty and
 * the header read `0 files · +0 −0` above a fully rendered patch. That is
 * #310, and it was two sources on one line rather than a hole to plug — filling
 * the rail from the patch and leaving the counts on the footprint would have
 * kept the contradiction one field along.
 *
 * `drawn` is the same split `DecidedDiff` renders, called on the same reading,
 * so the rail names exactly the files beside it in the order the patch wrote
 * them. It is a second call of one pure function rather than a second answer.
 *
 * **`null` is no reading and `[]` is a reading of nothing**, and the split
 * falls on the line the wire already draws. `work` absent is a Job with no
 * worktree; `work` present with no patch is a drone that changed nothing, which
 * is a real answer and truthfully reads `0 files · +0 −0`. Returning `[]` for
 * both would put a count of nothing over a Job nothing was read from, which is
 * this issue one state over.
 *
 * **No step against a file.** The drawing names the step that wrote each one
 * and nothing served says which step that was: the footprint carries
 * `planned_by`, which is the step that *promised* a path, and a file no step
 * declared would then read as a file no step wrote. The rail draws the counts
 * alone and says why underneath rather than guessing. Reported.
 */
function railOf(diff: Diff, jobId: string): JobDiffFile[] | null {
  // A reading of some other Job is not this Job's reading. `whyNoDiff` is the
  // sentence the body carries for each of these, and the header says only that
  // it has none.
  if (diff.state !== "read" || diff.jobId !== jobId || diff.work === undefined) return null;
  return railOfPatch(drawn(diff.work).files);
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
