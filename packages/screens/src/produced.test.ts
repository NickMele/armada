// What the live footprint last said, as the one value the diff re-reads on.

import { describe, expect, it } from "vitest";

import type { Turn } from "@armada/protocol";

import { wroteSoFar } from "./produced";

function produced(seq: number, paths: string[]): Turn {
  return {
    ts: "2026-09-10T14:30:00Z",
    seq,
    step: "fix",
    by: "fleet",
    saw: {
      event: "produced",
      files: paths.map((path) => ({ path, change: "modified", outside_plan: false })),
    },
  };
}

function said(seq: number): Turn {
  return { ts: "2026-09-10T14:30:01Z", seq, step: "fix", by: "drone", saw: { event: "said", text: "Now the tests." } };
}

describe("wroteSoFar", () => {
  it("is the last reading, not every reading joined", () => {
    const turns = [produced(1, ["a.ts"]), said(2), produced(3, ["a.ts", "b.ts"])];
    expect(wroteSoFar(turns)).toBe(wroteSoFar([produced(9, ["a.ts", "b.ts"])]));
  });

  // **The cost of the re-read is bounded here.** Fleet republishes a footprint
  // only where it moved, and a value that did not move asks for nothing.
  it("does not move for turns that are not a reading", () => {
    const before = [produced(1, ["a.ts"])];
    expect(wroteSoFar([...before, said(2), said(3)])).toBe(wroteSoFar(before));
  });

  it("is empty where nothing has been read yet", () => {
    expect(wroteSoFar([said(1)])).toBe("");
  });
});
