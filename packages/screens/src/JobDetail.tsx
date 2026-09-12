// One Job, read whole, and one arrangement whatever the Job is doing: the run
// as a tree on the left, the selected step in the panel, its story in the order
// it happened. What a status changes is which chapter is the reason you are
// here, not where anything sits.
//
// This file holds the open state of one reading — which step, which sheet,
// where the log was held, whether the report dialog is up — and it arranges the
// regions. What any of them says, it does not decide. The Job header is
// `heading.tsx`, the step's facts, band and question box are `step.tsx`, the
// story is `chapters.tsx`, the two sheets are `Sheets.tsx`, what a caller hands
// in is `detail-props.ts`, and which reading is this Job's is `mine.ts`.
//
// An addition to a region goes in that region's file. This one was cut twice
// and grew back both times, because it was the only place one could be written.

import { JobHoldsSummary } from "@armada/components";
import { ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { InsideAJob } from "./InsideAJob";

import type { FollowedLog } from "@armada/protocol";
import { heldForMoney, heldForTurns } from "./Acts";
import { useCallArguments } from "./calls";
import { useCheckOutputs, useFollowing } from "./outputs";

/** What `followed` reads as where the caller hands none in. */
const NOT_FOLLOWING: FollowedLog = { state: "none" };
import { useFrames } from "./frames";
import { openArtifact } from "./opening";
import { DIFF_CHAPTER, LOG_CHAPTER, useDetailKeys } from "./detail-keys";
import { named } from "./run-labels";
import { useAtFloor } from "@armada/shell";
import { DetailSheet, holdOf, type HeldAt, type OpenSheet } from "./Sheets";
import { chaptersOf } from "./chapters";
import { landingsOf, stepTimelineOf } from "./timeline";
import { againOf, useShowAgain } from "./again";
import { span } from "./duration";
import { ordered } from "./facts";
import { headingOf, Unrenderable } from "./heading";
import { detailOf, holdingOf, logOf, lookOf, turnsOf } from "./mine";
import { useDiffAgain } from "./produced";
import { checkEntryId, useRunSheet } from "./rehearsal";
import { verdictSlotOf } from "./verdict-answered";
// Which Check's output `o` opens. **The same call the Checks chapter's own act
// makes**, so the key and the control cannot open different files.
import { outputOf } from "./gates";
import { whileReading, whyUnreachable } from "./while-reading";
import { renderFor } from "./render";
import { runOf, whyNoSteps } from "./run";
import { answeringOf, askingOf, commandOf, fieldsOf, noticeOf, questionOf, waitingOf } from "./step";
import { StepActs } from "./StepActs";
import { whyNoNotes } from "./notes";
import { entriesOf, hideUnread, whyNotWatching } from "./story";
import { LOOK_FAILED, NOTHING_HAPPENED_YET, latestOf, movesOf, nothingToAsk, summarised, whyNoReading } from "./resources";
import { briefOf, whyNoBrief, workOf, workRehearsalOf } from "./work";

export type { ConfirmableAct, JobAct } from "./Acts";
export type { FoldedReads } from "./mine";
export { renderFor } from "./render";
export type { Render } from "./render";

/**
 * What a caller hands this screen. It lives in `detail-props.ts`, moved there
 * when this file crossed the 900-line refusal for the third time.
 *
 * **Re-exported here on purpose.** A caller names the screen it is
 * configuring, not the file the type sits in — the same reason
 * `ConfirmableAct` and `FoldedReads` above are re-exported rather than reached
 * for directly.
 */
export type { JobDetailProps } from "./detail-props";
import type { JobDetailProps } from "./detail-props";

export function JobDetail({
  onReadDiff,
  onOpenArtifact,
  onOpenPullRequest,
  onReadCall,
  onReadCheckOutput,
  followed,
  onFollowCheckOutput,
  onReadFrame,
  onFrameSrc,
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
  onAnswerCommand,
  onExplainCommand,
  onAnswerJudge,
  onSetWhenBlocked,
  onSetWhenRefused,
  onSetModel,
  onRemoveAllowedCommand,
  models,
  onOverrule,
  onRerun,
  onShowAgain,
  onReport,
  onApprove,
  onMergePullRequest,
  onResolvePullRequestConflict,
  onApproveReview,
  onRequestChanges,
  onReject,
  onTakeUpRemarks,
  onOpenRemarkLink,
  onCopied,
  onSaid,
  rehearsal,
}: JobDetailProps) {
  // Which step the panel is showing. **The whole of navigation inside a Job**:
  // `null` means the one Fleet says is current, so a Job that moves on carries
  // the reader with it until they choose a step themselves.
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => setSelected(null), [job.id]);

  // Which sheet is open, or none. **One value rather than two booleans**: the
  // two cannot both be open, and a pair of flags is a state that says they can.
  const [sheet, setSheet] = useState<OpenSheet>(null);
  useEffect(() => setSheet(null), [job.id]);

  // Where the log's reading was held, and what has arrived since. **The tail is
  // not followed while a sheet is open**: a stream that scrolls itself cannot
  // be read. `held` is how many rows the step had when the reading was taken,
  // and `Jump to now` takes it again. No reset effect: `closeSheet` clears it.
  const [held, setHeld] = useState<HeldAt | null>(null);

  // Whether the report dialog is up. Two controls open it — the Job header's
  // menu entry and `b` — and the keyboard is bound at the screen's level.
  const [reporting, setReporting] = useState(false);
  useEffect(() => setReporting(false), [job.id]);

  // Whether the raise dialog is up, on `reporting`'s terms: the header's
  // button and `B` both open it.
  const [raising, setRaising] = useState(false);
  useEffect(() => setRaising(false), [job.id]);

  // Whether the turn-cap dialog is up. Its own state beside the cost cap's:
  // `budget_hold` offers one control or the other, never both.
  const [raisingTurns, setRaisingTurns] = useState(false);
  useEffect(() => setRaisingTurns(false), [job.id]);
  const runHook = useRunSheet({ ...rehearsal, jobId: job.id, jobTitle: job.title, sheet, now, setSheet, onSaid });

  // The diff, for every Job that is open rather than only for one at review.
  // **A produced file opens to what it actually wrote**, and it did that on one
  // status because the review block was the only thing asking for the read. It
  // is still the expensive read, so the Produced chapter and the review
  // decision draw from this one answer.
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
  // What the Board's own row already answers, while this Job's read is out.
  const reading = whileReading(watched, job, workflow, selected);
  const watching = turnsOf(observed, job.id);
  const noted = logOf(journalled, job.id);
  // The open sheet's own reading of the patch, taken again as the Job writes.
  useDiffAgain(onReadDiff, job.id, sheet, watching?.rows ?? []);
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
    () => (recorded.evidence.state === "read" ? recorded.evidence.steps.find((one) => one.step_id === open?.step_id) : undefined),
    [recorded.evidence, open?.step_id],
  );
  // The criterion a live judge question holds open on this step. Read here and
  // handed to the story; the strip that also took it is gone.
  const asking =
    whole?.judge_question?.step_id === open?.step_id
      ? whole?.judge_question?.criterion_id
      : undefined;

  // The detail's contextual tier, and the open state it moves. Bound while a
  // Job is open and not before, so nothing on the Board listens for a key that
  // means nothing there — and the press is swallowed only where something
  // answered it. The story is read back through a function because it is built
  // from what this holds; see `DetailShape.chapters`.
  const keys = useDetailKeys({
    run,
    landings: () => landingsOf(timeline ?? []),
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
    onOpenRun: () => runHook.open(), // `r`, Journey 9 — freed from `review`
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
  const frames = useFrames(onReadFrame, job.id, onFrameSrc);

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
  function openSheet(which: Exclude<OpenSheet, null>): void {
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
          asking,
          onRunHere: (checkId) => runHook.open(checkEntryId(checkId)), // Journey 9
        });

  // The timeline arranges the chapters just built; it derives nothing they hold.
  const timeline = open && stepTimelineOf(open, watching?.rows ?? [], now, chapters);

  // The verdict sheet's slot: `Decide`'s place at the gate, and the finished
  // Job's own place, whichever of the three arrangements the render is —
  // `verdictSlotOf`, in `verdict-answered.tsx`.
  const atGate = render === "reviewing";
  const question = questionOf(whole, job.id, now, stale, acting, onAnswer);
  const verdictSlot = verdictSlotOf({
    job,
    whole,
    open,
    render,
    recorded,
    opensRecords,
    now,
    claimed,
    onNeedMaterial,
    onNeedRemarks,
    stale,
    deciding,
    onMergePullRequest,
    onResolvePullRequestConflict,
    onApproveReview,
    onRequestChanges,
    onReject,
    onTakeUpRemarks,
    onOpenRemarkLink,
    onOpenPullRequest,
    onSaid,
    onAnswerJudge,
    notes: noted?.notes ?? [],
  });

  // One way to answer a command the Drone was not given, for both places a
  // person meets one: the command it is waiting on, and a refused row.
  const answering = answeringOf(job.id, stale, acting, onAnswerCommand);
  // Bound to the Job being read, so nothing downstream carries an id back.
  const explain =
    onExplainCommand === undefined ? undefined : (call: string) => onExplainCommand(job.id, call);
  const waiting = waitingOf(question, commandOf(whole, now, answering, explain));

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
    onOpenSettings: () => openSheet("settings"),
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
      runReading={reading?.run}
      unreachable={whyUnreachable(watched, job.id)}
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
        <button type="button" className="armada-screen__eyebrow-act" onClick={() => openSheet("holds")}>
          Details
          <ChevronRight size={12} strokeWidth={2} aria-hidden />
        </button>
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
      where={workOf(
        onOpenArtifact,
        job,
        whole,
        manifest,
        workflow,
        // **Wrapped, never passed bare.** `open` takes an entry id, and this
        // reaches `WhereRow`'s `Run…` as its `onClick` — which React calls with
        // the click event, so a bare reference selected the event itself and
        // the next render asked it for `.indexOf`. `onOpenRun` above has always
        // wrapped it for the same reason.
        workRehearsalOf(() => runHook.open(), runHook.worktreeOnDisk, rehearsal),
      )}
      brief={whole === null ? undefined : briefOf(whole)}
      briefAbsent={whyNoBrief(watched, job.id)}
      briefLoading={reading !== undefined}
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
              notice: askingOf(whole) ?? noticeOf(job, whole, render, open, opensRecords, answering),
              // **The question sits where the redirect box does** — between the
              // strip and the story, because it is the same kind of thing: a
              // box a person acts in about the step they are looking at. A
              // command the Drone is waiting on is the same box, and at the gate
              // so is the review (the owner, 11 Sep 2026).
              before: atGate ? <>{waiting}{verdictSlot}</> : waiting,
              timelineAbsent: whyNoSteps(watched, job.id),
              timeline,
              openRow: keys.openChapterId,
              onOpenRow: keys.onOpenChapter,
              // A finished Job's verdict sheet is a record, read after the story.
              after: atGate ? undefined : verdictSlot,
            }
      }
      stepAbsent={whyNoSteps(watched, job.id)}
      stepReading={reading?.step}
      sheet={
        open === undefined ? null : (
          <DetailSheet
            which={sheet}
            job={job}
            whole={whole}
            step={open}
            rows={rows}
            // The turns those rows were folded from, which carry the tool and
            // the timing a row no longer does. The sheet folds runs of one tool
            // the way the chapter does, and this is what it folds them by.
            turns={watching === null ? [] : watching.rows}
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
            // Every setting a person can change on this Job, and what each sends.
            settings={{
              models, stale, acting, onSetWhenBlocked, onSetWhenRefused, onSetModel,
              onRemoveAllowedCommand, onRaiseCap, onRaiseTurnCap,
            }}
            run={runHook.slot}
            floor={floor}
            onClose={closeSheet}
          />
        )
      }
      onCopied={onCopied}
    />
  );
}
