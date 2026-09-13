import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  ConfidenceSheet,
  ViewSheet,
  type ConfidenceCi,
  type ConfidenceView,
  type DecisionChange,
} from "@armada/components";
import type { Diff, JobConfidence } from "@armada/protocol";
import { smallFixesOf } from "./changes";
import { viewStepsOf } from "./view";

export type ReviewAtGateProps = {
  confidence: JobConfidence;
  diff: Diff;
  jobId: string;
  /** Opens the Job's whole diff, from a View step's `Open the whole file`. */
  onOpenDiff?: () => void;
  /** Adds a note written in a View to What should change. #907. */
  onAddNote?: (view: string, note: string) => void;
  /** Dismisses a finding with the reason written in its View. #907. */
  onDismissFinding?: (finding: string, reason: string) => void;
  /** The pull request's CI, and what a person can do about it. #905. */
  ci?: ConfidenceCi;
};

/** Armada's review at the gate, and the View a row of it opens. #903, #904. */
export function ReviewAtGate({
  confidence,
  diff,
  jobId,
  onOpenDiff,
  onAddNote,
  onDismissFinding,
  ci,
}: ReviewAtGateProps) {
  const [viewing, setViewing] = useState<ConfidenceView | null>(null);
  return (
    <>
      <ConfidenceSheet
        confidence={confidence}
        onView={setViewing}
        {...(ci === undefined ? {} : { ci })}
      />
      {viewing === null ? null : (
        <ViewSheet
          open
          title={viewing.title}
          steps={viewStepsOf(diff, jobId, viewing.steps)}
          // One sheet at a time: the View closes as the diff opens.
          {...(onOpenDiff === undefined
            ? {}
            : {
                onOpenFile: () => {
                  setViewing(null);
                  onOpenDiff();
                },
              })}
          {...(onAddNote === undefined
            ? {}
            : { onAddNote: (note: string) => onAddNote(viewing.title, note) })}
          {...(onDismissFinding === undefined || viewing.finding === undefined
            ? {}
            : {
                onDismiss: (reason: string) => {
                  onDismissFinding(viewing.finding ?? "", reason);
                  setViewing(null);
                },
              })}
          onClose={() => setViewing(null)}
        />
      )}
    </>
  );
}

/** What should change, and how a row comes off it, handed to the decision box. */
export type PendingChanges = {
  changes: DecisionChange[];
  onRemoveChange: (id: string) => void;
};

export type ReviewedGateProps = Omit<ReviewAtGateProps, "onAddNote"> & {
  /** The verdict sheet, drawn with the changes the review and View have listed. */
  sheet: (pending: PendingChanges) => ReactNode;
};

/**
 * The review above the verdict sheet, and the list the two share. #907.
 *
 * **The list lives here** because View writes into it and the decision box sends it,
 * and those are two regions of the gate. It starts from the review's small fixes and
 * belongs to the Job it was gathered for.
 */
export function ReviewedGate({ sheet, ...review }: ReviewedGateProps) {
  const [changes, setChanges] = useState<DecisionChange[]>(() => smallFixesOf(review.confidence));
  const written = useRef(0);
  useEffect(() => {
    setChanges(smallFixesOf(review.confidence));
  }, [review.jobId]);
  return (
    <>
      <ReviewAtGate
        {...review}
        onAddNote={(view, note) => {
          written.current += 1;
          setChanges((was) => [
            ...was,
            { id: `view-${written.current}`, from: `From View: ${view}`, text: note },
          ]);
        }}
      />
      {sheet({
        changes,
        onRemoveChange: (id) => setChanges((was) => was.filter((change) => change.id !== id)),
      })}
    </>
  );
}
