import { useLayoutEffect, useRef } from "react";

/**
 * Places the strip's one fill behind the chosen tab — the contract's *one fill
 * that travels*, at `--duration-base`, so the eye follows the selection from
 * the tab it left; an edge or a fill per tab can only swap.
 *
 * **All four offsets, not two.** A strip too wide for its column wraps inside
 * its track rather than spilling out of it, so the chosen tab can be on a
 * second row — which the underline this replaced never answered for.
 *
 * Measured before paint, and again when any tab or the strip resizes: a count
 * arriving moves every tab after it. Written to the fill's style, because a
 * render would be a commit between layout and paint. **The first placement
 * does not travel** — `data-travel` is set a frame later.
 */
export function useSelectedFill(active: string | undefined, key: string) {
  const strip = useRef<HTMLDivElement>(null);
  const fill = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    const list = strip.current;
    const shape = fill.current;
    if (list === null || shape === null) return;

    function place() {
      if (list === null || shape === null) return;
      const tab = list.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
      if (tab === null) {
        shape.hidden = true;
        return;
      }
      shape.hidden = false;
      shape.style.setProperty("--armada-tab-fill-x", `${tab.offsetLeft}px`);
      shape.style.setProperty("--armada-tab-fill-y", `${tab.offsetTop}px`);
      shape.style.setProperty("--armada-tab-fill-w", `${tab.offsetWidth}px`);
      shape.style.setProperty("--armada-tab-fill-h", `${tab.offsetHeight}px`);
    }

    place();
    const frame = requestAnimationFrame(() => shape.setAttribute("data-travel", ""));
    const watching = new ResizeObserver(place);
    watching.observe(list);
    for (const tab of list.querySelectorAll('[role="tab"]')) watching.observe(tab);
    return () => {
      cancelAnimationFrame(frame);
      watching.disconnect();
    };
  }, [active, key]);

  return { strip, fill };
}
