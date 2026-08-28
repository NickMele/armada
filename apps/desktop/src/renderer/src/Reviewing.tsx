// A Job waiting on a person to take its work, and the three answers to it.
//
// # Why this is a render and not a section of the finished record
//
// #112 recorded that review is not a record section, because the diff and the
// reply are one loop. This goes one further: it is not reached from a finished
// Job at all. All three acts refuse anywhere but `awaiting_review`, which is a
// non-terminal status — by the time the finished render draws, every one of
// them is a 409. And `awaiting_review` without this took the *running* render,
// which puts a live rail and a per-step elapsed on a Job that stopped and is
// waiting. So the decision is drawn on the status it is legal on, and that
// status gets a screen.
//
// # The two reads are asked for here, and dropped on the way out
//
// `crates/adapter-traits/src/work_product.rs:110` separates the patch from the
// file list because the bytes are large and most steps ask no semantic
// question. **This is the act they were separated for**, so the read is made by
// the surface that draws it rather than folded into `JobDetail`, which is
// re-read every time an event names the open Job.
//
// # Reject confirms; approve does not
//
// Approving is the ordinary path and it is why the gate exists — a gate that
// costs two presses for the common case is a gate in the wrong place, which is
// the argument `approve_dispatch` already carries. Rejecting ends the Job and
// the Drone, so it takes a dialog, and the dialog's words name the drone and
// name the milder act rather than asking whether you are sure.

import { useEffect, useState } from "react";
import {
  AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop,
  Dialog,
  type JobDetailHeading,
  type ReviewDecisionProps,
  type UnifiedDiffProps,
} from "@armada/components";

import type { Diff, Evidence, Watched } from "../../shared/bridge";
import type { Work } from "../../shared/work";
import type {
  JobDetail as JobWhole,
  JobSummary,
  ManifestSummary,
} from "../../shared/protocol";
import { briefOf, whyNoWork, workOf } from "./work";
import {
  CHANGED_NOTHING,
  CLAIMED_NOTHING,
  CONFIRM_REJECT,
  claimsOf,
  diffNote,
  drawn,
  NO_WORKTREE,
  whyNoClaims,
  whyNoDiff,
} from "./review";

/**
 * Ask main for one Job's evidence and one Job's diff, or drop both.
 *
 * **Module scope, so they are stable.** An effect depending on a lambda rebuilt
 * every render would open and close the read on a loop, and the read publishes
 * state, so the loop would feed itself. The same shape the history read takes.
 */
function askForMaterial(jobId: string | null): void {
  void window.armada.readEvidence(jobId);
  void window.armada.readDiff(jobId);
}

export type ReviewingProps = {
  job: JobSummary;
  /** `GET /jobs/:job_id`, or `null` while it has not arrived for this Job. */
  whole: JobWhole | null;
  watched: Watched;
  manifest: ManifestSummary | undefined;
  /** `GET /jobs/:job_id/evidence`, as main published it. */
  evidence: Evidence;
  /** `GET /jobs/:job_id/diff`, as main published it. */
  diff: Diff;
  /** True while what is shown is not live. Every control is refused. */
  stale: boolean;
  /** A decision already in flight on this Job. A second press sends nothing. */
  deciding: boolean;
  heading: JobDetailHeading;
  /** Take the work. Sent on the press — see the header of this file. */
  onApprove: (jobId: string) => void;
  /** Send it back with the reviewer's own words. */
  onRequestChanges: (jobId: string, note: string) => void;
  /** End the Job and the Drone. Confirmed here before it is sent. */
  onReject: (jobId: string) => void;
  onCopied: (value: string) => void;
};

export function Reviewing({
  job,
  whole,
  watched,
  manifest,
  evidence,
  diff,
  stale,
  deciding,
  heading,
  onApprove,
  onRequestChanges,
  onReject,
  onCopied,
}: ReviewingProps) {
  // The reviewer's own words, held here: it is a draft until it is sent, and
  // nothing outside this screen knows or cares that one is being written.
  const [note, setNote] = useState("");
  const [confirming, setConfirming] = useState(false);

  // Both reads follow the Job on screen and are dropped on the way out. Asked
  // for here rather than handed down from the board, for the reason the history
  // read is: nothing outside this surface wants either of them.
  useEffect(() => {
    askForMaterial(job.id);
    return () => askForMaterial(null);
  }, [job.id]);

  // A draft belongs to the Job it was written about. Carrying one into the next
  // Job opened would put one drone's feedback in front of another's work.
  useEffect(() => setNote(""), [job.id]);

  const mineDiff = diff.state !== "none" && diff.jobId === job.id ? diff : null;
  const mineClaims = evidence.state !== "none" && evidence.jobId === job.id ? evidence : null;

  return (
    <>
      <AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop
        heading={heading}
        brief={whole === null ? undefined : briefOf(whole)}
        briefAbsent={whyNoBrief(watched, job.id)}
        claims={
          mineClaims === null || mineClaims.state !== "read"
            ? undefined
            : { entries: claimsOf(mineClaims.steps, whole) }
        }
        claimsAbsent={
          mineClaims?.state === "read" ? CLAIMED_NOTHING : whyNoClaims(evidence, job.id)
        }
        diff={mineDiff === null || mineDiff.state !== "read" ? undefined : diffOf(mineDiff.work)}
        diffAbsent={whyNoDiff(diff, job.id)}
        decision={decisionOf({
          note,
          setNote,
          off: stale || deciding,
          why: stale ? NOT_LIVE : deciding ? IN_FLIGHT : undefined,
          onApprove: () => onApprove(job.id),
          onRequestChanges: () => onRequestChanges(job.id, note),
          onReject: () => setConfirming(true),
        })}
        work={workOf(job, whole, manifest, true)}
        workAbsent={whyNoWork(watched, job.id)}
        onCopied={onCopied}
      />

      {/* The one act on this surface that ends something, and the only one that
          confirms. Cancel holds initial focus; the dialog owns that rule and
          this only supplies the words. */}
      <Dialog
        open={confirming}
        tone="destructive"
        title={CONFIRM_REJECT.title}
        confirmLabel="Reject the work"
        onCancel={() => setConfirming(false)}
        onConfirm={() => {
          setConfirming(false);
          onReject(job.id);
        }}
      >
        {CONFIRM_REJECT.body}
      </Dialog>
    </>
  );
}

/**
 * The diff region, or the sentence that says which silence this is.
 *
 * **Three answers, three sentences.** A Job with no worktree, a Drone that
 * changed nothing, and a patch git rendered are three different facts, and the
 * shape keeps them apart all the way from the wire: `work` absent is the first,
 * `work.patch` absent is the second.
 */
function diffOf(work: Work | undefined): UnifiedDiffProps {
  if (work === undefined) return { files: [], emptyNote: NO_WORKTREE };
  const { files, cut } = drawn(work);
  return {
    files,
    emptyNote: CHANGED_NOTHING,
    ...(cut === undefined ? {} : { cut }),
    note: diffNote(work),
  };
}

/** The decision region. Always offered: this status is what makes it legal. */
function decisionOf({
  note,
  setNote,
  off,
  why,
  onApprove,
  onRequestChanges,
  onReject,
}: {
  note: string;
  setNote: (note: string) => void;
  off: boolean;
  why: string | undefined;
  onApprove: () => void;
  onRequestChanges: () => void;
  onReject: () => void;
}): ReviewDecisionProps {
  return {
    note,
    onNote: setNote,
    onApprove,
    onRequestChanges,
    onReject,
    disabled: off,
    ...(why === undefined ? {} : { disabledNote: why }),
  };
}

/** Why every control is off, which is never the same sentence twice. */
const NOT_LIVE = "Fleet is not connected, so nothing here can be sent.";
const IN_FLIGHT = "A decision on this job is already in flight. It was not sent twice.";

/** Why there is no brief, which is never the same sentence twice. */
function whyNoBrief(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this job, so what done meant for it is unknown.";
  }
  return "Reading this job.";
}
