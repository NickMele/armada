// Helm's context, on its own — no window, no Fleet.

import { expect, test } from "vitest";
import { chippedJobId, contextOf, dismissed, NO_CHIP, opened, screenOf } from "./helm-context";

test("opening a Job chips it and points Helm at its repository", () => {
  const { state, point } = opened(NO_CHIP, { id: "12", manifestId: "M-armada" });
  expect(chippedJobId(state)).toBe("12");
  expect(point).toBe("M-armada");
});

test("leaving the Job drops the chip", () => {
  const { state: chipped } = opened(NO_CHIP, { id: "12", manifestId: "M-armada" });
  const { state: left, point } = opened(chipped, null);
  expect(chippedJobId(left)).toBeNull();
  expect(point).toBeNull();
});

test("the × drops the Job from the next ask's context, without closing it", () => {
  const { state: chipped } = opened(NO_CHIP, { id: "12", manifestId: "M-armada" });
  const dropped = dismissed(chipped);
  expect(chippedJobId(dropped)).toBeNull();
});

test("reopening the Job adds the chip again", () => {
  const { state: chipped } = opened(NO_CHIP, { id: "12", manifestId: "M-armada" });
  const dropped = dismissed(chipped);
  const { state: left } = opened(dropped, null);
  const { state: reopened, point } = opened(left, { id: "12", manifestId: "M-armada" });
  expect(chippedJobId(reopened)).toBe("12");
  expect(point).toBe("M-armada");
});

test("switching from one open Job to another chips and points at the new one", () => {
  const { state: first } = opened(NO_CHIP, { id: "12", manifestId: "M-armada" });
  const { state: second, point } = opened(first, { id: "14", manifestId: "M-shop" });
  expect(chippedJobId(second)).toBe("14");
  expect(point).toBe("M-shop");
});

test("screenOf follows the precedence App.tsx draws by", () => {
  expect(screenOf({ reading: true, clearing: true, manifesting: true, overviewing: true })).toBe("job_detail");
  expect(screenOf({ reading: false, clearing: true, manifesting: true, overviewing: true })).toBe("cleanup");
  expect(screenOf({ reading: false, clearing: false, manifesting: true, overviewing: true })).toBe("manifest");
  expect(screenOf({ reading: false, clearing: false, manifesting: false, overviewing: true })).toBe("overview");
  expect(screenOf({ reading: false, clearing: false, manifesting: false, overviewing: false })).toBe("board");
});

test("contextOf leaves an absent field off the wire", () => {
  expect(contextOf({ screen: "board", picked: null, chip: null, cursor: null })).toEqual({ screen: "board" });
  expect(contextOf({ screen: "job_detail", picked: "M-armada", chip: "12", cursor: "14" })).toEqual({
    screen: "job_detail",
    picked: "M-armada",
    chip: "12",
    cursor: "14",
  });
});
