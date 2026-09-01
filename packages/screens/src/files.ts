// What a Drone has changed, as the Produced chapter draws it.
//
// **Nothing here reads a directory.** The paths arrive as a named event like
// everything else. Nor is anything counted that Fleet did not send: there are
// no line counts here, because the patch is the expensive read and it is not on
// this seam.
//
// **The change kind renders as the wire spelled it.** `enum-verbs.toml` carries
// no `change_kind` rows, so there is no verb, glyph or hue for one — the same
// fallback a step state takes. A word chosen here would be the second
// vocabulary the generated module exists to prevent. Reported.
//
// # The note says what is in front of a reader, not where it came from
//
// It used to open with which of two readings this was and what the other one
// would have been — three sentences of epistemology in a region whose job is to
// show what the step produced. A surface shows what it has. What is worth
// saying about a list of paths is whether any of them went outside what the
// step promised, and that is the whole of what the note says now.

import type { ChangedFile } from "@armada/components";

import type { JobFilesChanged, ChangedFile as WireFile } from "@armada/protocol";
import type { Footprint } from "@armada/protocol";

/** The rows, in the order the reading found them. Never re-sorted. */
export function filesOf(reading: JobFilesChanged): ChangedFile[] {
  return reading.files.map((file: WireFile) => ({
    path: file.path,
    change: file.change,
    // Absent and `false` are the same fact here — the wire omits it rather than
    // sending null — so the row is marked only where it is true.
    outsidePlan: file.outside_plan === true || undefined,
  }));
}

/**
 * What the list says about itself: whether anything went outside what the step
 * promised.
 *
 * **`plan_declared` false is "there is no plan", not "nothing drifted."** A
 * note that stayed silent about it would let an unscoped step read as a step
 * perfectly on plan, which is the one thing this field exists to prevent — so
 * that case says the step scoped nothing, which is a fact about the step rather
 * than about the reading.
 */
export function footprintNote(reading: JobFilesChanged): string {
  if (!reading.plan_declared) return "This step declared no plan, so nothing here is measured against one.";
  const outside = reading.files.filter((file) => file.outside_plan === true).length;
  const total = reading.files.length;
  return outside === 0
    ? "Every path is inside the plan this step declared."
    : `${outside} of ${total} paths are outside the plan this step declared.`;
}

/** The reading for this Job, or nothing. A footprint belongs to one Job. */
export function readingFor(footprint: Footprint, jobId: string): JobFilesChanged | undefined {
  return footprint.state === "read" && footprint.jobId === jobId ? footprint.reading : undefined;
}

/**
 * What the chapter says while there is nothing to draw.
 *
 * **A Drone that has not reported yet and a Job with no Drone on it are two
 * different facts.** The first says wait and the second says nothing is coming,
 * and one sentence for both would leave a Job at the approval gate looking like
 * it had stalled.
 */
export function whyNoFootprint(hasDrone: boolean): string {
  return hasDrone
    ? "Nothing has been written yet."
    : "No drone is on this job yet.";
}
