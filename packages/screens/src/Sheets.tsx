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

import {
  ActivityLogSheet,
  JobDiffSheet,
  JobHoldsSheet,
  railOfPatch,
  type ActivityFilter,
  type JobDiffFile,
  type JobHoldsSheetProps,
} from "@armada/components";
import { useMemo, useState, type ReactNode } from "react";

import type { Diff, Observed } from "@armada/protocol";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { Calls } from "./calls";
import { DecidedDiff } from "./Decide";
import { clock } from "./duration";
import { Log } from "./Log";
import { recourseOf } from "./recovery";
import { drawn, WORKTREE_GIVEN_BACK } from "./review";
import { SettingsSheet, type SettingsSheetProps } from "./settings";
import { NOTHING_YET_ON_THIS_STEP, whyNotWatching, type LogRow } from "./story";

/**
 * Which sheet is open, or none. Two cannot be.
 *
 * **`holds` is the third, and it is here for a different reason than the other
 * two.** They left the panel because a reading has no end; this left the run
 * column because it was the largest thing on it and the run is what a person
 * opens a Job to read. Same layer, same two exits, same one-at-a-time rule.
 *
 * **`settings` is the fourth, and holds no reading at all** — every setting a
 * person can change on a running Job. It is here because the header's one line
 * for one of them read as the screen's main button, and a layer a person
 * already knows how to leave is where changing a Job does least to the reading.
 */
export type OpenSheet = "log" | "diff" | "holds" | "settings" | null;

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
  /**
   * Hold the reading where it is, or `null` to follow the tail again.
   *
   * **Null is what *Jump to now* means.** It used to hold again at the current
   * instant, which updates the timestamp and resets the count and leaves the
   * reading held — so the one control on the strip appeared to do nothing, and
   * the strip it belongs to stayed up saying the tail was not being followed.
   */
  onHold: (held: HeldAt | null) => void;
  /**
   * The full machine reading, exactly as the panel used to draw it — every
   * state and every argument, `Look now` included. Built by the caller, because
   * the arguments for how to read a `Holds` live in `resources.ts`.
   */
  holds: Omit<JobHoldsSheetProps, "open" | "floor" | "onClose">;
  /** What the Job settings panel reads and sends, beyond the Job it already has. */
  settings: Omit<SettingsSheetProps, "job" | "whole" | "floor" | "onClose">;
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
  holds,
  settings,
  floor,
  onClose,
}: DetailSheetProps) {
  // **Which actor's lines to show.** Held here rather than by the panel: it is
  // a reading of one sheet and it should start over the next time the sheet is
  // opened, which is what a state on the component that mounts with the sheet
  // gives. Above the early returns because a hook cannot be conditional; the
  // other sheets carry it and never read it.
  const [filter, setFilter] = useState<ActivityFilter>("all");
  // The four tabs were drawn from the component's own default and handed no
  // handler, so every one of them was a control that did nothing when pressed.
  // `LogActor` and `ActivityFilter` are the same three names plus `all`, so
  // selecting is the comparison and no mapping stands between them.
  const shown = useMemo(() => shownBy(rows, filter), [filter, rows]);

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
        filter={filter}
        onFilter={setFilter}
        heldAt={held?.at}
        arrived={held === null ? 0 : Math.max(rows.length - held.rows, 0)}
        onJumpToNow={() => onHold(null)}
        escalation={escalationOf(job, whole, step, onClose)}
        onClose={onClose}
      >
        {/* The sheet is the whole log, so a socket that stopped says so here
            for the reason the chapter's preview does: an empty sheet reading as
            a step that has not started is the panel's defect one layer out. */}
        <Log
          rows={shown}
          emptyNote={whyNotWatching(observed) ?? NOTHING_YET_ON_THIS_STEP}
          calls={calls}
          {...log}
        />
      </ActivityLogSheet>
    );
  }
  if (which === "diff") {
    return <DiffSheet job={job} whole={whole} diff={diff} floor={floor} onClose={onClose} />;
  }
  if (which === "holds") {
    return <JobHoldsSheet open floor={floor} onClose={onClose} {...holds} />;
  }
  if (which === "settings") {
    return <SettingsSheet job={job} whole={whole} floor={floor} onClose={onClose} {...settings} />;
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
  whole,
  diff,
  floor,
  onClose,
}: {
  job: JobSummary;
  /** Only for the footprint, which says whether there was a worktree to lose. */
  whole: JobWhole | null;
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
      // **Which silence this is**, where the reading can say. #381.
      whyNoReading={gaveBackTheWorktree(diff, job.id, whole) ? WORKTREE_GIVEN_BACK : undefined}
      // What the counts are counted against. Absent on a peer built before
      // 7.9, where `measured_whole` defaults to true and the header falls back
      // to the neutral phrase rather than claiming a base nothing named.
      measuredFrom={diff.state === "read" ? diff.work?.measured_from : undefined}
      measuredWhole={diff.state === "read" ? diff.work?.measured_whole : undefined}
      note={WHICH_STEP_WROTE_IT}
      onClose={onClose}
    >
      <DecidedDiff diff={diff} jobId={job.id} />
    </JobDiffSheet>
  );
}

/**
 * Whether the missing reading is a worktree that was given back.
 *
 * **`work: None` is two facts and only one of them may be named.** Fleet
 * answers it whenever `worktree_of` finds nothing, which covers a Job whose
 * worktree was reclaimed after it finished *and* a Job that never got one —
 * one that failed before a Drone was placed. Saying `the worktree was given
 * back` over the second is the same false certainty #381 was filed about,
 * pointed the other way, so the phrase needs evidence rather than a default.
 *
 * The footprint is that evidence. Fleet writes it from the worktree at the
 * instant the Job stopped, so a Job holding one had a worktree to read and no
 * longer has it. Absent, the header keeps the neutral phrase — which is
 * honest, because absent is exactly where Bridge does not know.
 */
export function gaveBackTheWorktree(diff: Diff, jobId: string, whole: JobWhole | null): boolean {
  if (diff.state !== "read" || diff.jobId !== jobId || diff.work !== undefined) return false;
  return whole?.footprint !== undefined;
}

/**
 * The lines one filter selects.
 *
 * **Its own function so the tabs can be checked without mounting a sheet.**
 * They were drawn from the component's own default and handed no handler at
 * all, so all four were controls that did nothing when pressed and nothing
 * said so — which is the shape of defect a rendered assertion catches late and
 * a function's return catches immediately.
 *
 * `LogActor` and `ActivityFilter` are the same three names plus `all`, so the
 * selection is the comparison and nothing maps between them.
 */
export function shownBy(rows: LogRow[], filter: ActivityFilter): LogRow[] {
  return filter === "all" ? rows : rows.filter((row) => row.actor === filter);
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
