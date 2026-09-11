// What a `take_up_remarks` press was refused for, where Fleet refused it
// because the chosen comments would not fit the room an opening brief leaves
// free — and never for any other reason.
//
// **Split from `review.ts` at the 500-line ask.** That file is the two reads a
// review is made of; this is a third subject beside them — one refusal, read
// as guidance rather than as a failure — and it was the file's own subject
// header that made the seam visible once this grew past a few lines.
//
// # The one Fleet code this package reads rather than leaves opaque
//
// `docs/contracts/error-contract.md` says Bridge never matches a code
// exhaustively — it looks one up or falls back. This is that lookup, for
// exactly one code, with a fallback to the generic failure notice for every
// other refusal `take_up_remarks` can meet.
//
// # No new field on the wire
//
// The ids Fleet names on the refusal are the same ones the press was made
// with, and the author and the opening words are read off the comments this
// surface already holds — `Remarks` — rather than sent a second time.
// `crates/fleet/src/refusing.rs` is the Rust side of this same decision.

import type { Outcome, Remarks } from "@armada/protocol";

/**
 * Fleet's own code for a `take_up_remarks` press whose chosen comments would
 * not fit the room an opening brief leaves free. `crates/fleet/src/refusing.rs`
 * raises it.
 */
export const REMARKS_TOO_LARGE = "fleet.remarks_too_large";

/** How much of a comment's opening reaches one line of guidance. */
const ENOUGH_OF_AN_OPENING = 80;

/** Guidance for a press refused as too large, drawn in place of a comment. */
export type TooLarge = {
  /** The line over the list of what to drop. */
  said: string;
  /** The ids to mark in the list below. */
  ids: ReadonlySet<string>;
};

/**
 * What a `take_up_remarks` press was refused for, where it was refused for
 * size — or `null` for every other outcome, including every other refusal.
 *
 * **Read for this Job alone.** `outcome` is one piece of state shared by
 * every command in the window; a stale refusal from a Job that is no longer
 * open must not paint guidance under a different one, and `job_id` is what
 * Fleet stamps a refusal with once a Job exists — `docs/contracts/
 * error-contract.md`.
 */
export function tooLargeIn(outcome: Outcome | null, jobId: string, remarks: Remarks): TooLarge | null {
  if (outcome === null || outcome.ok || outcome.why !== "refused") return null;
  const error = outcome.error;
  if (error.code !== REMARKS_TOO_LARGE || error.job_id !== jobId) return null;
  const raw = error.fields.remarks;
  const ids = new Set(typeof raw === "string" ? raw.split(", ").filter((id) => id !== "") : []);
  const known = remarks.state === "read" && remarks.jobId === jobId ? remarks.review.remarks : [];
  const named = known
    .filter((remark) => ids.has(remark.id))
    .map((remark) => `${remark.by} — "${openingOf(remark.said)}"`);
  const count = ids.size;
  const said =
    count === 1
      ? `One comment picked is too long for the drone's brief. Drop it and press again${
          named.length === 0 ? "." : `: ${named[0]}`
        }`
      : `${count} of the comments picked are too long for the drone's brief. Drop these and press again${
          named.length === 0 ? "." : `: ${named.join("; ")}`
        }`;
  return { said, ids };
}

/** A comment's opening words, short enough for one line of guidance. */
function openingOf(said: string): string {
  const cleaned = said.replace(/[\r\n]+/g, " ").trim();
  return cleaned.length > ENOUGH_OF_AN_OPENING ? `${cleaned.slice(0, ENOUGH_OF_AN_OPENING)}…` : cleaned;
}
