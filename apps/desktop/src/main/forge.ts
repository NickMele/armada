// Hand a Job's pull request to whatever browses the web on this machine.
//
// **Beside `open.ts` rather than in it, and the split is the two destinations.**
// That module derives a path and hands it to an editor or to Finder; this one
// takes an address off the record and hands it to a browser. They share the
// rule that matters — the renderer names a Job and never the thing being opened
// — and share nothing else: a path is built, an address is served, and the two
// fail at different things.
//
// **The address is read here, off what main published.** The renderer draws it,
// so it holds a copy; it does not send one. That is the same property
// `open.ts` keeps and it is worth restating, because an address is the one
// argument where a string arriving from a click handler would be enough on its
// own: `shell.openExternal` will hand `file:`, and on some platforms worse, to
// the OS. Nothing composed in the renderer reaches it.
//
// **And what is read is still checked.** The scheme has to be `https:` — the
// only thing a forge writes — so a record carrying anything else is refused by
// name rather than opened. That is a second guard on a value that already came
// from Fleet, because the cost of being wrong here is arbitrary URL handling
// and the cost of the check is one comparison.
//
// Fleet is not involved. Nothing here is a Job act, no protocol version moves,
// and the pull request Fleet opened is already on the wire.

import { shell } from "electron";

import type { Followed } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";

/**
 * The pull request address on the reading of this Job main is holding, or
 * `undefined`.
 *
 * **Read off `watched` alone**, for `open.ts`'s reason: `jobs` is the Board's
 * summaries and carries no delivery, and the only surface that draws this link
 * is the open Job's. A Job re-read between the draw and the click answers from
 * the new reading, which is what `no_address` reports.
 */
function addressOf(state: BridgeState, jobId: string): string | undefined {
  const watched = state.watched;
  if (watched.state !== "read" || watched.jobId !== jobId) return undefined;
  return watched.detail.delivery?.pull_request;
}

/**
 * Whether main will hand this to the OS. `https:` and nothing else.
 *
 * **A scheme test rather than a host allowlist.** Which forges exist is
 * `armada.yml`'s business and Fleet's; what this owns is that Bridge does not
 * become a way to open `file:`, `javascript:` or a registered protocol handler.
 * A malformed string is refused by the parse, which is the same answer.
 */
function addressable(address: string): boolean {
  try {
    return new URL(address).protocol === "https:";
  } catch {
    return false;
  }
}

/**
 * Open one Job's pull request.
 *
 * **Refusals are named, never silent.** Three of the four arms are states a
 * person cannot bring about — the link is only drawn where the address is — so
 * every one of them is a race or a record that surprised us, and a click that
 * did nothing is the defect `#422` is about with a click added to it.
 */
export async function openPullRequest(state: BridgeState, jobId: string): Promise<Followed> {
  const job =
    state.jobs.some((row) => row.id === jobId) ||
    (state.watched.state === "read" && state.watched.jobId === jobId);
  if (!job) return { ok: false, why: "unknown_job" };

  const address = addressOf(state, jobId);
  if (address === undefined) return { ok: false, why: "no_address" };
  if (!addressable(address)) return { ok: false, why: "not_addressable", address };

  try {
    await shell.openExternal(address);
    return { ok: true };
  } catch (error) {
    // The OS explaining itself, carried through rather than replaced with a
    // sentence Bridge made up — `open.ts`'s rule for the same class of failure.
    return { ok: false, why: "refused", address, detail: String(error) };
  }
}
