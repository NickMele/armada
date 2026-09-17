import { useLayoutEffect, useRef, useState, type HTMLAttributes } from "react";

/**
 * Sections of one object. The active tab takes a 2px `--accent` underline,
 * `--fg-default` and weight 500; the rest sit in `--fg-muted`. Nothing
 * animates in — no entrance animations on data — but the underline travels
 * from the tab it left to the tab that was chosen, and a body drawn under the
 * strip in a `TabPanel` crossfades in.
 *
 * These carry no count. They are views of one job and there is nothing to
 * tally. The counted form is a separate component, and a separate row in the
 * registry.
 */
export type TabsItem = {
  id: string;
  /** Sentence case. */
  label: string;
};

export type TabsProps = {
  items: TabsItem[];
  /** Controlled. Omit for the uncontrolled form. */
  value?: string;
  defaultValue?: string;
  onChange?: (id: string) => void;
};

/**
 * Places the strip's one underline under the selected tab. **One bar that
 * travels**, at `--duration-base`, so the eye follows the selection from the
 * tab it left; an edge per tab can only swap.
 *
 * Measured from `offsetLeft` and `offsetWidth` before paint, and again when any
 * tab or the strip changes size — a count arriving moves every tab after it.
 * Written to the bar's style rather than rendered: a render would be a commit
 * between the layout and the paint. **The first placement does not travel**:
 * `data-travel`, which the transition is keyed on, is set a frame later.
 */
function useActiveBar(active: string | undefined, key: string) {
  const strip = useRef<HTMLDivElement>(null);
  const bar = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    const list = strip.current;
    const line = bar.current;
    if (list === null || line === null) return;

    function place() {
      if (list === null || line === null) return;
      const tab = list.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
      if (tab === null) {
        line.hidden = true;
        return;
      }
      line.hidden = false;
      line.style.setProperty("--armada-tab-bar-x", `${tab.offsetLeft}px`);
      line.style.setProperty("--armada-tab-bar-w", `${tab.offsetWidth}px`);
    }

    place();
    const frame = requestAnimationFrame(() => line.setAttribute("data-travel", ""));
    const watching = new ResizeObserver(place);
    watching.observe(list);
    for (const tab of list.querySelectorAll('[role="tab"]')) watching.observe(tab);
    return () => {
      cancelAnimationFrame(frame);
      watching.disconnect();
    };
  }, [active, key]);

  return { strip, bar };
}

export function Tabs({ items, value, defaultValue, onChange }: TabsProps) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;
  const { strip, bar } = useActiveBar(active, items.map((item) => item.id).join("\u0000"));

  function select(id: string) {
    if (value === undefined) setInternal(id);
    onChange?.(id);
  }

  function onKey(event: React.KeyboardEvent<HTMLDivElement>) {
    const at = items.findIndex((item) => item.id === active);
    // A tab strip with nothing in it takes no arrow key. Without this the
    // modulo below is `% 0`, which throws rather than doing nothing — and it
    // is what makes the two lookups below in range, `at` of `-1` included.
    if (items.length === 0) {
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      select(items[(at + 1) % items.length]!.id);
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      select(items[(at - 1 + items.length) % items.length]!.id);
    }
  }

  return (
    <div ref={strip} className="armada-tabs" role="tablist" onKeyDown={onKey}>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="tab"
          aria-selected={item.id === active}
          tabIndex={item.id === active ? 0 : -1}
          className={
            item.id === active ? "armada-tabs__tab armada-tabs__tab--active" : "armada-tabs__tab"
          }
          onClick={() => select(item.id)}
        >
          {item.label}
        </button>
      ))}
      <span ref={bar} className="armada-tabs__bar" aria-hidden="true" hidden />
    </div>
  );
}

export type TabPanelProps = HTMLAttributes<HTMLDivElement> & {
  /**
   * The selected tab's id. **The crossfade is keyed on this and on nothing
   * else**, so the same panel re-rendering with new data does not fade: a
   * person switching tabs summoned the panel, data arriving did not.
   */
  tab: string | undefined;
};

/**
 * The body under a tab strip. When `tab` changes the panel fades in at
 * `--duration-fast`, opacity only, and it does not fade on first mount — the
 * contract's "a body crossfade". Under reduced motion the duration is zero and
 * the new panel is simply there; the strip's selection says which one it is.
 *
 * **Not keyed by React.** A `key` would restart the fade by remounting
 * everything under it, and a panel that holds state across a switch — a run in
 * flight, a draft — would lose it. The attribute the animation hangs on is
 * cleared and set again on the element instead, with one read of layout
 * between so the browser sees a new animation rather than the old one.
 *
 * Carries no role of its own; a consumer passes `role="tabpanel"` where a
 * strip is drawn over it.
 */
export function TabPanel({ tab, className, children, ...rest }: TabPanelProps) {
  const panel = useRef<HTMLDivElement>(null);
  const shown = useRef(tab);

  useLayoutEffect(() => {
    const el = panel.current;
    if (el === null || shown.current === tab) return;
    shown.current = tab;
    el.removeAttribute("data-switched");
    // Forces the style flush that makes the re-added attribute a new animation.
    void el.offsetWidth;
    el.setAttribute("data-switched", "");
  }, [tab]);

  return (
    <div
      ref={panel}
      className={className === undefined ? "armada-tab-panel" : `armada-tab-panel ${className}`}
      {...rest}
    >
      {children}
    </div>
  );
}
