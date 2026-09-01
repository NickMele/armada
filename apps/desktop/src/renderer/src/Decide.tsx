// The decision on a Job waiting for one, and the diff it is made against.
//
// **Review and reply stay one loop.** They are the last chapter of the open
// step's story and the block under it — the same panel, one scroll apart, never
// two surfaces and never two panels. That constraint outlived the render this
// was lifted out of.
//
// # The two reads are asked for here, and dropped on the way out
//
// `crates/adapter-traits/src/work_product.rs:110` separates the patch from the
// file list because the bytes are large and most steps ask no semantic
// question. **This is the act they were separated for**, so the read is made by
// the region that draws it rather than folded into `JobDetail`, which is
// re-read every time an event names the open Job.
//
// # Reject confirms; approve does not
//
// Approving is the ordinary path and is why the gate exists — a gate that costs
// two presses for the common case is a gate in the wrong place. Rejecting ends
// the Job and the Drone, so it takes a dialog, and the dialog's words name the
// drone and name the milder act rather than asking whether you are sure.

import { useEffect, useState } from "react";
import { Dialog, ReviewDecision, UnifiedDiff, type UnifiedDiffProps } from "@armada/components";

import type { Diff, Evidence } from "../../shared/bridge";
import type { JobSummary } from "../../shared/protocol";
import type { Work } from "../../shared/work";
import {
  CHANGED_NOTHING,
  CONFIRM_REJECT,
  diffNote,
  drawn,
  NO_WORKTREE,
  whyNoDiff,
} from "./review";

/**
 * Ask main for one Job's evidence and one Job's diff, or drop both.
 *
 * **Module scope, so it is stable** — an effect depending on a lambda rebuilt
 * every render would open and close the read on a loop, and the read publishes
 * state, so the loop would feed itself.
 */
function askForMaterial(jobId: string | null): void {
  void window.armada.readEvidence(jobId);
}

export type DecideProps = {
  job: JobSummary;
  evidence: Evidence;
  diff: Diff;
  /** True while what is shown is not live. Every control is refused. */
  stale: boolean;
  /** A decision on this Job already in flight. */
  deciding: boolean;
  onApprove: (jobId: string) => void;
  onRequestChanges: (jobId: string, note: string) => void;
  onReject: (jobId: string) => void;
};

/**
 * The work this Job is asking about, drawn as the Produced chapter's content.
 *
 * Separate from the decision below it because the two are read in order and the
 * chapter that holds this one collapses — a decision that collapsed with the
 * diff would be a gate a person could scroll past.
 */
export function DecidedDiff({ diff, jobId }: { diff: Diff; jobId: string }) {
  const mine = diff.state !== "none" && diff.jobId === jobId ? diff : null;
  if (mine === null || mine.state !== "read") {
    return <p className="text-fg-muted">{whyNoDiff(diff, jobId)}</p>;
  }
  return <UnifiedDiff {...diffOf(mine.work)} />;
}

export function Decide({
  job,
  evidence,
  diff,
  stale,
  deciding,
  onApprove,
  onRequestChanges,
  onReject,
}: DecideProps) {
  // The reviewer's own words, held here: it is a draft until it is sent, and
  // nothing outside this region knows or cares that one is being written.
  const [note, setNote] = useState("");
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    askForMaterial(job.id);
    return () => askForMaterial(null);
  }, [job.id]);

  // A draft belongs to the Job it was written about. Carrying one into the next
  // Job opened would put one drone's feedback in front of another's work.
  useEffect(() => setNote(""), [job.id]);

  const off = stale || deciding;
  const why = stale ? NOT_LIVE : deciding ? IN_FLIGHT : undefined;

  return (
    <>
      <ReviewDecision
        note={note}
        onNote={setNote}
        onApprove={() => onApprove(job.id)}
        onRequestChanges={() => onRequestChanges(job.id, note)}
        onReject={() => setConfirming(true)}
        disabled={off}
        {...(why === undefined ? {} : { disabledNote: why })}
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

      {/* Named, not read: the evidence is what the claims section of the record
          draws, and a second reading of it beside the decision would be the
          same value in two places. */}
      {evidence.state === "failed" && evidence.jobId === job.id ? (
        <p className="text-fg-muted">
          What this drone claimed could not be read. The record below names the same failure.
        </p>
      ) : null}
    </>
  );
}

/**
 * The diff region, or the sentence that says which silence this is.
 *
 * **Three answers, three sentences.** A Job with no worktree, a Drone that
 * changed nothing, and a patch git rendered are three different facts, and the
 * shape keeps them apart all the way from the wire.
 */
function diffOf(work: Work | undefined): UnifiedDiffProps {
  if (work === undefined) return { files: [], emptyNote: NO_WORKTREE };
  const { files, cut } = drawn(work);
  return {
    files,
    emptyNote: CHANGED_NOTHING,
    ...(cut === undefined ? {} : { cut }),
    // Readable, and this is the one state where it is: a Job held at
    // `awaiting_review` keeps its Drone, so the slot `get_diff` reads the
    // declaration out of is still the one that made it.
    note: diffNote(work, true),
  };
}

/** Why every control is off, which is never the same sentence twice. */
const NOT_LIVE = "Fleet is not connected, so nothing here can be sent.";
const IN_FLIGHT = "A decision on this job is already in flight. It was not sent twice.";
