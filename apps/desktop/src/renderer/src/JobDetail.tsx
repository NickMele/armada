// One Job, read whole. The four renders the design draws, chosen by the Job's
// status and fed from `GET /jobs/:job_id` rather than from the row beside it.
//
// # Four renders, and only the running one is drawn here
//
// A running Job is watched, so it leads with a rail and this file draws it. The
// other three are opened with a question and each one's question is different:
// `Reviewing.tsx` is the diff and the reply as one loop, `Finished.tsx` leads
// with what a Job was and what came out, and `Stopped.tsx` leads with what
// stopped it and whether anything resumes it. This file chooses between them,
// builds the header all four share, and draws the one that is left.
//
// # What is served is drawn, and what is not is named
//
// The rail is built in `rail.ts`, which says what of it the wire carries. What
// the Judge answered is on it too, beneath the step it judged: a refusal is not
// a failed Check and does not render as one, and `CriterionVerdicts` carries
// that difference — see its own header for the three ways.
//
// Where the work is is the one region built from both: the branch is served,
// and the worktree, log and transcript paths are derived in `work.ts` from the
// job id and the repository. A step's Checks and what each one did are served
// too, since protocol 3. Evidence and spend are not, and every region that
// wants one says so where it would have gone.
//
// # Absent is not empty, and the two get different sentences
//
// Every optional field on the detail is omitted rather than sent null, which
// makes the distinction readable: `write_targets` absent is scope undetermined
// and present-and-empty is determined to write nothing. Collapsing them would
// tell somebody a Job has no scope when what is true is that nobody set one.

import { ARunningJob, type JobDetailHeading } from "@armada/components";

import type { Diff, Evidence, Footprint, History, Observed, Watched } from "../../shared/bridge";
import type { JobSummary } from "../../shared/protocol";
import type { ManifestSummary, WorkflowSummary } from "../../shared/setup";
import { Acts, type ConfirmableAct } from "./Acts";
import { factsOf } from "./facts";
import { filesOf, footprintNote, readingFor, whyNoFootprint } from "./files";
import { Finished } from "./Finished";
import { railOf } from "./rail";
import { readingOf } from "./reading";
import { Reviewing } from "./Reviewing";
import { RECORDS_ITS_OWN_TURNS, renderFor } from "./render";
import { Stopped } from "./Stopped";
import { whyNoWork, workOf } from "./work";

export { ACT_LABEL } from "./Acts";
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
  /**
   * Send a redirect straight through — the dialog that collects the
   * instruction is the confirmation, so there is nothing left for `onAct` to
   * ask about.
   */
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Let this Job run. **Sent on the press, with no confirmation** — approving
   * is the ordinary path, it is reversible by killing, and a gate that costs
   * two clicks for the common case is a gate in the wrong place.
   */
  onApprove: (jobId: string) => void;
  /**
   * The three answers to a Job at `awaiting_review`. **Three props and not
   * one** — they differ by what survives them, and one handler taking which
   * decision as an argument would make that difference a flag. Approving is
   * sent on the press; rejecting is confirmed by the review render itself,
   * whose dialog names the drone that ends with it.
   */
  onApproveReview: (jobId: string) => void;
  onRequestChanges: (jobId: string, note: string) => void;
  onReject: (jobId: string) => void;
  /** What the second socket has said, where this Job's turns are being read. */
  observed: Observed;
  /**
   * The two reads a folded record section asks for, and the one the running
   * render draws live. **One prop rather than three**, because they arrive
   * together, are published together, and no render takes one without the
   * other.
   */
  recorded: FoldedReads;
  /**
   * Open or close this Job's turns. **Not an act on the Drone** — it opens a
   * read-only view and takes nothing over, which is why it takes no
   * confirmation and does not go through `onAct` with the three that end
   * something. The two renders that lead with a rail send `true` and reach the
   * turns as a screen; the finished render holds them as a section of its
   * record and sends whichever the open section calls for.
   */
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
  onApprove,
  onApproveReview,
  onRequestChanges,
  onReject,
  onObserve,
  onCopied,
}: JobDetailProps) {
  const reading = readingOf(job);
  const render = renderFor(job);
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);
  // The detail is only this Job's while it names this Job. A stale one from the
  // Job that was open a moment ago would draw another Job's steps under this
  // Job's title.
  const whole = watched.state === "read" && watched.jobId === job.id ? watched.detail : null;

  // The badge is the header, so a Job the registry has no glyph or verb for
  // cannot be drawn at all. Named rather than half-drawn — the same answer the
  // list gives for the same Job.
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
    actions: (
      <Acts
        job={job}
        render={render}
        acting={acting}
        approving={approving}
        stale={stale}
        onAct={onAct}
        onRedirect={onRedirect}
        onApprove={onApprove}
        onObserve={RECORDS_ITS_OWN_TURNS.has(render) ? undefined : () => onObserve(true)}
      />
    ),
  };

  const rail = whole === null ? [] : railOf(whole, now);
  const stepsAbsent = whyNoSteps(watched, job.id);
  // The brief and the paths, on every render. The finished one takes the
  // branch out: its handover names it, and one value drawn twice is two
  // places to keep in step.
  const workAbsent = whyNoWork(watched, job.id);

  if (render === "reviewing") {
    return (
      <Reviewing
        job={job}
        whole={whole}
        watched={watched}
        manifest={manifest}
        evidence={recorded.evidence}
        diff={recorded.diff}
        stale={stale}
        deciding={deciding}
        heading={heading}
        onApprove={onApproveReview}
        onRequestChanges={onRequestChanges}
        onReject={onReject}
        onCopied={onCopied}
      />
    );
  }

  if (render === "finished") {
    return (
      <Finished
        job={job}
        whole={whole}
        watched={watched}
        manifest={manifest}
        observed={observed}
        footprint={recorded.footprint}
        history={recorded.history}
        evidence={recorded.evidence}
        diff={recorded.diff}
        now={now}
        heading={heading}
        onWatchTurns={onObserve}
        onCopied={onCopied}
      />
    );
  }

  if (render === "stopped") {
    return (
      <Stopped
        job={job}
        whole={whole}
        manifest={manifest}
        observed={observed}
        history={recorded.history}
        evidence={recorded.evidence}
        diff={recorded.diff}
        heading={heading}
        steps={rail}
        stepsAbsent={stepsAbsent}
        workAbsent={workAbsent}
        onWatchTurns={onObserve}
        onCopied={onCopied}
      />
    );
  }

  // What the Drone has touched so far. **Live only, and named where it is
  // not there yet** — `job.files_changed` arrives while a Drone works, so a
  // Job with no Drone on it will never carry one and says so in its own words.
  const touched = readingFor(recorded.footprint, job.id);

  return (
    <ARunningJob
      heading={heading}
      steps={rail}
      stepsAbsent={stepsAbsent}
      footprint={
        touched === undefined
          ? undefined
          : {
              files: filesOf(touched),
              emptyNote: NOTHING_TOUCHED,
              note: footprintNote(touched, true),
            }
      }
      footprintAbsent={whyNoFootprint(job.assigned_drone !== undefined)}
      evidenceAbsent={NOT_SERVED.evidence}
      log={workOf(job, whole, manifest, true)}
      logAbsent={workAbsent}
      onCopied={onCopied}
    />
  );
}

/**
 * A reading that found nothing. **Ordinary, and never an error** — a Drone that
 * has just started has changed nothing yet, which is a different sentence from
 * a Drone that has reported nothing.
 */
const NOTHING_TOUCHED = "This drone has not changed anything yet.";

/**
 * The two reads the folded sections and the running footprint draw from.
 * Named for what they are rather than `Recorded`, which is one row of the
 * transition history and a different thing entirely.
 */
export type FoldedReads = {
  footprint: Footprint;
  history: History;
  /**
   * The two the review render asks for. Here rather than as two more props for
   * the reason the other two are: they arrive together, are published together,
   * and only one render takes them.
   */
  evidence: Evidence;
  diff: Diff;
};

/**
 * What the wire does not carry, said in the place the design puts it. One
 * sentence each, naming the operation that would have to serve it — a hole
 * that names its cause is a finding, one that reads "coming soon" is not.
 */
const NOT_SERVED = {
  // Served since protocol 4.6, and deliberately not read here. A running Job is
  // opened to see where it is, and a read on every open would make the common
  // case pay for a claim that is only decided on at the review gate — where it
  // is drawn, beside the diff it is a claim about.
  evidence:
    "Submissions are read at the review gate, beside the diff they are claims about. " +
    "Nothing on a running job reads them.",
} as const;

/** Why the rail has no rows, which is never the same sentence twice. */
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
