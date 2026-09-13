// What should change, gathered at the gate: the review's small fixes and the
// notes written in View. #907.
//
// **The list is the note.** Request changes sends one note, so the list and the
// typed words go out together and the Drone is never told in two places.

import type { DecisionChange } from "@armada/components";
import type { JobConfidence } from "@armada/protocol";

/** The review's small fixes, listed first. */
export function smallFixesOf(confidence: JobConfidence): DecisionChange[] {
  return confidence.small_fixes.map((fix, at) => ({
    id: `small-fix-${at}`,
    from: "Small fix",
    text: `${fix.finding.replaceAll("`", "")}: ${fix.why}`,
  }));
}

/** The note Request changes sends: every listed change, then the typed words. */
export function noteWithChanges(changes: readonly DecisionChange[], note: string): string {
  if (changes.length === 0) return note;
  const typed = note.trim();
  return [
    "What should change:",
    ...changes.map((change) => `- ${change.from}: ${change.text}`),
    ...(typed === "" ? [] : ["", typed]),
  ].join("\n");
}
