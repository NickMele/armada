import { useEffect, useMemo, useRef, useState } from "react";
import type { LucideIcon } from "lucide-react";

import { Kbd, KbdChord } from "../Kbd/Kbd";

/**
 * The palette is a superset of the UI, never a substitute for it, and it is
 * the discovery surface: it is how a person learns forty shortcuts without a
 * cheat sheet, which is why every entry displays its binding and no action may
 * exist outside it.
 *
 * **The empty query is the cheat sheet, and that is the load-bearing state.**
 * Opening the palette with nothing typed shows every section head and every
 * binding, so reading three on the way past to do one thing is the whole
 * mechanism by which forty get learned without a help screen. Every other
 * state here is a narrowing of it.
 *
 * A floating layer — `--bg-overlay`, `--border-default`, `--radius-lg`, a
 * shadow, and no blur. **Top-anchored, never centered**: a centered dialog
 * rises as results fall away, so the row under the cursor changes while a
 * person is still typing.
 *
 * Three safety rules from `### Safety rules for single-key actions` are
 * structural here, not decoration.
 *
 * 1. Single-key shortcuts are suppressed whenever a text input holds focus.
 *    The palette is a text input, so every key except the navigation set and
 *    `Esc` goes to the query. Typing "axe" cannot approve, kill and open
 *    something.
 * 2. Every destructive action confirms, even from the keyboard. A destructive
 *    entry does not act and does not close the palette; it hands the entry to
 *    `onConfirm`, and the host opens the confirmation over it.
 * 3. Kill is `x`, never `k`. The map is `actions.toml`'s and this only draws
 *    what it is given.
 *
 * **The palette obeys the lexicon.** `aliases` are searched and never
 * rendered, so "terminate" finds Kill and the row still reads Kill — with no
 * marked span, because an alias has none to mark and faking one would render
 * the alias.
 */
export type PaletteEntry = {
  id: string;
  /** The id of the section it groups under. */
  section: string;
  /** The lexicon term. Always what renders. */
  label: string;
  /** Searched, never rendered. */
  aliases?: readonly string[];
  /**
   * The binding, where the entry has one.
   *
   * **Jobs and settings carry none, and that is not a gap.** A binding belongs
   * to an act; opening a specific job is a search result.
   */
  shortcut?: string;
  /**
   * The glyph, 12px, from `packages/icons/icons.toml` by way of `actions.ts`.
   *
   * **Absent is common and says nothing is wrong.** Thirteen acts carry no
   * registered glyph, so the slot holds its width and draws nothing rather
   * than the column going ragged or a silhouette being invented for it.
   */
  icon?: LucideIcon;
  /** A setting's current value, in mono, right of the label. */
  value?: string;
  /** Danger is the dropdown-menu treatment: red label and glyph, no fill. */
  destructive?: boolean;
  /**
   * Why this row cannot be chosen, in a few words, where it cannot be. The row
   * draws dimmed and says so beside its binding.
   *
   * **Two kinds of dormancy, one rendering.** A binding the registry carries
   * and nothing answers — `not built · #250`, from `unbuilt` — and an act this
   * app cannot reach right now, like copying debug info with no failure on
   * screen. Both are facts the caller holds; neither is a list of exceptions
   * kept in here. The contract's rule is that a row a person presses and gets
   * nothing from is worse than one that is absent, and a dimmed row carrying
   * the reason is neither.
   */
  dormant?: string;
};

export type PaletteSection = {
  id: string;
  /**
   * The head. **The context block is titled with the job it acts on**, so a
   * palette that will Kill something says which something before the first row
   * is read.
   */
  title: string;
};

export type CommandPaletteProps = {
  open: boolean;
  /**
   * The sections, in the order the contract fixes: actions available on the
   * current context, navigation, jobs by id or name, settings. **The caller
   * owns the order** — the context block's title changes with the job, so the
   * list cannot be a constant in here.
   */
  sections: readonly PaletteSection[];
  entries: readonly PaletteEntry[];
  /**
   * What was searched, as a sentence, for the state where nothing matched.
   * "every job on this Manifest, at every status" — the host knows the extent
   * of its own index and this component does not.
   */
  searched: string;
  placeholder?: string;
  /** Storybook draws narrowed states, so the query can be seeded from outside. */
  defaultQuery?: string;
  onSelect?: (entry: PaletteEntry) => void;
  onConfirm?: (entry: PaletteEntry) => void;
  onClose?: () => void;
};

/** Whether the query matches, and where in the label it matched. */
type Hit = {
  entry: PaletteEntry;
  /** Where the query sits in the label, or `null` on an alias-only hit. */
  at: number | null;
};

function hit(entry: PaletteEntry, query: string): Hit | null {
  if (query === "") return { entry, at: null };
  const wanted = query.toLowerCase();
  const at = entry.label.toLowerCase().indexOf(wanted);
  if (at >= 0) return { entry, at };
  const alias = (entry.aliases ?? []).some((held) => held.toLowerCase().includes(wanted));
  return alias ? { entry, at: null } : null;
}

export function CommandPalette({
  open,
  sections,
  entries,
  searched,
  placeholder = "Search actions, jobs and settings",
  defaultQuery = "",
  onSelect,
  onConfirm,
  onClose,
}: CommandPaletteProps) {
  const [query, setQuery] = useState(defaultQuery);
  const [at, setAt] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const active = useRef<HTMLDivElement>(null);

  // Grouped by section in the caller's order, and in the caller's order inside
  // each. Nothing is ranked: the contract fixes what comes first, and a
  // relevance score would reorder a list a person is learning positions in.
  const grouped = useMemo(() => {
    const found = entries.flatMap((entry) => hit(entry, query) ?? []);
    return sections
      .map((section) => ({
        section,
        hits: found.filter((held) => held.entry.section === section.id),
      }))
      .filter((group) => group.hits.length > 0);
  }, [entries, sections, query]);

  const flat = useMemo(() => grouped.flatMap((group) => group.hits), [grouped]);

  // **The first row is always active**, so a query narrowed to one result is a
  // two-key act. Reset on every change to the query rather than clamped: a
  // cursor left at position four in a list that now has two is a cursor
  // somewhere the person did not put it.
  useEffect(() => setAt(0), [query]);

  useEffect(() => {
    if (open) input.current?.focus();
  }, [open]);

  // The active row is kept in view. `nearest` and not `center`: the list is
  // scrolled past 400px and centering would jump a row that was already
  // visible, which is the moving target the top anchor exists to prevent.
  useEffect(() => {
    active.current?.scrollIntoView({ block: "nearest" });
  }, [at]);

  if (!open) return null;

  function choose(held: Hit | undefined) {
    if (held === undefined) return;
    // Drawn so the binding can be learned, and inert because the row already
    // says why. Pressing it does nothing rather than reaching something
    // invented.
    if (held.entry.dormant !== undefined) return;
    if (held.entry.destructive === true) {
      onConfirm?.(held.entry);
      return;
    }
    onSelect?.(held.entry);
    onClose?.();
  }

  // Only the navigation set and `Esc` are intercepted. Every other key belongs
  // to the query, which is the suppression rule holding.
  function onKey(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setAt((n) => (flat.length === 0 ? 0 : (n + 1) % flat.length));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setAt((n) => (flat.length === 0 ? 0 : (n - 1 + flat.length) % flat.length));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(flat[at]);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onClose?.();
    }
  }

  let n = -1;

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
          aria-activedescendant={flat[at] === undefined ? undefined : rowId(flat[at]!.entry)}
          aria-label={placeholder}
          placeholder={placeholder}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKey}
        />
        <div className="armada-palette__list" id="armada-palette-list" role="listbox">
          {flat.length === 0 ? (
            /* **Names the query and says what was searched.** No suggestions
               and no did-you-mean: the palette is the discovery surface, so
               the honest answer to a miss is the extent of the index. */
            <p className="armada-palette__empty">
              {`Nothing matches “${query}”. Searched ${searched}.`}
            </p>
          ) : null}
          {grouped.map((group) => (
            <div key={group.section.id} className="armada-palette__group">
              {/* Section heads are what stop four kinds of result reading as
                  one list. Drawn only where the section has a hit. */}
              <div className="armada-palette__section">{group.section.title}</div>
              {group.hits.map((held) => {
                n += 1;
                const where = n;
                return (
                  <Row
                    key={held.entry.id}
                    hit={held}
                    query={query}
                    active={where === at}
                    ref={where === at ? active : undefined}
                    onEnter={() => setAt(where)}
                    onChoose={() => choose(held)}
                  />
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/** The dom id of a row, so `aria-activedescendant` can name it. */
function rowId(entry: PaletteEntry): string {
  return `armada-palette-row-${entry.id}`;
}

function Row({
  hit: held,
  query,
  active,
  ref,
  onEnter,
  onChoose,
}: {
  hit: Hit;
  query: string;
  active: boolean;
  ref?: React.Ref<HTMLDivElement>;
  onEnter: () => void;
  onChoose: () => void;
}) {
  const { entry } = held;
  const Glyph = entry.icon;
  const dormant = entry.dormant !== undefined;
  const classes = [
    "armada-palette__row",
    active ? "armada-palette__row--active" : "",
    entry.destructive === true ? "armada-palette__row--danger" : "",
    dormant ? "armada-palette__row--dormant" : "",
  ]
    .filter((held) => held !== "")
    .join(" ");

  return (
    // A row is an option and not a button: focus stays in the query field, so
    // typing carries on while the cursor moves, and `aria-activedescendant` on
    // the field is what says which row the cursor is on.
    <div
      ref={ref}
      id={rowId(entry)}
      role="option"
      aria-selected={active}
      aria-disabled={dormant ? true : undefined}
      className={classes}
      onMouseEnter={onEnter}
      onClick={onChoose}
    >
      {/* The slot holds its width whether or not there is a glyph, so a
          section with two icons and six blanks still reads as a column. */}
      <span className="armada-palette__glyph">
        {Glyph === undefined ? null : <Glyph size={12} strokeWidth={2} aria-hidden />}
      </span>
      <span className="armada-palette__label">
        <Marked label={entry.label} query={query} at={held.at} />
      </span>
      {entry.value === undefined ? null : (
        <span className="armada-palette__value">{entry.value}</span>
      )}
      {dormant ? <span className="armada-palette__dormant">{entry.dormant}</span> : null}
      {entry.shortcut === undefined ? null : <Shortcut shortcut={entry.shortcut} />}
    </div>
  );
}

/**
 * The label, with the matched span marked.
 *
 * **An alias hit marks nothing.** `at` is `null` there, because the match was
 * on a word that never renders — no "matched terminate", no highlight, and
 * nothing faked onto the lexicon term standing in its place.
 *
 * The mark is weight and not colour. The palette row has one text colour and
 * status hue is never chosen; weight is the channel already in the type scale.
 */
function Marked({ label, query, at }: { label: string; query: string; at: number | null }) {
  if (at === null || query === "") return <>{label}</>;
  return (
    <>
      {label.slice(0, at)}
      <mark className="armada-palette__mark">{label.slice(at, at + query.length)}</mark>
      {label.slice(at + query.length)}
    </>
  );
}

/**
 * The binding, cut into one box per key.
 *
 * **The registry's spelling is the input and nothing is re-spelled here.**
 * `⌘K`, `j / k / ↓ / ↑`, `⌘1–⌘4` and `[ ]` are all how `actions.toml` writes
 * them, and the gate holds that file to the contract's map — so the palette
 * shows exactly the string a person will read in the contract. What this does
 * is put each key in its own box, because a chord set in one box reads as a
 * key that does not exist.
 */
function Shortcut({ shortcut }: { shortcut: string }) {
  return (
    <KbdChord className="armada-palette__keys">
      {piecesOf(shortcut).map((piece, i) =>
        piece.key === undefined ? (
          <span key={i} className="armada-palette__between">
            {piece.between}
          </span>
        ) : (
          <Kbd key={i}>{piece.key}</Kbd>
        ),
      )}
    </KbdChord>
  );
}

/** A key that gets a box, or a separator that is drawn between two. */
type Piece = { key: string; between?: undefined } | { key?: undefined; between: string };

/**
 * Cut a binding into boxes and separators.
 *
 * A space separates two whole bindings and needs no mark — the gap between
 * boxes is the separation. `/` and `–` are kept and drawn, because `j / k`
 * means either key and `⌘1–⌘4` means a range, and dropping them would turn
 * both into a chord. A leading `⌘` takes a box of its own for the same reason
 * the chord rule exists.
 */
function piecesOf(shortcut: string): Piece[] {
  return shortcut
    .split(/(\s+|\/|–)/)
    .flatMap((part): Piece[] => {
      if (part.trim() === "") return [];
      if (part === "/" || part === "–") return [{ between: part }];
      if (part.length > 1 && part.startsWith("⌘")) {
        return [{ key: "⌘" }, { key: part.slice(1) }];
      }
      return [{ key: part }];
    });
}
