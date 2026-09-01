// Asking main to open one of a Job's files, and saying why it did not.
//
// # It is here rather than in `work.ts` because two regions open things now
//
// The Where region opens the worktree; the phase strip opens the three records
// a person reads *because a verdict went against them* — a Check's output, the
// document the Judge read, and the question it was asked. Both need the same
// five sentences, and two copies of them would drift into two vocabularies for
// one failure.
//
// # The renderer never builds the path
//
// It hands back the Job id and either one of three words or the exact string
// Fleet put on the wire. `shared/artifacts.ts` owns why that is safe and
// `main/open.ts` is what enforces it; nothing here concatenates anything.

import type { Artifact, Opened } from "../../shared/artifacts";
import { isKept } from "../../shared/artifacts";

/** What each kept record is called, in the sentence a failure writes. */
const CALLED = {
  check: "That check's output",
  brief: "The brief the judge answered",
  deliverable: "The document the judge read",
} as const;

/** The row's subject, for a sentence about it. */
function subjectOf(what: Artifact): string {
  return isKept(what) ? CALLED[what.what] : "That file";
}

/**
 * Why an open did not happen, in the app's voice, or `null` because it did.
 *
 * **Five sentences, not one.** A reclaimed directory, a Manifest Fleet no
 * longer holds, a record no longer on this screen and an OS with no handler
 * need different next steps, and a shared sentence would send a person to look
 * for the wrong one.
 *
 * **Every one of them names the path.** The whole defect being fixed is a
 * record nobody could reach, and an open that failed silently — or failed
 * without saying where it looked — is that defect with a click added to it.
 */
export function whyNotOpened(opened: Opened, what: Artifact): string | null {
  if (opened.ok) return null;
  switch (opened.why) {
    case "unknown_job":
      return "Bridge no longer holds this job, so it could not work out where that is.";
    case "no_repository":
      return (
        "Fleet holds no manifest with this job's id, so Bridge cannot say which repository " +
        "the work is in. The path above follows from the job id once it can."
      );
    case "not_there":
      if (what === "worktree") {
        return "Nothing is there. It was reclaimed, and the branch survives it.";
      }
      // A kept record names a file Fleet said was there when it answered, so a
      // miss here is the file having gone since — a hand-cleaned directory, or
      // a re-run that has not written this one yet. Saying which path was tried
      // is the difference between that and a dead control.
      return isKept(what)
        ? `${subjectOf(what)} is no longer at ${opened.path}. Fleet had it when it last answered.`
        : "That file has not been written, or it has been removed. The path is still where it goes.";
    case "not_named":
      return (
        `${subjectOf(what)} is not on the reading of this job Bridge is holding. ` +
        "Re-open the job and try again."
      );
    case "refused":
      return `This machine did not open it: ${opened.detail}`;
  }
}

/**
 * Ask main to open one file, and answer with the sentence to say where it did
 * not.
 *
 * `null` is success and is the whole of the confirmation: the file is in front
 * of the person, in their editor, which says it better than a toast would.
 */
export async function openArtifact(jobId: string, what: Artifact): Promise<string | null> {
  return whyNotOpened(await window.armada.openArtifact(jobId, what), what);
}
