// What the machine panel says when it has no reading, and the two failures
// where it offers nothing to press.
//
// **Its own file because it is a decision, and a decision is unit-tested.** A
// `play` that computed rather than read would be a unit test paying a browser's
// price — `docs/practices/react.md` is explicit — and this is the decision that
// either offers a person a next move or admits there is none.
//
// # Nothing to ask
//
// A person opened a Job that stopped because Fleet itself did not answer, and
// the one control on the screen asked Fleet a question. The panel read "Nobody
// has asked whether this job is working. Looking costs no model call." over a
// live `Look now`, because a read that failed and a read nobody had made drew
// the same reading. #462.
//
// The act is withdrawn on the failure and never on the fact of failure. A
// refusal carries a code and a wait that ran out may already have been served,
// so both leave the control where it is — a panel that withdrew its act on a
// timeout would strand somebody whose Fleet is fine.
//
// Two of them withdraw it, for opposite reasons, and the panel says which. One
// is a Fleet that is not there. The other is a Fleet that is demonstrably up
// and answering something Bridge cannot read, which is structural rather than
// transient: the same request down the same route meets the same disagreement,
// and a restart brings back the build that caused it. #344 classifies that
// same condition as a fault on the command seam, for the same reason.

import type { NothingToAsk } from "@armada/components";
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
 * Whether pressing `Look now` could work, and where it could not, which of the
 * two reasons it is. `undefined` leaves the act on the panel.
 *
 * **The failure decides it, never the fact that something failed.** Every
 * reading on that panel is Fleet's and its one control asks Fleet for another,
 * so the question is whether another attempt could answer — not whether the
 * last one did.
 *
 * **`no_answer` is Fleet not being on the other end.** `not_connected` is
 * Bridge holding no port at all: no runtime file, a pid that did not verify, or
 * a socket that has not come up, so there is no address to send to.
 * `unreachable` is the fetch itself failing, which is the same thing one layer
 * down. Starting Fleet is what fixes either.
 *
 * **`unreadable` is Fleet being up and the answer being unreadable.** Fleet
 * returned a status and the body under it was not something Bridge could read,
 * which is the two sides disagreeing about this route rather than a Job going
 * wrong. Fleet being demonstrably alive is not a reason to keep the control:
 * the disagreement is in the builds, so the same request meets it again, and
 * a restart brings back the Fleet that caused it. Starting Fleet is the wrong
 * fix, which is why this cannot share `no_answer`'s sentence.
 *
 * **`refused` and `timed_out` keep the act.** A refusal carries a code and is
 * Fleet answering the route as designed; a wait that ran out may already have
 * been served, and Fleet has its own budget. Both are a Fleet that is there.
 *
 * A kept reading is not any of this: `keepsLastGood` holds the last good answer
 * for the open Job, so `failed` here is a Job whose machine reading never
 * arrived.
 */
export function nothingToAsk(resources: Holds): NothingToAsk | undefined {
  if (resources.state !== "failed") return undefined;
  const outcome = resources.outcome;
  if (outcome.ok) return undefined;
  if (outcome.why === "not_connected") return "no_answer";
  if (outcome.why !== "transport") return undefined;
  switch (outcome.fault.why) {
    case "unreachable":
      return "no_answer";
    case "unanswerable":
      return "unreadable";
    case "timed_out":
      return undefined;
  }
}
