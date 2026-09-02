// What a turn's payload lines say about themselves.
//
// **The block heading is the whole subject.** Everything else `story.ts` names
// on a line — the echoed command, what a Check's run came to, the trailer —
// comes off a `CheckRun`'s own fields and is read where those are. A heading
// comes off a list of line numbers Fleet wrote down as it wrote the blocks, and
// the defect it closes is that the list did not exist: the renderer's only
// options were to guess or to draw every line the same.
//
// So each case here is written against a guess. A short line, a shouted line
// and a line at the top of a block are all body unless the wire named them.

import { describe, expect, it } from "vitest";

import type { Turn } from "@armada/protocol";

import { entriesOf } from "./story";

/** One `instructed` row, with whatever the wire said about its lines. */
function instructed(text: string, headings?: number[]): Turn {
  return {
    ts: "2026-09-02T13:11:00Z",
    seq: 1,
    step: "plan",
    by: "armada",
    saw: { event: "instructed", occasion: "opening", text, ...(headings ? { headings } : {}) },
  };
}

/** The one row's payload. */
function payload(row: Turn) {
  const rows = entriesOf([row], "plan");
  expect(rows).toHaveLength(1);
  return rows[0]!.payload;
}

const BRIEF = ["JOB BRIEF", "", "Yes.", "", "STOP.", "", "WHERE YOU ARE"].join("\n");

describe("a block heading in a turn's payload", () => {
  it("names the lines the wire named and no others", () => {
    const said = payload(instructed(BRIEF, [0, 6]));
    expect(said.filter((line) => line.named === "heading").map((line) => line.text)).toEqual([
      "JOB BRIEF",
      "WHERE YOU ARE",
    ]);
  });

  it("leaves a short line as body, however much it looks like a heading", () => {
    // The defect the marker exists to prevent. `Yes.` is shorter than every
    // heading in the brief and `STOP.` is short and in capitals, so both are
    // caught by the two rules a renderer would reach for.
    const said = payload(instructed(BRIEF, [0, 6]));
    for (const looks of ["Yes.", "STOP."]) {
      expect(said.find((line) => line.text === looks)?.named).toBeUndefined();
    }
  });

  it("names nothing on a row the wire said nothing about", () => {
    // A turn with no headed blocks, and a row written before Fleet stamped the
    // field. The two are the same to a reader on purpose.
    const said = payload(instructed(BRIEF));
    expect(said.some((line) => line.named === "heading")).toBe(false);
    expect(said.map((line) => line.text)).toEqual(BRIEF.split("\n"));
  });

  it("keeps every line of the turn, blank ones included", () => {
    // The blank lines are the block boundary `DroneBrief` groups at, so a
    // payload that dropped them would lose the boundary the marker sits on.
    expect(payload(instructed(BRIEF, [0, 6]))).toHaveLength(7);
  });

  it("marks nothing for an index the text is too short for", () => {
    // A Fleet and a Bridge that disagree, or a text one of them cut. Nothing
    // is marked and nothing throws — an index is not a promise about length.
    const said = payload(instructed("JOB BRIEF", [0, 40]));
    expect(said.map((line) => line.named)).toEqual(["heading"]);
  });
});
