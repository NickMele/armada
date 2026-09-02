// What `story.ts` answers: why a log has no rows, and what a turn's payload
// lines say about themselves.
//
// **Three answers where there were two, and that is the first subject.** Four
// of the five `Observed` states carry no live rows and `story.ts` had one
// sentence for all of them, so a socket that had failed, one that had closed
// and a Bridge that was not connected all read as a step that had not started.
// Each case below is written against that sentence: it survives for the step
// nothing has happened on, and nothing else is given it. #324.
//
// **The block heading is the second.** Everything else `story.ts` names on a
// line — the echoed command, what a Check's run came to, the trailer — comes
// off a `CheckRun`'s own fields and is read where those are. A heading comes
// off a list of line numbers Fleet wrote down as it wrote the blocks, and the
// defect it closes is that the list did not exist: the renderer's only options
// were to guess or to draw every line the same. So each of those cases is
// written against a guess. A short line, a shouted line and a line at the top
// of a block are all body unless the wire named them.

import { describe, expect, it } from "vitest";

import type { Observed, Turn, Turns } from "@armada/protocol";

import { entriesOf, NOTHING_YET_ON_THIS_STEP, whyNotWatching } from "./story";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

/** One connection that has said something and lost nothing. */
const CARRIED: Turns = { live: true, skipped: 0, missed: 0, rows: [] };

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

describe("why the log is not being read", () => {
  it("says nothing at all while it is being read", () => {
    const watching: Observed = { state: "watching", jobId: A_JOB, turns: CARRIED };
    // Nothing, so the sentence about a step that has not started is what a step
    // that has not started gets — and only that step.
    expect(whyNotWatching(watching)).toBeUndefined();
  });

  it("says which reading failed, in the detail main gave it", () => {
    const failed: Observed = {
      state: "failed",
      jobId: A_JOB,
      turns: CARRIED,
      detail: "Fleet is not connected.",
    };
    const said = whyNotWatching(failed);
    expect(said).toContain("Fleet is not connected.");
    expect(said).not.toBe(NOTHING_YET_ON_THIS_STEP);
  });

  it("tells a drone that has finished from a job nothing is writing", () => {
    const drone: Observed = {
      state: "ended",
      jobId: A_JOB,
      turns: CARRIED,
      because: "drone_ended",
    };
    const nothing: Observed = { ...drone, because: "nothing_writing" };
    expect(whyNotWatching(drone)).not.toBe(whyNotWatching(nothing));
  });

  it("renders a reason of its own, so a transport close is not a wire word", () => {
    const closed: Observed = {
      state: "ended",
      jobId: A_JOB,
      turns: CARRIED,
      because: "the connection closed",
    };
    expect(whyNotWatching(closed)).toContain("the connection closed");
  });

  it("says the transcript is being opened rather than that nothing happened", () => {
    expect(whyNotWatching({ state: "opening", jobId: A_JOB })).toBeDefined();
    expect(whyNotWatching({ state: "none" })).toBeDefined();
  });
});

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
