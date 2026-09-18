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
  ConsoleOutput,
  EvidenceSheet,
  JobDiffSheet,
  JobHoldsSheet,
  PlanTaskSheet,
  type PlanTaskSheetProps,
  RunSheet,
  railOfPatch,
  type ActivityFilter,
  type JobDiffFile,
  type JobHoldsSheetProps,
  type RunSheetProps,
} from "@armada/components";
import { useEffect, useMemo, useState, type ReactNode } from "react";

import type {
  Diff,
  Observed,
  Turn,
} from "@armada/protocol";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { PlanTaskRow } from "./InsideAJob";
import type { Calls } from "./calls";
import { checkSheetOf } from "./checks";
import { DecidedDiff } from "./Decide";
import { DroneMessageControl } from "./DroneMessage";
import { clock } from "./duration";
import { WorkGrouped } from "./grouped";
import {
  liveNoteFor,
  liveRegionOf,
  liveRowsOf,
  noteFor,
  regionOf,
  rowsOf,
  type Following,
  type Outputs,
} from "./outputs";
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
 *
 * **`run` is the fifth, Journey 9's.** It opens from `r`, from the worktree
 * row's `Run…`, and from a refused Check's `Run it here` — never from a
 * chapter, so it lands nowhere on close, `holds`'s way.
 *
 * **`check` is the sixth**: one Check's output, from its row, closing onto the Checks chapter.
 */
export type OpenSheet = "log" | "diff" | "holds" | "settings" | "run" | "check" | "task" | null;

/**
 * Where the log's reading was held, and how much it had then.
 *
 * **Held once the reader scrolls away from the tail, not for the whole time
 * the sheet is open.** #1155. At the bottom the sheet follows, the way a chat
 * does; a stream that scrolls itself out from under someone who scrolled up
 * to read one of 1676 entries is the failure the hold exists to prevent, so it
 * takes over exactly there rather than from the moment the sheet opens. `rows`
 * is the count at the moment it was held, and the difference is what *Jump to
 * now* carries — scrolling back down does the same thing the button does.
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
  /**
   * Which run of the step the rows are, where they are one run's.
   *
   * **The header has to say so**, because a log opened from attempt 1 and one
   * opened from the step are the same layer with different contents, and
   * nothing else on it distinguishes a short run from a quiet step.
   */
  ofAttempt?: number;
  /**
   * The same turns those rows were folded from, which carry the tool and the
   * timing the rows no longer do. **The sheet is where the folding earns its
   * keep**: one real step put 1763 rows behind this layer.
   */
  turns: readonly Turn[];
  observed: Observed;
  diff: Diff;
  calls: Calls;
  /** What one log takes, by name, so the sheet's rows are not the preview's. */
  log: { region: string; openId: string | null; onOpen: (rowId: string | null) => void };
  held: HeldAt | null;
  /** Now, for `holdOf` — the clock a scroll-away hold is stamped with. #1155. */
  now: number;
  /**
   * Which Check the output sheet is open on — `which === "check"`'s own
   * reading. `checkSheetOf(step, checkId)` is what turns this into live or
   * kept; absent whenever `which` is not `"check"`.
   */
  checkId?: string;
  /**
   * What each Check on this Job has printed, where somebody opened one, and
   * how to ask for the rest. **Held for the Job**, because a recorded output
   * never moves — the sheet's own fetch, not the chapter's any more. #1021.
   */
  outputs: Outputs;
  /**
   * The running Check's log this window is following, and how to follow one.
   * **Started when the sheet mounts on a live Check and stopped when it
   * unmounts** — closing the sheet or switching Jobs both unmount it, which is
   * what lets go of the socket now that the chapter no longer does.
   */
  following: Following;
  /**
   * Hold the reading where it is, or `null` to follow the tail again.
   *
   * **Null is what *Jump to now* means.** It used to hold again at the current
   * instant, which updates the timestamp and resets the count and leaves the
   * reading held — so the one control on the strip appeared to do nothing, and
   * the strip it belongs to stayed up saying the tail was not being followed.
   */
  onHold: (held: HeldAt | null) => void;
  /** Sends a redirect, from the log sheet's own message box. #1154. */
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * The full machine reading, exactly as the panel used to draw it — every
   * state and every argument, `Look now` included. Built by the caller, because
   * the arguments for how to read a `Holds` live in `resources.ts`.
   */
  holds: Omit<JobHoldsSheetProps, "open" | "floor" | "onClose">;
  /**
   * Which task the task sheet is reading, where one is open.
   *
   * **Named rather than handed the task.** `SheetReading` carries the id the
   * way it carries a Check's, so the sheet reads the plan as it stands now —
   * a task marked done while its sheet is open says done.
   */
  taskId?: string;
  /** Every task of the plan, for the sheet to find `taskId` among. */
  planTasks?: readonly PlanTaskRow[];
  /**
   * What the open task declared against what it touched, already read.
   *
   * **Built by the caller**, like `holds` and `run`: the inputs are the Job's
   * own turns and its diff, which `JobDetail` holds and this layer does not.
   * **Absent draws no comparison at all** rather than one saying every
   * declared file went untouched. `#1432`.
   */
  taskTouched?: PlanTaskSheetProps["touched"];
  /** What the Job settings panel reads and sends, beyond the Job it already has. */
  settings: Omit<SettingsSheetProps, "job" | "whole" | "floor" | "onClose">;
  /**
   * The run sheet, Journey 9 — built by the caller from `RunSheetRead`,
   * `RunFollowed` and the run-sheet's own selection state, `holds`'s reason:
   * the arguments belong to `rehearsal.ts` and `JobDetail`, not to this file.
   */
  run: Omit<RunSheetProps, "open" | "floor" | "onClose">;
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
  ofAttempt,
  turns,
  observed,
  diff,
  calls,
  log,
  held,
  now,
  checkId,
  taskId,
  planTasks = [],
  taskTouched,
  outputs,
  following,
  onHold,
  onRedirect,
  holds,
  settings,
  run,
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
        step={ofAttempt === undefined ? step.label : `${step.label} · attempt ${ofAttempt}`}
        jobId={job.handle}
        total={rows.length}
        // **A closed run's log is not live**, and the hold strip has nothing to
        // jump to: both of those describe a tail, and this run has no tail.
        live={ofAttempt === undefined && observed.state === "watching"}
        // When the stream stopped. Nothing on the wire carries a Job's end, so
        // this is the open step's own `updated_at` — the instant the panel's
        // own `Took` is measured to, rather than a second reading of it.
        endedAt={clock(step.updated_at)}
        filter={filter}
        onFilter={setFilter}
        heldAt={held?.at}
        arrived={held === null ? 0 : Math.max(rows.length - held.rows, 0)}
        onJumpToNow={() => onHold(null)}
        // The scroll itself decides, not a row landing: at the bottom resumes
        // following exactly as *Jump to now* does; scrolled away holds exactly
        // where the reader put it, stamped now for the strip and the count. #1155.
        onFollowingChange={(following) => onHold(following ? null : holdOf(now, rows.length))}
        escalation={escalationOf(job, whole, step, onClose)}
        // Fixed under the stream in `Sheet`'s own footer slot, so it holds its
        // place while the body above it scrolls — #1154, and the reason #1155
        // has to land after it: "the tail" is the last row above this box, not
        // the row this box would otherwise sit on top of.
        footer={<DroneMessageControl job={job} whole={whole} onRedirect={onRedirect} />}
        onClose={onClose}
      >
        {/* The sheet is the whole log, so a socket that stopped says so here
            for the reason the chapter's preview does: an empty sheet reading as
            a step that has not started is the panel's defect one layer out. */}
        <WorkGrouped
          rows={shown}
          turns={turns}
          stepId={step.step_id}
          emptyNote={whyNotWatching(observed) ?? NOTHING_YET_ON_THIS_STEP}
          calls={calls}
          log={log}
        />
      </ActivityLogSheet>
    );
  }
  if (which === "diff") {
    return <DiffSheet job={job} whole={whole} diff={diff} floor={floor} onClose={onClose} />;
  }
  if (which === "check" && checkId !== undefined) {
    return (
      <CheckSheet
        job={job}
        step={step}
        checkId={checkId}
        outputs={outputs}
        following={following}
        floor={floor}
        onClose={onClose}
      />
    );
  }
  if (which === "task" && taskId !== undefined) {
    const task = planTasks.find((one) => one.id === taskId);
    // A task the plan no longer holds draws nothing rather than an empty
    // sheet: a recording replaces the plan whole, so an id can go.
    return task === undefined ? null : (
      <PlanTaskSheet
        open
        id={task.id}
        title={task.title}
        state={task.state}
        reason={task.reason}
        note={task.note}
        scope={task.scope}
        {...(taskTouched === undefined ? {} : { touched: taskTouched })}
        expects={task.expects}
        shown={task.shown}
        floor={floor}
        onClose={onClose}
      />
    );
  }
  if (which === "holds") {
    return <JobHoldsSheet open floor={floor} onClose={onClose} {...holds} />;
  }
  if (which === "settings") {
    return <SettingsSheet job={job} whole={whole} floor={floor} onClose={onClose} {...settings} />;
  }
  if (which === "run") {
    return <RunSheet open floor={floor} onClose={onClose} {...run} />;
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
      branch={job.branch ?? job.handle}
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
 * One Check's output, on the layer that can hold it — #1021.
 *
 * **`EvidenceSheet`, wired in for the first time.** It existed as a component
 * and a story — `AChecksConsoleOutput` — and neither was built into the screen
 * that ships, which is exactly the gap `evidence.tsx`'s own header names for
 * the Checks and Verdicts chapters. A check's console output is the artifact
 * that sheet was drawn for.
 *
 * **`checkSheetOf` decides live or kept, every render.** A Check open in this
 * sheet while the gate rules moves from one to the other without the sheet
 * closing — the same Check, a different file, which is why this asks fresh
 * rather than fixing the answer at open.
 */
function CheckSheet({
  job,
  step,
  checkId,
  outputs,
  following,
  floor,
  onClose,
}: {
  job: JobSummary;
  step: StepDetail;
  checkId: string;
  outputs: Outputs;
  following: Following;
  floor: boolean;
  onClose: () => void;
}) {
  const read = checkSheetOf(step, checkId);
  return (
    <EvidenceSheet
      open
      floor={floor}
      kind="Console output"
      name={`${checkId} — output`}
      step={step.label}
      jobId={job.handle}
      onClose={onClose}
    >
      {read === undefined ? (
        <ConsoleOutput rows={[]} emptyNote={NOTHING_TO_READ} />
      ) : read.kind === "live" ? (
        <LiveCheckOutput kept={read.kept} following={following} />
      ) : (
        <KeptCheckOutput kept={read.kept} outputs={outputs} />
      )}
    </EvidenceSheet>
  );
}

/**
 * The kept file, read where the Check is. **The fetch is the open, and it
 * happens once** — `outputs.fetch` drops a second ask for a name it already
 * holds, so mounting this is what asks for the file rather than a press
 * inside it.
 */
function KeptCheckOutput({ kept, outputs }: { kept: string; outputs: Outputs }) {
  useEffect(() => outputs.fetch(kept), [outputs, kept]);
  const held = outputs.of(kept);
  const output = held?.state === "got" ? held.output : undefined;
  return (
    <ConsoleOutput
      rows={output === undefined ? [] : rowsOf(output)}
      {...(output === undefined ? {} : { region: regionOf(output) })}
      emptyNote={noteFor(held)}
    />
  );
}

/**
 * One running Check's log, followed as it is written. **Followed while the
 * sheet is on screen and let go the moment it is not** — mounting and
 * unmounting this is the whole of starting and stopping the socket, so
 * closing the sheet or switching Jobs both end it.
 */
function LiveCheckOutput({ kept, following }: { kept: string; following: Following }) {
  const { follow } = following;
  useEffect(() => {
    follow(kept);
    return () => follow(null);
  }, [follow, kept]);
  const region = liveRegionOf(following.reading, kept);
  return (
    <ConsoleOutput
      rows={liveRowsOf(following.reading, kept)}
      {...(region === undefined ? {} : { region })}
      emptyNote={liveNoteFor(following.reading, kept)}
    />
  );
}

/** A Check named for the sheet that no longer has anything behind it. */
const NOTHING_TO_READ = "This Check has nothing recorded to read.";

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
 * Which sheet is up, and what it is reading — one value.
 *
 * **Three pieces that only ever change together.** The log's attempt and where
 * its reading was held mean nothing without the log up, and held as three
 * pieces of state a sheet could close and leave either behind for the next
 * sheet to inherit.
 */
export type SheetReading =
  | { which: null }
  | { which: "log"; attempt?: number; held: HeldAt | null }
  | { which: "check"; checkId: string }
  | { which: "task"; taskId: string }
  | { which: Exclude<OpenSheet, "log" | "check" | "task" | null> };

/** What can happen to it: a sheet goes up, comes down, or the log is held or let go. */
export type SheetMove =
  | { move: "open"; which: "log"; attempt?: number; held?: HeldAt }
  | { move: "open"; which: "check"; checkId: string }
  | { move: "open"; which: "task"; taskId: string }
  | { move: "open"; which: Exclude<OpenSheet, "log" | "check" | "task" | null> }
  | { move: "close" }
  | { move: "hold"; held: HeldAt | null };

export const NO_SHEET: SheetReading = { which: null };

/** The next reading. A second sheet replaces the first; holding means nothing off the log. */
export function sheetMoved(was: SheetReading, move: SheetMove): SheetReading {
  if (move.move === "close") return NO_SHEET;
  if (move.move === "hold") return was.which === "log" ? { ...was, held: move.held } : was;
  if (move.which === "check") return { which: "check", checkId: move.checkId };
  if (move.which === "task") return { which: "task", taskId: move.taskId };
  if (move.which !== "log") return { which: move.which };
  return {
    which: "log",
    ...(move.attempt === undefined ? {} : { attempt: move.attempt }),
    held: move.held ?? null,
  };
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
