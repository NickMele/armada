// Where a job came from, as the board row and the header both read it. #1362.
//
// **The words are the registry's and are asserted as such.** A test that wrote
// its own expected sentence would pass while the row drew a different one.

import { describe, expect, it } from "vitest";

import type { JobSummary } from "@armada/protocol";
import { fromAStudio, originReading } from "./origin";

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-13T09:00:00Z",
    ...over,
  };
}

describe("what a row says about where a job came from", () => {
  it("reads a Studio dispatch as where from, then who pressed", () => {
    expect(originReading(job({ origin: "studio_dispatched" }))).toBe("From a Studio, by you");
    expect(originReading(job({ origin: "studio_helm_drafted" }))).toBe("From a Studio, via Helm");
  });

  it("fills sub_dispatched's slot with the parent's id", () => {
    const sub = job({ origin: "sub_dispatched", dispatched_by: "01PARENT0000000000000000" });
    expect(originReading(sub)).toBe("Sub-dispatched by 01PARENT0000000000000000");
  });

  it("draws nothing where the form has nothing to fill it", () => {
    // A Fleet built before `JobSummary.dispatched_by`. The literal template is
    // worse than a blank cell.
    expect(originReading(job({ origin: "sub_dispatched" }))).toBeUndefined();
  });

  it("draws nothing for a spelling this Bridge's vocabulary does not hold", () => {
    // A Fleet ahead of this build. Nothing is what it can truthfully say.
    expect(originReading(job({ origin: "a_value_from_a_newer_fleet" }))).toBeUndefined();
  });
});

describe("whether a job came off a Studio", () => {
  it("is read off origin, which survives the Studio being deleted", () => {
    expect(fromAStudio(job({ origin: "studio_dispatched" }))).toBe(true);
    expect(fromAStudio(job({ origin: "studio_helm_drafted" }))).toBe(true);
  });

  it("is false for every origin that is not a Studio's", () => {
    for (const origin of ["manual", "helm_drafted", "auto_detected", "sub_dispatched", "drone_drafted"]) {
      expect(fromAStudio(job({ origin }))).toBe(false);
    }
  });
});
