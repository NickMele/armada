// Four gate states per step, and the line the ticks cannot turn off.

import type { LimitValues } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { gateReadingOf, gateViewOf, proposalViewOf } from "./proposal";
import type { GateView } from "./proposal";
import { sampleDetail, sampleStep } from "./sample";

const limits: LimitValues = {
  concurrency: 4,
  memory_spare_percent: 20,
  disk_floor_gib: 10,
  checks_at_once: 4,
};

describe("what gates one step", () => {
  it("reads auto as the Checks deciding and nobody looking", () => {
    const gate = gateViewOf(
      sampleStep({ advance_gate: "auto", checks: [{ kind: "manifest_check", name: "test" }] }),
    );

    expect(gate).toEqual({ step_id: "implement", checks: true, judge: false, you: false });
  });

  it("reads auto_if_judge_passes as the Judge looking", () => {
    expect(gateViewOf(sampleStep({ advance_gate: "auto_if_judge_passes" })).judge).toBe(true);
  });

  it("reads human_always as stopping for you", () => {
    expect(gateViewOf(sampleStep({ advance_gate: "human_always" })).you).toBe(true);
  });

  it("holds the repository's rule as a fourth state, not as three bits", () => {
    const merge = gateViewOf(sampleStep({ advance_gate: "manifest_rule:auto_merge" }));
    const review = gateViewOf(sampleStep({ advance_gate: "manifest_rule:review_gate" }));

    expect(merge.repository_decides).toBe("auto_merge");
    expect(review.repository_decides).toBe("review_gate");
  });

  it("says whether this Job overrode that rule, and only where one applies", () => {
    const repository = gateViewOf(
      sampleStep({ advance_gate: "manifest_rule:auto_merge", overridden: true }),
    );
    const ordinary = gateViewOf(sampleStep({ advance_gate: "auto", overridden: true }));

    expect(repository.overridden).toBe(true);
    expect(ordinary.overridden).toBeUndefined();
  });

  it("reads Fleet not knowing the workflow as stopping for you, never as ungated", () => {
    const gate = gateViewOf(sampleStep());

    expect(gate.you).toBe(true);
    expect(gate.checks).toBe(false);
  });

  it("says the Judge looks where the step declares a tier, whatever the gate", () => {
    const gate = gateViewOf(
      sampleStep({
        advance_gate: "auto",
        judge_checks: [{ criteria: 2, gaming_check: false }],
      }),
    );

    expect(gate.judge).toBe(true);
  });
});

describe("the line no tick turns off", () => {
  it("is always true, so a screen cannot draw a step as unwatched", () => {
    const view = proposalViewOf(sampleDetail(), limits);

    expect(view.fleet_always_looks).toBe(true);
  });
});

describe("what locks at approval", () => {
  it("gives every tier the Job's one model, since no tier map is served", () => {
    const detail = sampleDetail();
    detail.job.model = "opus";

    expect(proposalViewOf(detail, limits).tiers).toEqual({
      difficult: "opus",
      medium: "opus",
      easy: "opus",
    });
  });

  it("takes the machine's cap from the concurrency limit", () => {
    expect(proposalViewOf(sampleDetail(), limits).machine_cap).toBe(4);
  });

  it("caps this Job's Drones at nothing, because nothing serves a per-Job cap", () => {
    expect(proposalViewOf(sampleDetail(), limits).drone_cap).toBeUndefined();
  });

  it("offers a ready pull request and no from-ref unless one is given", () => {
    const view = proposalViewOf(sampleDetail(), limits);

    expect(view.pr_mode).toBe("ready");
    expect(view.from_ref).toBeNull();
    expect(proposalViewOf(sampleDetail(), limits, "main").from_ref).toBe("main");
  });

  it("carries no notes for the planner, which was dropped", () => {
    const view = proposalViewOf(sampleDetail(), limits);

    expect("notes_for_planner" in view).toBe(false);
  });

  it("dates the approval from when the Job started, and leaves it out before", () => {
    const started = sampleDetail();
    started.job.started_at = "2026-09-22T09:02:00Z";

    expect(proposalViewOf(started, limits).approved_at).toBe("2026-09-22T09:02:00Z");
    expect(proposalViewOf(sampleDetail(), limits).approved_at).toBeUndefined();
  });

  it("gives one gate per step of the frozen workflow", () => {
    const detail = sampleDetail({
      steps: [sampleStep({ step_id: "plan" }), sampleStep({ step_id: "implement" })],
    });

    expect(proposalViewOf(detail, limits).gates.map((gate) => gate.step_id)).toEqual([
      "plan",
      "implement",
    ]);
  });
});


/** One gate, with the three boxes off unless the case turns one on. */
const boxes = (over: Partial<GateView> = {}): GateView => ({
  step_id: "implement",
  checks: false,
  judge: false,
  you: false,
  ...over,
});

describe("what a combination of the boxes is on the wire", () => {
  it("reads nothing ticked as auto, which stops nothing", () => {
    const reading = gateReadingOf(boxes());

    expect(reading.advance_gate).toBe("auto");
    expect(reading.does).toBe("Nothing stops it.");
    expect(reading.unmeant).toBe(false);
  });

  it("reads Checks alone as auto with the Checks as the whole gate", () => {
    expect(gateReadingOf(boxes({ checks: true })).advance_gate).toBe("auto");
  });

  it("reads Checks and a Judge as auto_if_judge_passes", () => {
    const reading = gateReadingOf(boxes({ checks: true, judge: true }));

    expect(reading.advance_gate).toBe("auto_if_judge_passes");
    expect(reading.unmeant).toBe(false);
  });

  // The one combination the issue is named after. `AutoIfJudgePasses` is the
  // mechanical tier holding *and* the Judge not refusing, and the enum's own
  // words are that there is no such thing as a Judge pass — so a step with no
  // Checks produces no mechanical pass for the Judge to decline.
  it("names a Judge with no Checks as a combination Fleet does nothing with yet", () => {
    const reading = gateReadingOf(boxes({ judge: true }));

    expect(reading.advance_gate).toBe("auto_if_judge_passes");
    expect(reading.unmeant).toBe(true);
  });

  it("reads You as human_always, and says what ran before you read it", () => {
    const alone = gateReadingOf(boxes({ you: true }));
    const after = gateReadingOf(boxes({ you: true, checks: true, judge: true }));

    expect(alone.advance_gate).toBe("human_always");
    expect(alone.does).toContain("nothing run before you read it");
    expect(after.does).toContain("its Checks and the Judge");
  });

  it("reads a step the repository decides as the repository's, whatever is ticked", () => {
    const reading = gateReadingOf(
      boxes({ you: true, repository_decides: "review_gate", overridden: false }),
    );

    expect(reading.advance_gate).toBe("manifest_rule:review_gate");
    expect(reading.does).toContain("review_gate policy");
  });

  // Overriding hands the step back to the three boxes, which is the whole of
  // what "you can override it for this Job" means — #1548.
  it("reads an overridden step as its own boxes again", () => {
    const reading = gateReadingOf(
      boxes({ you: true, repository_decides: "review_gate", overridden: true }),
    );

    expect(reading.advance_gate).toBe("human_always");
  });
});
