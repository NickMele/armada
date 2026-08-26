import { useEffect, useRef, useState } from "react";

/**
 * The row menu, and the one place besides a dialog, sheet, popover, tooltip
 * and the palette where a shadow is legal. `--bg-overlay`, `--border-default`,
 * `--radius-lg`, and no blur.
 *
 * Items carry a right-aligned kbd where the action has a binding, and item
 * height is unchanged by it. The destructive item sits last and below a
 * separator.
 *
 * No glyphs on items. Iconography puts icons on ghost and icon-only row
 * actions, in confirmation dialogs and in toolbars — a menu item is none of
 * those, and the component sheet's own annotation on the split button says the
 * same. One drawing in the sheet disagrees; see the report.
 */
export type DropdownMenuEntry =
  | { kind: "item"; id: string; label: string; shortcut?: string; danger?: boolean }
  | { kind: "separator"; id: string }
  | { kind: "label"; id: string; label: string };

export type DropdownMenuProps = {
  /** Sentence case, and it names what the menu is for. */
  triggerLabel: string;
  entries: DropdownMenuEntry[];
  defaultOpen?: boolean;
  onSelect?: (id: string) => void;
};

export function DropdownMenu({
  triggerLabel,
  entries,
  defaultOpen = false,
  onSelect,
}: DropdownMenuProps) {
  const [open, setOpen] = useState(defaultOpen);
  const root = useRef<HTMLDivElement>(null);

  // Esc closes an overlay, per the global tier.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    function onDown(event: MouseEvent) {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  return (
    <div className="armada-dropdown-menu" ref={root}>
      <button
        type="button"
        className="armada-dropdown-menu__trigger"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {triggerLabel}
      </button>
      {open ? (
        <div className="armada-dropdown-menu__panel" role="menu">
          {entries.map((entry) => {
            if (entry.kind === "separator") {
              return <div key={entry.id} className="armada-dropdown-menu__separator" role="separator" />;
            }
            if (entry.kind === "label") {
              return (
                <div key={entry.id} className="armada-dropdown-menu__label">
                  {entry.label}
                </div>
              );
            }
            return (
              <button
                key={entry.id}
                type="button"
                role="menuitem"
                className={
                  entry.danger
                    ? "armada-dropdown-menu__item armada-dropdown-menu__item--danger"
                    : "armada-dropdown-menu__item"
                }
                onClick={() => {
                  setOpen(false);
                  onSelect?.(entry.id);
                }}
              >
                <span className="armada-dropdown-menu__text">{entry.label}</span>
                {entry.shortcut ? (
                  <kbd className="armada-dropdown-menu__kbd">{entry.shortcut}</kbd>
                ) : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
