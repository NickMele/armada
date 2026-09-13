// What an edit and a Write answered, told apart. `appeared` is the one that is not a
// failure: a file is already at the path, and it rides back so a person can read it.

import { describe, expect, it } from "vitest";

import { proposalAnswerOf } from "./setting-up";

function refused(code: string, fields: Record<string, unknown>) {
  return {
    ok: false as const,
    outcome: {
      ok: false as const,
      why: "refused" as const,
      error: { code, message: "Fleet said no", run_id: "r", fields, chain: [] },
    },
  };
}

describe("a proposal's answer", () => {
  it("took, carrying the proposal after the edit", () => {
    const proposal = { dir: "web", file: "web/armada.yml" };
    expect(proposalAnswerOf({ ok: true, body: proposal })).toEqual({ state: "took", proposal });
  });

  it("carries the file already on disk where one appeared", () => {
    const answer = proposalAnswerOf(refused("fleet.manifest_appeared", { on_disk: "version: 1\n" }));
    expect(answer).toEqual({ state: "appeared", onDisk: "version: 1\n" });
  });

  it("reads every fault a Write was refused for, and drops a pair it cannot read", () => {
    const answer = proposalAnswerOf(
      refused("fleet.proposal_refused", { faults: [["checks.e2e.requires", "names no Command"], [1]] }),
    );
    expect(answer).toEqual({
      state: "refused",
      saying: "Fleet said no",
      faults: [{ key: "checks.e2e.requires", fault: "names no Command" }],
    });
  });

  it("says a move that would drop a key in Fleet's words, with no faults", () => {
    const answer = proposalAnswerOf(refused("fleet.proposal_not_amended", {}));
    expect(answer).toEqual({ state: "refused", saying: "Fleet said no", faults: [] });
  });

  it("is a failure for any other refusal, carried whole", () => {
    const other = refused("fleet.proposal_unwritable", {});
    expect(proposalAnswerOf(other)).toEqual({ state: "failed", outcome: other.outcome });
  });
});
