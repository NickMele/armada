import { describe, expect, it } from "vitest";
import { noteWithChanges, smallFixesOf } from "./changes";

const LISTED = [
  { id: "small-fix-0", from: "Small fix", text: "Move the migration into its own file." },
  { id: "view-0", from: "From View: The hold goes", text: "Say in the status bar why." },
];

describe("noteWithChanges", () => {
  it("sends the typed words alone when nothing is listed", () => {
    expect(noteWithChanges([], "Add a test.")).toBe("Add a test.");
  });

  it("sends every listed change, then the typed words", () => {
    expect(noteWithChanges(LISTED, "  Add a test.  ")).toBe(
      [
        "What should change:",
        "- Small fix: Move the migration into its own file.",
        "- From View: The hold goes: Say in the status bar why.",
        "",
        "Add a test.",
      ].join("\n"),
    );
  });

  it("sends the list alone when nothing is typed", () => {
    expect(noteWithChanges(LISTED, "   ")).toBe(
      [
        "What should change:",
        "- Small fix: Move the migration into its own file.",
        "- From View: The hold goes: Say in the status bar why.",
      ].join("\n"),
    );
  });
});

describe("smallFixesOf", () => {
  it("lists each small fix with its reason, and no backticks", () => {
    expect(
      smallFixesOf({
        says: "confident",
        reasons: [],
        areas: [],
        needs_you: [],
        small_fixes: [{ finding: "`store.rs` is over 500 lines", why: "Move the migration out" }],
        for_context: [],
      }),
    ).toEqual([{ id: "small-fix-0", from: "Small fix", text: "store.rs is over 500 lines: Move the migration out" }]);
  });
});
