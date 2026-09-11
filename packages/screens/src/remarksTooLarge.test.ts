// The guidance a `take_up_remarks` press draws when Fleet refuses it for
// size — never a code, never a raw handle, always who wrote the comment and
// how it opens.

import { describe, expect, it } from "vitest";

import type { Outcome, Remark, Remarks, WireError } from "@armada/protocol";
import { REMARKS_TOO_LARGE, tooLargeIn } from "./remarksTooLarge";

const JOB_ID = "01M130Y1380016YK5S0JXBXDQ5";
const OTHER_JOB_ID = "01M130Y1380016YK5S0JXBXDQ6";

function wireError(over: Partial<WireError> = {}): WireError {
  return {
    code: REMARKS_TOO_LARGE,
    message: "the comments picked would not fit the room an opening brief leaves free",
    run_id: "a-run",
    fields: { remarks: "IC_big, IC_medium" },
    chain: [],
    job_id: JOB_ID,
    ...over,
  };
}

function refused(error: Partial<WireError> = {}): Outcome {
  return { ok: false, why: "refused", error: wireError(error) };
}

function remark(over: Partial<Remark>): Remark {
  return { id: "IC_x", by: "somebody", at: "2026-09-08T10:00:00Z", said: "said something", taken_up: false, ...over };
}

function remarks(review: Remark[]): Remarks {
  return { state: "read", jobId: JOB_ID, review: { job_id: JOB_ID, pull_request: "https://forge.invalid/pr/1", remarks: review } };
}

describe("tooLargeIn", () => {
  it("is null for every outcome that is not this refusal", () => {
    expect(tooLargeIn(null, JOB_ID, { state: "none" })).toBeNull();
    expect(tooLargeIn({ ok: true }, JOB_ID, { state: "none" })).toBeNull();
    expect(tooLargeIn(refused({ code: "fleet.remarks_gone" }), JOB_ID, { state: "none" })).toBeNull();
  });

  it("is null where the refusal names a different job", () => {
    const answer = tooLargeIn(refused({ job_id: OTHER_JOB_ID }), JOB_ID, { state: "none" });
    expect(answer).toBeNull();
  });

  it("names each dropped comment by author and opening words, read off the comments already held", () => {
    const held = remarks([
      remark({ id: "IC_big", by: "alice", said: "rename the flag everywhere" }),
      remark({ id: "IC_medium", by: "bob", said: "the migration needs a down step" }),
      remark({ id: "IC_small", by: "carol", said: "left alone" }),
    ]);
    const answer = tooLargeIn(refused(), JOB_ID, held);
    expect(answer).not.toBeNull();
    expect(answer!.ids).toEqual(new Set(["IC_big", "IC_medium"]));
    expect(answer!.said).toContain("alice");
    expect(answer!.said).toContain("rename the flag everywhere");
    expect(answer!.said).toContain("bob");
    expect(answer!.said).toContain("the migration needs a down step");
    expect(answer!.said).not.toContain("IC_big");
    expect(answer!.said).not.toContain("IC_medium");
    expect(answer!.said).not.toContain("carol");
  });

  it("still marks the ids where the comments have not loaded yet", () => {
    const answer = tooLargeIn(refused(), JOB_ID, { state: "none" });
    expect(answer).not.toBeNull();
    expect(answer!.ids).toEqual(new Set(["IC_big", "IC_medium"]));
  });

  it("says 'one' rather than a count of one", () => {
    const held = remarks([remark({ id: "IC_big", by: "alice", said: "rename the flag" })]);
    const answer = tooLargeIn(refused({ fields: { remarks: "IC_big" } }), JOB_ID, held);
    expect(answer!.said).toMatch(/^One comment picked/);
  });
});
