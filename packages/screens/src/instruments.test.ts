import { describe, expect, it } from "vitest";

import type { JobFootprint, TouchedFile, Turn } from "@armada/protocol";

import { ACTIVITY_WINDOWS, activityOf, footprintOf } from "./instruments";

const NOW = Date.parse("2026-09-17T12:00:00Z");

function called(secondsAgo: number, step: string | undefined = "implement", seq = 1): Turn {
  return {
    ts: new Date(NOW - secondsAgo * 1000).toISOString(),
    seq,
    ...(step === undefined ? {} : { step }),
    by: "drone",
    saw: { event: "called", tool: "Bash", call: `c${seq}`, detail: "", truncated: false },
  };
}

describe("activity", () => {
  it("draws twenty-four windows even with nothing in them", () => {
    const quiet = activityOf([], "implement", NOW);
    expect(quiet.windows).toHaveLength(ACTIVITY_WINDOWS);
    expect(quiet.windows.every((count) => count === 0)).toBe(true);
    expect(quiet.description).toBe("No tool calls for the last 12 minutes");
  });

  it("puts a call in the window it happened in, oldest first", () => {
    const { windows } = activityOf([called(1), called(29), called(31), called(719)], "implement", NOW);
    expect(windows[23]).toBe(2);
    expect(windows[22]).toBe(1);
    expect(windows[0]).toBe(1);
  });

  it("leaves out a call twelve minutes old or older", () => {
    expect(activityOf([called(720), called(900)], "implement", NOW).calls).toBe(0);
  });

  it("counts a call stamped just after now in the newest window", () => {
    expect(activityOf([called(-2)], "implement", NOW).windows[23]).toBe(1);
  });

  it("counts only called rows, and only this step's or a row with no step", () => {
    const answered: Turn = {
      ts: new Date(NOW - 5000).toISOString(),
      seq: 9,
      step: "implement",
      by: "drone",
      saw: { event: "answered", call: "c1", failed: false },
    };
    const rows = [called(5, "implement"), called(5, "plan"), called(5, undefined), answered];
    expect(activityOf(rows, "implement", NOW).calls).toBe(2);
  });

  it("says how long the tail has been quiet", () => {
    const busyThenQuiet = activityOf([called(600), called(590), called(250)], "implement", NOW);
    expect(busyThenQuiet.description).toBe("3 tool calls in the last 12 minutes, none for the last 4 minutes");
    expect(activityOf([called(40)], "implement", NOW).description).toBe(
      "1 tool call in the last 12 minutes, none for the last 30 seconds",
    );
    expect(activityOf([called(3)], "implement", NOW).description).toBe(
      "1 tool call in the last 12 minutes, some in the last 30 seconds",
    );
  });
});

function touched(path: string, lines?: [number, number], plannedBy?: string[]): TouchedFile {
  return {
    path,
    change: "modified",
    ...(lines === undefined ? {} : { lines: { added: lines[0], deleted: lines[1] } }),
    ...(plannedBy === undefined ? {} : { planned_by: plannedBy }),
  };
}

function kept(files: TouchedFile[]): JobFootprint {
  return { files, recorded_at: "2026-09-17T12:00:00Z" };
}

describe("footprint", () => {
  it("never gives a file without a count a column, and counts it instead", () => {
    const drawing = footprintOf(kept([touched("a.ts", [3, 1]), touched("logo.png")]));
    expect(drawing.columns.map((column) => column.path)).toEqual(["a.ts"]);
    expect(drawing.uncounted).toBe(1);
    expect(drawing.description).toBe("1 file changed 4 lines, +3 −1; 1 file not counted");
  });

  it("orders columns widest first", () => {
    const drawing = footprintOf(kept([touched("small", [1, 0]), touched("big", [80, 20]), touched("mid", [5, 5])]));
    expect(drawing.columns.map((column) => column.path)).toEqual(["big", "mid", "small"]);
    expect(drawing.description).toBe("3 files changed 111 lines, +86 −25; big took 90% of them");
  });

  it("outlines only a path outside every declared plan, never one nothing measured", () => {
    const drawing = footprintOf(
      kept([touched("drift", [2, 0], []), touched("planned", [4, 0], ["plan"]), touched("unmeasured", [1, 0])]),
    );
    const outside = drawing.columns.filter((column) => column.outsidePlan).map((column) => column.path);
    expect(outside).toEqual(["drift"]);
    expect(drawing.description).toContain("1 file outside every declared plan");
  });

  it("draws nothing for a file counted at zero, and does not call it uncounted", () => {
    const drawing = footprintOf(kept([touched("moved", [0, 0])]));
    expect(drawing.columns).toEqual([]);
    expect(drawing.uncounted).toBe(0);
    expect(drawing.description).toBe("No changed lines were counted");
  });
});
