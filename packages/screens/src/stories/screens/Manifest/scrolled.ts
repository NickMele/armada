// Geometry for a play that scrolls a view: which box scrolls, and whether a thing is on screen
// inside it. Read off computed style and layout, since a clipped element still reads as visible.

/** The nearest ancestor that scrolls vertically, or `null` where nothing does. */
export function scrollerOf(element: Element): HTMLElement | null {
  for (let at = element.parentElement; at !== null; at = at.parentElement) {
    if (/(auto|scroll)/.test(getComputedStyle(at).overflowY)) return at;
  }
  return null;
}

/** Whether `element`'s bottom edge is inside `scroller`'s box, and that box inside the window. */
export function endShownIn(element: Element, scroller: Element): boolean {
  const shown = scroller.getBoundingClientRect();
  const { bottom } = element.getBoundingClientRect();
  return bottom > shown.top && bottom <= shown.bottom + 1 && shown.bottom <= window.innerHeight + 1;
}

/** Whether a press at `element`'s centre lands on it, so nothing scrolled over it or clipped it. */
export function pressable(element: Element): boolean {
  const { left, top, width, height } = element.getBoundingClientRect();
  const hit = document.elementFromPoint(left + width / 2, top + height / 2);
  return hit !== null && element.contains(hit);
}
