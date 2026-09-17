import { useLayoutEffect, useRef, useState } from "react";
import { Kbd } from "../Kbd/Kbd";

/**
 * Separate queues, each carrying how much is waiting. A separate component and
 * a separate row in the registry, because the count changes what the tab
 * claims: it answers how much is waiting before a tab is chosen.
 *
 * A tab carries a count only if the number is a backlog. A stream always has
 * items and none of them want anything, so a number on it would read as work
 * outstanding. Counts belong to queues, not to feeds.
 *
 * Zero renders as no count, never as `0`. An empty queue is the resting state
 * of a healthy fleet, and a row of zeros trains the eye to skip the number —
 * which is the one thing it must not do when the number changes.
 *
 * The count is a trailing mono value in `--fg-subtle`, never a filled pill: a
 * pill is a badge and a badge carries status.
 *
 * **A tab may carry the key that selects it, and then the key is bordered and
 * the count is not.** Both are numerals in the same row on the Job Board —
 * `1` selects the tab and `15` is how many jobs are behind it — so the `kbd`
 * chip is the whole of what separates the key you press from the number, and
 * the two sit on opposite sides of the label besides. A tab with no shortcut
 * renders none; nothing here invents one from the tab's position.
 */
export type TabsWithCountsItem = {
  id: string;
  /** Sentence case. */
  label: string;
  /** A backlog. Omitted, or zero, renders as nothing. */
  count?: number;
  /**
   * The single key that selects this tab, drawn leading the label as a `kbd`.
   * Displayed only — binding it is the surface's, because a tab strip cannot
   * know whether a text input on the same screen holds focus, and a single-key
   * shortcut that fires while somebody is typing is the failure the design
   * contract's safety rules name first.
   */
  shortcut?: string;
};

export type TabsWithCountsProps = {
  items: TabsWithCountsItem[];
  value?: string;
  defaultValue?: string;
  onChange?: (id: string) => void;
  /**
   * The strip is not narrowing anything right now — something else on the
   * surface is. Every tab steps back to `--fg-subtle` and the selected one
   * gives up its accent underline, because an underline that reads as "this is
   * what you are looking at" would be saying something untrue.
   *
   * **Set back, never disabled.** The selection is still there and still says
   * which tab a person chose; pressing one is how they get it back, so the
   * tabs stay pressable and keep their counts. A disabled strip would make the
   * way out of the state the way that does not work.
   *
   * The Job Board sets it while its search field holds text: a text match is
   * not a state, so it bypasses the state tab rather than changing it.
   */
  suspended?: boolean;
};

/**
 * The same bar `Tabs` draws.
 *
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

export function TabsWithCounts({
  items,
  value,
  defaultValue,
  onChange,
  suspended = false,
}: TabsWithCountsProps) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;
  const { strip, bar } = useActiveBar(
    active,
    items.map((item) => `${item.id}:${item.count ?? 0}:${item.shortcut ?? ""}`).join("\u0000"),
  );

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
    <div
      ref={strip}
      className="armada-tabs-counts"
      role="tablist"
      data-suspended={suspended || undefined}
      onKeyDown={onKey}
    >
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="tab"
          aria-selected={item.id === active}
          tabIndex={item.id === active ? 0 : -1}
          className={
            item.id === active
              ? "armada-tabs-counts__tab armada-tabs-counts__tab--active"
              : "armada-tabs-counts__tab"
          }
          onClick={() => select(item.id)}
          aria-keyshortcuts={item.shortcut}
        >
          {item.shortcut ? <Kbd aria-hidden>{item.shortcut}</Kbd> : null}
          {item.label}
          {item.count ? <span className="armada-tabs-counts__count">{item.count}</span> : null}
        </button>
      ))}
      <span ref={bar} className="armada-tabs-counts__bar" aria-hidden="true" hidden />
    </div>
  );
}
