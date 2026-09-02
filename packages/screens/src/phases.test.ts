// The human tier's label and state, which is the one thing on the strip a
// reader cannot recover from anywhere else.
//
// **Two facts used to share one chip.** A step whose `advance_gate` never asks
// for a person and a step whose gate will ask and has not been reached both
// came out `You` sitting `ahead`, with the difference only in `stands` — the
// hover. This pins the pair apart at the point they are derived, and
// `Compositions/Phase strip`'s `TheHumanTierThatCanNeverAsk` pins the same two
// values as a rendering. A change to either has to move both.
//
// **What the tier *says* is no longer here.** `PhaseCard` keys its standing
// copy by state as well as kind, so the sentences are the component's and
// `Compositions/Phase card`'s `TheHumanTierThatCanNeverAsk` reads them off a
// rendering. What this file is checked for is that it writes none of them.
//
// The rest of `phasesOf` is drawn rather than computed and belongs to the
// stories; what is tested here is the branch, which the type does not show.

import { describe, expect, it } from "vitest";

import { phasesOf, type Opens } from "./phases";

import type { StepDetail } from "@armada/protocol";

const OPENS: Opens = {
  jobId: "01M130Y1380016YK5S0JXBXDQ5",
  open: () => Promise.resolve({ ok: true }),
  onSaid: () => {},
};

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "plan",
    label: "Plan the change",
    ordinal: 0,
    state: "not_started",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    entered_at: "2026-09-02T09:00:00Z",
    updated_at: "2026-09-02T09:00:00Z",
    ...over,
  };
}

/** The `You` tier, which `phasesOf` always draws last. */
function you(over: Partial<StepDetail> = {}) {
  const { stages } = phasesOf(step(over), [], OPENS);
  const last = stages[stages.length - 1];
  expect(last?.id).toBe("you");
  return last;
}

describe("the human tier", () => {
  it("is a state of its own where the gate can never ask", () => {
    const auto = you({ advance_gate: "auto", state: "advanced" });
    const judgeOnly = you({ advance_gate: "auto_if_judge_passes", state: "advanced" });

    for (const tier of [auto, judgeOnly]) {
      expect(tier?.state).toBe("never");
      expect(tier?.label).toBe("No one");
    }
  });

  // The defect. `ahead` on a tier that will ask and `ahead` on a tier that
  // cannot are the same value, so the two have to differ somewhere a reader
  // sees without hovering — the label and the state, not `stands`.
  it("does not draw a gate that cannot ask as one not yet reached", () => {
    const never = you({ advance_gate: "auto_if_judge_passes", state: "advanced" });
    const notReached = you({ advance_gate: "human_always", state: "not_started" });

    expect(notReached?.state).toBe("ahead");
    expect(notReached?.label).toBe("You");
    expect(never?.state).not.toBe(notReached?.state);
    expect(never?.label).not.toBe(notReached?.label);
  });

  // This screen used to write both of the never tier's sentences itself, so
  // that the card's standing amber closer could not reach a tier that can
  // never be amber. `PhaseCard` keys that copy by state now, so an override
  // here would be a second place the sentence is written — and worse, it would
  // put the guard back on the caller. What is left to check is that this file
  // hands over no copy at all: the state and the label are the whole of what
  // it says, and `Compositions/Phase card`'s `TheHumanTierThatCanNeverAsk` is
  // where the sentence itself is read off a rendering. #320.
  it("hands the never tier's copy to the card rather than writing it", () => {
    const never = you({ advance_gate: "auto" });
    expect(never?.state).toBe("never");
    expect(never?.said).toBeUndefined();
    expect(never?.detail).toBeUndefined();
    // The gate that will ask says nothing either, and takes the standing line.
    expect(you({ advance_gate: "human_always" })?.detail).toBeUndefined();
  });

  it("lights amber only where a person is being waited on", () => {
    expect(you({ advance_gate: "human_always", state: "awaiting_human" })?.state).toBe("waiting");
    // The same step state under a gate that never asks. It is not waiting on
    // anybody, whatever `job_steps.state` says.
    expect(you({ advance_gate: "auto", state: "awaiting_human" })?.state).toBe("never");
  });

  it("clears where a person answered, and never where none was asked", () => {
    expect(you({ advance_gate: "human_always", state: "advanced" })?.state).toBe("cleared");
    expect(you({ advance_gate: "auto", state: "advanced" })?.state).toBe("never");
  });

  // A cleared tier is one a person answered. It stood "not reached", which is
  // the claim the closing lines were keyed by state to stop making, arriving
  // through the caller: the state read had three branches and the standing had
  // two, so cleared fell into the branch written for a tier nobody has got to.
  it("does not stand not reached on a tier a person already answered", () => {
    expect(you({ advance_gate: "human_always", state: "advanced" })?.stands).toBe("answered");
    expect(you({ advance_gate: "human_always", state: "awaiting_human" })?.stands).toBe(
      "waiting on you",
    );
    expect(you({ advance_gate: "human_always", state: "not_started" })?.stands).toBe("not reached");
  });

  // An absent `advance_gate` means Fleet does not hold the workflow this Job
  // named. A tier that cannot say whether it asks must not answer `No one`,
  // and must not close with the standing amber line either.
  it("does not answer for a gate Fleet cannot name", () => {
    const unknown = you({ advance_gate: undefined, state: "not_started" });
    expect(unknown?.state).toBe("ahead");
    expect(unknown?.label).toBe("You");
    expect(unknown?.stands).toBe("Fleet cannot say");
    expect(unknown?.detail).toBeNull();
  });
});
