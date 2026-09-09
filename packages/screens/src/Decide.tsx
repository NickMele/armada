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
// # Merge and reject confirm; approve and request changes do not
//
// Approving is the ordinary path and is why the gate exists — a gate that costs
// two presses for the common case is a gate in the wrong place. Rejecting ends
// the Job and the Drone, so it takes a dialog, and the dialog's words name the
// drone and name the milder act rather than asking whether you are sure.
//
// **Merging takes one for a different reason, and it is the reason it is not
// the same dialog.** Rejecting is terminal inside Armada; merging is the one
// act here that writes into a repository Fleet did not make, so what it costs
// is not a Job but what everybody else builds on — `crates/fleet/src/merging.rs`
// opens on that sentence and writes its log line before the write for it. The
// other two answers stay one press, because each is confined to a worktree
// Fleet cut.
//
// # Merging is offered where there is a pull request, and nowhere else
//
// `pullRequest` is a value off the Job's own detail, and its presence is the
// whole of what decides the control. A workflow that declares no delivering
// step opened none — `design-plan`, `code-review`, `epic` and `prototype` are
// that shape — and so did a Job whose push failed; on either, Fleet answers
// `fleet.nothing_to_merge`, and an act a person can press to be refused is a
// worse surface than one that is not drawn.
//
// **It confirms, decided on 2026-09-08.** It shipped in #532 without one, on
// the reading that it is the ordinary ending of a Job whose branch already went
// out and that `deciding` already blocks a second press. `deciding` is about the
// second press; this is about the first one being deliberate, and the act is
// still the only one on this surface that reaches outside a worktree Fleet
// made. The dialog is what stands between the press and the write. #533.

import { useEffect, useState } from "react";
import { Dialog, ReviewDecision, UnifiedDiff, type UnifiedDiffProps } from "@armada/components";

import type { Diff, Evidence } from "@armada/protocol";
import type { JobSummary } from "@armada/protocol";
import type { Work } from "@armada/protocol";
import {
  CHANGED_NOTHING,
  CONFIRM_MERGE,
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
export type DecideProps = {
  /**
   * Ask the host to open or close the evidence read for a Job, or for none.
   *
   * **It has to be stable**, for the reason the reports read is: an effect
   * depends on it, and a lambda rebuilt every render would open and close the
   * read on a loop that feeds itself.
   */
  onNeedMaterial: (jobId: string | null) => void;
  job: JobSummary;
  evidence: Evidence;
  diff: Diff;
  /** True while what is shown is not live. Every control is refused. */
  stale: boolean;
  /** A decision on this Job already in flight. */
  deciding: boolean;
  /**
   * The address of the pull request this Job's branch went out on, where it
   * has one. **Absent is a Job with nothing to merge**, and it is what decides
   * whether the merge control is drawn at all.
   */
  pullRequest?: string;
  /** Merge that pull request, then take the work. */
  onMerge: (jobId: string) => void;
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
  onNeedMaterial,
  job,
  evidence,
  diff,
  stale,
  deciding,
  pullRequest,
  onMerge,
  onApprove,
  onRequestChanges,
  onReject,
}: DecideProps) {
  // The reviewer's own words, held here: it is a draft until it is sent, and
  // nothing outside this region knows or cares that one is being written.
  const [note, setNote] = useState("");
  // Which answer is being confirmed, and never two flags. Both dialogs are
  // modal and only one act is in flight at a time, so a union says that in the
  // type rather than leaving a state where both layers are up.
  const [asking, setAsking] = useState<"merge" | "reject" | null>(null);

  useEffect(() => {
    onNeedMaterial(job.id);
    return () => onNeedMaterial(null);
  }, [job.id]);

  // A draft belongs to the Job it was written about. Carrying one into the next
  // Job opened would put one drone's feedback in front of another's work.
  //
  // **A question in the air belongs to it too**, and this one is why the reset
  // is not only about the note: `onConfirm` reads `job.id` at press time, so a
  // dialog left standing across a switch would ask about the Job that was on
  // screen and answer for the one that is. It closes with the Job it was asked
  // about.
  useEffect(() => {
    setNote("");
    setAsking(null);
  }, [job.id]);

  const off = stale || deciding;
  const why = stale ? NOT_LIVE : deciding ? IN_FLIGHT : undefined;

  return (
    <>
      <ReviewDecision
        note={note}
        onNote={setNote}
        {...(pullRequest === undefined ? {} : { onMerge: () => setAsking("merge") })}
        onApprove={() => onApprove(job.id)}
        onRequestChanges={() => onRequestChanges(job.id, note)}
        onReject={() => setAsking("reject")}
        disabled={off}
        {...(why === undefined ? {} : { disabledNote: why })}
      />

      {/* The one act here that writes outside a worktree Fleet made.
          **Neutral, not destructive**: nothing ends and nothing is destroyed,
          and the confirm keeps the accent fill the control a person just
          pressed had — a red confirm would make the merge read as an error
          state on its way in. `restart_step` is the same reading, and it is the
          only other neutral confirmation Bridge draws.

          The confirm carries the opener's own words, because the contract says
          an act keeps its name through the flow. */}
      <Dialog
        open={asking === "merge"}
        tone="neutral"
        title={CONFIRM_MERGE.title}
        confirmLabel="Merge and take the work"
        onCancel={() => setAsking(null)}
        onConfirm={() => {
          setAsking(null);
          onMerge(job.id);
        }}
      >
        {CONFIRM_MERGE.body}
      </Dialog>

      {/* The one act on this surface that ends something. Cancel holds initial
          focus; the dialog owns that rule and this only supplies the words. */}
      <Dialog
        open={asking === "reject"}
        tone="destructive"
        title={CONFIRM_REJECT.title}
        confirmLabel="Reject the work"
        onCancel={() => setAsking(null)}
        onConfirm={() => {
          setAsking(null);
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
