// The activity log's own controls, and whether pressing one does anything.
//
// Three controls on that sheet did nothing at all, and none of them looked
// broken: the four filter tabs were drawn from the component's own default and
// handed no handler, `Jump to now` held the reading again instead of releasing
// it, and the escalation notice captioned its button `Esc` beside a `Close`
// captioned `Esc` under a `Back to the list` captioned `Esc`.
//
// What is checked here is the selection, because that is the half with a
// return value. The other two are a handler that is now passed and a caption
// that is now absent, and both are read off the rendered tree — the sheet's own
// stories — rather than from a function.

import { describe, expect, it } from "vitest";

import { keyFor } from "@armada/components";

import type { Diff, JobDetail as JobWhole } from "@armada/protocol";

import { gaveBackTheWorktree, NO_SHEET, sheetMoved, shownBy } from "./Sheets";
import type { LogRow } from "./story";

function row(actor: LogRow["actor"], id: string): LogRow {
  return { id, at: "16:33:52", actor, kind: "note", message: id, payload: [] };
}

const ROWS = [
  row("armada", "a1"),
  row("drone", "d1"),
  row("fleet", "f1"),
  row("drone", "d2"),
];

describe("the activity log's filters", () => {
  it("shows every line under All, and the same array rather than a copy", () => {
    expect(shownBy(ROWS, "all")).toBe(ROWS);
  });

  it("selects one actor's lines, in the order they arrived", () => {
    expect(shownBy(ROWS, "drone").map((one) => one.id)).toEqual(["d1", "d2"]);
    expect(shownBy(ROWS, "fleet").map((one) => one.id)).toEqual(["f1"]);
    expect(shownBy(ROWS, "armada").map((one) => one.id)).toEqual(["a1"]);
  });

  // The filter names and the wire's actor names are the same three words, so a
  // rename on either side that broke the pairing would show up as an empty
  // sheet rather than as a type error.
  it("leaves no actor unreachable, so no tab is a dead control", () => {
    const reached = (["drone", "fleet", "armada"] as const).flatMap((filter) =>
      shownBy(ROWS, filter).map((one) => one.id),
    );
    expect(reached.sort()).toEqual(ROWS.map((one) => one.id).sort());
  });

  it("comes back empty where an actor wrote nothing, rather than falling back to all", () => {
    expect(shownBy([row("drone", "d1")], "fleet")).toEqual([]);
  });
});

// The detail's keys, read off the registry rather than off this file.
//
// **The caption and the binding come from one place or they drift.** `Open the
// log` was captioned `Enter` for four rounds of feedback after `actions.toml`
// moved it to `L`: the shared configuration existed the whole time and the one
// call site drawing the caption did not read it. What is asserted here is that
// the reader agrees with the registry, so a move in `actions.toml` that this
// file does not follow fails rather than ships.
describe("the detail's key captions", () => {
  it("reads the log's key from the registry, and it is not Enter", () => {
    expect(keyFor("open_log")).toBe("L");
    expect(keyFor("open_log")).not.toBe("Enter");
  });

  it("reads the diff's key from the registry", () => {
    expect(keyFor("open_diff")).toBe("f");
  });

  it("refuses an act the registry does not carry, rather than drawing nothing", () => {
    expect(() => keyFor("open_the_log")).toThrow(/actions\.toml/);
  });

  // The press map is the other half of this and is not asserted here: reading a
  // press goes through `holdsText`, which asks whether focus is in a text field
  // and needs a DOM to answer. It belongs in a browser test, and what stands in
  // for it meanwhile is the compiler — `DetailShape` requires both sheet
  // openers now, so a screen that binds the key and passes no handler does not
  // build.
});

// Which silence the diff header names. #381.
//
// The Produced chapter lists what a job wrote from a record Fleet keeps, and
// the diff reads the worktree live — so a finished job whose worktree has been
// given back shows a file list above an empty patch, and neither surface says
// why. The header can say it, but only where Bridge actually knows: `work`
// absent is also a job that never got a worktree at all, and naming that one
// `given back` would be the same false certainty pointed the other way.
describe("whether a missing reading is a worktree that was given back", () => {
  const JOB = "01M130Y1380016YK5S0JXBXDQ5";

  /** A footprint with one file in it. Only its presence is read. */
  const KEPT = { files: [{ path: "packages/screens/src/Sheets.tsx", change: "modified" }] };

  function whole(footprint: unknown): JobWhole {
    return { footprint } as JobWhole;
  }

  it("says so where a footprint proves there was a worktree to lose", () => {
    expect(gaveBackTheWorktree({ state: "read", jobId: JOB }, JOB, whole(KEPT))).toBe(true);
  });

  it("stays neutral with no footprint, because that is where Bridge cannot tell", () => {
    expect(gaveBackTheWorktree({ state: "read", jobId: JOB }, JOB, whole(undefined))).toBe(false);
    expect(gaveBackTheWorktree({ state: "read", jobId: JOB }, JOB, null)).toBe(false);
  });

  it("says nothing about a reading that came back", () => {
    // Present with no files is a drone that changed nothing, which is a
    // reading and not a silence. The header counts it.
    const read = {
      state: "read",
      jobId: JOB,
      work: { files: [], plan_declared: false, measured_whole: true },
    } as unknown as Diff;
    expect(gaveBackTheWorktree(read, JOB, whole(KEPT))).toBe(false);
  });

  it("says nothing before a reading, or about another job's", () => {
    expect(gaveBackTheWorktree({ state: "none" }, JOB, whole(KEPT))).toBe(false);
    expect(gaveBackTheWorktree({ state: "reading", jobId: JOB }, JOB, whole(KEPT))).toBe(false);
    const other = { state: "read", jobId: "01M130Y1380016YK5S0JXBXDQ6" } as Diff;
    expect(gaveBackTheWorktree(other, JOB, whole(KEPT))).toBe(false);
  });
});

describe("what the sheet is reading, as one value", () => {
  const HELD = { at: "10:31:00", rows: 12 };

  it("drops the log's attempt and hold when another sheet replaces it", () => {
    const log = sheetMoved(NO_SHEET, { move: "open", which: "log", attempt: 1, held: HELD });
    expect(sheetMoved(log, { move: "open", which: "diff" })).toEqual({ which: "diff" });
  });

  it("forgets the attempt when the log is opened again for the whole step", () => {
    const one = sheetMoved(NO_SHEET, { move: "open", which: "log", attempt: 1 });
    expect(sheetMoved(one, { move: "open", which: "log", held: HELD })).toEqual({
      which: "log",
      held: HELD,
    });
  });

  it("holds only the log, and closing clears everything", () => {
    const diff = sheetMoved(NO_SHEET, { move: "open", which: "diff" });
    expect(sheetMoved(diff, { move: "hold", held: HELD })).toBe(diff);
    const log = sheetMoved(NO_SHEET, { move: "open", which: "log" });
    expect(sheetMoved(log, { move: "hold", held: HELD })).toEqual({ which: "log", held: HELD });
    expect(sheetMoved(log, { move: "close" })).toBe(NO_SHEET);
  });

  // #1021 — a press names which Check, and that name has to survive the move
  // the log's attempt and hold already have to.
  describe("the Check output sheet", () => {
    it("opens on the Check a press named", () => {
      expect(sheetMoved(NO_SHEET, { move: "open", which: "check", checkId: "check:test_suite" })).toEqual(
        { which: "check", checkId: "check:test_suite" },
      );
    });

    it("replaces one Check with another on a second press, rather than stacking", () => {
      const first = sheetMoved(NO_SHEET, { move: "open", which: "check", checkId: "check:build" });
      expect(sheetMoved(first, { move: "open", which: "check", checkId: "check:test" })).toEqual({
        which: "check",
        checkId: "check:test",
      });
    });

    it("is replaced by another sheet, and replaces one in turn", () => {
      const log = sheetMoved(NO_SHEET, { move: "open", which: "log", attempt: 1, held: HELD });
      const checked = sheetMoved(log, { move: "open", which: "check", checkId: "check:test_suite" });
      expect(checked).toEqual({ which: "check", checkId: "check:test_suite" });
      expect(sheetMoved(checked, { move: "open", which: "diff" })).toEqual({ which: "diff" });
    });

    it("closes like every other sheet", () => {
      const checked = sheetMoved(NO_SHEET, { move: "open", which: "check", checkId: "check:test_suite" });
      expect(sheetMoved(checked, { move: "close" })).toBe(NO_SHEET);
    });
  });

  // `#1421`'s fields left the 380px rail for this layer. The task sheet carries
  // an id the way a Check's does, so it reads the plan as it stands rather
  // than a copy taken when it opened.
  describe("the task sheet", () => {
    it("carries which task it is reading", () => {
      expect(sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" })).toEqual({
        which: "task",
        taskId: "T1",
      });
    });

    it("replaces one task with another on a second press, rather than stacking", () => {
      const first = sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" });
      expect(sheetMoved(first, { move: "open", which: "task", taskId: "T4" })).toEqual({
        which: "task",
        taskId: "T4",
      });
    });

    it("is replaced by another sheet, and replaces one in turn", () => {
      const task = sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" });
      expect(sheetMoved(task, { move: "open", which: "diff" })).toEqual({ which: "diff" });
      const diff = sheetMoved(NO_SHEET, { move: "open", which: "diff" });
      expect(sheetMoved(diff, { move: "open", which: "task", taskId: "T2" })).toEqual({
        which: "task",
        taskId: "T2",
      });
    });

    it("closes like every other sheet", () => {
      const task = sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" });
      expect(sheetMoved(task, { move: "close" })).toBe(NO_SHEET);
    });
  });
});
