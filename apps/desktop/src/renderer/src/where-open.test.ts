// Where things are' fold, on its own — no window, no save, no Fleet.

import { expect, test } from "vitest";
import { fold } from "./where-open";

test("pressing opens at once", () => {
  const state = fold({ open: false, lastFromFleet: false }, { kind: "press", open: true });
  expect(state.open).toBe(true);
});

test("a later publish from Fleet updates it", () => {
  const state = fold({ open: false, lastFromFleet: false }, { kind: "fleet", open: true });
  expect(state).toEqual({ open: true, lastFromFleet: true });
});

test("stays open when the save answers not_connected — nothing publishes, so nothing arrives to undo it", () => {
  const pressed = fold({ open: false, lastFromFleet: false }, { kind: "press", open: true });
  // A refused or unreachable save publishes no new preference, so the one
  // event that could arrive is a stale echo of the value from before the
  // press — `lastFromFleet` is still `false`, and this is that moment.
  const stale = fold(pressed, { kind: "fleet", open: false });
  expect(stale).toBe(pressed);
});
