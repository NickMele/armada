import { useLayoutEffect, useRef, useState, type HTMLAttributes } from "react";
import { useSelectedFill } from "./selected-fill";

/**
 * Sections of one object, drawn as the contract's segmented control: a
 * `--bg-sunken` track, and the chosen tab filled `--accent-muted` inside it.
 * Nothing animates in, but the fill travels and a `TabPanel` under the strip
 * crossfades.
 *
 * **Filled rather than underlined since #1383**, because the rail's selected
 * row is filled and these sit in the same eyeline. Still the shadcn `tabs`
 * primitive, painted differently.
 *
 * These carry no count: they are views of one job and there is nothing to
 * tally. The counted form is its own component and its own registry row.
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

export function Tabs({ items, value, defaultValue, onChange }: TabsProps) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;
  const { strip, fill } = useSelectedFill(active, items.map((item) => item.id).join("\u0000"));

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
      <span ref={fill} className="armada-tabs__fill" aria-hidden="true" hidden />
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
