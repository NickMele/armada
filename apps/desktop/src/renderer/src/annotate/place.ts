// Where the note card goes, #1226. The window is the frame — the card is
// `position: fixed` — so every answer here is inside it on all four edges.
//
// **This is arithmetic, not CSS, because CSS cannot see how tall the card is.**
// `left` was clamped in the stylesheet and `top` was not, and it could not be:
// clamping the bottom edge needs the card's height, which the stylesheet never
// learns. So a person who picked an element as tall as the window got a card
// drawn off the top of it, with the text box he was typing into off screen
// entirely. The card measures itself and this fold places it.

import type { Box } from "../../../shared/annotations";

/** Which way the card sits from its element. `over` is the fallback: neither side had room. */
export type CardSide = "below" | "above" | "over";

/** How big the card wants to be, uncapped — what it would draw given the whole window. */
export type CardSize = { width: number; height: number };

/**
 * The window the card is placed inside. `floor` is the lowest edge it may
 * reach: the layer's own status bar is pinned to the bottom, over everything,
 * and a card drawn under it is on screen but cannot be read or pressed.
 */
export type Viewport = { width: number; height: number; floor: number };

/**
 * The layer's own geometry, declared once in `annotate.css` as tokens and read
 * back through {@link metricsOf}. Zero is a safe reading: the card still lands
 * inside the window, just flush against its edges.
 */
export type Metrics = { gap: number; pinSize: number; pinInset: number };

export type Placement = {
  x: number;
  y: number;
  /** What the card may draw before it scrolls inside itself. Never more than the window holds. */
  maxHeight: number;
  side: CardSide;
};

export const NO_CARD: CardSize = { width: 0, height: 0 };
export const NO_METRICS: Metrics = { gap: 0, pinSize: 0, pinInset: 0 };

/** CSS `clamp()`: the floor wins where the window is too small for both bounds. */
const clamp = (low: number, value: number, high: number): number => Math.max(low, Math.min(value, high));

/**
 * The pin's top and bottom, as `.armada-annotate__pin` draws them: centred on
 * the element's top-trailing corner and kept inside the window. The card is
 * placed around this, so a note's own pin is never the thing it hides.
 */
function pinSpan(box: Box, view: Viewport, m: Metrics): { top: number; bottom: number } {
  const centre = clamp(m.pinInset, box.y, view.height - m.pinInset);
  return { top: centre - m.pinSize / 2, bottom: centre + m.pinSize / 2 };
}

/**
 * Below the element where it fits, above it where that fits instead, and over
 * it when the element is taller than the room either side of it leaves — which
 * is the case that was broken, and the case a full-height sheet always is.
 */
export function placeCard(box: Box, card: CardSize, view: Viewport, m: Metrics): Placement {
  const x = clamp(m.gap, box.x, view.width - card.width - m.gap);
  const pin = pinSpan(box, view, m);
  const full = Math.max(view.floor - m.gap * 2, 0);

  // Both sides clear the pin as well as the element, so the card's edge never
  // crosses it on a narrow element, where the pin sits inside the card's width.
  const below = Math.max(box.y + box.height, pin.bottom) + m.gap;
  const above = Math.min(box.y, pin.top) - m.gap - card.height;
  const fits = (y: number): boolean => y >= m.gap && y + card.height <= view.floor - m.gap;
  if (fits(below)) return { x, y: below, maxHeight: full, side: "below" };
  if (fits(above)) return { x, y: above, maxHeight: full, side: "above" };

  // Neither side has room — the element is most of the window, which is what a
  // full-height sheet is. Covering it is then unavoidable, so the card takes
  // the taller of the two bands its own pin leaves free, from that band's top,
  // and caps itself to it rather than running past an edge.
  const overPin = Math.max(pin.top - m.gap * 2, 0);
  const underPin = Math.max(view.floor - pin.bottom - m.gap * 2, 0);
  const maxHeight = Math.max(overPin, underPin);
  return { x, y: overPin >= underPin ? m.gap : pin.bottom + m.gap, maxHeight, side: "over" };
}

const pixels = (style: CSSStyleDeclaration, name: string): number => {
  const read = Number.parseFloat(style.getPropertyValue(name));
  return Number.isFinite(read) ? read : 0;
};

/**
 * The geometry off the layer's own stylesheet. Read rather than repeated here,
 * so the gap and the pin stay tokens declared in one place — a literal in this
 * file would be the second copy that drifts.
 */
export function metricsOf(element: Element): Metrics {
  const style = getComputedStyle(element);
  return {
    gap: pixels(style, "--annotate-gap"),
    pinSize: pixels(style, "--annotate-pin-size"),
    pinInset: pixels(style, "--annotate-pin-inset"),
  };
}

/**
 * How tall the card would be uncapped, taken from its content rather than its
 * box: a card already capped by a previous placement still reports what it
 * wants, so the side it is on cannot flip back and forth between two frames.
 */
export function sizeOf(node: HTMLElement): CardSize {
  return { width: node.offsetWidth, height: node.scrollHeight + node.offsetHeight - node.clientHeight };
}

export function samePlacementInput(a: { size: CardSize; metrics: Metrics }, b: { size: CardSize; metrics: Metrics }): boolean {
  return (
    a.size.width === b.size.width &&
    a.size.height === b.size.height &&
    a.metrics.gap === b.metrics.gap &&
    a.metrics.pinSize === b.metrics.pinSize &&
    a.metrics.pinInset === b.metrics.pinInset
  );
}
