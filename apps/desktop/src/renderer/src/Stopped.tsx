// A Job that stopped, and the record it left behind.
//
// # Why this render is not the finished one with a different word at the top
//
// A finished Job is read once, to decide whether to take the work, so its two
// questions are what it was and what it produced. A Job that stopped produced
// nothing, so that second region would be empty on every one of them. The two
// questions here are the ones this state actually raises — what stopped it, and
// whether anything resumes it — and they hold the top of the page together, in
// one well, because a person who reads the first and not the second is the
// person who presses a button to find out.
//
// # The diagnosis stays at full weight; the archive folds
//
// The rail is what a person triages on, so it keeps its region rather than
// becoming a tab. What folds beneath it is everything that was reachable only
// from a database, a transcript on disk or the source: the moves the Job made,
// its Drone's turns, what it changed, and what it claimed. Each is a read, each
// is made when its own section opens, and none of them is made for a Job nobody
// unfolded.
//
// # What it was told is drawn once
//
// The brief sits in "Where the work is", both halves of it, because a person
// chasing a stopped Job asks what it was told and where its files are in one
// breath. So there is no record section for it — one value drawn twice is two
// places to keep in step.

import { useEffect, useState } from "react";
import {
  AFailedJobADeadEndReadAsOne,
  type JobDetailHeading,
  type JobRecordSection,
  type WorkflowRailStep,
} from "@armada/components";

import type { Diff, Evidence, History, Observed } from "../../shared/bridge";
import type {
  JobDetail as JobWhole,
  JobSummary,
} from "../../shared/protocol";
import type { ManifestSummary } from "../../shared/setup";
import { Changed, Claims, Moves, Turns } from "./Panels";
import { stoppedAt } from "./rail";
import { recourseOf } from "./recovery";
import { escalation } from "./render";
import { workOf } from "./work";

/**
 * Which section opens first. The history, because the rail above already
 * answers "what ran" and the next question a stopped Job raises is how it got
 * there — the one #123 was filed for, and the one that says whether a restart
 * will land in the same place.
 */
const FIRST = "moves";

/** The section that holds the transcript, and so the one that owns the socket. */
const TURNS = "turns";

/** The section that holds the submissions, and so the one that asks for them. */
const CLAIMS = "claims";

/**
 * The section that holds the diff.
 *
 * **The patch is read when this tab opens and dropped when it closes**, the
 * same bargain the finished record strikes: a record nobody unfolded has asked
 * no semantic question, which is what makes offering the expensive read here
 * affordable at all.
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

export type StoppedProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id`, or `null` while it has not arrived for this Job. */
  whole: JobWhole | null;
  manifest: ManifestSummary | undefined;
  /** What the second socket has said, where this Job's turns are being read. */
  observed: Observed;
  /** `GET /jobs/:job_id/events`, where the history section has asked for it. */
  history: History;
  /** `GET /jobs/:job_id/evidence`, where the claims section has asked for it. */
  evidence: Evidence;
  /** `GET /jobs/:job_id/diff`, where the diff section has asked for it. */
  diff: Diff;
  heading: JobDetailHeading;
  /** The rail, built by the caller — the running render draws the same one. */
  steps: WorkflowRailStep[];
  /** Why the rail has no rows, where it has none. */
  stepsAbsent?: string;
  /** Why there is nothing to name under "Where the work is". */
  workAbsent?: string;
  /**
   * Open or close the turns socket. **Driven by which section is open**, so a
   * record nobody has unfolded has opened nothing — the second socket exists
   * only while somebody is watching, and a tab is that somebody.
   */
  onWatchTurns: (on: boolean) => void;
  onCopied: (value: string) => void;
};

export function Stopped({
  job,
  whole,
  manifest,
  observed,
  history,
  evidence,
  diff,
  heading,
  steps,
  stepsAbsent,
  workAbsent,
  onWatchTurns,
  onCopied,
}: StoppedProps) {
  const [section, setSection] = useState(FIRST);

  // The socket follows the open section, and is closed on the way out. Nothing
  // about it is written onto the Job, so opening and closing it is free.
  useEffect(() => {
    onWatchTurns(section === TURNS);
    return () => onWatchTurns(false);
  }, [section, onWatchTurns]);

  // The three reads follow the open section too, and are dropped on the way
  // out. **Three effects and not one**, because they are three operations: the
  // moves are rows, the claims are four lines a step, and the patch is the
  // expensive half — a reader chasing the history must not pay for a megabyte.
  useEffect(() => {
    askForHistory(section === FIRST ? job.id : null);
    return () => askForHistory(null);
  }, [section, job.id]);

  useEffect(() => {
    askForClaims(section === CLAIMS ? job.id : null);
    return () => askForClaims(null);
  }, [section, job.id]);

  useEffect(() => {
    askForDiff(section === CHANGED ? job.id : null);
    return () => askForDiff(null);
  }, [section, job.id]);

  return (
    <AFailedJobADeadEndReadAsOne
      heading={heading}
      why={whyOf(job, whole)}
      recourse={recourseOf(job, whole).note}
      steps={steps}
      stepsAbsent={stepsAbsent}
      work={workOf(job, whole, manifest, true)}
      workAbsent={workAbsent}
      outputAbsent={NOT_SERVED_OUTPUT}
      record={recordOf({ job, whole, observed, history, evidence, diff, onCopied })}
      recordValue={section}
      onRecordChange={setSection}
      onCopied={onCopied}
    />
  );
}

/**
 * What the wire does not carry, said in the place the design puts it. The path
 * is served per Check run and is drawn on the gate row that owns it — a step
 * with three Checks wrote three files, and one region can only hold one. The
 * contents are not served, and Bridge does not read the filesystem, so naming
 * the file is the whole of what it can do.
 */
const NOT_SERVED_OUTPUT =
  "Each check names its output file on its own row. Nothing serves the contents.";

/**
 * Everything the Job left behind, folded.
 *
 * **Four sections and not the finished record's eight.** "Steps and checks" is
 * the region above rather than a tab, "What it was told" and "Where the work
 * is" are both in the region beside it, and a footprint section would say
 * `#127`'s sentence on every stopped Job Bridge did not happen to be open for —
 * a tab that is empty every time is the hole this record exists to close,
 * pointed the other way. The diff names the files, and it names them from a
 * read that a stopped Job can still make.
 *
 * **A section is a node and not a component reference**, so a module that lands
 * later takes a row in this list and nothing else moves. Only the open one is
 * built, because `JobRecord` renders one panel.
 */
function recordOf({
  job,
  whole,
  observed,
  history,
  evidence,
  diff,
  onCopied,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  observed: Observed;
  history: History;
  evidence: Evidence;
  diff: Diff;
  onCopied: (value: string) => void;
}): JobRecordSection[] {
  return [
    {
      id: FIRST,
      label: "Every move it made",
      panel: <Moves job={job} history={history} />,
    },
    {
      // After the moves, because the transitions say where to look and the
      // transcript is where looking happens.
      id: TURNS,
      label: "The drone's turns",
      panel: <Turns job={job} whole={whole} observed={observed} />,
    },
    {
      id: CHANGED,
      // `planNote` off: #157 reads the plan declaration out of the live working
      // slot, which a stopped Job no longer holds, so the sentence about the
      // declared plan would be false on every one of them.
      label: "What it changed",
      panel: <Changed job={job} diff={diff} planNote={false} onCopied={onCopied} />,
    },
    {
      id: CLAIMS,
      label: "What the drone claimed",
      panel: <Claims job={job} whole={whole} evidence={evidence} />,
    },
  ];
}

/**
 * Why a Job stopped: the reason's own verb, the criteria it still owes, and
 * the step it stopped at with what the gate found there. The label above it
 * supplies the grammar, so no sentence is composed around a word the registry
 * chose.
 *
 * **Where it stopped is stated even where no reason was stored.** Four of the
 * five statuses this screen draws store none — a failed Job, a killed one, a
 * rejected one and a superseded one all arrive with `reason` absent — and
 * without the step they say only that something ended. `stoppedAt` reads the
 * step and its Check runs, which are served; nothing here is inferred and
 * nothing is composed beyond the separators the rail already uses.
 */
function whyOf(job: JobSummary, whole: JobWhole | null) {
  const reason = escalation(job);
  const owed = job.reason?.criteria_owed ?? [];
  const at = whole === null ? undefined : stoppedAt(whole);
  if (reason?.verb == null && at === undefined) return undefined;
  return (
    <>
      {reason?.verb == null ? null : (
        <>
          {reason.verb}
          {owed.length === 0 ? null : (
            <>
              {" · owes "}
              <span className="mono">{owed.join(", ")}</span>
            </>
          )}
          {at === undefined ? null : " · "}
        </>
      )}
      {at === undefined ? null : (
        <>
          {"stopped at "}
          {at.labelIsAnIdentifier ? <span className="mono">{at.label}</span> : at.label}
          {at.check === undefined ? null : (
            <>
              {" · "}
              <span className="mono">{at.check}</span>
            </>
          )}
          {at.outputPath === undefined ? null : (
            <>
              {" · "}
              <span className="mono">{at.outputPath}</span>
            </>
          )}
        </>
      )}
    </>
  );
}
