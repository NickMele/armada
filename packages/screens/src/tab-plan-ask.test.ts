// What an ask says, and what the Drone is told. The dialog's own rendering is
// claimed through `App`, in `arc.test.tsx`. `#1552`.

import { describe, expect, test } from "vitest";

import { arcGroups } from "./fixtures/build/arc-plan";
import { askHolds, askInstruction, askTitle, rewriteInstruction } from "./tab-plan-ask";
import { asksOf, reorderWarning } from "./tab-plan-read";

const GROUPS = arcGroups();

describe("which asks a group offers", () => {
  test("the first group cannot move up and the last cannot move down", () => {
    expect(asksOf(GROUPS, 0).map((one) => [one.id, one.disabled === true])).toEqual([
      ["move_up", true],
      ["move_down", false],
      ["remove", false],
    ]);
    expect(asksOf(GROUPS, 3).map((one) => [one.id, one.disabled === true])).toEqual([
      ["move_up", false],
      ["move_down", true],
      ["remove", false],
    ]);
  });

  test("a group in the middle offers all three, and every group offers the same three controls", () => {
    expect(asksOf(GROUPS, 1).every((one) => one.disabled !== true)).toBe(true);
    expect(asksOf(GROUPS, 0)).toHaveLength(3);
  });
});

describe("the one mechanical catch", () => {
  test("moving a group past one that claims the same file warns, naming the file and who writes it first", () => {
    expect(reorderWarning(GROUPS, "g3", "move_down")).toBe(
      "Group 3 and group 4 both claim packages/screens/src/running-rows.tsx. " +
        "Group 3 writes it first as the plan stands, and this ask reverses that.",
    );
  });

  test("the pair warns from either side, and says the same thing about who writes first", () => {
    expect(reorderWarning(GROUPS, "g4", "move_up")).toBe(
      "Group 4 and group 3 both claim packages/screens/src/running-rows.tsx. " +
        "Group 3 writes it first as the plan stands, and this ask reverses that.",
    );
  });

  test("a reorder the scopes do not disagree with warns about nothing", () => {
    expect(reorderWarning(GROUPS, "g2", "move_up")).toBeUndefined();
    expect(reorderWarning(GROUPS, "g1", "move_down")).toBeUndefined();
  });

  test("a move with nothing on the other side of it warns about nothing", () => {
    expect(reorderWarning(GROUPS, "g1", "move_up")).toBeUndefined();
    expect(reorderWarning(GROUPS, "g4", "move_down")).toBeUndefined();
  });

  test("removing a group is not a reorder, so the catch does not apply to it", () => {
    expect(reorderWarning(GROUPS, "g3", "remove")).toBeUndefined();
  });

  test("a group the plan does not hold warns about nothing rather than throwing", () => {
    expect(reorderWarning(GROUPS, "g9", "move_up")).toBeUndefined();
  });
});

describe("what the ask says", () => {
  test("a move names the order being asked for, not the control that was pressed", () => {
    expect(askTitle(GROUPS, "g3", "move_up")).toBe(
      "Ask the Drone to run group 3 before group 2?",
    );
    expect(askTitle(GROUPS, "g3", "move_down")).toBe(
      "Ask the Drone to run group 3 after group 4?",
    );
  });

  test("a remove names the group it would drop", () => {
    expect(askTitle(GROUPS, "g4", "remove")).toBe("Ask the Drone to drop group 4?");
  });

  test("a remove says which tasks go with the group", () => {
    expect(askHolds(GROUPS, "g4")).toBe("Group 4 holds T7, T8.");
  });
});

describe("what the Drone is told", () => {
  test("the instruction names the new order and says a refusal is an answer", () => {
    const said = askInstruction(GROUPS, "g3", "move_up", "");
    expect(said).toContain("Run group 3 before group 2.");
    expect(said).toContain("refuse it and say what that reason is");
  });

  test("the mechanical warning rides along, because the Drone is the one that knows whether it matters", () => {
    expect(askInstruction(GROUPS, "g3", "move_down", "")).toContain(
      "both claim packages/screens/src/running-rows.tsx",
    );
  });

  test("a reason a person typed is carried, and an empty one adds no paragraph", () => {
    expect(askInstruction(GROUPS, "g2", "move_up", "the rows come first")).toContain(
      "the rows come first",
    );
    expect(askInstruction(GROUPS, "g2", "move_up", "").split("\n\n")).toHaveLength(2);
  });

  test("a rewrite is addressed to its task and carries the same standing", () => {
    const said = rewriteInstruction("T5", "split the rows out");
    expect(said).toContain("on T5: split the rows out");
    expect(said).toContain("refuse it and say what that reason is");
  });
});
