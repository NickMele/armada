import { describe, expect, it } from "vitest";

import type { JobDetail, JobSummary } from "@armada/protocol";

import { waveOf } from "./wave";

function row(over: Partial<JobSummary> & Pick<JobSummary, "id">): JobSummary {
  return {
    handle: `${over.id}-a-job`,
    title: `Job ${over.id}`,
    status: "running",
    workflow_id: "feature",
    owner_manifest_id: "m",
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-22T07:20:00Z",
    ...over,
  };
}

function parent(): JobDetail {
  return {
    job: row({ id: "parent", title: "Carry the error contract through every surface" }),
    created_at: "2026-09-22T07:10:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
  };
}

describe("the wave a Job dispatched", () => {
  it("is every row the Job dispatched, and nothing else", () => {
    const wave = waveOf(parent(), [
      row({ id: "a", origin: "sub_dispatched", dispatched_by: "parent" }),
      row({ id: "b", origin: "sub_dispatched", dispatched_by: "somebody-else" }),
      row({ id: "c", origin: "sub_dispatched", dispatched_by: "parent" }),
    ]);
    expect(wave?.jobs.map((one) => one.job)).toEqual(["a", "c"]);
  });

  // Absent, never an empty graph: an empty one reads as Jobs that failed to
  // load, which is a different sentence from a Job that dispatched none.
  it("is absent where the Job dispatched nothing", () => {
    expect(waveOf(parent(), [row({ id: "a" })])).toBeUndefined();
  });

  it("carries each Job's status and where its pull request settled", () => {
    const wave = waveOf(parent(), [
      row({ id: "a", origin: "sub_dispatched", dispatched_by: "parent", status: "completed_success", landed: "merged" }),
    ]);
    expect(wave?.jobs[0]).toMatchObject({ status: "completed_success", landed: "merged" });
  });

  // Nothing on the wire records the order, and an invented one would be a
  // graph saying something Fleet never said.
  it("leaves the waiting order empty, because the wire carries none", () => {
    const wave = waveOf(parent(), [
      row({ id: "a", origin: "sub_dispatched", dispatched_by: "parent" }),
      row({ id: "b", origin: "sub_dispatched", dispatched_by: "parent" }),
    ]);
    expect(wave?.jobs.every((one) => one.waits_on.length === 0)).toBe(true);
  });

  it("reads as one pass, because a Board row does not say which it came from", () => {
    const wave = waveOf(parent(), [
      row({ id: "a", origin: "sub_dispatched", dispatched_by: "parent" }),
    ]);
    expect(wave?.rounds).toEqual([
      { round: 1, says: "Carry the error contract through every surface", live: true },
    ]);
  });
});
