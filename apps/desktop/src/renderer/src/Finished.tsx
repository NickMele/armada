// A finished Job, arranged around the two questions it is opened to answer.
//
// # Why this render is a different shape from the other two
//
// A running Job is watched, and the question is where it is now. A finished one
// is read once, to decide whether to take the work — so step state, per-step
// elapsed and a live rail answer a question nobody is asking. What this was and
// what came out hold the top of the page, and everything else is a section of
// the record beneath them.
//
// # Tucked away is not gone
//
// Every fact the other renders draw is still here and still one press away:
// the rail with its Checks and criterion verdicts, the context the Job was
// given, the worktree, the log and the transcript paths, and the Drone's turns.
// Nobody has to leave for a terminal to read the whole record.
//
// # What is not served is named where it would have gone
//
// Four of the five parts of "produced" are not on the wire. Each keeps its row
// in the outcome and names the operation that would have to serve it, because a
// region that closed up around the branch would draw a finished outcome that is
// a fifth of one.

import { useEffect, useState } from "react";
import {
  AFinishedJobWhatItWasAndWhatItProduced,
  ChangedFiles,
  JobBrief,
  JobLogReference,
  WorkflowRail,
  type JobDetailHeading,
  type JobOutcomePart,
  type JobRecordSection,
} from "@armada/components";
import { FileCheck, GitBranch, GitCommitHorizontal, GitPullRequest } from "lucide-react";

import type { Diff, Evidence, Footprint, History, Observed, Watched } from "../../shared/bridge";
import type {
  JobDetail as JobWhole,
  JobFilesChanged,
  JobSummary,
} from "../../shared/protocol";
import type { ManifestSummary } from "../../shared/setup";
import {
  filesOf,
  footprintNote,
  footprintSummary,
  NOT_SERVED_WHEN_FINISHED,
  readingFor,
} from "./files";
import { Changed, Claims, Moves, Turns } from "./Panels";
import { railOf } from "./rail";
import { briefOf, workOf } from "./work";

/** Which section of the record opens first. The rail, as the other renders lead. */
const FIRST = "steps";

/** The section that holds the transcript, and so the one that owns the socket. */
const TURNS = "turns";

/** The section that holds the history, and so the one that asks for the read. */
const MOVES = "moves";

/** The section that holds the submissions, and so the one that asks for them. */
const CLAIMS = "claims";

/**
 * The section that holds the diff.
 *
 * **The patch is read when this tab opens and dropped when it closes.**
 * `crates/adapter-traits/src/work_product.rs:110` separates the bytes from the
 * file list because most reads ask no semantic question; a record section
 * nobody unfolded has asked for none of them, which is what makes it
 * affordable to offer here at all.
 */
const CHANGED = "changed";

/**
 * Ask main for one Job's transition history, or drop it.
 *
 * **Module scope, so it is stable.** An effect depending on a lambda rebuilt
 * every render would open and close the read on a loop, and the read publishes
 * state, so the loop would feed itself.
 */
function askForHistory(jobId: string | null): void {
  void window.armada.readHistory(jobId);
}

/** The claims, and the patch. Module scope for `askForHistory`'s reason. */
function askForClaims(jobId: string | null): void {
  void window.armada.readEvidence(jobId);
}

function askForDiff(jobId: string | null): void {
  void window.armada.readDiff(jobId);
}

export type FinishedProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id`, or `null` while it has not arrived for this Job. */
  whole: JobWhole | null;
  watched: Watched;
  manifest: ManifestSummary | undefined;
  /** What the second socket has said, where this Job's turns are being read. */
  observed: Observed;
  /**
   * The last `job.files_changed` reading main holds, where the window was open
   * while this Job's Drone worked. `none` for a Job opened after it stopped.
   */
  footprint: Footprint;
  /** `GET /jobs/:job_id/events`, where the history section has asked for it. */
  history: History;
  /** `GET /jobs/:job_id/evidence`, where the claims section has asked for it. */
  evidence: Evidence;
  /** `GET /jobs/:job_id/diff`, where the diff section has asked for it. */
  diff: Diff;
  now: number;
  heading: JobDetailHeading;
  /**
   * Open or close the turns socket. **Driven by which section is open**, so a
   * record nobody has unfolded has opened nothing — the second socket exists
   * only while somebody is watching, and a tab is that somebody.
   */
  onWatchTurns: (on: boolean) => void;
  onCopied: (value: string) => void;
};

export function Finished({
  job,
  whole,
  watched,
  manifest,
  observed,
  footprint,
  history,
  evidence,
  diff,
  now,
  heading,
  onWatchTurns,
  onCopied,
}: FinishedProps) {
  const [section, setSection] = useState(FIRST);

  // The socket follows the open section, and is closed on the way out. Nothing
  // about it is written onto the Job, so opening and closing it is free.
  useEffect(() => {
    onWatchTurns(section === TURNS);
    return () => onWatchTurns(false);
  }, [section, onWatchTurns]);

  // The history read follows the open section too, and is dropped on the way
  // out. **Asked for here rather than handed down from the board**: the turns
  // socket swaps a whole screen and so belongs to navigation, and this does
  // not — nothing outside this record knows or cares that it was read.
  useEffect(() => {
    askForHistory(section === MOVES ? job.id : null);
    return () => askForHistory(null);
  }, [section, job.id]);

  // The claims and the patch follow the open section too, and are dropped on
  // the way out. **Two reads and not one**, because they are two operations:
  // the claims are four lines a step and the patch is the expensive half, and a
  // reader who only wants to know what was claimed must not pay for a megabyte.
  useEffect(() => {
    askForClaims(section === CLAIMS ? job.id : null);
    return () => askForClaims(null);
  }, [section, job.id]);

  useEffect(() => {
    askForDiff(section === CHANGED ? job.id : null);
    return () => askForDiff(null);
  }, [section, job.id]);

  const work = workOf(job, whole, manifest, false);
  const reading = readingFor(footprint, job.id);

  return (
    <AFinishedJobWhatItWasAndWhatItProduced
      heading={heading}
      brief={whole === null ? undefined : briefOf(whole)}
      briefAbsent={whyNotRead(watched, job.id, "acceptance criteria")}
      outcome={{ parts: producedOf(job, reading), note: HANDOVER_NOTE }}
      record={recordOf({
        job,
        whole,
        watched,
        work,
        observed,
        reading,
        history,
        evidence,
        diff,
        now,
        onCopied,
      })}
      recordValue={section}
      onRecordChange={setSection}
      recordAbsent={whyNotRead(watched, job.id, "record")}
      onCopied={onCopied}
    />
  );
}

/**
 * What the Job produced, one row per part.
 *
 * **One row is served and four are not.** The branch comes off the row, which
 * carries it and cannot disagree with the detail. The other four each name what
 * would have to serve them: `job.files_changed` exists and is published while a
 * Drone is working, which is not the same as a finished Job's footprint, and
 * the sentence says so rather than implying the event is missing.
 */
function producedOf(job: JobSummary, reading: JobFilesChanged | undefined): JobOutcomePart[] {
  return [
    {
      name: "Branch",
      icon: GitBranch,
      iconLabel: "Branch",
      value: job.branch,
      absent: NOT_SERVED.branch,
    },
    {
      name: "Commit",
      icon: GitCommitHorizontal,
      iconLabel: "Commit",
      absent: NOT_SERVED.commit,
    },
    {
      name: "Pull request",
      icon: GitPullRequest,
      iconLabel: "Pull request",
      absent: NOT_SERVED.pullRequest,
    },
    // No glyph. `file` is reserved to the log row and `file-check` to a
    // submission that landed, so a changed-file row has nothing in the registry
    // to take and none is invented. The mark column stays and renders empty.
    //
    // **The count is the last live reading, or nothing.** A footprint is only
    // held where this window was open while the Drone worked; a Job opened
    // after it stopped has none, and the row says why rather than showing a
    // zero that would read as a Drone that changed nothing.
    { name: "Files changed", ...counted(reading) },
    {
      name: "Evidence",
      icon: FileCheck,
      iconLabel: "Evidence",
      absent: NOT_SERVED.evidence,
    },
  ];
}

/**
 * The count of what was changed, or why there is none. `value` and `meta`
 * together, so the row never carries a count with no list behind it.
 */
function counted(reading: JobFilesChanged | undefined): Partial<JobOutcomePart> {
  return reading === undefined
    ? { absent: NOT_SERVED.filesChanged }
    : footprintSummary(reading);
}

/**
 * Everything else, folded. The order is what a reader asks for in order: what
 * ran, how it got there, what the Drone did, what it changed, what it was told,
 * where the files are.
 *
 * **A section is a node and not a component reference**, so a module that lands
 * later takes a row in this list and nothing else moves. Only the open one is
 * built, because `JobRecord` renders one panel.
 */
function recordOf({
  job,
  whole,
  watched,
  work,
  observed,
  reading,
  history,
  evidence,
  diff,
  now,
  onCopied,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  watched: Watched;
  work: ReturnType<typeof workOf>;
  observed: Observed;
  reading: JobFilesChanged | undefined;
  history: History;
  evidence: Evidence;
  diff: Diff;
  now: number;
  onCopied: (value: string) => void;
}): JobRecordSection[] {
  if (whole === null || work === undefined) return [];
  const rail = railOf(whole, now);
  return [
    {
      id: FIRST,
      label: "Steps and checks",
      panel:
        rail.length === 0 ? (
          <p className="text-fg-muted">This job&apos;s frozen workflow has no steps.</p>
        ) : (
          // Never pulsing: nothing is running, and a breathing mark on a Job
          // that is over would claim work that stopped.
          <WorkflowRail steps={rail} onCopied={onCopied} />
        ),
    },
    {
      // After the rail, because "what ran" is the question asked first and
      // "how did it get there" is the one asked when that answer surprises.
      id: MOVES,
      label: "Every move it made",
      panel: <Moves job={job} history={history} />,
    },
    {
      id: TURNS,
      label: "The drone's turns",
      panel: <Turns job={job} whole={whole} observed={observed} />,
    },
    {
      id: "files",
      label: "Files changed",
      panel:
        reading === undefined ? (
          <ChangedFiles files={[]} emptyNote={NOT_SERVED_WHEN_FINISHED} />
        ) : (
          <ChangedFiles
            files={filesOf(reading)}
            emptyNote={NOTHING_TOUCHED}
            note={footprintNote(reading, false)}
            onCopied={onCopied}
          />
        ),
    },
    {
      // After the file names, because a diff is the same question one level
      // deeper — and it is the one section that costs anything to open.
      id: CHANGED,
      label: "What it changed",
      panel: <Changed job={job} diff={diff} onCopied={onCopied} />,
    },
    {
      id: CLAIMS,
      label: "What the drone claimed",
      panel: <Claims job={job} whole={whole} evidence={evidence} />,
    },
    {
      id: "told",
      label: "What it was told",
      panel: <JobBrief {...briefOf(whole)} only="facts" />,
    },
    {
      id: "paths",
      label: "Where the work is",
      panel: (
        <JobLogReference rows={work.rows} actions={work.actions} onCopied={onCopied}>
          {work.note}
        </JobLogReference>
      ),
    },
  ];
}

/** What a footprint with no rows in it says. Ordinary, and never an error. */
const NOTHING_TOUCHED =
  "The last reading found no changed files. This drone had written nothing to the worktree.";

/**
 * What the wire does not carry, said in the row it would have filled. One
 * sentence each, naming what would have to serve it — a hole that names its
 * cause is a finding, one that reads "coming soon" is not.
 */
const NOT_SERVED = {
  branch: "This job has no worktree, so it has no branch.",
  commit: "Fleet does not commit at the last step yet, so there is nothing to name.",
  pullRequest: "Fleet does not open one yet, so there is nothing to open.",
  filesChanged: NOT_SERVED_WHEN_FINISHED,
  // Served since protocol 4.6, and read when the record's own section is
  // opened rather than here: the row would otherwise make every finished Job
  // opened pay for a read most of them are not opened to see.
  evidence: "Every submission is under What the drone claimed, in the record below.",
} as const;

/**
 * What is still owed after a Job finishes. **Armada does not push and does not
 * merge**, so the region that hands over a branch says what is left to do
 * rather than implying the work has landed.
 */
const HANDOVER_NOTE = "Armada does not push and does not merge. The branch is yours to take.";

/** Why a region has nothing yet, which is never the same sentence twice. */
function whyNotRead(watched: Watched, jobId: string, what: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return `Fleet did not answer for this job, so its ${what} is unknown.`;
  }
  return "Reading this job.";
}
