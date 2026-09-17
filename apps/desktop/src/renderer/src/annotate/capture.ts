// What a note records about the element it was left on, read from the page.

import { annotationId, type Annotation } from "../../../shared/annotations";
import { componentsOf, fiberOf } from "./fiber";
import { selectorFor } from "./selector";

const TEXT = 80;

/** Visible text, collapsed and cut, so a note names what a person read on screen. */
export function shortText(text: string | null | undefined): string {
  const collapsed = (text ?? "").replace(/\s+/g, " ").trim();
  return collapsed.length > TEXT ? `${collapsed.slice(0, TEXT - 1)}…` : collapsed;
}

/** The accessible name the rail marks as current: `Sidebar` sets `aria-current="page"`. */
function currentScreen(doc: Document): string | null {
  const current = doc.querySelector('[aria-current="page"]');
  if (current === null) return null;
  const label = current.getAttribute("aria-label");
  if (label !== null) return label;
  // Beside the label the item draws a count and a binding, and neither is the screen's name.
  const named = current.querySelector(".armada-sidebar__label");
  if (named !== null) return shortText(named.textContent) || null;
  const copy = current.cloneNode(true) as Element;
  copy.querySelectorAll("kbd").forEach((kbd) => kbd.remove());
  return shortText(copy.textContent) || null;
}

/** The dialog or sheet the element is inside, by its accessible name. */
function layerOf(element: Element): string | null {
  const dialog = element.closest('[role="dialog"], [role="alertdialog"], dialog');
  if (dialog === null) return null;
  const labelledBy = dialog.getAttribute("aria-labelledby");
  const named = labelledBy === null ? null : element.ownerDocument.getElementById(labelledBy);
  return dialog.getAttribute("aria-label") ?? (named === null ? null : shortText(named.textContent)) ?? "dialog";
}

/** The short selector when it finds this element first, and the whole chain when it does not. */
function findableSelector(element: Element): string {
  const selector = selectorFor(element);
  try {
    if (element.ownerDocument.querySelector(selector) === element) return selector;
  } catch {
    // An attribute value the engine would not parse; the whole chain is the better record.
  }
  return selectorFor(element, Number.POSITIVE_INFINITY);
}

/** A new, open note on `element`, with nothing said yet. */
export function capture(element: Element, now: Date, win: Window = window): Annotation {
  const rect = element.getBoundingClientRect();
  const { component, owners, from } = componentsOf(fiberOf(element));
  const at = now.toISOString();
  return {
    id: annotationId(now),
    status: "open",
    text: "",
    component,
    owners,
    ownersFrom: from,
    selector: findableSelector(element),
    element: {
      tag: element.tagName.toLowerCase(),
      text: shortText(element.textContent),
      label: element.getAttribute("aria-label"),
    },
    screen: currentScreen(element.ownerDocument),
    layer: layerOf(element),
    location: `${win.location.pathname}${win.location.search}${win.location.hash}`,
    scenario: new URLSearchParams(win.location.search).get("scenario"),
    box: {
      x: Math.round(rect.x),
      y: Math.round(rect.y),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    },
    window: { width: win.innerWidth, height: win.innerHeight },
    createdAt: at,
    updatedAt: at,
  };
}

/** The element a saved note points at, if the page still has one. */
export function locate(note: Annotation, doc: Document = document): Element | null {
  try {
    return doc.querySelector(note.selector);
  } catch {
    return null;
  }
}
