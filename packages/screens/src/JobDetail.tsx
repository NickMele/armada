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

import { GAMING_PATTERN } from "@armada/components";
import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import {
  DroneQuestion,
  GamingFlags,
  InsideAJob,
  type JobDetailField,
  type JobDetailHeading,
  type RunTreeStep,
  type StepNotice,
} from "@armada/components";

import type { Diff, Evidence, Footprint, Observed, Outcome, Turn, Watched } from "@armada/protocol";
import type { FileReport, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { JobFootprint } from "@armada/protocol";
import type { ManifestSummary, WorkflowSummary } from "@armada/protocol";
import { Acts, StepActs, type ConfirmableAct } from "./Acts";
import { useCallArguments, type Calls, type ReadCall } from "./calls";
import type { OpenArtifact } from "./opening";
import { DIFF_CHAPTER, LOG_CHAPTER, namesStep, useDetailKeys } from "./detail-keys";
import { useAtFloor } from "@armada/shell";
import { DetailSheet, holdOf, type HeldAt, type OpenSheet } from "./Sheets";
import { chaptersOf } from "./chapters";
import { clock, span } from "./duration";
import { Decide } from "./Decide";
import { factsOf, ordered } from "./facts";
import { Opening, phasesOf, type Opens } from "./phases";
import { recourseOf } from "./recovery";
import { escalation, renderFor } from "./render";
import { runOf } from "./run";
import { entriesOf } from "./story";
import { readingOf } from "./reading";
import { briefOf, whyNoWork, workOf } from "./work";
import { stoppedAt } from "./stopped";

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

  const reading = readingOf(job);
  const render = renderFor(job);
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);
  // The detail is only this Job's while it names this Job. A stale one from the
  // Job that was open a moment ago would draw another Job's steps under this
  // Job's title.
  const whole = watched.state === "read" && watched.jobId === job.id ? watched.detail : null;

  const steps = whole === null ? [] : ordered(whole);
  const open = steps.find((step) => step.step_id === (selected ?? job.current_step_id)) ?? steps[0];
  // The rows this Job's socket has carried, or none. Checked against the id
  // it was opened for: the socket lags a selection by a round trip, and another
  // Job's turns under this Job's step would be a transcript under the wrong
  // title.
  const watching = "turns" in observed && observed.jobId === job.id ? observed.turns : null;

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
      : phasesOf(open, whole.acceptance_criteria, opensRecords);

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
  function openSheet(which: "log" | "diff"): void {
    setSheet(which);
    if (which === "log") setHeld(holdOf(now, rows.length));
  }

  /**
   * Close it, and put focus back where it came from. **The chapter line is the
   * way back** — `4k`'s third still — so `[` `]` carry on from the chapter the
   * reader opened rather than from the top of the story.
   */
  function closeSheet(): void {
    const was = sheet;
    setSheet(null);
    setHeld(null);
    if (was !== null) keys.onFocusChapter(was === "log" ? LOG_CHAPTER : DIFF_CHAPTER);
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
          log: keys.inLog,
          calls,
          sheet,
          // The Produced chapter opens the step's deliverable, which the phase
          // strip's Submitted tier was the only route to. Same handler, because
          // two would be two vocabularies for one failed open — #307.
          opens: opensRecords,
          onOpenSheet: openSheet,
        });

  // The badge is the header, so a Job the registry has no glyph or verb for
  // cannot be drawn at all. Named rather than half-drawn.
  if (reading.as !== "badge" || render === "unrenderable") {
    return <Unrenderable job={job} />;
  }

  const heading: JobDetailHeading = {
    status: reading.status,
    statusIcon: reading.icon,
    // The registry's own verb, opening a line. `enum-verbs.toml` spells it
    // lowercase because most of its readings are mid-sentence; here it is the
    // first word in the badge, and the badge is the header.
    statusLabel: leading(reading.verb),
    headline: job.title,
    jobId: job.id,
    fields: factsOf(job, whole, workflow, now),
    // The acts that end or replace the Job. **Pilot's slot is this one**, left
    // of the kill group — #250, and it lands without this line changing.
    actions: (
      <Acts
        job={job}
        whole={whole}
        render={render}
        acting={acting}
        approving={approving}
        stale={stale}
        onAct={onAct}
        onApprove={onApprove}
        onReport={onReport}
        reporting={reporting}
        onReporting={setReporting}
        onCopied={onCopied}
      />
    ),
  };

  return (
    <InsideAJob
      heading={heading}
      run={run.map(named)}
      runElapsed={span(job.created_at, now) ?? undefined}
      runAbsent={whyNoSteps(watched, job.id)}
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

/**
 * A registry verb, opening a line.
 *
 * **The word is still the registry's.** `enum-verbs.toml` spells its verbs for
 * the sentence they usually sit in the middle of; capitalising the first letter
 * where one leads a label is presentation, not a second spelling — nothing here
 * chooses, shortens or rewrites the word.
 */
function leading(verb: string): string {
  return verb.charAt(0).toUpperCase() + verb.slice(1);
}

/** What the pointers under the run are for, said once. */
const NAMED_NOT_NEEDED =
  "A path opens where it lives; an identifier copies. Nothing above needs these — they are here " +
  "for when you want them anyway.";

/**
 * The step's own short facts. **Figures, never a chart** — a filled bar reads
 * as progress and a step has no percentage.
 *
 * **The attempt is which run this is**, from `attempts`, and it is absent on a
 * step nothing has entered rather than drawn as a zero. A step run once still
 * says `Attempt 1`, because the drawing does and because it is the fact a
 * person checks before deciding a Drone is going in circles.
 */
function fieldsOf(step: StepDetail, now: number): JobDetailField[] {
  const running = step.state === "running" || step.state === "retrying";
  const elapsed = running
    ? span(step.entered_at, now)
    : step.entered_at === step.updated_at
      ? undefined
      : span(step.entered_at, step.updated_at);
  return [
    ...(elapsed === undefined
      ? []
      : [{ label: running ? "Running for" : "Took", value: elapsed, mono: true }]),
    ...(step.attempts.length === 0
      ? []
      : [{ label: "Attempt", value: String(step.attempts.length), mono: true }]),
  ];
}

/**
 * The band above the story: what happened, and why you are looking at this
 * step.
 *
 * **Waiting, stopped and failed are three kinds of stopped and never share a
 * tone.** Waiting on you is amber and carries no surface, because everything
 * mechanical cleared and the workflow is working; a Job that is over is red.
 */
/**
 * The band, where this Job's drone is waiting on an answer.
 *
 * **`waiting`, the same tone `reviewing` takes**, and for the reason the screen
 * already gives it: everything mechanical has cleared and nothing advances until
 * a person answers. Amber, never red — a drone that asked rather than guessed
 * did the right thing.
 *
 * It says only that a question is open. What was asked, and what each answer
 * commits to, is the box beneath: this band is scanned and that is read.
 */
function askingOf(whole: JobWhole | null): StepNotice | undefined {
  if (whole?.asking === undefined) return undefined;
  return {
    tone: "waiting",
    title: "The drone asked a question and is waiting for you.",
    children: "Nothing advances until you answer, and nothing is wrong.",
  };
}

/**
 * The question itself. `undefined` where nothing is outstanding, which is every
 * drone that knows what it is doing.
 *
 * **The elapsed is computed here and nowhere else.** `asked_at` crosses once and
 * nothing on the wire ticks, so the surface subtracts for itself — the same
 * arrangement `JudgeInFlight.since` has, on the `now` this screen re-renders
 * from.
 *
 * **Stale and in-flight both disable, and each says which.** A window showing a
 * reading it knows is not live must not send an answer against it.
 */
function questionOf(
  whole: JobWhole | null,
  jobId: string,
  now: number,
  stale: boolean,
  acting: boolean,
  onAnswer: (jobId: string, questionId: string, chose: string) => void,
): ReactNode {
  const asking = whole?.asking;
  if (asking === undefined) return undefined;
  return (
    <DroneQuestion
      question={asking.question}
      options={asking.options}
      waiting={span(asking.asked_at, now) ?? undefined}
      disabled={stale || acting}
      disabledNote={
        stale
          ? "This Job is not live, so nothing can be sent. The drone is still waiting."
          : acting
            ? "That answer is already on its way to the drone."
            : undefined
      }
      onAnswer={(label) => onAnswer(jobId, asking.question_id, label)}
    />
  );
}

/**
 * The band above the story: what stopped this step, and what the machine that
 * stopped it actually found.
 *
 * **`flagged` renders here, and it is the whole point of the band on a step
 * where the evidence was disputed.** Everything mechanical can pass, every
 * criterion can be met, and the step still stop — and a person reading
 * `7 of 7 passed`, `2 of 2 met` and a stopped step, with nothing reconciling
 * them, can only conclude the app is broken. The gaming check's finding is
 * what reconciles them, and it was reachable only by pressing *Overrule the
 * flag* and reading it in the dialog that confirms the act it exists to
 * inform.
 *
 * **What each act does is not here.** That was ninety words describing four
 * acts, in the imperative, detached from every control it named. Each sentence
 * is on its act's tooltip now, with its binding, and the band says where the
 * step stands.
 */
function noticeOf(
  job: JobSummary,
  whole: JobWhole | null,
  render: string,
  step: StepDetail,
  opens: Opens,
): StepNotice | undefined {
  if (render === "reviewing") {
    return {
      tone: "waiting",
      title: "Nothing is wrong. The workflow asks for a person here.",
      children: "Everything mechanical has cleared. Nothing advances until you answer.",
    };
  }
  if (render !== "stopped") return undefined;
  const reason = escalation(job);
  const at = whole === null ? undefined : stoppedAt(whole);
  const said = [
    reason?.verb,
    at === undefined ? undefined : `stopped at ${at.label}`,
    at?.check,
  ].filter((part) => part != null);
  // **The line that named the file a person could not open.** This band is the
  // first thing read on a Job that stopped and it has always ended with the log
  // path — as text, which is where `#246` was reported from. The strip opens it
  // too, on the Check's own row; this one is where somebody is already looking.
  const log = at?.outputPath;
  const recourse = recourseOf(job, whole);
  const flagged = step.flagged;
  return {
    // A Job that is over is red; one holding with a live Drone is not, because
    // a person deciding what happens next is not a failure.
    tone: job.status === "escalated" ? "stopped" : "failed",
    title: (
      <>
        {said.length === 0 ? "This Job stopped." : said.join(" · ")}
        {log === undefined ? null : (
          <>
            {" · "}
            <Opening path={log} what="check" opens={opens} />
          </>
        )}
      </>
    ),
    children: (
      <>
        {flagged.length === 0 ? null : (
          <GamingFlags
            flags={flagged.map((flag) => ({
              ...flag,
              // The registry carries a verb per pattern since #279; the wire
              // spelling is the key, never the copy. A pattern with no row
              // falls back to it rather than rendering nothing.
              verb: GAMING_PATTERN[flag.pattern]?.verb ?? undefined,
            }))}
            said={WHAT_THE_CHECK_FOUND}
            citation="whole"
          />
        )}
        <span>{recourse.stands}</span>
        {recourse.withheld === undefined ? null : (
          <span className="text-fg-subtle">{recourse.withheld}</span>
        )}
      </>
    ),
  };
}

/** What the flag rows are, said once over them rather than once on each. */
const WHAT_THE_CHECK_FOUND = "What the gaming check found, and where:";

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

/**
 * A Job the registry has no sanctioned glyph, verb or hue for. The badge is
 * the header, so there is no partial render to fall back to — and no glyph is
 * invented for it here any more than in the list.
 */
function Unrenderable({ job }: { job: JobSummary }) {
  const reading = readingOf(job);
  const missing = reading.as === "badge" ? ["variant"] : reading.missing;
  return (
    <p className="text-fg-muted">
      {`${job.title} — `}
      <span className="mono">{job.status}</span>
      {`. The registry carries no ${missing.join(" and no ")} for it, so this Job has no detail to draw.`}
    </p>
  );
}
