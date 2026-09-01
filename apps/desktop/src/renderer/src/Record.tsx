// Everything the Job left behind, folded — one region, at every state.
//
// **Lifted out of the finished and stopped renders, which each had their own.**
// One had eight sections and the other five, and the difference was never about
// the Job: it was about which screen a status happened to route to. Job detail
// is one arrangement now, so the record is one region in one place, and a
// reader who learns where it is on a running Job finds it in the same place on
// a dead one.
//
// **A section is a node, not a component reference**, so a module that lands
// later takes a row in this list and nothing else moves. Only the open one is
// built, because `JobRecord` renders one panel — which is what makes offering
// the expensive reads here affordable at all.

import { useEffect, useState } from "react";
import { ChangedFiles, JobRecord, type JobRecordSection } from "@armada/components";

import type { Diff, Evidence, History, Observed } from "../../shared/bridge";
import type { JobDetail as JobWhole, JobSummary } from "../../shared/protocol";
import {
  NO_FOOTPRINT_RECORDED,
  planRecorded,
  recordNote,
  touchedOf,
  TOUCHED_NOTHING,
} from "./files";
import { Changed, Claims, Moves, Turns } from "./Panels";

/**
 * Which section opens first. The moves, because the run above already answers
 * what ran and the next question is how it got there — the one that says
 * whether a restart will land in the same place.
 */
const MOVES = "moves";
/** The section that holds the transcript, and so the one that owns the socket. */
const TURNS = "turns";
/** The section that holds the submissions, and so the one that asks for them. */
const CLAIMS = "claims";
/** The footprint. **It asks for nothing** — it arrives with the Job. */
const FILES = "files";
/** The patch. Read when this tab opens and dropped when it closes. */
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

export type RecordProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id`, or `null` while it has not arrived for this Job. */
  whole: JobWhole | null;
  /** What the second socket has said, where this Job's turns are being read. */
  observed: Observed;
  history: History;
  evidence: Evidence;
  diff: Diff;
  /**
   * Whether this Job's diff is readable from the slot its own Drone holds.
   * **False once the Drone has gone**: `get_diff` takes the declaration from
   * that slot, so a stopped Job's plan is unreadable and the note says so
   * rather than reporting a step that declared a plan as one that declared
   * none.
   */
  planReadable: boolean;
  /**
   * Open or close the turns socket. **Driven by which section is open**, so a
   * record nobody has unfolded has opened nothing.
   */
  onWatchTurns: (on: boolean) => void;
  onCopied: (value: string) => void;
};

export function Record({
  job,
  whole,
  observed,
  history,
  evidence,
  diff,
  planReadable,
  onWatchTurns,
  onCopied,
}: RecordProps) {
  const [section, setSection] = useState(MOVES);

  useEffect(() => {
    onWatchTurns(section === TURNS);
    return () => onWatchTurns(false);
  }, [section, onWatchTurns]);

  // Three effects and not one, because they are three operations: the moves are
  // rows, the claims are four lines a step, and the patch is the expensive
  // half — a reader chasing the history must not pay for a megabyte.
  useEffect(() => {
    askForHistory(section === MOVES ? job.id : null);
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

  // The record Fleet wrote at the terminal transition, and never a live
  // reading: `job.files_changed` stops arriving when the Drone goes, so what
  // main holds is whatever landed while this window happened to be open.
  const touched = whole?.footprint;

  const sections: JobRecordSection[] = [
    { id: MOVES, label: "Every move it made", panel: <Moves job={job} history={history} /> },
    {
      // After the moves, because the transitions say where to look and the
      // transcript is where looking happens.
      id: TURNS,
      label: "The drone's turns",
      panel: <Turns job={job} whole={whole} observed={observed} />,
    },
    {
      // Before the diff, because the file list is the same question one level
      // up and it is the cheap half — it arrives with the Job.
      id: FILES,
      label: "Files changed",
      panel:
        touched === undefined ? (
          <ChangedFiles files={[]} emptyNote={NO_FOOTPRINT_RECORDED} />
        ) : (
          <ChangedFiles
            files={touchedOf(touched)}
            emptyNote={TOUCHED_NOTHING}
            note={recordNote(touched)}
            onCopied={onCopied}
          />
        ),
    },
    {
      id: CHANGED,
      label: "What it changed",
      panel: (
        <Changed
          job={job}
          diff={diff}
          planReadable={planReadable}
          markedInRecord={touched !== undefined && planRecorded(touched)}
          onCopied={onCopied}
        />
      ),
    },
    { id: CLAIMS, label: "What the drone claimed", panel: <Claims job={job} whole={whole} evidence={evidence} /> },
  ];

  return <JobRecord sections={sections} value={section} onChange={setSection} />;
}
