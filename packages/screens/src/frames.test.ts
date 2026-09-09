// What a step's frames draw as: the rows, married to what has been fetched.
//
// **The arithmetic half.** What is asserted here is the reading — wire order,
// the weight, and what a collapsed chapter's summary says. The half that mints
// and revokes object URLs is a hook, so it is mounted, and it is in
// `frames.test.tsx` for the reason this package's config gives.

import { describe, expect, it } from "vitest";

import type { KeptFrame } from "@armada/protocol";

import { framesSummary, shownFrames, NO_FRAMES } from "./frames";

function frame(over: Partial<KeptFrame> = {}): KeptFrame {
  return {
    attempt: 1,
    name: "home.png",
    path: ".armada/frames/12-a-job/show.1/home.png",
    bytes: 41_002,
    kept: "show.1/home.png",
    ...over,
  };
}

describe("what a step's frames draw as", () => {
  it("keeps wire order, because that ordering is the record's", () => {
    const rows = [
      frame({ kept: "show.1/home.png", name: "home.png" }),
      frame({ kept: "show.1/a.png", name: "a.png" }),
    ];
    // Sorted, `a.png` would lead — and a reader comparing two runs would be
    // reading them in an order Fleet never answered in.
    expect(shownFrames(rows, NO_FRAMES).map((shown) => shown.name)).toEqual([
      "home.png",
      "a.png",
    ]);
  });

  it("carries the weight before the bytes, so a slow one reads as a large file", () => {
    const shown = shownFrames([frame({ bytes: 41_002 })], NO_FRAMES);
    expect(shown[0]!.weight).toBe("40.0 KB");
    // Neither drawn nor failed: the plate says the wait. A frame nothing has
    // asked for and one still in flight are the same wait from where the
    // person is sitting, and this is what makes them read the same.
    expect(shown[0]!.src).toBeUndefined();
    expect(shown[0]!.why).toBeUndefined();
  });

  it("carries the run on every frame rather than implying it by position", () => {
    const shown = shownFrames(
      [frame(), frame({ attempt: 3, kept: "show.3/home.png" })],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.attempt)).toEqual([1, 3]);
    // The same file name from two runs, told apart by `kept` and never by the
    // harness's own name for it.
    expect(shown.map((one) => one.kept)).toEqual(["show.1/home.png", "show.3/home.png"]);
  });

  it("weighs a small file in bytes and a large one in megabytes", () => {
    expect(shownFrames([frame({ bytes: 900 })], NO_FRAMES)[0]!.weight).toBe("900 B");
    expect(shownFrames([frame({ bytes: 4_200_000 })], NO_FRAMES)[0]!.weight).toBe("4.0 MB");
  });
});

describe("what a collapsed chapter says about them", () => {
  it("says nothing about a step with no frames", () => {
    expect(framesSummary([])).toBeUndefined();
  });

  it("counts one frame in the singular", () => {
    expect(framesSummary([frame()])).toBe("1 frame");
  });

  it("leaves the runs out where a step was worked once", () => {
    expect(framesSummary([frame(), frame({ kept: "show.1/b.png" })])).toBe("2 frames");
  });

  /**
   * Two frames from two runs and two frames from one are different things to
   * be looking at. A summary that read `2 frames` for both would hide the fact
   * that decides whether what a reader is about to open is current.
   */
  it("says how many runs where a step was worked more than once", () => {
    expect(framesSummary([frame(), frame({ attempt: 2, kept: "show.2/home.png" })])).toBe(
      "2 frames · 2 runs",
    );
  });

  /**
   * Whether there is a before is the fact that decides what a collapsed
   * chapter is worth opening for: with both sides it answers what the change
   * did to the screen, with one it answers only what the screen is.
   */
  it("says whether there is a before, where there is one", () => {
    expect(
      framesSummary([
        frame({ kept: "show.1.base/home.png", side: "base" }),
        frame({ kept: "show.1.branch/home.png", side: "branch" }),
      ]),
    ).toBe("2 frames · before and after");
    expect(framesSummary([frame({ side: "branch" })])).toBe("1 frame");
  });
});

describe("which side a frame is a photograph of", () => {
  /**
   * **`before` and `after`, not `base` and `branch`.** The wire's words name
   * the two checkouts Fleet had to serve; what a reviewer wants to know is
   * which of these is how the screen was, and the translation happens once so
   * no component learns what a base branch is.
   */
  it("says the reader's word for each side where there are two", () => {
    const shown = shownFrames(
      [
        frame({ kept: "show.1.base/home.png", side: "base" }),
        frame({ kept: "show.1.branch/home.png", side: "branch" }),
      ],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.side)).toEqual(["before", "after"]);
  });

  /**
   * A step with one side is a repository with no base, a base run that would
   * not start, or a Fleet older than 9.5. `after` written on every frame of a
   * set with no before is a word that says nothing and implies a missing half.
   */
  it("labels nothing where there is only one side to be on", () => {
    const shown = shownFrames(
      [
        frame({ kept: "show.1.branch/home.png", side: "branch" }),
        frame({ kept: "show.1.branch/settings.png", side: "branch" }),
      ],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.side)).toEqual([undefined, undefined]);
  });

  /** A Fleet older than 9.5 sends no side at all, and those rows are branch. */
  it("reads a row with no side as the branch, which is what it is", () => {
    const shown = shownFrames(
      [frame({ kept: "show.1.base/home.png", side: "base" }), frame({ side: undefined })],
      NO_FRAMES,
    );
    expect(shown.map((one) => one.side)).toEqual(["before", "after"]);
  });
});
