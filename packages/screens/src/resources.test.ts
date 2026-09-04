// Whether the machine panel offers to ask Fleet, and the failures where it
// must not.
//
// **A person opened a job Fleet had not answered about and the one control on
// the screen asked Fleet a question.** The panel read "Nobody has asked whether
// this job is working. Looking costs no model call." over a live `Look now`,
// because a read that failed and a read nobody had made were the same reading.
// #462.
//
// The fix turns on the failure and not on the fact of failure, so these are the
// cases that separate the two: a Fleet that is not there to ask, against a
// Fleet that is there and answered badly, late, or with a refusal. Getting that
// wrong in the other direction is just as bad — a panel that withdrew its act
// on a timeout would take the next move away from somebody whose Fleet is fine.

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
  it("withdraws the act where Bridge holds no connection", () => {
    expect(nothingToAsk(failed({ ok: false, why: "not_connected" }))).toBe(true);
  });

  it("withdraws the act where the request could not be sent", () => {
    expect(nothingToAsk(transport("unreachable"))).toBe(true);
  });

  // Fleet may well have carried the read out. It is there, and a second press
  // is the reasonable move rather than a dead end.
  it("keeps the act where the read timed out", () => {
    expect(nothingToAsk(transport("timed_out"))).toBe(false);
  });

  // Fleet answered a status. It is running, and the two disagree about the
  // route — which is a thing to say, not a reason to take the control away.
  it("keeps the act where Fleet answered something Bridge could not read", () => {
    expect(nothingToAsk(transport("unanswerable"))).toBe(false);
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
    ).toBe(false);
  });

  // The three states that are not a failure at all. `read` is the reading in
  // hand, and `keepsLastGood` means a re-read that fails leaves it there.
  it("offers the act at every state that is not a failure", () => {
    expect(nothingToAsk({ state: "none" })).toBe(false);
    expect(nothingToAsk({ state: "reading", jobId: JOB })).toBe(false);
  });
});
