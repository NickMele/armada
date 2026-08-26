import { useState } from "react";

/**
 * Sections of one object. The active tab takes a 2px `--accent` underline,
 * `--fg-default` and weight 500; the rest sit in `--fg-muted`. Nothing
 * animates in — no entrance animations on data.
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

export function Tabs({ items, value, defaultValue, onChange }: TabsProps) {
  const [internal, setInternal] = useState(defaultValue ?? items[0]?.id);
  const active = value ?? internal;

  function select(id: string) {
    if (value === undefined) setInternal(id);
    onChange?.(id);
  }

  function onKey(event: React.KeyboardEvent<HTMLDivElement>) {
    const at = items.findIndex((item) => item.id === active);
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
    <div className="armada-tabs" role="tablist" onKeyDown={onKey}>
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
    </div>
  );
}
