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
// Fleet put on the wire. `@armada/protocol`'s `artifacts.ts` owns why that is safe and
// `main/open.ts` is what enforces it; nothing here concatenates anything.

import type { Artifact, Followed, Opened } from "@armada/protocol";
import { isKept } from "@armada/protocol";

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
export async function openArtifact(
  open: OpenArtifact,
  jobId: string,
  what: Artifact,
): Promise<string | null> {
  return whyNotOpened(await open(jobId, what), what);
}

/**
 * Asking the host to open a file, as the caller hands it in.
 *
 * **Taken rather than reached for.** This module says what an unopenable file
 * means, which is a reading a screen does; the socket that opens one belongs to
 * the process that has a filesystem. Screens sit above that process and cannot
 * import it, so the call arrives as an argument.
 */
export type OpenArtifact = (jobId: string, what: Artifact) => Promise<Opened>;

// # Going somewhere is not opening a file, and this is where the two meet
//
// The rest of this module hands main a Job id and a word and gets a file on
// screen. What follows hands main a Job id and gets a browser — a different
// process, a different failure set and a different sentence — and it is here
// rather than in a module of its own because both are one thought: *the thing
// this Job left is somewhere, take me to it*. A second module would be a second
// vocabulary for a click that did nothing.

/**
 * Why the pull request did not open, in the app's voice, or `null` because it
 * did.
 *
 * `null` on success is the whole of the confirmation, exactly as it is for a
 * file: the page is in front of the person, which says it better than a toast.
 */
export function whyNotFollowed(followed: Followed): string | null {
  if (followed.ok) return null;
  switch (followed.why) {
    case "unknown_job":
      return "Bridge no longer holds this job, so it could not say what it opened.";
    case "no_address":
      return (
        "Fleet's record of this job names no pull request. It did when this screen was " +
        "drawn, so the job has been re-read since — re-open it and try again."
      );
    case "not_addressable":
      return `Fleet recorded ${followed.address} as this job's pull request, and Bridge only opens a web address.`;
    case "refused":
      return `This machine did not open ${followed.address}: ${followed.detail}`;
  }
}

/**
 * Asking the host to open one Job's pull request, as the caller hands it in.
 *
 * **A Job id and nothing else**, which is `OpenArtifact`'s rule for
 * `OpenArtifact`'s reason: the renderer never holds the argument that decides
 * what is opened. Main reads the address off the same record it published, so
 * a string built anywhere in the renderer cannot reach the OS through here.
 */
export type OpenPullRequest = (jobId: string) => Promise<Followed>;

/**
 * Ask main to open one Job's pull request, and answer with the sentence to say
 * where it did not. `null` is success.
 */
export async function openPullRequest(
  open: OpenPullRequest,
  jobId: string,
): Promise<string | null> {
  return whyNotFollowed(await open(jobId));
}
