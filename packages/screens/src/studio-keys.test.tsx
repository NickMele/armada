// In the browser project rather than node: `holdsText` asks whether focus is
// in a text field, which needs a real element to answer — `sheets.test.ts` says
// the same about the Board's map, and this is where that half is asserted.

import { describe, expect, it } from "vitest";

import { addPressOf } from "./studio-keys";

const press = (key: string, over: Partial<Parameters<typeof addPressOf>[0]> = {}) =>
  addPressOf({ key, metaKey: false, ctrlKey: false, altKey: false, target: null, ...over });

describe("the keys on an open Studio", () => {
  it("reads the three kinds off the registry", () => {
    expect(press("N")).toBe("note");
    expect(press("V")).toBe("link");
    expect(press("S")).toBe("sketch");
  });

  // The letters the design drew are all spoken for unshifted: `n` dispatches
  // anywhere, `v` observes and `s` restarts a step.
  it("leaves the unshifted letters to the acts that hold them", () => {
    expect(press("n")).toBeNull();
    expect(press("v")).toBeNull();
    expect(press("s")).toBeNull();
  });

  it("means another tier where a modifier is down", () => {
    expect(press("N", { metaKey: true })).toBeNull();
    expect(press("N", { ctrlKey: true })).toBeNull();
    expect(press("N", { altKey: true })).toBeNull();
  });

  // Typing "seven notes" into a note must not open three more fields.
  it("is suppressed while a field holds focus", () => {
    expect(press("N", { target: document.createElement("textarea") })).toBeNull();
    expect(press("V", { target: document.createElement("input") })).toBeNull();
    expect(press("S", { target: document.createElement("select") })).toBeNull();
    expect(press("N", { target: document.createElement("div") })).toBe("note");
  });
});
