import { describe, expect, it } from "vitest";

import { componentName, componentsOf, fiberOf, type FiberLike } from "./fiber";

function Board() {}
function ActiveJobsList() {}
function JobRowStacked() {}
function Button() {}
const Named = Object.assign(() => undefined, { displayName: "Shown" });

describe("componentName", () => {
  it("reads a function's name, and prefers its displayName", () => {
    expect(componentName(JobRowStacked)).toBe("JobRowStacked");
    expect(componentName(Named)).toBe("Shown");
  });

  it("unwraps memo and forwardRef", () => {
    expect(componentName({ $$typeof: Symbol.for("react.memo"), type: Board })).toBe("Board");
    expect(componentName({ $$typeof: Symbol.for("react.forward_ref"), render: Button })).toBe("Button");
  });

  it("is null for a host element or a fragment", () => {
    expect(componentName("div")).toBeNull();
    expect(componentName(null)).toBeNull();
    expect(componentName(Symbol.for("react.fragment"))).toBeNull();
  });
});

describe("fiberOf", () => {
  it("finds the fiber on the node or the nearest ancestor that has one", () => {
    const fiber: FiberLike = { type: "li" };
    const parent = { ["__reactFiber$abc"]: fiber, parentNode: null };
    expect(fiberOf({ parentNode: parent })).toBe(fiber);
    expect(fiberOf({ parentNode: null })).toBeNull();
  });
});

/** host `button` → Button → JobRowStacked → ActiveJobsList → Board */
function tree(): { host: FiberLike; row: FiberLike; list: FiberLike; board: FiberLike } {
  const board: FiberLike = { type: Board, return: null };
  const list: FiberLike = { type: ActiveJobsList, return: { type: "section", return: board } };
  const row: FiberLike = { type: JobRowStacked, return: list };
  const button: FiberLike = { type: Button, return: { type: "div", return: row } };
  const host: FiberLike = { type: "button", return: button };
  return { host, row, list, board };
}

describe("componentsOf", () => {
  it("falls back to the parent chain where React kept no owners, and says so", () => {
    expect(componentsOf(tree().host)).toEqual({
      component: "Button",
      owners: ["JobRowStacked", "ActiveJobsList", "Board"],
      from: "parent",
    });
  });

  it("follows owners from the element's own, which is where its JSX is written", () => {
    const { host, row, list, board } = tree();
    host._debugOwner = row;
    row._debugOwner = list;
    list._debugOwner = board;
    expect(componentsOf(host)).toEqual({
      component: "Button",
      owners: ["JobRowStacked", "ActiveJobsList", "Board"],
      from: "owner",
    });
  });

  it("does not repeat the component when it is also the owner", () => {
    const button: FiberLike = { type: Button, return: null };
    const host: FiberLike = { type: "button", return: button, _debugOwner: button };
    button._debugOwner = { name: "ServerThing" };
    expect(componentsOf(host)).toEqual({ component: "Button", owners: ["ServerThing"], from: "owner" });
  });

  it("names nothing for an element outside any component", () => {
    expect(componentsOf({ type: "div", return: null })).toEqual({ component: null, owners: [], from: "parent" });
    expect(componentsOf(null).component).toBeNull();
  });
});
