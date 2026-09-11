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

import { Button, JobHoldsSummary } from "@armada/components";
import { ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { type RunTreeStep } from "@armada/components";
import { InsideAJob } from "./InsideAJob";

import type {
  Diff,
  Evidence,
  Examination,
  Footprint,
  History,
  Holds,
  FollowedLog,
  Journalled,
  Observed,
  Outcome,
  Remarks,
  Watched,
} from "@armada/protocol";
import type { CommandAnswer, FileReport, JobSummary, WhenBlocked } from "@armada/protocol";
import type { ManifestSummary, WorkflowSummary } from "@armada/protocol";
import { heldForMoney, heldForTurns, type ConfirmableAct } from "./Acts";
import { useCallArguments, type ReadCall } from "./calls";
import {
  useCheckOutputs,
  useFollowing,
  type FollowCheckOutput,
  type ReadCheckOutput,
} from "./outputs";

/** What `followed` reads as where the caller hands none in. */
const NOT_FOLLOWING: FollowedLog = { state: "none" };
import { useFrames, type ReadFrame } from "./frames";
import { openArtifact } from "./opening";
import type { OpenArtifact, OpenPullRequest } from "./opening";
import { DIFF_CHAPTER, LOG_CHAPTER, namesStep, useDetailKeys } from "./detail-keys";
import { useAtFloor } from "@armada/shell";
import { DetailSheet, holdOf, type HeldAt, type OpenSheet } from "./Sheets";
import { chaptersOf } from "./chapters";
import { againOf, useShowAgain, type ShowAgainCall } from "./again";
import { span } from "./duration";
import { ordered } from "./facts";
import { headingOf, Unrenderable } from "./heading";
import { detailOf, holdingOf, logOf, lookOf, turnsOf } from "./mine";
import { phasesOf } from "./phases";
import { neverAsksAPerson, verdictSlotAtGate, verdictSlotFinished } from "./verdict";
// Which Check's output `o` opens. **The same call the Checks chapter's own act
// makes**, so the key and the control cannot open different files.
import { outputOf } from "./gates";
import { renderFor } from "./render";
import { runOf } from "./run";
import { askingOf, fieldsOf, noticeOf, questionOf } from "./step";
import { StepActs } from "./StepActs";
import { whyNoNotes } from "./notes";
import { entriesOf, hideUnread, whyNotWatching } from "./story";
import { LOOK_FAILED, NOTHING_HAPPENED_YET, latestOf, movesOf, nothingToAsk, summarised, whyNoReading } from "./resources";
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
  /**
   * Allow or reject a command the drone was not given, by its call id. The
   * same call answers a command a drone is waiting on and a refused row on a
   * stopped job. Straight through, for `onAnswer`'s reason.
   */
  onAnswerCommand: (jobId: string, call: string, answer: CommandAnswer) => void;
  /** How this job meets the next such command. Live; nothing restarts. */
  onSetWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => void;
  /** Overrule a Judge that refused the work, with the reason. */
  onOverrule: (jobId: string, reason: string) => void;
  /**
   * Give this job a higher cost ceiling, in millionths of a dollar. **The one
   * handler here that moves no part of the job** — it stops the next dispatch
   * being refused for money, and the job stays exactly where it was.
   */
  onRaiseCap: (jobId: string, costCapMicros: number) => void;
  /**
   * Let this job take more turns. **The other ceiling `over_budget` folds**,
   * and it moves no part of the job either — it stops the next dispatch being
   * refused for turns. A plain turn count: the cost cap converts and this one
   * does not.
   */
  onRaiseTurnCap: (jobId: string, turnCap: number) => void;
  /** Ask the gate again on a step it could not decide. Nothing is at stake. */
  onRerun: (jobId: string) => void;
  /**
   * Ask the Job to show its work again. **Answered to this screen**, like
   * `onReport`, because what a press came to is said beside its control.
   * Absent draws no control, only the sets earlier presses kept.
   */
  onShowAgain?: ShowAgainCall;
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
  /**
   * Read one Check's own output. **`onReadCall`'s shape one record over**, and
   * its own prop for the same reason: it is a round trip to the process that
   * has the file, and the screen does the reading rather than the fetching.
   */
  onReadCheckOutput: ReadCheckOutput;
  /**
   * Read one frame a step's harness produced. **`onReadCheckOutput`'s shape one
   * record over** — the bytes come from the process that can reach Fleet, and
   * the screen is handed the way to ask rather than the way to connect.
   */
  onReadFrame: ReadFrame;
  /** Stable, like `onReadDiff` — an effect in the decision block depends on it. */
  onNeedMaterial: (jobId: string | null) => void;
  /**
   * Open or close the read of the pull request's comments. Stable for
   * `onNeedMaterial`'s reason, and its own prop because it is its own read: one
   * reaches a record and this one reaches a forge.
   */
  onNeedRemarks: (jobId: string | null) => void;
  /** Say this job failed in error, with the record attached. */
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  /** Let this Job run. Sent on the press, with no confirmation. */
  onApprove: (jobId: string) => void;
  /**
   * The four answers to a Job at `awaiting_review`. Four props, not one — each
   * does something different to the Job, and one prop taking which would read
   * as one act and perform four.
   *
   * `onMergePullRequest` is drawn only where the Job's record holds a pull
   * request, which `Decide` decides from the detail rather than from a flag.
   */
  onMergePullRequest: (jobId: string) => void;
  onApproveReview: (jobId: string) => void;
  onRequestChanges: (jobId: string, note: string) => void;
  onReject: (jobId: string) => void;
  /**
   * The fifth answer at the same gate: the comments a person picked off the
   * pull request reach a Drone. **Handles and never words** — Fleet reads the
   * pull request again on the press, so nothing here decides what a Drone is
   * told.
   */
  onTakeUpRemarks: (jobId: string, remarks: string[]) => void;
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
   * The running Check's log main is following, as it is written. Optional,
   * because a surface drawing a Job with no gate running has nothing to follow.
   */
  followed?: FollowedLog;
  /** Follow one running Check's log, or `null` to stop. */
  onFollowCheckOutput?: FollowCheckOutput;
  /**
   * What the open Job holds on this machine. **Opened with the Job**, like the
   * two sockets above: the panel it draws answers *is this working*, which is
   * the question somebody opening a Job they suspect has wedged came with.
   */
  resources: Holds;
  /**
   * The Job's history, for the one line of Pulse that says what a person last
   * did. Optional: without it, Pulse draws the latest of the Drone and Fleet.
   */
  history?: History;
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
  onReadCheckOutput,
  followed,
  onFollowCheckOutput,
  onReadFrame,
  onNeedMaterial,
  onNeedRemarks,
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
  history,
  examination,
  onExamine,
  recorded,
  onAct,
  onRaiseCap,
  onRaiseTurnCap,
  onRedirect,
  onAnswer,
  onOverrule,
  onRerun,
  onShowAgain,
  onReport,
  onApprove,
  onMergePullRequest,
  onApproveReview,
  onRequestChanges,
  onReject,
  onTakeUpRemarks,
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

  // Whether the raise dialog is up, on `reporting`'s terms and for its reasons:
  // two controls open it — the header's button and `B` — and the keyboard is
  // bound at this level. Dropped with the Job, because a figure half typed is
  // about the Job it was typed on.
  const [raising, setRaising] = useState(false);
  useEffect(() => setRaising(false), [job.id]);

  // Whether the turn-cap dialog is up. Its own state beside the cost cap's:
  // `budget_hold` offers one control or the other, never both, and one flag
  // would open whichever dialog happened to be mounted.
  const [raisingTurns, setRaisingTurns] = useState(false);
  useEffect(() => setRaisingTurns(false), [job.id]);

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
  // The open step's own submission, which the strip's Submitted tier draws and
  // its Judge tier points at. Read off the same `evidence` the trail below is
  // built from rather than fetched again — one call, drawn twice, which is the
  // rule `chapters.tsx` already follows for the same records.
  // `read` is the only state carrying rows, and a read that has not arrived is
  // not a step that claimed nothing — the tier draws its documents either way.
  const claimed = useMemo(
    () =>
      recorded.evidence.state === "read"
        ? recorded.evidence.steps.find((one) => one.step_id === open?.step_id)
        : undefined,
    [recorded.evidence, open?.step_id],
  );
  const phases =
    whole === null || open === undefined
      ? undefined
      : phasesOf(open, whole.acceptance_criteria, opensRecords, job.status, claimed);

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
    // `L`, from `actions.toml` — `open_log`, scope `detail`. **This is the line
    // that was missing.** The registry carried the key, `detail-keys` carried
    // the binding and dispatched it, and nothing here handed it a handler, so
    // the press found `undefined`, answered nothing and read as a key that was
    // never bound. `DetailShape` requires both openers now, so the next one
    // cannot go missing quietly.
    onOpenLog: () => openSheet("log"),
    // `o` — a Check's output, from the open step. The failed Check first: a
    // person reaching for an output on a step that stopped wants the one that
    // says why, and on a step where nothing failed the key opens the first
    // output there is rather than nothing.
    onOpenOutput: () => {
      const kept = outputOf(open);
      if (kept === undefined) return;
      void openArtifact(onOpenArtifact, job.id, { kept, what: "check" }).then((because) => {
        if (because !== null) onSaid(because);
      });
    },
    // `b`, on the one render that offers the act. Elsewhere the shape carries
    // nothing and the press is left alone rather than answered with a dialog
    // the header is not offering.
    ...(render === "stopped" && !stale ? { onReport: () => setReporting(true) } : {}),
    // `B`, on the one state that offers the act: a job stopped for money.
    // Elsewhere the shape carries nothing and the press is left alone rather
    // than answered with a dialog no control on this screen is offering.
    ...(heldForMoney(job) && !stale ? { onRaiseCap: () => setRaising(true) } : {}),
    // `T`, on the other ceiling. The two are exclusive because `budget_hold`
    // is, so a job held for turns answers this key and not `B`.
    ...(heldForTurns(job) && !stale ? { onRaiseTurnCap: () => setRaisingTurns(true) } : {}),
  });

  // The rest of any call argument the socket cut, for as long as this Job is
  // open. **Held for the Job rather than for a log**, because the story draws
  // the same row twice — chapter one's turns and chapter two's preview — and a
  // fetch made in one is the same argument in the other.
  const calls = useCallArguments(onReadCall, job.id);

  // What each Check printed, for as long as this Job is open. **Held for the
  // Job rather than for a step**, because a Job's steps each have their own
  // Checks and a reader moving between them should not re-fetch a file it
  // already has.
  const outputs = useCheckOutputs(onReadCheckOutput, job.id);
  // The running Check's log somebody is reading, for as long as this Job is
  // open. Streamed by main, and let go when the Job changes.
  const following = useFollowing(onFollowCheckOutput, followed ?? NOT_FOLLOWING, job.id);

  // The frames each step captured, for as long as this Job is open. Held for
  // the Job for `outputs`' reason, and it owns the object URLs it mints — a
  // `blob:` lives until it is revoked, so a person walking six Jobs would
  // otherwise leave six sets of screenshots behind them.
  const frames = useFrames(onReadFrame, job.id);

  // **Asked for when the step is opened, not when a chapter is pressed.** The
  // claim #209 makes is that a change whose point is not the code is reviewed
  // by looking at it, and a panel that draws a button saying there are pictures
  // has not led with anything. `want` is idempotent per `kept`, so this asking
  // again on every tick of the clock sends nothing twice.
  const shownBy = open?.frames;
  useEffect(() => {
    if (shownBy !== undefined && shownBy.length > 0) frames.want(shownBy);
  }, [shownBy, frames]);
  // Asking the Job to show its work again, and the frames its presses kept on
  // the open step. `again.tsx` holds all of it.
  const pressing = useShowAgain(onShowAgain, job.id, whole?.show_again, open?.step_id, frames);

  const rows = hideUnread(
    watching === null || open === undefined ? [] : entriesOf(watching.rows, open.step_id),
  ).rows;

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
          // Every step, in the frozen workflow's order, for the Drone brief's
          // `steps` section. Same nullable read as `criteria` below: a Job
          // whose detail has not arrived yet has no order to report.
          steps: whole?.steps ?? [],
          // The Job's frozen criteria, for the Verdicts chapter. The same list
          // the phase strip's Judge tier joins against, from the same reading.
          criteria: whole?.acceptance_criteria ?? [],
          watching,
          footprint: recorded.footprint,
          kept: whole?.footprint,
          diff: recorded.diff,
          live: observed.state === "watching",
          transcript,
          log: keys.inLog,
          calls,
          outputs,
          frames,
          again: againOf(
            onShowAgain === undefined ? undefined : whole?.show_again,
            open.step_id,
            frames,
            pressing,
          ),
          sheet,
          // The Produced chapter opens the step's deliverable, which the phase
          // strip's Submitted tier was the only route to. Same handler, because
          // two would be two vocabularies for one failed open — #307.
          opens: opensRecords,
          onOpenSheet: openSheet,
          now,
          following,
          // Scoped to the step `stuck` is actually about — a reader may have
          // navigated to a different step, and `stuck.undecided` is not that
          // step's reason for anything.
          undecided: whole?.stuck?.step_id === open.step_id ? whole?.stuck?.undecided : undefined,
        });

  // The verdict sheet's slot: `Decide`'s place at the gate, and the finished
  // Job's own place where the workflow never asked anybody anything. Neither
  // is drawn without an open step — a Job with no steps has nothing this
  // screen is built to read.
  const neverAsked = neverAsksAPerson(whole?.steps ?? []) === true;
  // Scoped the same way the Checks chapter's own read is, and for the same
  // reason: a reader may have navigated to a different step, and
  // `stuck.undecided` is not that step's reason for anything.
  const undecided = whole?.stuck?.step_id === open?.step_id ? whole?.stuck?.undecided : undefined;
  const verdictSlot =
    open === undefined
      ? undefined
      : render === "reviewing"
        ? verdictSlotAtGate({
            job,
            whole,
            open,
            render,
            recorded,
            opensRecords,
            now,
            claimed,
            undecided,
            onNeedMaterial,
            onNeedRemarks,
            stale,
            deciding,
            onMergePullRequest,
            onApproveReview,
            onRequestChanges,
            onReject,
            onTakeUpRemarks,
          })
        : render === "finished" && neverAsked
          ? verdictSlotFinished({ job, whole, open, render, recorded, opensRecords, now, claimed, undecided })
          : undefined;

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
    onRaiseCap,
    raising,
    onRaising: setRaising,
    onRaiseTurnCap,
    raisingTurns,
    onRaisingTurns: setRaisingTurns,
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
      unreachable={
        watched.state === "failed" && watched.jobId === job.id ? "Fleet did not answer" : undefined
      }
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
          latest={latestOf(watching?.rows ?? [], noted?.notes ?? [], movesOf(history, job.id))}
          latestNote={whyNoNotes(journalled) ?? NOTHING_HAPPENED_YET}
          figures={summarised(holding, examinedNow)}
          note={whyNoReading(resources)}
          age={holding === null ? undefined : (span(holding.read_at, now) ?? undefined)}
        />
      }
      machineAct={
        <Button variant="ghost" size="sm" onClick={() => openSheet("holds")}>
          Details
          <ChevronRight size={12} strokeWidth={2} aria-hidden />
        </Button>
      }
      // One animated mark per screen, on the thing being read — and nothing
      // pulses on a Job that is over, where "still working" is a claim no step
      // is making. **The pulse moves with the reading**: with a sheet open the
      // tree's current step is behind the layer, so its mark stops and the
      // sheet's live mark takes it.
      pulsing={render === "working" && sheet === null}
      onSelectStep={setSelected}
      // A count in the tree opens its chapter: the step, then the reader on it.
      onOpenChapter={(stepId, chapterId) => {
        setSelected(stepId);
        keys.onFocusChapter(chapterId);
      }}
      // The tree draws exactly what the keyboard holds. **Selecting a step
      // still does not open its facts** — that is `RunTree`'s rule and it is
      // the reason the two are separate props at all.
      openSteps={keys.openSteps}
      onOpenStep={keys.onOpenStep}
      where={workOf(onOpenArtifact, job, whole, manifest, workflow)}
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
                  opens={opensRecords}
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
              // second surface and never a second panel. The verdict sheet is
              // what draws that block now — `Decide`'s acts sit inside it,
              // unchanged, at `verdictSlot`'s `actions`.
              after: verdictSlot,
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

/** The reads the panel's own chapters draw from. */
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

/** Why the run has no rows, which is never the same sentence twice. */
function whyNoSteps(watched: Watched, jobId: string): string | undefined {
  if (watched.state === "read" && watched.jobId === jobId) {
    return watched.detail.steps.length === 0
      ? "This Job's frozen workflow has no steps."
      : undefined;
  }
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer";
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
    return "Fleet did not answer";
  }
  return "Reading this job.";
}
