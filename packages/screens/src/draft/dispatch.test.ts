// Nothing set is a real answer, and how a workflow lands is read off the step
// that delivers.

import type { WorkflowStep, WorkflowSummary } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { dispatchSettingsOf, howManySet, landsAt, NOTHING_SET } from "./dispatch";
import type { ProposalView } from "./proposal";

function step(over: Partial<WorkflowStep> = {}): WorkflowStep {
  return {
    step_id: "implement",
    label: "Implement",
    checks: [],
    judge_checks: [],
    advance_gate: "auto_if_judge_passes",
    delivers: false,
    ...over,
  };
}

function workflow(steps: WorkflowStep[]): WorkflowSummary {
  return { id: "feature", name: "feature", version: 1, manifest_id: "01M", steps };
}

function proposal(over: Partial<ProposalView> = {}): ProposalView {
  return {
    status: "classifying",
    title: "Show what is running in the Drones stat",
    gates: [],
    fleet_always_looks: true,
    tiers: { difficult: "opus", medium: "sonnet", easy: null },
    machine_cap: 4,
    from_ref: "main",
    pr_mode: "ready",
    ...over,
  };
}

describe("nothing set", () => {
  it("counts none", () => {
    expect(howManySet(NOTHING_SET)).toBe(0);
  });

  it("counts each field a person moved, and never a field that is absent", () => {
    expect(howManySet({ drone_cap: 2 })).toBe(1);
    expect(howManySet({ drone_cap: 2, lands: "auto", workflow_id: "bug" })).toBe(3);
  });

  it("counts a cap of zero, because somebody typed it", () => {
    expect(howManySet({ drone_cap: 0 })).toBe(1);
  });
});

describe("the same settings one moment later", () => {
  it("takes the tier map a proposal resolved", () => {
    expect(dispatchSettingsOf(proposal()).tiers).toEqual({
      difficult: "opus",
      medium: "sonnet",
      easy: null,
    });
  });

  it("leaves a cap absent where the proposal carries none", () => {
    expect(dispatchSettingsOf(proposal())).not.toHaveProperty("drone_cap");
  });

  it("carries a cap the proposal does hold", () => {
    expect(dispatchSettingsOf(proposal({ drone_cap: 2 })).drone_cap).toBe(2);
  });
});

describe("how a workflow lands", () => {
  it("reads the delivering step, not the last one", () => {
    const review = workflow([
      step({ step_id: "review", delivers: true, advance_gate: "human_always" }),
      step({ step_id: "record", advance_gate: "auto" }),
    ]);

    expect(landsAt(review)).toBe("you_at_review");
  });

  it("reads auto where the delivering step stops for nobody", () => {
    expect(landsAt(workflow([step({ delivers: true, advance_gate: "auto" })]))).toBe("auto");
  });

  it("reads auto where nothing is delivered at all", () => {
    expect(landsAt(workflow([step()]))).toBe("auto");
  });
});
