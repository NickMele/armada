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
// # The story is three chapters and none of them is behind a tab
//
// Drone instructions, then Activity log, then Produced, in the order they
// happened. The log streams while the Job runs and is on the page at every
// state — it used to be one of four tabs inside a region called *What it left
// behind*, which the drawing has none of, so the chapter that says what is
// happening right now was the one thing a person had to go and find. That
// region is gone: the turns are chapter two, the files are chapter three, and
// the raw event table is not something this screen needs at all.

import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  ChangedFiles,
  DroneQuestion,
  InsideAJob,
  type JobDetailField,
  type JobDetailHeading,
  type StepChapter,
  type StepNotice,
} from "@armada/components";

import type {
  Diff,
  Evidence,
  Footprint,
  Observed,
  Outcome,
  Turn,
  Watched,
} from "../../shared/bridge";
import type { FileReport, JobDetail as JobWhole, JobSummary, StepDetail } from "../../shared/protocol";
import type { ManifestSummary, WorkflowSummary } from "../../shared/setup";
import { Acts, StepActs, type ConfirmableAct } from "./Acts";
import { Decide, DecidedDiff } from "./Decide";
import { span } from "./duration";
import { factsOf, ordered } from "./facts";
import { filesOf, footprintNote, readingFor, whyNoFootprint } from "./files";
import { Log } from "./Log";
import { phasesOf } from "./phases";
import { recourseOf } from "./recovery";
import { escalation, renderFor } from "./render";
import { runOf } from "./run";
import { readingOf } from "./reading";
import { entriesOf, NOTHING_YET_ON_THIS_STEP } from "./story";
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
};

export function JobDetail({
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
}: JobDetailProps) {
  // Which step the panel is showing. **The whole of navigation inside a Job**:
  // `null` means the one Fleet says is current, so a Job that moves on carries
  // the reader with it until they choose a step themselves.
  const [selected, setSelected] = useState<string | null>(null);
  // A selection belongs to the Job it was made in. Carried into the next Job it
  // would name a step that Job may not have.
  useEffect(() => setSelected(null), [job.id]);

  const reading = readingOf(job);
  const render = renderFor(job);
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);
  // The detail is only this Job's while it names this Job. A stale one from the
  // Job that was open a moment ago would draw another Job's steps under this
  // Job's title.
  const whole = watched.state === "read" && watched.jobId === job.id ? watched.detail : null;

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
        onCopied={onCopied}
      />
    ),
  };

  const steps = whole === null ? [] : ordered(whole);
  const open = steps.find((step) => step.step_id === (selected ?? job.current_step_id)) ?? steps[0];
  // The rows this Job's socket has carried, or none. Checked against the id
  // it was opened for: the socket lags a selection by a round trip, and another
  // Job's turns under this Job's step would be a transcript under the wrong
  // title.
  const watching =
    "turns" in observed && observed.jobId === job.id ? observed.turns : null;

  return (
    <InsideAJob
      heading={heading}
      run={whole === null ? [] : runOf(whole, now, selected ?? undefined)}
      runElapsed={span(job.created_at, now) ?? undefined}
      runAbsent={whyNoSteps(watched, job.id)}
      // One animated mark per screen, on the thing being read — and nothing
      // pulses on a Job that is over, where "still working" is a claim no step
      // is making.
      pulsing={render === "working"}
      onSelectStep={setSelected}
      where={workOf(job, whole, manifest, workflow)}
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
              notice: askingOf(whole) ?? noticeOf(job, whole, render),
              // **The question sits where the redirect box does** — between the
              // strip and the story, because it is the same kind of thing: a
              // box a person acts in about the step they are looking at.
              before: questionOf(whole, job.id, now, stale, acting, onAnswer),
              phases: whole === null ? undefined : phasesOf(open, whole.acceptance_criteria),
              phasesAbsent: whyNoSteps(watched, job.id),
              chapters: chaptersOf({
                job,
                step: open,
                render,
                watching,
                footprint: recorded.footprint,
                diff: recorded.diff,
                live: observed.state === "watching",
              }),
              // Review and reply are one loop: the decision is the block under
              // the story, one scroll from the diff it is made against, never a
              // second surface and never a second panel.
              after:
                render === "reviewing" ? (
                  <Decide
                    job={job}
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
      onCopied={onCopied}
    />
  );
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
 * **The attempt count is named as missing rather than left out.** The design
 * draws attempts as rows, and `StepDetail` carries nothing that counts them —
 * no `retry_count`, no attempt list — so the field says what is not served
 * rather than the panel quietly having one fewer fact than the drawing.
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
    { label: "Attempts", value: "not served" },
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

function noticeOf(job: JobSummary, whole: JobWhole | null, render: string): StepNotice | undefined {
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
    at?.outputPath,
  ].filter((part) => part != null);
  return {
    // A Job that is over is red; one holding with a live Drone is not, because
    // a person deciding what happens next is not a failure.
    tone: job.status === "escalated" ? "stopped" : "failed",
    title: said.length === 0 ? "This Job stopped." : said.join(" · "),
    children: recourseOf(job, whole).note,
  };
}

/**
 * The story: Drone instructions, then Activity log, then Produced. **The same
 * three chapters in the same order at every state** — what changes is which one
 * is the reason you are here.
 */
function chaptersOf({
  job,
  step,
  render,
  watching,
  footprint,
  diff,
  live,
}: {
  job: JobSummary;
  step: StepDetail;
  render: string;
  watching: { rows: readonly Turn[]; skipped: number } | null;
  footprint: Footprint;
  diff: Diff;
  /** Whether the socket is still carrying rows, for the chapter's live mark. */
  live: boolean;
}): StepChapter[] {
  const rows = watching === null ? [] : entriesOf(watching.rows, step.step_id);
  const told = rows.filter((row) => row.actor === "armada");
  const opened = told[0];
  const touched = readingFor(footprint, job.id);
  return [
    {
      id: "instructions",
      ordinal: 1,
      title: "Drone instructions",
      // The turn the step opened with, in the words the Drone was given.
      // Armada's own turns are on the transcript beside the Drone's, so this
      // is the same stream chapter two draws, filtered to one voice.
      summary: opened === undefined ? undefined : opened.at,
      preview:
        opened === undefined ? (
          <p className="text-2xs text-fg-muted">{NOT_OPENED_YET}</p>
        ) : (
          <p className="text-fg-muted">{opened.payload.map((line) => line.text).join("\n")}</p>
        ),
      ...(told.length <= 1
        ? {}
        : {
            content: <Log rows={told} emptyNote={NOT_OPENED_YET} />,
            openLabel: `Everything Armada told it — ${told.length} turns`,
          }),
    },
    {
      id: "log",
      ordinal: 2,
      title: "Activity log",
      // `live` is a word here rather than the running dot the drawing puts
      // before it: `Chapter` draws that mark and `StepStory` does not compose
      // `Chapter`, so the claim is made in the only channel this header has.
      summary: [
        ...(live ? ["live"] : []),
        `${rows.length} ${rows.length === 1 ? "entry" : "entries"}`,
        "every line opens",
      ].join(" · "),
      // Always drawn, and never behind a control. The log is what says what is
      // happening right now, so it is on the page while the Job runs rather
      // than a thing to go and open.
      preview: <Log rows={rows.slice(-PREVIEWED)} emptyNote={NOTHING_YET_ON_THIS_STEP} />,
      ...(rows.length <= PREVIEWED
        ? {}
        : {
            content: <Log rows={rows} emptyNote={NOTHING_YET_ON_THIS_STEP} />,
            openLabel: `Open the log — all ${rows.length} entries`,
          }),
    },
    {
      id: "produced",
      ordinal: 3,
      title: "Produced",
      summary: touched === undefined ? undefined : `${touched.files.length} files`,
      preview:
        touched === undefined ? (
          <p className="text-2xs text-fg-muted">{whyNoFootprint(job.assigned_drone !== undefined)}</p>
        ) : (
          <ChangedFiles
            files={filesOf(touched)}
            emptyNote={NOTHING_TOUCHED}
            note={footprintNote(touched, render === "working")}
          />
        ),
      // A produced file opens to what it actually wrote, at every state. The
      // diff is the expensive read and opening the chapter is what spends it —
      // which is the whole reason one chapter is open at a time.
      content: <DecidedDiff diff={diff} jobId={job.id} />,
      openLabel:
        touched === undefined ? "Open the diff" : `Open the diff — ${touched.files.length} files`,
    },
  ];
}

/** How many entries the log's collapsed preview shows. The drawing's own five. */
const PREVIEWED = 5;

/** What chapter one says before Armada has opened the step. */
const NOT_OPENED_YET = "Armada has not opened this step yet.";

/**
 * A reading that found nothing. **Ordinary, and never an error** — a Drone that
 * has just started has changed nothing yet.
 */
const NOTHING_TOUCHED = "This drone has not changed anything yet.";

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

/** Why there is no brief, which is never the same sentence twice. */
function whyNoBrief(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this job, so what done meant for it is unknown.";
  }
  return "Reading this Job.";
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
