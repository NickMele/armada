import { useState } from "react";
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
};

export function TabsWithCounts({ items, value, defaultValue, onChange }: TabsWithCountsProps) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;

  function select(id: string) {
    if (value === undefined) setInternal(id);
    onChange?.(id);
  }

  function onKey(event: React.KeyboardEvent<HTMLDivElement>) {
    const at = items.findIndex((item) => item.id === active);
    // A tab strip with nothing in it takes no arrow key. Without this the
    // modulo below is `% 0`, which throws rather than doing nothing.
    if (items.length === 0) {
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      select(items[(at + 1) % items.length].id);
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      select(items[(at - 1 + items.length) % items.length].id);
    }
  }

  return (
    <div className="armada-tabs-counts" role="tablist" onKeyDown={onKey}>
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
    </div>
  );
}
