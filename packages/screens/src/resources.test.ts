// Whether the machine panel offers to ask Fleet, and the failures where it
// must not.
//
// **A person opened a job Fleet had not answered about and the one control on
// the screen asked Fleet a question.** The panel read "Nobody has asked whether
// this job is working. Looking costs no model call." over a live `Look now`,
// because a read that failed and a read nobody had made were the same reading.
// #462.
//
// The fix turns on the failure and not on the fact of failure, and on which of
// two ways an attempt could not work. A Fleet that is not there is one; a Fleet
// that is up and answering something Bridge cannot read is the other, and it
// withdraws the act for the opposite reason — the disagreement is in the
// builds, so the same request meets it again however alive Fleet is.
//
// A refusal and a wait that ran out keep the act. Getting that wrong is just as
// bad as the original defect: a panel that withdrew its control on a timeout
// would take the next move away from somebody whose Fleet is fine.

import { describe, expect, it } from "vitest";

import type { Holds, Outcome } from "@armada/protocol";
import { nothingToAsk } from "./resources";

const JOB = "01M130Y1380016YK5S0JXBXDQ5";

function failed(outcome: Outcome): Holds {
  return { state: "failed", jobId: JOB, outcome };
}

/** A transport failure, with the route it was about, as `request.ts` builds one. */
function transport(
  why: "timed_out" | "unreachable" | "unanswerable",
  over: { status?: number } = {},
): Holds {
  const fault =
    why === "timed_out"
      ? ({ why, method: "GET", path: `/jobs/${JOB}/resources`, waitedMs: 5000 } as const)
      : why === "unanswerable"
        ? ({
            why,
            method: "GET",
            path: `/jobs/${JOB}/resources`,
            status: over.status ?? 502,
          } as const)
        : ({ why, method: "GET", path: `/jobs/${JOB}/resources` } as const);
  return failed({ ok: false, why: "transport", detail: "fetch failed", fault });
}

describe("nothing to ask", () => {
  it("says Fleet is not there where Bridge holds no connection", () => {
    expect(nothingToAsk(failed({ ok: false, why: "not_connected" }))).toBe("no_answer");
  });

  it("says Fleet is not there where the request could not be sent", () => {
    expect(nothingToAsk(transport("unreachable"))).toBe("no_answer");
  });

  // Fleet is demonstrably up and it still withdraws the act, because what
  // stops the answer is the two builds and not the daemon. A second press
  // sends the same request down the same route to the same disagreement.
  it("says the answer was unreadable where Fleet answered a status", () => {
    expect(nothingToAsk(transport("unanswerable"))).toBe("unreadable");
  });

  // The two readings have opposite fixes — start Fleet, against rebuild the
  // pair — so a caller that folded them into one flag would send somebody to
  // restart a Fleet that is already running.
  it("tells the two apart rather than reporting that something failed", () => {
    expect(nothingToAsk(transport("unreachable"))).not.toBe(
      nothingToAsk(transport("unanswerable")),
    );
  });

  // Fleet may well have carried the read out. It is there, and a second press
  // is the reasonable move rather than a dead end.
  it("keeps the act where the read timed out", () => {
    expect(nothingToAsk(transport("timed_out"))).toBeUndefined();
  });

  it("keeps the act where Fleet refused the route", () => {
    expect(
      nothingToAsk(
        failed({
          ok: false,
          why: "refused",
          error: {
            code: "job.not_found",
            message: "No job with that id.",
            run_id: "01M1RUN000000000000000000",
            fields: {},
            chain: [],
          },
        }),
      ),
    ).toBeUndefined();
  });

  // The states that are not a failure at all. `read` is the reading in hand,
  // and `keepsLastGood` means a re-read that fails leaves it there.
  it("offers the act at every state that is not a failure", () => {
    expect(nothingToAsk({ state: "none" })).toBeUndefined();
    expect(nothingToAsk({ state: "reading", jobId: JOB })).toBeUndefined();
  });
});
