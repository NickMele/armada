// The panels a finished Job's record folds away, one component each.
//
// Split out of `Finished.tsx` when that file grew past the gate's 500-line
// warning, the same way `Acts.tsx` came out of `JobDetail.tsx`. The screen
// beside this decides which section is open and asks for the read that section
// needs; these draw what came back, and each of them says in its own words why
// there is nothing yet — a shared sentence would make "nobody asked", "still
// reading" and "the read failed" one state on screen when they are three.
//
// **Every one of them is read-only.** Nothing here can reach a Drone, record a
// transition or spend a byte that was not already fetched.

import {
  Alert,
  DroneTurns,
  EvidenceTrail,
  TransitionHistory,
  UnifiedDiff,
} from "@armada/components";

import type { Diff, Evidence, History, Observed } from "../../shared/bridge";
import type { JobDetail as JobWhole, JobSummary } from "../../shared/protocol";
import { said } from "./copy";
import { movesOf, NOTHING_RECORDED as NO_MOVES, WHAT_THIS_IS } from "./history";
import {
  CHANGED_NOTHING,
  CLAIMED_NOTHING,
  claimsOf,
  diffNote,
  drawn,
  NO_WORKTREE,
  whyNoClaims,
  whyNoDiff,
} from "./review";
import { turnsOf } from "./turns";

/**
 * One Job's transition history, as a section of its record.
 *
 * **The rows are drawn, never folded.** `crates/store/src/fold.rs` owns the
 * machine, and Fleet loads the Job before it reads the log — so a history that
 * arrives is one the machine already admitted. A read that failed says so
 * rather than drawing an empty list, which would read as a Job that never
 * moved: the one thing this section exists to tell apart.
 */
export function Moves({ job, history }: { job: JobSummary; history: History }) {
  const mine = history.state !== "none" && history.jobId === job.id;
  if (!mine || history.state === "reading") {
    return <TransitionHistory moves={[]} emptyNote="Reading this job's history." />;
  }
  if (history.state === "failed") {
    return (
      <Alert tone="escalated" title="This job's history could not be read">
        {said(history.outcome)}
      </Alert>
    );
  }
  return (
    <TransitionHistory moves={movesOf(history.moves)} emptyNote={NO_MOVES} note={WHAT_THIS_IS} />
  );
}

/**
 * One Job's turns, as a section of its record.
 *
 * **Read-only, and there is no way for it to be anything else** — the preload
 * entry behind it opens a socket that only receives. The three quiet cases stay
 * three sentences: a Job that was never dispatched, a bounded backfill that
 * left rows out, and a viewer that fell behind. A transcript with a silent gap
 * reads as a Drone that went quiet, which is the one thing this record exists
 * to tell apart.
 *
 * `whole` is here for the step boundaries and nothing else: the transcript
 * carries `step_id`s and the frozen workflow is what names them.
 */
export function Turns({
  job,
  whole,
  observed,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  observed: Observed;
}) {
  const mine = observed.state !== "none" && observed.jobId === job.id;
  if (!mine || observed.state === "opening") {
    return <DroneTurns turns={[]} emptyNote="Reading this job's turns." />;
  }
  if (observed.state === "failed") {
    return (
      <Alert tone="escalated" title="This job's turns could not be read">
        {observed.detail}
      </Alert>
    );
  }
  const turns = observed.turns;
  return (
    <>
      {turns.missed > 0 ? (
        <Alert tone="escalated" title="Rows were dropped before this window saw them">
          {`${turns.missed} turns will never arrive. What follows is everything else, in order.`}
        </Alert>
      ) : null}
      {turns.skipped > 0 ? (
        <Alert tone="neutral" title="Older turns are not shown">
          {`${turns.skipped} earlier turns are on disk and were left out of this history.`}
        </Alert>
      ) : null}
      <DroneTurns
        turns={turnsOf(turns.rows, whole)}
        emptyNote={NOTHING_RECORDED}
        live={turns.live}
      />
    </>
  );
}

/**
 * One Job's submissions, as a section of its record.
 *
 * **The claims are drawn and never summarised.** A work submission is a signal
 * and never the source of truth; a paragraph assembled from three fields here
 * would be a Drone's own account presented as the record, which is the
 * distinction the whole verification loop exists to hold.
 */
export function Claims({
  job,
  whole,
  evidence,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  evidence: Evidence;
}) {
  const mine = evidence.state !== "none" && evidence.jobId === job.id;
  if (!mine || evidence.state === "reading") {
    return <p className="text-fg-muted">{whyNoClaims(evidence, job.id)}</p>;
  }
  if (evidence.state === "failed") {
    return (
      <Alert tone="escalated" title="This job's evidence could not be read">
        {said(evidence.outcome)}
      </Alert>
    );
  }
  if (evidence.steps.length === 0) {
    return <p className="text-fg-muted">{CLAIMED_NOTHING}</p>;
  }
  return <EvidenceTrail entries={claimsOf(evidence.steps, whole)} />;
}

/**
 * One Job's diff, as a section of its record.
 *
 * **The bytes are spent on this tab and nowhere else on the page.** Armada
 * leaves a finished Job's worktree in place, so the reading is real long after
 * the Drone has gone — and where it has been reclaimed, `work` is absent and
 * says so rather than reading as a Drone that changed nothing.
 */
export function Changed({
  job,
  diff,
  planReadable = true,
  markedInRecord = false,
  onCopied,
}: {
  job: JobSummary;
  diff: Diff;
  /**
   * Whether a Drone is still holding the pen on this Job, and so whether the
   * plan it declared can be read at all.
   *
   * **False on a stopped Job and on a finished one.** `get_diff` takes the
   * declaration from the slot this Job's own Drone holds, so `plan_declared`
   * comes back false for a step that declared one — and the note beneath the
   * diff then says the declaration is unreadable rather than saying none was
   * made. The caller knows which of the two it is drawing and the wire does
   * not. **Not "the working slot", which is what this said**: Fleet works
   * several Jobs at once, and the slot in question is this Job's.
   */
  planReadable?: boolean;
  /**
   * Whether the footprint on this Job's `JobDetail` carries a declaration, and
   * so whether the record beside this tab already marks the drift.
   *
   * **What keeps two tabs of one record from answering one question two ways.**
   * The declaration is unreadable here either way; where it is readable in the
   * record, the note says where rather than stopping at the silence.
   */
  markedInRecord?: boolean;
  onCopied: (value: string) => void;
}) {
  const mine = diff.state !== "none" && diff.jobId === job.id;
  if (!mine || diff.state === "reading") {
    return <p className="text-fg-muted">{whyNoDiff(diff, job.id)}</p>;
  }
  if (diff.state === "failed") {
    return (
      <Alert tone="escalated" title="This job's diff could not be read">
        {said(diff.outcome)}
      </Alert>
    );
  }
  const work = diff.work;
  if (work === undefined) {
    return <UnifiedDiff files={[]} emptyNote={NO_WORKTREE} />;
  }
  const { files, cut } = drawn(work);
  return (
    <UnifiedDiff
      files={files}
      emptyNote={CHANGED_NOTHING}
      cut={cut}
      note={diffNote(work, planReadable, markedInRecord)}
      onCopied={onCopied}
    />
  );
}

/**
 * What a Job with nothing recorded says. Ordinary, and never an error — and
 * deliberately not the sentence a Job whose read failed gets.
 */
const NOTHING_RECORDED =
  "This job has no turns. Nothing was writing when this opened, so this is the whole history.";
