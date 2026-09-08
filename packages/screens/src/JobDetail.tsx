// One Job, read whole — and one arrangement, whatever the Job is doing.
//
// # What replaced four renders
//
// This file used to choose between four screens: a running Job led with a rail,
// a Job at review led with a diff, a finished one led with what it produced and
// a stopped one led with what stopped it. Below the shared header no region sat
// in the same place twice, so a person who learned where something was on one
// Job could not find it on the next. There is one arrangement now — the run as
// a tree on the left, the selected step in the panel, its story in the order it
// happened — and what a status changes is which chapter is the reason you are
// here and what the panel offers you to do about it.
//
// `render.ts` is still what says which state a Job is in. What it no longer
// does is pick a screen.
//
// # Acts are split by what they act on
//
// Four of the eight acted on a step and were drawn in the Job header. Redirect,
// restart step, override the verdict and re-run the gate are in the panel
// header now, beside the step they change, and the accent goes with them. Kill,
// redispatch and approve stay in the Job header, which is also where Pilot
// lands — `#250`, and nothing here has to change to take it.
//
// # Two chapters leave the panel
//
// The activity log holds 1676 entries on a real Job and the diff is the Job's
// whole patch. Neither is a longer version of something a chapter can hold — an
// expander pushes everything under it off the screen and gives a patch a 602px
// column — so both open as a trailing sheet instead, and the panel stays
// exactly as it was underneath. #286, and Journey 4's frames 4i-4m.
//
// **One sheet at a time.** Opening the diff while the log is open replaces it,
// and `Esc` returns to the panel rather than to the previous sheet: a layer
// that pops back to another layer makes one key mean two depths of *back*.
//
// **Two exits and no third.** The labelled control and `Esc`. A click on the
// screen behind does not close a sheet — a 1676-entry read must not be
// dismissed by a stray click, and `Sheet` is where that is held.
//
// # The story is three chapters and none of them is behind a tab
//
// Drone instructions, then Activity log, then Produced, in the order they
// happened. The log streams while the Job runs and is on the page at every
// state — it used to be one of four tabs inside a region called *What it left
// behind*, which the drawing has none of, so the chapter that says what is
// happening right now was the one thing a person had to go and find. That
// region is gone: the turns are chapter two, the files are chapter three, and
// the raw event table is not something this screen needs at all.
//
// # What this file keeps, and where the next thing goes
//
// It holds the open state of one reading — which step, which sheet, where the
// log was held, whether the report dialog is up — and it arranges the regions.
// **What each region says, it no longer decides.** The Job header is
// `heading.tsx`, the step's facts, band and question box are `step.tsx`, the
// story is `chapters.tsx`, the two sheets are `Sheets.tsx`, and which reading
// is this Job's is `mine.ts`.
//
// That is the seam, and it is the one this file was missing: it was cut twice
// and grew back both times, because it was the only place an addition to a
// region could be written. The pull request link and Pilot's slot are the two
// most recent, and both are the header's — they go in `heading.tsx` now.

import { JobHoldsSummary } from "@armada/components";
import { useEffect, useMemo, useState } from "react";
import { InsideAJob, type RunTreeStep } from "@armada/components";

import type {
  Diff,
  Evidence,
  Examination,
  Footprint,
  Holds,
  Journalled,
  Observed,
  Outcome,
  Watched,
} from "@armada/protocol";
import type { FileReport, JobSummary } from "@armada/protocol";
import type { ManifestSummary, WorkflowSummary } from "@armada/protocol";
import type { ConfirmableAct } from "./Acts";
import { useCallArguments, type ReadCall } from "./calls";
import type { OpenArtifact, OpenPullRequest } from "./opening";
import { DIFF_CHAPTER, LOG_CHAPTER, namesStep, useDetailKeys } from "./detail-keys";
import { useAtFloor } from "@armada/shell";
import { DetailSheet, holdOf, type HeldAt, type OpenSheet } from "./Sheets";
import { chaptersOf } from "./chapters";
import { span } from "./duration";
import { Decide } from "./Decide";
import { ordered } from "./facts";
import { headingOf, Unrenderable } from "./heading";
import { Log } from "./Log";
import { detailOf, holdingOf, logOf, lookOf, turnsOf } from "./mine";
import { phasesOf } from "./phases";
import { renderFor } from "./render";
import { runOf } from "./run";
import { askingOf, fieldsOf, noticeOf, questionOf } from "./step";
import { StepActs } from "./StepActs";
import { NOTHING_FROM_FLEET_YET, notesOf, whyNoNotes } from "./notes";
import { entriesOf, whyNotWatching } from "./story";
import { LOOK_FAILED, nothingToAsk, summarised, tailOf, whyNoReading } from "./resources";
import { briefOf, whyNoWork, workOf } from "./work";

export type { ConfirmableAct, JobAct } from "./Acts";
export { renderFor } from "./render";
export type { Render } from "./render";

export type JobDetailProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id` for this Job, as main published it. */
  watched: Watched;
  workflows: readonly WorkflowSummary[];
  manifests: readonly ManifestSummary[];
  /** True while what is shown is not live. Every control is refused. */
  stale: boolean;
  /** Now, injected. A whole-Job elapsed is read, so it has to move. */
  now: number;
  /** In flight. A second press does not send a second command. */
  acting: boolean;
  /** An approval already sent for this Job. */
  approving: boolean;
  /** A decision on this Job's work already in flight. */
  deciding: boolean;
  /** Ask for a confirmation. Nothing destructive is one press from here. */
  onAct: (act: ConfirmableAct, jobId: string) => void;
  /** Send a redirect straight through — its own dialog is the confirmation. */
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Answer the question this Job's drone asked, by the label picked.
   *
   * **Straight through, with no confirmation.** Picking an option and pressing
   * send are already two deliberate acts on a closed set the drone chose, and a
   * dialog on top would be a third press for the ordinary path.
   */
  onAnswer: (jobId: string, questionId: string, chose: string) => void;
  /** Overrule a Judge that refused the work, with the reason. */
  onOverrule: (jobId: string, reason: string) => void;
  /** Ask the gate again on a step it could not decide. Nothing is at stake. */
  onRerun: (jobId: string) => void;
  /**
   * Which Job's diff the host should hold open, or `null` for none.
   *
   * **It has to be stable**: an effect depends on it, and a lambda rebuilt on
   * every tick of the clock would reopen the read on every tick with it.
   */
  onReadDiff: (jobId: string | null) => void;
  /**
   * The host calls this screen makes on a person's behalf, handed in.
   *
   * **Every one of them is a round trip to a process with a filesystem or a
   * socket.** This screen decides what they mean and when to make them; it
   * does not reach for the thing that makes them, because a screen that did
   * could not be rendered anywhere but inside the app.
   */
  onOpenArtifact: OpenArtifact;
  /**
   * Open the pull request this Job opened, in whatever browses the web here.
   *
   * **Beside `onOpenArtifact` and not folded into it.** That one takes a word
   * from a closed set and lands a file in an editor; this takes no word at all
   * and leaves the app. One prop taking which would read as one act and perform
   * two — and the two fail at different things, which is why `Followed` is not
   * `Opened`.
   */
  onOpenPullRequest: OpenPullRequest;
  onReadCall: ReadCall;
  /** Stable, like `onReadDiff` — an effect in the decision block depends on it. */
  onNeedMaterial: (jobId: string | null) => void;
  /** Say this job failed in error, with the record attached. */
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  /** Let this Job run. Sent on the press, with no confirmation. */
  onApprove: (jobId: string) => void;
  /** The three answers to a Job at `awaiting_review`. Three props, not one. */
  onApproveReview: (jobId: string) => void;
  onRequestChanges: (jobId: string, note: string) => void;
  onReject: (jobId: string) => void;
  /**
   * What the second socket has said. **Opened for every Job that is open**, not
   * on a press: the activity log is a chapter of the step's story and a chapter
   * that filled only after somebody asked is the tab this screen removed.
   */
  observed: Observed;
  /**
   * What the Job's own log has said. **The third voice, and the one thing there
   * is to read before a Drone exists** — `observed` is a Drone's transcript and
   * is empty for the whole of preparation, which is precisely when somebody
   * opens this screen to find out what is going on.
   */
  journalled: Journalled;
  /**
   * What the open Job holds on this machine. **Opened with the Job**, like the
   * two sockets above: the panel it draws answers *is this working*, which is
   * the question somebody opening a Job they suspect has wedged came with.
   */
  resources: Holds;
  /**
   * What the last look found, where somebody pressed for one. **Never opened
   * with the Job** — an answer that appeared unasked would be the machine
   * noticing on its own, which is a different capability.
   */
  examination: Examination;
  /** Ask Fleet to go and look at this Job now. It costs no model call. */
  onExamine: (jobId: string) => void;
  /** The reads the panel's chapters draw from. */
  recorded: FoldedReads;
  onCopied: (value: string) => void;
  /**
   * Say a sentence to the person. **Only ever a failure**, today — an open that
   * did nothing is the defect `#246` is about, and success is the file being in
   * front of them.
   */
  onSaid: (sentence: string) => void;
};

export function JobDetail({
  onReadDiff,
  onOpenArtifact,
  onOpenPullRequest,
  onReadCall,
  onNeedMaterial,
  job,
  watched,
  workflows,
  manifests,
  stale,
  now,
  acting,
  approving,
  deciding,
  observed,
  journalled,
  resources,
  examination,
  onExamine,
  recorded,
  onAct,
  onRedirect,
  onAnswer,
  onOverrule,
  onRerun,
  onReport,
  onApprove,
  onApproveReview,
  onRequestChanges,
  onReject,
  onCopied,
  onSaid,
}: JobDetailProps) {
  // Which step the panel is showing. **The whole of navigation inside a Job**:
  // `null` means the one Fleet says is current, so a Job that moves on carries
  // the reader with it until they choose a step themselves.
  const [selected, setSelected] = useState<string | null>(null);
  // A selection belongs to the Job it was made in. Carried into the next Job it
  // would name a step that Job may not have.
  useEffect(() => setSelected(null), [job.id]);

  // Which sheet is open, or none. **One value rather than two booleans**: the
  // two cannot both be open, and a pair of flags is a state that says they can.
  // Dropped with the Job, as every reading of one Job is.
  const [sheet, setSheet] = useState<OpenSheet>(null);
  useEffect(() => setSheet(null), [job.id]);

  // Where the log's reading was held, and what has arrived since. **The tail is
  // not followed while a sheet is open**: a stream that scrolls itself cannot
  // be read. `held` is how many rows the step had when the reading was taken,
  // and `Jump to now` takes it again.
  const [held, setHeld] = useState<HeldAt | null>(null);

  // Whether the report dialog is up. **Here rather than in `Acts`**, because
  // two controls open it — the Job header's menu entry and `b` — and the
  // keyboard is bound at this level. Dropped with the Job for the reason a
  // selection is: a half-written report is about the Job it was written on.
  const [reporting, setReporting] = useState(false);
  useEffect(() => setReporting(false), [job.id]);

  // The diff, for every Job that is open rather than only for one at review.
  // **A produced file opens to what it actually wrote**, and it did that on one
  // status because the review block was the only thing asking for the read. It
  // is still the expensive read and it is still made once, here, so the Produced
  // chapter and the review decision draw from one answer.
  useEffect(() => {
    onReadDiff(job.id);
    return () => {
      onReadDiff(null);
    };
  }, [job.id]);

  // Whether the window is at `--window-floor`. Read from the token rather than
  // from a media query, which cannot see one — `floor.ts` carries the whole of
  // why, and the measurement behind it.
  const floor = useAtFloor();

  const render = renderFor(job);
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);
  // Every reading, narrowed to the Job on screen. **All five carry the id they
  // were taken for and all five are checked against it** — each lags a
  // selection by a round trip, so another Job's answer landing here would be
  // that Job's steps under this Job's title, its turns under this Job's step,
  // its disk under this Job's panel. `mine.ts` is the one place the check is
  // written, and the counterpart to `main/reader.ts` on this side of the seam.
  const whole = detailOf(watched, job.id);
  const watching = turnsOf(observed, job.id);
  const noted = logOf(journalled, job.id);
  const holding = holdingOf(resources, job.id);
  const looked = lookOf(examination, job.id);
  // The finding itself, or none. Read twice — the summary asks whether an
  // absence is a fault, the sheet draws every look — so it is named once.
  const examinedNow = looked?.state === "found" ? looked.examined : null;

  const steps = whole === null ? [] : ordered(whole);
  const open = steps.find((step) => step.step_id === (selected ?? job.current_step_id)) ?? steps[0];
  // What the observe socket says about itself, where it is not reading. **A
  // third answer the story needs**: four of the five states carry no rows, and
  // a chapter drawn from the rows alone reads every one of them as a step that
  // has not started. `story.ts` holds the sentences. #324.
  const transcript = whyNotWatching(observed);
  const fleetSaid = useMemo(() => notesOf(noted?.notes ?? []), [noted]);

  // What the keyboard can name, built before it is drawn. **The three regions
  // the contextual tier reaches are values here rather than queries later** —
  // the run, the story and the strip — which is what lets `detail-keys` open a
  // step, a chapter or a stage by name. #271.
  const run = whole === null ? [] : runOf(whole, now, selected ?? undefined, watching?.rows ?? []);
  // The strip's rows carry the three records a person reads because a verdict
  // went against them, and each opens. The Job id and the toast are the panel's,
  // so they are handed down rather than reached for; `phases.tsx` says why.
  const opensRecords = useMemo(
    () => ({ jobId: job.id, open: onOpenArtifact, onSaid }),
    [job.id, onSaid],
  );
  const phases =
    whole === null || open === undefined
      ? undefined
      : phasesOf(open, whole.acceptance_criteria, opensRecords, job.status);

  // The detail's contextual tier, and the open state it moves. Bound while a
  // Job is open and not before, so nothing on the Board listens for a key that
  // means nothing there — and the press is swallowed only where something
  // answered it. The story is read back through a function because it is built
  // from what this holds; see `DetailShape.chapters`.
  const keys = useDetailKeys({
    run,
    chapters: () => chapters,
    stages: phases?.stages,
    // `f`, from `actions.toml` — `open_diff`, scope `detail`. It opens the
    // layer now rather than a chapter: the patch stopped being something the
    // panel draws. `Enter` needs nothing here, because `[` `]` land focus on
    // the chapter's own control and Enter is what a focused control already
    // answers — which is the reading `open_log`'s registry row gives it.
    onOpenSheet: () => openSheet("diff"),
    // `b`, on the one render that offers the act. Elsewhere the shape carries
    // nothing and the press is left alone rather than answered with a dialog
    // the header is not offering.
    ...(render === "stopped" && !stale ? { onReport: () => setReporting(true) } : {}),
  });

  // The rest of any call argument the socket cut, for as long as this Job is
  // open. **Held for the Job rather than for a log**, because the story draws
  // the same row twice — chapter one's turns and chapter two's preview — and a
  // fetch made in one is the same argument in the other.
  const calls = useCallArguments(onReadCall, job.id);

  const rows = watching === null || open === undefined ? [] : entriesOf(watching.rows, open.step_id);

  /**
   * Open a sheet. **The second one replaces the first** rather than stacking on
   * it, and opening the log takes the reading's position: from here on the tail
   * is not followed, and what arrives is counted rather than scrolled to.
   */
  function openSheet(which: "log" | "diff" | "holds"): void {
    setSheet(which);
    if (which === "log") setHeld(holdOf(now, rows.length));
  }

  /**
   * Close it, and put focus back where it came from. **The chapter line is the
   * way back** — `4k`'s third still — so `[` `]` carry on from the chapter the
   * reader opened rather than from the top of the story.
   *
   * **The holdings sheet lands nowhere, because it came from nowhere in the
   * story.** It opens from the run column rather than from a chapter, and
   * putting a reader who closed it onto a chapter they never opened would move
   * them further than `Esc` promised. Its control is the natural landing and
   * the summary is not part of the keyboard's chapter line. Reported.
   */
  function closeSheet(): void {
    const was = sheet;
    setSheet(null);
    setHeld(null);
    if (was === "log" || was === "diff") {
      keys.onFocusChapter(was === "log" ? LOG_CHAPTER : DIFF_CHAPTER);
    }
  }

  const chapters =
    open === undefined
      ? []
      : chaptersOf({
          job,
          step: open,
          render,
          watching,
          footprint: recorded.footprint,
          kept: whole?.footprint,
          diff: recorded.diff,
          live: observed.state === "watching",
          transcript,
          log: keys.inLog,
          calls,
          sheet,
          // The Produced chapter opens the step's deliverable, which the phase
          // strip's Submitted tier was the only route to. Same handler, because
          // two would be two vocabularies for one failed open — #307.
          opens: opensRecords,
          onOpenSheet: openSheet,
        });

  // The Job header, and everything that goes in it. `heading.tsx` holds what
  // it is made of — the badge, the facts, the acts that end or replace the Job,
  // and the way out to the pull request — which is where the next thing added
  // to this header goes rather than here.
  const heading = headingOf({
    job,
    whole,
    workflow,
    now,
    render,
    stale,
    acting,
    approving,
    reporting,
    onReporting: setReporting,
    onAct,
    onApprove,
    onReport,
    onOpenPullRequest,
    onCopied,
    onSaid,
  });

  // The badge is the header, so a Job the registry has no glyph or verb for
  // cannot be drawn at all — which is the `null` above. Named rather than
  // half-drawn.
  if (heading === null || render === "unrenderable") {
    return <Unrenderable job={job} />;
  }

  return (
    <InsideAJob
      heading={heading}
      run={run.map(named)}
      runElapsed={span(job.created_at, now) ?? undefined}
      runAbsent={whyNoSteps(watched, job.id)}
      // What it holds on this machine, below the run and above the pointers.
      // **Five lines, and the reading a press away.** It answers *is this
      // working*, which is what a person suspecting a wedged Job came with —
      // but the run is what they opened the Job to read, so the reading is on
      // the sheet and what stays here is what changes the answer.
      //
      // **Fleet's last lines are the tail of it.** They had a region of their
      // own above the run — #437 — and both regions answered *what is happening
      // on this machine right now*, which is one region too many. Drawn at
      // every state, not only while a Job is preparing: the lines that belong
      // to no step are also the ones a reader wants after it stopped.
      machine={
        <JobHoldsSummary
          tail={tailOf(fleetSaid)}
          tailNote={whyNoNotes(journalled) ?? NOTHING_FROM_FLEET_YET}
          figures={summarised(holding, examinedNow)}
          note={whyNoReading(resources)}
          age={holding === null ? undefined : (span(holding.read_at, now) ?? undefined)}
          onOpen={() => openSheet("holds")}
        />
      }
      // One animated mark per screen, on the thing being read — and nothing
      // pulses on a Job that is over, where "still working" is a claim no step
      // is making. **The pulse moves with the reading**: with a sheet open the
      // tree's current step is behind the layer, so its mark stops and the
      // sheet's live mark takes it.
      pulsing={render === "working" && sheet === null}
      onSelectStep={setSelected}
      // The tree draws exactly what the keyboard holds. **Selecting a step
      // still does not open its facts** — that is `RunTree`'s rule and it is
      // the reason the two are separate props at all.
      openSteps={keys.openSteps}
      onOpenStep={keys.onOpenStep}
      where={workOf(onOpenArtifact, job, whole, manifest, workflow)}
      whereNote={NAMED_NOT_NEEDED}
      whereAbsent={whyNoWork(watched, job.id)}
      brief={whole === null ? undefined : briefOf(whole)}
      briefAbsent={whyNoBrief(watched, job.id)}
      step={
        open === undefined
          ? undefined
          : {
              label: open.label,
              labelIsAnIdentifier: open.label === open.step_id || undefined,
              fields: fieldsOf(open, now),
              acts: (
                <StepActs
                  job={job}
                  whole={whole}
                  render={render}
                  acting={acting}
                  stale={stale}
                  onAct={onAct}
                  onRedirect={onRedirect}
                  onOverrule={onOverrule}
                  onRerun={onRerun}
                />
              ),
              // A question outranks the render's own notice: nothing else on
              // this step is what a person is here for while one is open, and
              // the two would otherwise both claim the band.
              notice: askingOf(whole) ?? noticeOf(job, whole, render, open, opensRecords),
              // **The question sits where the redirect box does** — between the
              // strip and the story, because it is the same kind of thing: a
              // box a person acts in about the step they are looking at.
              before: questionOf(whole, job.id, now, stale, acting, onAnswer),
              // The strip draws the stage the keyboard pinned, and hover stays
              // its own: hovering reports where the pointer is rather than what
              // a reader decided, so nothing up here holds it.
              phases:
                phases === undefined
                  ? undefined
                  : { ...phases, pinnedStage: keys.pinnedStage, onPin: keys.onPinStage },
              phasesAbsent: whyNoSteps(watched, job.id),
              chapters,
              openChapterId: keys.openChapterId,
              onOpenChapter: keys.onOpenChapter,
              // Review and reply are one loop: the decision is the block under
              // the story, one scroll from the diff it is made against, never a
              // second surface and never a second panel.
              after:
                render === "reviewing" ? (
                  <Decide
                    job={job}
                    onNeedMaterial={onNeedMaterial}
                    evidence={recorded.evidence}
                    diff={recorded.diff}
                    stale={stale}
                    deciding={deciding}
                    onApprove={onApproveReview}
                    onRequestChanges={onRequestChanges}
                    onReject={onReject}
                  />
                ) : undefined,
            }
      }
      stepAbsent={whyNoSteps(watched, job.id)}
      sheet={
        open === undefined ? null : (
          <DetailSheet
            which={sheet}
            job={job}
            whole={whole}
            step={open}
            rows={rows}
            observed={observed}
            diff={recorded.diff}
            calls={calls}
            // Its own name, so a row opened in the sheet is not a row opened in
            // the chapter's preview. Two logs over one stream hold equal ids.
            log={keys.inLog("sheet")}
            held={held}
            onHold={setHeld}
            // The full reading, unchanged from what the run column used to
            // draw — `Look now` came with it, because it acts on this reading
            // and not on the five lines that open it.
            holds={{
              jobId: job.id,
              reading: holding,
              note: whyNoReading(resources),
              age: holding === null ? undefined : (span(holding.read_at, now) ?? undefined),
              examined: examinedNow,
              looking: looked?.state === "looking",
              lookFailed: looked?.state === "failed" ? LOOK_FAILED : undefined,
              nothingToAsk: nothingToAsk(resources),
              onExamine: () => onExamine(job.id),
            }}
            now={now}
            floor={floor}
            onClose={closeSheet}
          />
        )
      }
      onCopied={onCopied}
    />
  );
}

/**
 * A step of the run, with its name marked so the keyboard can find the control
 * the name is drawn in.
 *
 * **The marker draws nothing.** It is `display: contents`, so the row lays out
 * exactly as it did with a bare string — which matters on this row, where the
 * name is the only column that flexes and the ellipsis it truncates with is the
 * whole reason the duration column never moves.
 *
 * It is here rather than in `run.ts` because that file builds data and this one
 * builds elements, and it is here at all because `j`/`k` move focus: focus is
 * the only cursor the tree can draw, so the keyboard has to be able to reach
 * the control. Everything else it does to the run goes through `openSteps`.
 */
function named(step: RunTreeStep): RunTreeStep {
  return {
    ...step,
    label: (
      <span className="contents" {...namesStep(step.id)}>
        {step.label}
      </span>
    ),
  };
}

/** What the pointers under the run are for, said once. */
const NAMED_NOT_NEEDED =
  "A path opens where it lives; an identifier copies. Nothing above needs these — they are here " +
  "for when you want them anyway.";

/** The reads the panel's own chapters draw from. */
export type FoldedReads = {
  footprint: Footprint;
  evidence: Evidence;
  diff: Diff;
};

/** Why the run has no rows, which is never the same sentence twice. */
function whyNoSteps(watched: Watched, jobId: string): string | undefined {
  if (watched.state === "read" && watched.jobId === jobId) {
    return watched.detail.steps.length === 0
      ? "This Job's frozen workflow has no steps."
      : undefined;
  }
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this Job, so its steps are unknown.";
  }
  return "Reading this Job.";
}

/**
 * Why there is no brief. **Two sentences, and neither describes the wire** —
 * one is a Job that has not arrived and one is a Job Fleet would not answer
 * for, which are different things to do next.
 */
function whyNoBrief(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this job.";
  }
  return "Reading this job.";
}
