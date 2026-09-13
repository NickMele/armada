// What a freeze says on a Job, and about a press it took. The claim: a press is
// never read as refused, and nothing is said about a freeze that is not holding.

import { describe, expect, it } from "vitest";
import type { JobSummary } from "@armada/protocol";

import { job } from "./fixtures/build/base";
import { freezeLineOf, named, takenNotice, takenStands, type Taken } from "./freeze";

const at = (status: string, over: Partial<JobSummary> = {}): JobSummary => job(status, over);

describe("a Job a freeze holds", () => {
  it("names the frozen repository on a queued row that reads frozen", () => {
    expect(freezeLineOf(at("queued", { queued_reason: "frozen", frozen_by: ["armada"] }))).toEqual({
      lead: "Waits for",
      names: "armada",
      tail: "to unfreeze",
    });
  });

  it("says nothing lands on a row at review, whose status is its own", () => {
    expect(freezeLineOf(at("awaiting_review", { frozen_by: ["armada", "web"] }))).toEqual({
      lead: "Nothing lands until",
      names: "armada and web",
      tail: "unfreeze",
    });
  });

  it("says nothing where no freeze holds, or where the queue holds it for another reason", () => {
    expect(freezeLineOf(at("queued", { queued_reason: "waiting_on_resources" }))).toBeNull();
    expect(freezeLineOf(at("awaiting_review"))).toBeNull();
    expect(freezeLineOf(at("queued", { queued_reason: "frozen" }))).toBeNull();
  });

  it("lists several repositories the way a sentence does", () => {
    expect(named(["armada"])).toBe("armada");
    expect(named(["armada", "web", "api"])).toBe("armada, web and api");
  });
});

describe("a press taken at a frozen repository", () => {
  const merge: Taken = { jobId: at("awaiting_review").id, act: "merge", from: "awaiting_review" };

  it("says a merge was taken and waits, and that a restart drops it", () => {
    const notice = takenNotice(merge, at("awaiting_review", { frozen_by: ["armada"] }));
    expect(notice?.title).toBe("Merge taken");
    expect(notice?.body).toMatch(/merges when the freeze lifts/);
    expect(notice?.body).toMatch(/press Merge again/);
  });

  it("says an approval waits once the Job reaches the queue frozen", () => {
    const approve: Taken = { jobId: merge.jobId, act: "approve", from: "awaiting_approval" };
    expect(takenNotice(approve, at("awaiting_approval"))).toBeNull();
    expect(takenNotice(approve, at("queued", { queued_reason: "frozen", frozen_by: ["armada"] }))?.title).toBe(
      "Approval taken",
    );
  });

  it("stands until the Job moves on unfrozen or is over", () => {
    const restart: Taken = { jobId: merge.jobId, act: "restart", from: "escalated" };
    expect(takenStands(restart, at("escalated"))).toBe(true);
    expect(takenStands(restart, at("queued", { queued_reason: "frozen", frozen_by: ["armada"] }))).toBe(true);
    expect(takenStands(restart, at("running"))).toBe(false);
    expect(takenStands(merge, at("completed_success"))).toBe(false);
    expect(takenStands(merge, undefined)).toBe(false);
  });
});
