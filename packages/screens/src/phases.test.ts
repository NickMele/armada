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

import { ADVANCE_GATE, CRITERION_VERDICT_CHECK } from "@armada/components";

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
    verdicts: [],
    entered_at: "2026-09-02T09:00:00Z",
    updated_at: "2026-09-02T09:00:00Z",
    ...over,
  };
}

/** The Job status under which a step's own `running` is the whole truth. */
const RUNNING = "running";

/** The `You` tier, which `phasesOf` always draws last. */
function you(over: Partial<StepDetail> = {}) {
  const { stages } = phasesOf(step(over), [], OPENS, RUNNING);
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
    // The registry's own word, never one typed here — a copy in the assertion
    // is what would let this pass over a tier that had stopped reading it.
    expect(you({ advance_gate: "human_always", state: "not_started" })?.stands).toBe(
      CRITERION_VERDICT_CHECK.not_reached?.verb,
    );
  });

  // A `manifest_rule:` gate is a policy the repository owns, resolved at the
  // gate rather than written into the workflow. It is not `auto` and not
  // `human_always`, so it must not take either of their branches — the tier
  // asks, the word is the registry's, and the sentence says where the answer
  // came from. `#264`.
  it("names the policy where the gate is one, rather than an answer", () => {
    for (const gate of ["manifest_rule:review_gate", "manifest_rule:auto_merge"]) {
      const tier = you({ advance_gate: gate, state: "awaiting_human" });
      expect(tier?.state).toBe("waiting");
      // Never a phrase written here: a copy in the assertion is what would let
      // this pass over a tier that had stopped reading the generated map.
      expect(tier?.label).toBe(`You · ${ADVANCE_GATE[gate]?.verb}`);
      expect(tier?.detail).toContain(gate);
      expect(tier?.detail).not.toContain("dispatched");
    }
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

/** When a run started. Every attempt carries one; none of these reads it. */
const AT = "2026-09-02T09:00:00Z";

describe("the hand-backs", () => {
  it("draws no loop on a step worked once", () => {
    const { loops } = phasesOf(step({ attempts: [{ attempt: 1, outcome: "advanced", started_at: AT }] }), [], OPENS, RUNNING);
    expect(loops).toEqual([]);
  });

  it("draws one edge however many times the Drone was handed the step back", () => {
    // Three arcs between one pair of nodes is a picture of nothing. The count
    // lives in the words on the edge instead.
    const { loops } = phasesOf(
      step({
        attempts: [
          { attempt: 1, outcome: "retrying", started_at: AT },
          { attempt: 2, outcome: "retrying", started_at: AT },
          { attempt: 3, outcome: "stopped", started_at: AT },
        ],
      }),
      [],
      OPENS,
      RUNNING,
    );
    expect(loops).toEqual([{ from: "checks", to: "working", says: "handed back 2 times" }]);
  });

  it("counts a single hand-back in the singular", () => {
    const { loops } = phasesOf(
      step({
        attempts: [
          { attempt: 1, outcome: "retrying", started_at: AT },
          { attempt: 2, outcome: "advanced", started_at: AT },
        ],
      }),
      [],
      OPENS,
      RUNNING,
    );
    expect(loops?.[0]?.says).toBe("handed back once");
  });

  it("draws nothing for a refusal, which never returns to the Drone", () => {
    // A Judge refusal goes straight to `Ruling::Refused` and escalates with the
    // Drone kept alive and idle; it never re-enters `working`. Only `retrying`
    // is a hand-back, so a step the gate stopped has no edge to draw.
    const { loops } = phasesOf(
      step({ state: "stopped", attempts: [{ attempt: 1, outcome: "stopped", started_at: AT }] }),
      [],
      OPENS,
      RUNNING,
    );
    expect(loops).toEqual([]);
  });
});

describe("the strip and the badge are one reading", () => {
  // **The defect that made the screen look broken.** `phasesOf` was given the
  // step and never the Job, so a Job holding at `awaiting_review` — whose last
  // step is still `running` — drew Working two inches from a badge saying
  // awaiting review. Every status below leaves a step `running` around a Job
  // that has stopped.
  it.each(["awaiting_review", "awaiting_approval", "escalated", "awaiting_repair", "piloted"])(
    "does not draw Working on a running step beneath a Job that is %s",
    (status) => {
      const { stages } = phasesOf(step({ state: "running" }), [], OPENS, status);
      expect(stages.find((held) => held.id === "working")?.state).not.toBe("current");
    },
  );

  it("still draws Working on a running step beneath a running Job", () => {
    const { stages } = phasesOf(step({ state: "running" }), [], OPENS, RUNNING);
    expect(stages.find((held) => held.id === "working")?.state).toBe("current");
  });
});

describe("a panel's members", () => {
  // Three calls, one criterion. Before `member` these three rows carried an
  // identical `(attempt, criterion_id)` and only the count could tell there
  // were three — so the strip drew the criterion three times and counted the
  // calls as if each were a criterion of its own.
  function panel(verdicts: ("met" | "not_met")[]) {
    const { stages } = phasesOf(
      step({
        state: "advanced",
        judge_checks: [
          {
            criteria: 1,
            gaming_check: false,
            // Absent at one, the convention the wire keeps for both this and
            // `Judged.member`. A fixture that sent `1` would say "a panel of
            // one", which is not a thing the daemon ever sends.
            ...(verdicts.length > 1 ? { panel_size: verdicts.length } : {}),
          },
        ],
        judged: verdicts.map((verdict, at) => ({
          attempt: 1,
          criterion_id: "c1",
          member: verdicts.length > 1 ? at + 1 : undefined,
          verdict,
          brief_path: ".armada/briefs/01JOB/fix.1.c1.txt",
          ...(verdict === "not_met"
            ? {
                expected: "the bound narrowed",
                produced: "the bound widened",
                consequence: "every caller reads one row too many",
              }
            : {}),
        })),
        attempts: [{ attempt: 1, outcome: "advanced", started_at: AT }],
      }),
      [{ criterion_id: "c1", text: "The bound narrows", source: "judge" }],
      OPENS,
      // The judge tier reads the step, not the Job. A running Job is the
      // neutral value here; what this block is about is what the panel said.
      RUNNING,
    );
    const judge = stages.find((held) => held.id === "judge");
    expect(judge).toBeDefined();
    return judge!;
  }

  it("read as one row per criterion, not one per call", () => {
    expect(panel(["met", "met", "met"]).rows).toHaveLength(1);
  });

  it("count criteria rather than calls, so three judges are not three criteria", () => {
    expect(panel(["met", "met", "met"]).label).toBe("Judge · 1 of 1 met");
  });

  it("refuse the criterion when one member of three objects, because unanimity", () => {
    const one = panel(["met", "not_met", "met"]);
    expect(one.state).toBe("failed");
    expect(one.rows?.[0]?.named).toBe("not_met");
  });

  it("say how many of how many refused, which is what makes it a close call", () => {
    expect(panel(["met", "not_met", "met"]).rows?.[0]?.result).toBe("refused by 1 of 3");
    expect(panel(["not_met", "not_met", "not_met"]).rows?.[0]?.result).toBe("refused by 3 of 3");
  });

  it("say a panel cleared it, and how big the panel was", () => {
    expect(panel(["met", "met", "met"]).rows?.[0]?.result).toBe("no objection · 3 judges");
  });

  it("say nothing about a panel where one judge answered", () => {
    // `member` is absent at one, and the row reads as it always did.
    expect(panel(["met"]).rows?.[0]?.result).toBe("no objection");
  });
});
