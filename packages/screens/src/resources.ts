// What the machine panel says when it has no reading, and the one failure
// where it offers nothing to press.
//
// **Its own file because it is a decision, and a decision is unit-tested.** A
// `play` that computed rather than read would be a unit test paying a browser's
// price — `docs/practices/react.md` is explicit — and the case below is the one
// where the screen either offers a person a next move or admits there is none.
//
// # Nothing to ask
//
// A person opened a Job that stopped because Fleet itself did not answer, and
// the one control on the screen asked Fleet a question. The panel read "Nobody
// has asked whether this job is working. Looking costs no model call." over a
// live `Look now`, because a read that failed and a read nobody had made drew
// the same reading. #462.
//
// The act is withdrawn on the failure and never on the fact of failure. Fleet
// refusing the route, answering a status Bridge cannot read, or taking longer
// than the wait are all a Fleet that is there, and a panel that took its
// control away on a timeout would strand somebody whose Fleet is fine.

import type { Holds } from "@armada/protocol";

/**
 * What a failed look says, and it says nothing about the Job.
 *
 * **Fleet did not answer, which is not a finding.** Drawing a failure as
 * `not_working` would report a Job as broken on the strength of a connection.
 */
export const LOOK_FAILED = "Fleet did not answer the look. Nothing here is a finding about the Job.";

/**
 * Why there is no machine reading, which is never the same sentence twice.
 *
 * **A read that has not answered and a Job that holds nothing are different
 * things**, and this is the half that says which — the panel's own arm says the
 * other. `undefined` where the reading is in hand.
 */
export function whyNoReading(resources: Holds): string | undefined {
  switch (resources.state) {
    case "none":
      return "Nothing is being read.";
    case "reading":
      return "Reading the machine.";
    case "failed":
      return "Fleet did not answer, so what this Job holds is unknown.";
    case "read":
      return undefined;
  }
}

/**
 * Whether Fleet is the thing that did not answer, which is the one reading
 * where the panel offers no act at all.
 *
 * **The failure decides it, never the fact that something failed.** Every
 * reading on that panel is Fleet's and its one control asks Fleet for another,
 * so the question is whether there is anything on the other end to ask — not
 * whether the last attempt worked. Fleet refusing this route, answering a
 * status Bridge cannot read, or taking longer than the wait are all a Fleet
 * that is there: pressing again is a reasonable move and the act stays.
 * `not_connected` and `unreachable` are the two where it is not.
 *
 * **`not_connected` is Bridge holding no port at all** — no runtime file, a pid
 * that did not verify, or a socket that has not come up. There is no address to
 * send to. **`unreachable` is the fetch itself failing**, which is the same
 * thing one layer down.
 *
 * A kept reading is not this state: `keepsLastGood` holds the last good answer
 * for the open Job, so `failed` here is a Job whose machine reading never
 * arrived.
 */
export function nothingToAsk(resources: Holds): boolean {
  if (resources.state !== "failed") return false;
  const outcome = resources.outcome;
  if (outcome.ok) return false;
  if (outcome.why === "not_connected") return true;
  return outcome.why === "transport" && outcome.fault.why === "unreachable";
}
