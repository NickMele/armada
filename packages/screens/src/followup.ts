// What an issue drafted from a For context finding starts as, for a person to edit. #906.

import type { JobConfidence } from "@armada/protocol";

export type IssueDraft = { finding: string; title: string; body: string };

/** The finding's words as the title, and the finding with why it was raised as the body. */
export function issueDraftOf(confidence: JobConfidence, finding: string): IssueDraft {
  const why = confidence.for_context.find((row) => row.finding === finding)?.why ?? "";
  const reason = why === "" ? "" : `\n\nWhy it was raised: ${why}`;
  return {
    finding,
    title: finding.replaceAll("`", ""),
    body: `${finding}${reason}\n\nRaised by Armada's review.`,
  };
}
