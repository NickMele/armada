// A picture, and the node that produced it.

import type { CaptureStudioNote, StagedFrame } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { sketchFromFrame, sketchOf } from "./sketch";

const frame: StagedFrame = {
  staged_path: "/tmp/armada/staged/1.png",
  width: 1440,
  height: 900,
};

function note(over: Partial<CaptureStudioNote> = {}): CaptureStudioNote {
  return {
    said: "the tab strip sits too low",
    capture: {
      element: { tag: "div", text: "Plan" },
      selector: "main > div.tabs",
      markup: "<div class=\"tabs\">Plan</div>",
      screen: "job-detail",
      location: "/jobs/01J",
      bounds: { x: 0, y: 0, width: 1440, height: 64 },
      window: { width: 1440, height: 900 },
    },
    position: { x: 120, y: 240 },
    ...over,
  };
}

describe("the sketch on a note", () => {
  it("is undefined where the note carries no picture, which is most of them", () => {
    expect(sketchOf(note())).toBeUndefined();
  });

  it("carries the staged path and its dimensions", () => {
    const sketch = sketchOf(note({ frame }));

    expect(sketch?.staged_path).toBe("/tmp/armada/staged/1.png");
    expect(sketch?.width).toBe(1440);
    expect(sketch?.height).toBe(900);
  });

  it("carries what the person said about it", () => {
    expect(sketchOf(note({ frame }))?.said).toBe("the tab strip sits too low");
  });
});

describe("provenance", () => {
  it("names the node the capture was made from", () => {
    const sketch = sketchOf(note({ frame, produced_by: "01NODE" }));

    expect(sketch?.produced_by).toBe("01NODE");
  });

  it("leaves it out for a sketch drawn from nothing, rather than sending a blank", () => {
    const sketch = sketchOf(note({ frame }));

    expect(sketch && "produced_by" in sketch).toBe(false);
  });
});

describe("a frame Bridge just staged, before any note exists", () => {
  it("makes the same shape", () => {
    expect(sketchFromFrame(frame, "here", "01NODE")).toEqual({
      staged_path: "/tmp/armada/staged/1.png",
      width: 1440,
      height: 900,
      said: "here",
      produced_by: "01NODE",
    });
  });
});
