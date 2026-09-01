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
// # What is served is drawn, and what is not is named
//
// The run is built in `run.ts`, the strip in `phases.ts`, the log in `story.ts`,
// and each of the three says at its own head what the wire does not carry. Two
// are worth repeating here because they are visible on every Job: **a per-step
// attempt count is not served**, so "an attempt is a row, not a counter" cannot
// be drawn at all; and **the activity log carries the Drone's turns only**,
// because Fleet's own events are not on the Observe socket.

import { useEffect, useState } from "react";
import {
  ChangedFiles,
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
  History,
  Observed,
  Outcome,
  Watched,
} from "../../shared/bridge";
import type { FileReport, JobDetail as JobWhole, JobSummary, StepDetail } from "../../shared/protocol";
import type { ManifestSummary, WorkflowSummary } from "../../shared/setup";
import { Acts, StepActs, type ConfirmableAct } from "./Acts";
import { Decide, DecidedDiff } from "./Decide";
import { span } from "./duration";
import { factsOf, ordered } from "./facts";
import { filesOf, footprintNote, readingFor, whyNoFootprint } from "./files";
import { phasesOf } from "./phases";
import { Record } from "./Record";
import { recourseOf } from "./recovery";
import { RECORDS_ITS_OWN_TURNS, escalation, renderFor } from "./render";
import { runOf } from "./run";
import { readingOf } from "./reading";
import { entriesOf, NOT_ONE_STREAM, NOTHING_YET_ON_THIS_STEP, NOT_WATCHING } from "./story";
import { ActivityLog } from "@armada/components";
import { briefOf, whyNoWork, workOf } from "./work";
import { stoppedAt } from "./rail";

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
  /** What the second socket has said, where this Job's turns are being read. */
  observed: Observed;
  /** The reads the record's sections and the running footprint draw from. */
  recorded: FoldedReads;
  /** Open or close this Job's turns. Not an act on the Drone. */
  onObserve: (on: boolean) => void;
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
  onOverrule,
  onRerun,
  onReport,
  onApprove,
  onApproveReview,
  onRequestChanges,
  onReject,
  onObserve,
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
    statusLabel: reading.verb,
    headline: job.title,
    jobId: job.id,
    fields: factsOf(job, whole, workflow, manifest, now),
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
        onObserve={RECORDS_ITS_OWN_TURNS.has(render) ? undefined : () => onObserve(true)}
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
      where={whole === null ? undefined : workOf(job, whole, manifest, render === "working")?.rows}
      whereNote={NAMED_NOT_NEEDED}
      whereAbsent={whyNoWork(watched, job.id)}
      brief={whole === null ? undefined : briefOf(whole)}
      briefAbsent={whyNoBrief(watched, job.id)}
      record={
        <Record
          job={job}
          whole={whole}
          observed={observed}
          history={recorded.history}
          evidence={recorded.evidence}
          diff={recorded.diff}
          // `get_diff` reads the declaration out of the slot this Job's own
          // Drone holds, and a Job that is over has let go of it.
          planReadable={render === "working" || render === "reviewing"}
          onWatchTurns={onObserve}
          onCopied={onCopied}
        />
      }
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
              notice: noticeOf(job, whole, render),
              phases: whole === null ? undefined : phasesOf(open, whole.acceptance_criteria),
              phasesAbsent: whyNoSteps(watched, job.id),
              chapters: chaptersOf({
                job,
                step: open,
                render,
                watching,
                footprint: recorded.footprint,
                diff: recorded.diff,
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
}: {
  job: JobSummary;
  step: StepDetail;
  render: string;
  watching: { rows: readonly import("../../shared/bridge").Turn[]; skipped: number } | null;
  footprint: Footprint;
  diff: Diff;
}): StepChapter[] {
  const entries = watching === null ? [] : entriesOf(watching.rows, step.step_id);
  const touched = readingFor(footprint, job.id);
  return [
    {
      id: "instructions",
      ordinal: 1,
      title: "Drone instructions",
      // **Not served.** Nothing on `StepDetail` carries the brief Armada wrote
      // for the step; the Job's own brief is above, and this is the turn the
      // step opened with. Named where it would have gone.
      summary: "not served",
      preview:
        "What Armada told the Drone at the top of this step is not on the wire. The Job's brief is " +
        "above; this would be the injected turn that opened the step.",
    },
    {
      id: "log",
      ordinal: 2,
      title: "Activity log",
      summary:
        watching === null
          ? "not being read"
          : `${entries.length} ${entries.length === 1 ? "entry" : "entries"} · every line opens`,
      preview:
        watching === null ? (
          NOT_WATCHING
        ) : (
          <ActivityLog
            entries={entries.slice(-PREVIEWED)}
            emptyNote={NOTHING_YET_ON_THIS_STEP}
            cut={NOT_ONE_STREAM}
          />
        ),
      ...(watching === null || entries.length <= PREVIEWED
        ? {}
        : {
            content: (
              <ActivityLog
                entries={entries}
                emptyNote={NOTHING_YET_ON_THIS_STEP}
                cut={cutOf(watching.skipped)}
              />
            ),
            openLabel: `Open the log — all ${entries.length} entries`,
          }),
    },
    {
      id: "produced",
      ordinal: 3,
      title: "Produced",
      // **The Job's reading, not the step's.** `job.files_changed` and
      // `JobDetail.footprint` are the whole Job's, so a per-step file list
      // would be the same list under every step.
      summary: touched === undefined ? "not read" : `${touched.files.length} files`,
      preview:
        touched === undefined ? (
          whyNoFootprint(job.assigned_drone !== undefined)
        ) : (
          <ChangedFiles
            files={filesOf(touched)}
            emptyNote={NOTHING_TOUCHED}
            note={footprintNote(touched, render === "working")}
          />
        ),
      // The diff is the expensive read and only the review gate asks for it
      // here; everywhere else it is a section of the record, which is one place
      // rather than two.
      ...(render === "reviewing"
        ? {
            content: <DecidedDiff diff={diff} jobId={job.id} />,
            openLabel: "Open the diff",
          }
        : {}),
    },
  ];
}

/** How many entries the log's collapsed preview shows. The drawing's own five. */
const PREVIEWED = 5;

/** What the stream left out, where the socket's backfill was bounded. */
function cutOf(skipped: number): string {
  return skipped === 0
    ? NOT_ONE_STREAM
    : `${NOT_ONE_STREAM} The backfill also left out ${skipped} older rows; the whole transcript is the file named under Where things are.`;
}

/**
 * A reading that found nothing. **Ordinary, and never an error** — a Drone that
 * has just started has changed nothing yet.
 */
const NOTHING_TOUCHED = "This drone has not changed anything yet.";

/** The two reads the record and the running footprint draw from. */
export type FoldedReads = {
  footprint: Footprint;
  history: History;
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
