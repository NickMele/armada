// What a criterion says about how it is answered, and where its words began.

import { describe, expect, it } from "vitest";

import { criterionViewOf, criterionViewsOf } from "./criterion";
import { sampleDetail } from "./sample";

function criterion(source: string, text = "The gate refuses a main-process import") {
  return { criterion_id: "AC1", text, source };
}

describe("how a criterion is answered", () => {
  it("renames the wire's source rather than keeping two meanings on one row", () => {
    const view = criterionViewOf(criterion("check"));

    expect(view.verified_by).toBe("check");
    expect("source" in view).toBe(false);
  });

  it("carries all three of the registry's spellings", () => {
    expect(criterionViewOf(criterion("check")).verified_by).toBe("check");
    expect(criterionViewOf(criterion("judge")).verified_by).toBe("judge");
    expect(criterionViewOf(criterion("attested")).verified_by).toBe("attested");
  });

  it("reads a spelling it does not know as judge, never as check", () => {
    expect(criterionViewOf(criterion("something_new")).verified_by).toBe("judge");
  });
});

describe("where the words came from", () => {
  it("is the prompt, because the wire records no issue and no person", () => {
    expect(criterionViewOf(criterion("judge")).origin).toEqual({ origin: "prompt" });
  });

  it("says nothing about the source having moved", () => {
    expect(criterionViewOf(criterion("judge")).origin_moved_at).toBeUndefined();
  });
});

describe("a Job's criteria", () => {
  it("keeps the order they were given in", () => {
    const detail = sampleDetail({
      acceptance_criteria: [
        { criterion_id: "AC1", text: "first", source: "check" },
        { criterion_id: "AC2", text: "second", source: "judge" },
      ],
    });

    expect(criterionViewsOf(detail).map((view) => view.text)).toEqual(["first", "second"]);
  });

  it("is empty on a Job held to nothing", () => {
    expect(criterionViewsOf(sampleDetail())).toEqual([]);
  });
});
