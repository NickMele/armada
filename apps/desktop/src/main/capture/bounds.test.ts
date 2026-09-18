// Every bound re-applied on what the page answered — #1294,
// `docs/practices/capture-window.md`, *What may come back*.
//
// The subject is a hostile answer. The page can say anything at all on the
// promise main created, so each case here is the page saying something a
// truthful layer never would.

import { describe, expect, it } from "vitest";

import type { CaptureServed } from "@armada/protocol";
import { aimed, bounded, chainOf, rectOf, styled } from "./bounds";
import { MOST_MARKUP } from "./layer";

const VIEWPORT = { width: 1200, height: 800 };

const SERVED: CaptureServed = { run: "01SERVER", name: "web_dev", address: "http://localhost:41207" };

const answer = (capture: Record<string, unknown>) => ({ box: { x: 0, y: 0, width: 1, height: 1 }, capture });

const honest = {
  component: "Checkout",
  owners: ["Cart", "App"],
  selector: "main > form[role=\"form\"] > button",
  element: { tag: "button", text: "Pay now", label: "Pay now, £42" },
  screen: "Checkout",
  location: "/checkout",
  bounds: { x: 312, y: 148, width: 96, height: 28 },
  window: { width: 900, height: 600 },
  styles: { color: "rgb(0, 0, 0)", "text-align": "center" },
  markup: "<button>Pay now</button>",
};

describe("what may come back", () => {
  it("keeps an honest capture whole, and records the Run beside it", () => {
    const capture = bounded(answer(honest), VIEWPORT, SERVED);
    expect(capture).toMatchObject({
      component: "Checkout",
      owners: ["Cart", "App"],
      element: { tag: "button", text: "Pay now", label: "Pay now, £42" },
      screen: "Checkout",
      location: "/checkout",
      markup: "<button>Pay now</button>",
      served: SERVED,
    });
  });

  it("takes the window from main's own measurement, never the page's claim", () => {
    // The page said 900×600; the viewport main holds is what a box reads against.
    const capture = bounded(answer(honest), VIEWPORT, SERVED);
    expect(capture?.window).toEqual(VIEWPORT);
  });

  it("drops a `served` the page answered with: that field is main's", () => {
    const forged = { ...honest, served: { run: "x", name: "y", address: "https://elsewhere.test" } };
    expect(bounded(answer(forged), VIEWPORT, SERVED)?.served).toEqual(SERVED);
  });

  it("cuts fifty megabytes of markup to the bound, on the side that is not the page", () => {
    const capture = bounded(answer({ ...honest, markup: "x".repeat(50_000_000) }), VIEWPORT, SERVED);
    expect(capture?.markup).toHaveLength(MOST_MARKUP);
  });

  it("keeps only the declared style properties, whatever the page offers", () => {
    const capture = bounded(
      answer({ ...honest, styles: { color: "red", "--secret-token": "abc", cursor: "pointer" } }),
      VIEWPORT,
      SERVED,
    );
    expect(capture?.styles).toEqual({ color: "red" });
  });

  it("re-types every field, so a number where a string goes is not kept", () => {
    const lying = { ...honest, component: 7, element: { tag: "button", text: { evil: true } }, owners: [1, "Cart"] };
    const capture = bounded(answer(lying), VIEWPORT, SERVED);
    expect(capture?.component).toBeUndefined();
    expect(capture?.element.text).toBe("");
    expect(capture?.owners).toEqual(["Cart"]);
  });

  it("is nothing at all where the two fields a Note cannot do without are missing", () => {
    expect(bounded(answer({ ...honest, selector: 3 }), VIEWPORT, SERVED)).toBeNull();
    expect(bounded(answer({ ...honest, markup: null }), VIEWPORT, SERVED)).toBeNull();
    expect(bounded(null, VIEWPORT, SERVED)).toBeNull();
    expect(bounded({ box: {} }, VIEWPORT, SERVED)).toBeNull();
  });
});

describe("a box", () => {
  it("is clamped to the viewport, so a crop cannot reach past the page", () => {
    expect(rectOf({ x: -50, y: -50, width: 99_999, height: 99_999 }, VIEWPORT)).toEqual({
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
    });
  });

  it("is zero where the page answered with something that is not a number", () => {
    // A string, a NaN, an Infinity and a null are each nothing to crop by, and
    // a zero box is what stops a frame being taken rather than a wrong one.
    expect(rectOf({ x: "12", y: Number.NaN, width: Infinity, height: null }, VIEWPORT)).toEqual({
      x: 0,
      y: 0,
      width: 0,
      height: 0,
    });
  });
});

describe("an aim", () => {
  it("is the box and what to call it", () => {
    expect(aimed({ box: { x: 10, y: 20, width: 30, height: 40 }, named: "Checkout" }, VIEWPORT)).toEqual({
      box: { x: 10, y: 20, width: 30, height: 40 },
      named: "Checkout",
    });
  });

  it("names the element rather than nothing where the page answered no name", () => {
    expect(aimed({ box: { x: 0, y: 0, width: 1, height: 1 } }, VIEWPORT)?.named).toBe("element");
  });

  it("is nothing where the pointer was over nothing", () => {
    expect(aimed(null, VIEWPORT)).toBeNull();
  });
});

describe("the line the bar draws", () => {
  it("names the chain and the last steps of the selector", () => {
    const capture = bounded(answer({ ...honest, selector: "html > body > main > form > button" }), VIEWPORT, SERVED);
    expect(chainOf(capture!)).toBe("Checkout ← Cart ← App · … > form > button");
  });

  it("falls back to the tag on a page React did not draw", () => {
    const plain = { ...honest, component: undefined, owners: [], selector: "button" };
    expect(chainOf(bounded(answer(plain), VIEWPORT, SERVED)!)).toBe("button · button");
  });
});

describe("the declared style list", () => {
  it("answers nothing for anything that is not an object", () => {
    expect(styled("everything")).toEqual({});
    expect(styled(null)).toEqual({});
  });
});
