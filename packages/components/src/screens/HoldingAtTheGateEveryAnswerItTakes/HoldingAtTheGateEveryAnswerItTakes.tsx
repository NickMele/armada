import { useState } from "react";
import { Dialog } from "../../primitives/Dialog/Dialog";
import {
  ReviewComments,
  type ReviewComment,
} from "../../compositions/ReviewComments/ReviewComments";
import { ReviewDecision } from "../../compositions/ReviewDecision/ReviewDecision";
import { VerdictSheet, type VerdictSheetProps } from "../../compositions/VerdictSheet/VerdictSheet";
import { InsideAJob } from "../InsideAJob/InsideAJob";
import type { InsideAJobProps } from "../InsideAJob/InsideAJob";

/**
 * A Job holding at a human gate, at every shape the answer takes.
 *
 * **The same `InsideAJob` the sheet beside this one draws, with one slot
 * filled.** Nothing about the arrangement changes at a gate: the run is still
 * on the left, the step is still the panel, and the decision is the block after
 * the story — one scroll from the diff it is made against, never a second
 * surface and never a second panel. `docs/practices/bridge.md` is where that
 * constraint is written down, and it is the one v1 broke.
 *
 * **What changes between these states is whether there is a pull request**, and
 * that single fact moves three things: the fourth answer appears, the accent
 * fill moves from Approve to Merge, and the comments block is drawn at all.
 * `Decide.tsx` decides all three off `pullRequest` being present, so this does
 * too rather than taking a flag.
 *
 * **The block after the story is a verdict sheet now, since #10.2.** What was
 * asked for, what came back, what proves it and what it left alone wrap the
 * same three answers this file always drew — `verdict` is the sheet's own data,
 * minus `actions`, which stays this file's.
 *
 * # This is `Decide`'s arrangement without `Decide`'s reads
 *
 * `packages/screens/src/Decide.tsx` and `packages/screens/src/verdict.tsx`
 * compose exactly these parts — the record, the decision, the merge
 * confirmation and the comments — and hold exactly this state: the reviewer's
 * draft note, and which answer is being confirmed. Neither can be imported
 * here: `@armada/components` does not depend on `@armada/screens` and must
 * not, so the arrangement is stated twice and the second statement is this
 * one. Reported, and the same seam `ActivityLog.stories.tsx` names over its
 * own quoted constant.
 */
export type HoldingAtTheGateProps = InsideAJobProps & {
  /** The answers, and what each one reaches. */
  answers: TheAnswersProps;
  /** The verdict sheet's own data — everything `VerdictSheet` takes besides `actions`. */
  verdict: Omit<VerdictSheetProps, "actions" | "recordNote">;
};

export function HoldingAtTheGate({ answers, verdict, step, ...rest }: HoldingAtTheGateProps) {
  return (
    <div className="armada-screen">
      <InsideAJob
        {...rest}
        step={
          step === undefined
            ? undefined
            : { ...step, after: <VerdictSheet {...verdict} actions={<TheAnswers {...answers} />} /> }
        }
      />
    </div>
  );
}

export type TheAnswersProps = {
  /**
   * Merge the pull request and take the work. **Absent is a Job with no pull
   * request to merge** — a workflow that declares no delivering step opened
   * none, and so did one whose push failed. Presence is the whole of what
   * decides the control, exactly as it is in `Decide`.
   */
  onMerge?: () => void;
  /**
   * What people wrote on the pull request. **Absent is the same fact `onMerge`
   * absent is**: with no pull request there is nothing to read, so the block is
   * not drawn rather than drawn empty.
   */
  comments?: readonly ReviewComment[];
  /** Which confirmation is drawn open. A story states a moment. */
  confirming?: "merge" | "reject";
};

/**
 * The decision, the two confirmations and the comments — the block under the
 * story, in the order a person reads them.
 *
 * **Exported because the sheet next door draws it too.** A Job at the mid-run
 * gate is the same block with no pull request behind it, and two spellings of
 * one decision on two sheets is the drift this whole change is about.
 */
export function TheAnswers({ onMerge, comments, confirming }: TheAnswersProps) {
  // Held here because it is a draft until it is sent, which is where `Decide`
  // holds it and why the field is on the surface rather than behind a control.
  const [note, setNote] = useState("");
  const [asking, setAsking] = useState<"merge" | "reject" | null>(confirming ?? null);

  return (
    <>
      <ReviewDecision
        note={note}
        onNote={setNote}
        {...(onMerge === undefined ? {} : { onMerge: () => setAsking("merge") })}
        onApprove={nothingPressedYet}
        onRequestChanges={nothingPressedYet}
        onReject={() => setAsking("reject")}
      />

      {/* Neutral, not destructive: nothing ends and nothing is destroyed, and
          the confirm keeps the accent fill the control a person just pressed
          had. The words are `CONFIRM_MERGE`'s. */}
      <Dialog
        open={asking === "merge"}
        tone="neutral"
        title={CONFIRM_MERGE.title}
        confirmLabel="Merge and take the work"
        onCancel={() => setAsking(null)}
        onConfirm={() => setAsking(null)}
      >
        {CONFIRM_MERGE.body}
      </Dialog>

      {/* The one act on this surface that ends something. */}
      <Dialog
        open={asking === "reject"}
        tone="destructive"
        title={CONFIRM_REJECT.title}
        confirmLabel="Reject the work"
        onCancel={() => setAsking(null)}
        onConfirm={() => setAsking(null)}
      >
        {CONFIRM_REJECT.body}
      </Dialog>

      {comments === undefined ? null : (
        <ReviewComments comments={comments} onTakeUp={nothingPressedYet} />
      )}
    </>
  );
}

/** A drawing, not a wiring. Every press here is recorded and answered nowhere. */
const nothingPressedYet = () => {};

/**
 * What the merge confirmation says.
 *
 * **Quoted from `CONFIRM_MERGE` in `packages/screens/src/review.ts`**, which is
 * where the product reads it. A component cannot depend on `@armada/screens` to
 * import the constant — the dependency runs the other way and a cycle would be
 * the fix that breaks the build — so the sentence is copied and its owner is
 * named on the line above. That is the seam `ActivityLog.stories.tsx` already
 * records, and it is exactly the seam this change was filed about: a fixture
 * string with no owner is one that drifts and nothing says so.
 */
const CONFIRM_MERGE = {
  title: "Merge this job's pull request?",
  body:
    "The pull request merges on the forge, so the job's commits land on the base branch and " +
    "everybody working from it gets them on their next pull. Armada then runs the " +
    "repository's after-merge checks against what landed — merging on the forge instead " +
    "skips them, and running them is the reason this button exists. The job is approved with " +
    "it. Bridge cannot take a merge back: undoing one is a revert made in the repository.",
} as const;

/** `CONFIRM_REJECT`'s words, from the same file and on the same terms. */
const CONFIRM_REJECT = {
  title: "Reject this job's work?",
  body:
    "The job ends at rejected, which is terminal and carries a verdict — this is a decision " +
    "about the work rather than a kill. The drone is stopped and nothing resumes it, and the " +
    "branch stays where it left it. To send the work back instead, close this and request " +
    "changes: that keeps the drone, the worktree and the step.",
} as const;
