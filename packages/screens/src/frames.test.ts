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
});
