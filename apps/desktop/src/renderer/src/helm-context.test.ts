// Helm's context, on its own — no window, no Fleet.

import { expect, test } from "vitest";
import type { JobSummary } from "@armada/protocol";
import { chippedJobId, contextOf, cursorRowFor, dismissed, locationOf, NO_CHIP, opened, screenOf } from "./helm-context";

const JOBS = [{ id: "j16", handle: "16-preserve-job-metadata" }] as unknown as readonly JobSummary[];

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
  const none = {
    reading: false,
    clearing: false,
    manifesting: false,
    overviewing: false,
    studying: false,
    kitting: false,
    settling: false,
  };
  const every = {
    reading: true,
    clearing: true,
    manifesting: true,
    overviewing: true,
    studying: true,
    kitting: true,
    settling: true,
  };
  expect(screenOf(every)).toBe("job_detail");
  expect(screenOf({ ...every, reading: false })).toBe("cleanup");
  expect(screenOf({ ...every, reading: false, clearing: false })).toBe("manifest");
  expect(screenOf({ ...none, overviewing: true, studying: true, kitting: true, settling: true })).toBe("overview");
  expect(screenOf({ ...none, studying: true, kitting: true, settling: true })).toBe("studio");
  expect(screenOf({ ...none, kitting: true, settling: true })).toBe("kit");
  expect(screenOf(none)).toBe("board");
});

/**
 * **Settings was missing from this enum until #1275**, so a person on it was
 * reported to Helm as being on the Board — the gap #1287 left when it added
 * `studio` and stopped. Its own case, because the one above proves precedence
 * and this proves the value exists at all.
 */
test("screenOf names Settings rather than falling through to the Board", () => {
  const none = {
    reading: false,
    clearing: false,
    manifesting: false,
    overviewing: false,
    studying: false,
    kitting: false,
    settling: false,
  };
  expect(screenOf({ ...none, settling: true })).toBe("settings");
});

test("cursorRowFor sends the Board's cursor on the Board and Overview's on Overview", () => {
  const rows = { board: "12", overview: "14" };
  expect(cursorRowFor({ screen: "board", ...rows })).toBe("12");
  expect(cursorRowFor({ screen: "overview", ...rows })).toBe("14");
});

test("cursorRowFor sends neither off the Board and Overview — a stale row is worse than none", () => {
  const rows = { board: "12", overview: "14" };
  expect(cursorRowFor({ screen: "manifest", ...rows })).toBeNull();
  expect(cursorRowFor({ screen: "cleanup", ...rows })).toBeNull();
  expect(cursorRowFor({ screen: "job_detail", ...rows })).toBeNull();
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

test("locationOf names the screen and the cursor row, off the wire's own job number", () => {
  expect(locationOf({ screen: "overview", cursor: "j16" }, JOBS)).toBe("Overview · cursor on Job 16");
});

test("locationOf names the screen alone with no cursor to report", () => {
  expect(locationOf({ screen: "board" }, JOBS)).toBe("Job Board");
});

test("locationOf names the Job's detail as read, in context while the chip stands", () => {
  expect(locationOf({ screen: "job_detail", chip: "j16" }, JOBS)).toBe("Job 16's detail (in context)");
});

test("locationOf reports the Job's detail generically once the wire carries no chip for it", () => {
  expect(locationOf({ screen: "job_detail" }, JOBS)).toBe("Job's detail");
});

test("locationOf never invents a number for an id the Board does not hold", () => {
  expect(locationOf({ screen: "overview", cursor: "missing" }, JOBS)).toBe("Overview");
});

test("a Studio rides on the context only on the Studios surface, and a node only beside its Studio", () => {
  const base = { picked: null, chip: null, cursor: null };
  expect(contextOf({ ...base, screen: "studio", studio: "s1", node: "n1" })).toEqual({ screen: "studio", studio: "s1", node: "n1" });
  expect(contextOf({ ...base, screen: "studio", studio: null, node: "n1" })).toEqual({ screen: "studio" });
  expect(contextOf({ ...base, screen: "board", studio: "s1", node: "n1" })).toEqual({ screen: "board" });
});

test("locationOf names the Studio open, and the node selected on it", () => {
  expect(locationOf({ screen: "studio" }, JOBS)).toBe("Studios");
  expect(locationOf({ screen: "studio", studio: "s1" }, JOBS, { name: "Untitled Studio" })).toBe("Studios · Untitled Studio");
  expect(locationOf({ screen: "studio", studio: "s1", node: "n1" }, JOBS, { name: "Legend", node: "Note The legend" })).toBe(
    "Studios · Legend · Note The legend selected",
  );
});
