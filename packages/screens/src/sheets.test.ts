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

import { shownBy } from "./Sheets";
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
