import { describe, expect, it } from "vitest";
import { ciOf, ciShown, saidOf } from "./ci";

describe("saidOf", () => {
  it("counts what failed out of what ran", () => {
    expect(saidOf({ kind: "some_failed", checks: 4, failed: ["unit tests", "lint"] })).toBe(
      "2 of 4 failed.",
    );
  });

  it("says when every check passed, and when nothing ran", () => {
    expect(saidOf({ kind: "all_passed", checks: 3, finished: 3 })).toBe("All 3 passed.");
    expect(saidOf({ kind: "nothing_ran", checks: 0 })).toBe("Nothing ran against the pull request.");
  });

  it("says Fleet has not read it where there is no reading", () => {
    expect(saidOf(undefined)).toBe("Fleet has not read the pull request's CI yet.");
  });
});

describe("ciOf", () => {
  it("puts a conflict with main ahead of a failed run", () => {
    const ci = ciOf({ kind: "some_failed", checks: 2, failed: ["unit tests"] }, true, {});
    expect(ci.said).toBe("The branch conflicts with main.");
    expect(ci.failed).toEqual([]);
    expect(ci.conflicted).toBe(true);
  });
});

describe("ciShown", () => {
  it("draws nothing where nothing ran against the pull request", () => {
    expect(ciShown({ kind: "nothing_ran", checks: 0 }, false)).toBe(false);
    expect(ciShown(undefined, false)).toBe(false);
  });

  it("draws the row where CI ran, or where the branch conflicts whatever ran", () => {
    expect(ciShown({ kind: "all_passed", checks: 3, finished: 3 }, false)).toBe(true);
    expect(ciShown({ kind: "nothing_ran", checks: 0 }, true)).toBe(true);
  });
});
