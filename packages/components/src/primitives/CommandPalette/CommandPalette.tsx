import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

/**
 * The palette is a superset of the UI, never a substitute for it, and it is
 * the discovery surface: it is how a person learns forty shortcuts without a
 * cheat sheet, which is why every entry displays its binding and no action may
 * exist outside it.
 *
 * A floating layer — `--bg-overlay`, `--border-default`, `--radius-lg`, a
 * shadow, and no blur. Top-anchored, never centered: a centered dialog shifts
 * vertically as the result count changes, and a target that moves while you
 * type is a target you misclick.
 *
 * Three safety rules from `### Safety rules for single-key actions` are
 * structural here, not decoration.
 *
 * 1. Single-key shortcuts are suppressed whenever a text input holds focus.
 *    The palette is a text input, so every key except the navigation set and
 *    Esc goes to the query. Typing "axe" cannot approve, kill and open
 *    something.
 * 2. Every destructive action confirms, even from the keyboard. A destructive
 *    entry does not act and does not close the palette; it hands the entry to
 *    `onConfirm`, and the host opens the confirmation.
 * 3. Kill is `x`, never `k`. The map is the caller's, and the stories draw it
 *    that way.
 *
 * The palette obeys the lexicon. `aliases` are searched and never rendered, so
 * "terminate" finds Kill and the row still reads Kill.
 */
export type CommandPaletteEntry = {
  id: string;
  /** The section heading it groups under. Rendered once, above the group. */
  section: string;
  /** The lexicon term. Always what renders. */
  label: string;
  /** Searchable, never rendered. */
  aliases?: string[];
  /** Every entry displays its binding. */
  shortcut: string;
  /** 12px, strokeWidth 2, from the icon registry. */
  icon: ReactNode;
  /** Confirms rather than acting. */
  destructive?: boolean;
};

export type CommandPaletteProps = {
  open: boolean;
  entries: CommandPaletteEntry[];
  /**
   * Contents, in order: actions available on the current context, navigation,
   * jobs by id or name, settings.
   */
  sectionOrder?: string[];
  placeholder?: string;
  /** Storybook draws resting states, so the query can be set from outside. */
  defaultQuery?: string;
  onSelect?: (entry: CommandPaletteEntry) => void;
  onConfirm?: (entry: CommandPaletteEntry) => void;
  onClose?: () => void;
};

const DEFAULT_SECTIONS = ["Actions", "Navigation", "Jobs", "Settings"];

function matches(entry: CommandPaletteEntry, query: string) {
  if (!query) return true;
  const q = query.toLowerCase();
  if (entry.label.toLowerCase().includes(q)) return true;
  return (entry.aliases ?? []).some((alias) => alias.toLowerCase().includes(q));
}

export function CommandPalette({
  open,
  entries,
  sectionOrder = DEFAULT_SECTIONS,
  placeholder = "Search actions, jobs and settings",
  defaultQuery = "",
  onSelect,
  onConfirm,
  onClose,
}: CommandPaletteProps) {
  const [query, setQuery] = useState(defaultQuery);
  const [at, setAt] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  const results = useMemo(() => {
    const kept = entries.filter((entry) => matches(entry, query));
    const rank = (section: string) => {
      const found = sectionOrder.indexOf(section);
      return found === -1 ? sectionOrder.length : found;
    };
    return kept
      .map((entry, i) => ({ entry, i }))
      .sort((a, b) => rank(a.entry.section) - rank(b.entry.section) || a.i - b.i)
      .map((held) => held.entry);
  }, [entries, query, sectionOrder]);

  useEffect(() => setAt(0), [query]);

  useEffect(() => {
    if (open) input.current?.focus();
  }, [open]);

  if (!open) return null;

  function choose(entry: CommandPaletteEntry | undefined) {
    if (!entry) return;
    if (entry.destructive) {
      onConfirm?.(entry);
      return;
    }
    onSelect?.(entry);
    onClose?.();
  }

  // Only the navigation set and Esc are intercepted. Every other key belongs
  // to the query, which is the suppression rule holding.
  function onKey(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setAt((n) => (results.length ? (n + 1) % results.length : 0));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setAt((n) => (results.length ? (n - 1 + results.length) % results.length : 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(results[at]);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onClose?.();
    }
  }

  let lastSection = "";

  return (
    <div className="armada-palette-layer">
      <div className="armada-palette" role="dialog" aria-modal="true" aria-label="Command palette">
        <input
          ref={input}
          className="armada-palette__input"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="armada-palette-list"
          placeholder={placeholder}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKey}
        />
        <div className="armada-palette__list" id="armada-palette-list" role="listbox">
          {results.length === 0 ? (
            <p className="armada-palette__empty">
              No match. Every action Armada has is in this list, under the name Armada uses for it.
            </p>
          ) : null}
          {results.map((entry, n) => {
            const heading = entry.section !== lastSection ? entry.section : "";
            lastSection = entry.section;
            return (
              <div key={entry.id}>
                {heading ? <div className="armada-palette__section">{heading}</div> : null}
                <button
                  type="button"
                  role="option"
                  aria-selected={n === at}
                  className={
                    n === at ? "armada-palette__row armada-palette__row--active" : "armada-palette__row"
                  }
                  onMouseEnter={() => setAt(n)}
                  onClick={() => choose(entry)}
                >
                  <span className="armada-palette__glyph">{entry.icon}</span>
                  <span className="armada-palette__label">{entry.label}</span>
                  <kbd className="armada-palette__kbd">{entry.shortcut}</kbd>
                </button>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
