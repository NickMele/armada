import { useState } from "react";
import { ConfidenceSheet, ViewSheet, type ConfidenceView } from "@armada/components";
import type { Diff, JobConfidence } from "@armada/protocol";
import { viewStepsOf } from "./view";

export type ReviewAtGateProps = {
  confidence: JobConfidence;
  diff: Diff;
  jobId: string;
  /** Opens the Job's whole diff, from a View step's `Open the whole file`. */
  onOpenDiff?: () => void;
};

/** Armada's review at the gate, and the View a row of it opens. #903, #904. */
export function ReviewAtGate({ confidence, diff, jobId, onOpenDiff }: ReviewAtGateProps) {
  const [viewing, setViewing] = useState<ConfidenceView | null>(null);
  return (
    <>
      <ConfidenceSheet confidence={confidence} onView={setViewing} />
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
          onClose={() => setViewing(null)}
        />
      )}
    </>
  );
}
