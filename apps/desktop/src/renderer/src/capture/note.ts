// What a Studio Note keeps of the element it was captured on — #1290.
//
// **The development annotation layer's own reading, reused rather than
// copied.** `annotate/capture.ts` finds the component chain, the selector, the
// screen, the layer, the location and the box; this adds the four a Note keeps
// beyond them. Nothing in `annotate/` changes: it stays as it is, permanently.

import type { StudioCapture } from "@armada/protocol";

import { capture as pointedAt, shortText } from "../annotate/capture";
import { componentsOf, fiberOf } from "../annotate/fiber";

/**
 * The computed properties a Note keeps. **A declared list, not every
 * property**: `getComputedStyle` resolves some 350, which would be most of a
 * Studio's weight and none of its meaning. These are the ones a note about how
 * something looks is about — colour, type, edge, spacing and how it is laid
 * out.
 */
export const CAPTURED_STYLES = [
  "display",
  "position",
  "width",
  "height",
  "padding",
  "margin",
  "gap",
  "overflow",
  "color",
  "background-color",
  "border-width",
  "border-color",
  "border-radius",
  "opacity",
  "font-family",
  "font-size",
  "font-weight",
  "line-height",
  "letter-spacing",
  "text-align",
  "white-space",
  "visibility",
] as const;

/** The most markup one Note keeps. A row is read; a subtree is a page. */
export const MOST_MARKUP = 2000;

/** `outerHTML`, collapsed and cut, so one Note is a row rather than a screen. */
export function trimmedMarkup(html: string, most: number = MOST_MARKUP): string {
  const collapsed = html.replace(/\s+/g, " ").trim();
  return collapsed.length > most ? `${collapsed.slice(0, most - 1)}…` : collapsed;
}

type Computed = (element: Element) => CSSStyleDeclaration;

/** The declared properties, as this element resolves them. */
export function stylesOf(
  element: Element,
  computed: Computed = (of) => window.getComputedStyle(of),
): Record<string, string> {
  const resolved = computed(element);
  const styles: Record<string, string> = {};
  for (const property of CAPTURED_STYLES) {
    const value = resolved.getPropertyValue(property);
    if (value !== "") styles[property] = value;
  }
  return styles;
}

/**
 * What a Note keeps about `element`. **No frame and no source**: the frame is
 * main's to take and Fleet's to keep, and React 19 carries no `_debugSource`,
 * so no path is guessed for one.
 */
export function captureOf(
  element: Element,
  now: Date,
  win: Window = window,
  computed?: Computed,
): StudioCapture {
  const pointed = pointedAt(element, now, win);
  return {
    ...(pointed.component === null ? {} : { component: pointed.component }),
    ...(pointed.owners.length === 0 ? {} : { owners: pointed.owners }),
    selector: pointed.selector,
    element: {
      tag: pointed.element.tag,
      text: pointed.element.text,
      ...(pointed.element.label === null ? {} : { label: pointed.element.label }),
    },
    ...(pointed.screen === null ? {} : { screen: pointed.screen }),
    ...(pointed.layer === null ? {} : { layer: pointed.layer }),
    location: pointed.location,
    bounds: pointed.box,
    window: pointed.window,
    styles: stylesOf(element, computed),
    markup: trimmedMarkup(element.outerHTML),
  };
}

/** `FilterChip ← Board · button.armada-chip`, so a person sees what they hit. */
export function chainOf(capture: StudioCapture): string {
  const names = [capture.component, ...(capture.owners ?? [])].filter((name): name is string => name !== undefined);
  return `${names.length > 0 ? names.slice(0, 4).join(" ← ") : capture.element.tag} · ${capture.selector}`;
}

/** What the outline calls the element under the pointer. */
export function namedOf(element: Element): string {
  return componentsOf(fiberOf(element)).component ?? shortText(element.tagName.toLowerCase());
}
