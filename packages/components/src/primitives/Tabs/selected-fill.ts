import { useLayoutEffect, useRef } from "react";

/**
 * Places the strip's one fill behind the chosen tab — the contract's *one fill
 * that travels*, at `--duration-base`, so the eye follows the selection from
 * the tab it left; an edge or a fill per tab can only swap.
 *
 * Measured from `offsetLeft` and `offsetWidth` before paint, and again when any
 * tab or the strip changes size — a count arriving moves every tab after it.
 * Written to the fill's style rather than rendered: a render would be a commit
 * between the layout and the paint. **The first placement does not travel**:
 * `data-travel`, which the transition is keyed on, is set a frame later.
 *
 * **Both strips call this one hook.** It was written twice, verbatim, in `Tabs`
 * and `TabsWithCounts`; #1383 restyled the fill in both, which is the edit that
 * would have had to be made twice and drifted on the third.
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
      shape.style.setProperty("--armada-tab-fill-w", `${tab.offsetWidth}px`);
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
