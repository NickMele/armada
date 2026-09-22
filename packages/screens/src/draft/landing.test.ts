// What a landing rule says while Fleet cuts one branch and opens one PR.

import { describe, expect, it } from "vitest";

import { COMPLETE_WHEN_SERVED, landingRuleOf } from "./landing";

describe("where the work starts and where it lands", () => {
  it("takes both from the Manifest's base, which is why they agree today", () => {
    const rule = landingRuleOf({ checks: [], commands: [], ports: [], auto_merge: { written: "never", offered: [] }, review_gate: { written: "never", offered: [] }, base: "main" });

    expect(rule.target).toBe("main");
    expect(rule.from_ref).toBe("main");
  });

  it("is null where the Manifest names none, and never a branch nobody chose", () => {
    const rule = landingRuleOf();

    expect(rule.target).toBeNull();
    expect(rule.from_ref).toBeNull();
  });
});

describe("the shape one Job lands in today", () => {
  it("is one pull request and one branch per Job, never per group", () => {
    const rule = landingRuleOf();

    expect(rule.prs).toBe("job");
    expect(rule.branching).toBe("job");
  });

  it("offers the pull request ready, since nothing on the wire opens a draft", () => {
    expect(landingRuleOf().pr_mode).toBe("ready");
  });

  it("lands no groups together, which is every group landing on its own", () => {
    expect(landingRuleOf().land_together).toEqual([]);
  });

  it("completes when the work is delivered", () => {
    expect(landingRuleOf().complete_when).toBe("delivered");
  });
});

describe("which completion rules Fleet can answer", () => {
  it("serves only the one the derivation picks", () => {
    expect(COMPLETE_WHEN_SERVED.delivered).toBe(true);
  });

  it("does not serve pr_merged: the repository's policy decides at the gate", () => {
    expect(COMPLETE_WHEN_SERVED.pr_merged).toBe(false);
  });

  it("does not serve the two that need members or an opened pull request", () => {
    expect(COMPLETE_WHEN_SERVED.all_members_landed).toBe(false);
    expect(COMPLETE_WHEN_SERVED.pr_opened).toBe(false);
  });
});
