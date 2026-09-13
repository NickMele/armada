import { describe, expect, it } from "vitest";
import type { JobConfidence } from "@armada/protocol";
import { issueDraftOf } from "./followup";

const confidence: JobConfidence = {
  says: "confident",
  reasons: [],
  areas: [],
  needs_you: [],
  small_fixes: [],
  for_context: [{ finding: "`headroom.rs` has grown", why: "The author flagged it." }],
};

describe("issueDraftOf", () => {
  it("titles the draft with the finding's words, and carries why it was raised", () => {
    expect(issueDraftOf(confidence, "`headroom.rs` has grown")).toEqual({
      finding: "`headroom.rs` has grown",
      title: "headroom.rs has grown",
      body: "`headroom.rs` has grown\n\nWhy it was raised: The author flagged it.\n\nRaised by Armada's review.",
    });
  });

  it("leaves the reason out where the review gave none it can find", () => {
    expect(issueDraftOf(confidence, "Something else").body).toBe(
      "Something else\n\nRaised by Armada's review.",
    );
  });
});
