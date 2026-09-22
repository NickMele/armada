// A picture, and the node that produced it.

import type { CaptureStudioNote, StagedFrame } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import type { Drawing } from "./sketch";
import {
  NOTHING_DRAWN,
  drawingOf,
  isDrawn,
  nextShapeId,
  sketchDrawn,
  sketchFromFrame,
  sketchOf,
  withBody,
  withJoin,
  withPlace,
  withShape,
  withoutShapes,
} from "./sketch";

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

  it("carries no drawing, because a capture has no boxes behind it", () => {
    expect(sketchFromFrame(frame, "here").drawn).toBeUndefined();
  });
});

/** Two boxes and the join between them, which is the smallest real pad. */
function pad(): Drawing {
  return {
    shapes: [
      { id: "b1", x: 0, y: 0, body: "the stat" },
      { id: "b2", x: 280, y: 0, body: "the panel it opens" },
    ],
    joins: [{ id: "b1-b2", from: "b1", to: "b2" }],
  };
}

describe("what is on the pad", () => {
  it("is what the sketch was drawn from", () => {
    expect(drawingOf({ ...sketchFromFrame(frame, ""), drawn: pad() })).toEqual(pad());
  });

  it("is empty for a sketch carrying none, rather than undefined a caller has to guard", () => {
    expect(drawingOf(sketchFromFrame(frame, ""))).toEqual(NOTHING_DRAWN);
    expect(drawingOf(undefined)).toEqual(NOTHING_DRAWN);
  });

  it("counts as drawn once a box is down, and a join alone is not a picture", () => {
    expect(isDrawn(NOTHING_DRAWN)).toBe(false);
    expect(isDrawn(pad())).toBe(true);
  });

  it("keeps the drawing beside the staged path when the sketch goes out", () => {
    expect(sketchDrawn(frame, "here", pad(), "01NODE").drawn).toEqual(pad());
  });
});

describe("the id the next box takes", () => {
  it("is the first one free, so removing and adding does not count up forever", () => {
    expect(nextShapeId(NOTHING_DRAWN)).toBe("b1");
    expect(nextShapeId(pad())).toBe("b3");
    expect(nextShapeId(withoutShapes(pad(), ["b1"]))).toBe("b1");
  });
});

describe("drawing on it", () => {
  it("puts a box down at the end", () => {
    const next = withShape(pad(), { id: "b3", x: 0, y: 200, body: "" });

    expect(next.shapes.map((one) => one.id)).toEqual(["b1", "b2", "b3"]);
  });

  it("replaces a box whose id is already down rather than drawing two", () => {
    const next = withShape(pad(), { id: "b2", x: 9, y: 9, body: "moved" });

    expect(next.shapes).toHaveLength(2);
    expect(next.shapes[1]).toEqual({ id: "b2", x: 9, y: 9, body: "moved" });
  });

  it("takes every join off with the box it hung on", () => {
    const next = withoutShapes(pad(), ["b2"]);

    expect(next.shapes.map((one) => one.id)).toEqual(["b1"]);
    expect(next.joins).toEqual([]);
  });

  it("rewrites the words in one box and leaves the rest alone", () => {
    const next = withBody(pad(), "b1", "the Drones stat");

    expect(next.shapes[0]?.body).toBe("the Drones stat");
    expect(next.shapes[1]?.body).toBe("the panel it opens");
  });

  it("changes nothing for a box the pad does not hold", () => {
    expect(withBody(pad(), "b9", "nowhere")).toEqual(pad());
    expect(withPlace(pad(), "b9", { x: 1, y: 1 })).toEqual(pad());
  });

  it("puts a box down somewhere new", () => {
    expect(withPlace(pad(), "b1", { x: 40, y: 80 }).shapes[0]).toEqual({
      id: "b1",
      x: 40,
      y: 80,
      body: "the stat",
    });
  });
});

describe("joining two boxes", () => {
  it("draws the line in the direction it was made", () => {
    const next = withJoin({ ...pad(), joins: [] }, "b2", "b1");

    expect(next.joins).toEqual([{ id: "b2-b1", from: "b2", to: "b1" }]);
  });

  it("leaves a pair already joined exactly as it was, whichever way round", () => {
    expect(withJoin(pad(), "b1", "b2")).toEqual(pad());
    expect(withJoin(pad(), "b2", "b1")).toEqual(pad());
  });

  it("refuses to join a box to itself", () => {
    expect(withJoin(pad(), "b1", "b1")).toEqual(pad());
  });
});
