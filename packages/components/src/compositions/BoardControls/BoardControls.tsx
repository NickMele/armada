import type { KeyboardEvent, Ref } from "react";
import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Select } from "../../primitives/Select/Select";
import {
  TabsWithCounts,
  type TabsWithCountsItem,
} from "../../primitives/TabsWithCounts/TabsWithCounts";

/**
 * Board controls — the filter set above the Job Board's list.
 *
 * Two axes, and the Manifest is neither — the Board is already scoped to
 * one Manifest, and origin was drawn as a filter and rejected: what needs
 * me, what is running, why has that not started are all state. What's left
 * is state plus one text match, above the tabs with sort rather than beside
 * them — a sixth tab is not what a text match is.
 */

/**
 * Search reads every job whatever tab is set: the surface bypasses the tab,
 * passes `suspended`, and the strip steps back without losing the
 * selection, so the tab's counts reflect what search matched rather than
 * the whole board. Resetting to `All` was the other reading and lost — it
 * spends a choice to make the sentence true and has nothing to restore.
 */

/**
 * The search's key is drawn inside the field, not beside it as a second
 * control. Nothing else here is bound to a key — the surface alone knows
 * whether the field holds focus, and a single-key shortcut firing while
 * someone types is the first failure the safety rules name. `Esc` is the
 * exception: it is the field's own behaviour, not a single-key action.
 */
export type BoardSortOption = {
  /** The stored value: `critical_first`, `oldest_first`. */
  id: string;
  /** Sentence case: `Critical first`. */
  label: string;
};

export type BoardControlsProps = {
  /** The text match, as typed. Empty is no match rather than no results. */
  query: string;
  onQuery: (query: string) => void;
  /**
   * What the field says when it is empty. Names what is searched, since the
   * whole point of the control is that it is not narrowed by the tab.
   */
  placeholder?: string;
  /**
   * The field itself, so a surface can put the cursor in it. It is what `/`
   * binds to, and the surface binds `/`.
   */
  searchRef?: Ref<HTMLInputElement>;
  /**
   * What `Esc` does after it has cleared the field: hand the cursor back to
   * the list. Clearing is unconditional and happens here.
   */
  onLeaveSearch?: () => void;
  /** The orders offered. `Critical first` is the Board's default. */
  sorts: readonly BoardSortOption[];
  sort: string;
  onSort: (sort: string) => void;
  /** The state tabs, each carrying its own count and its own key. */
  tabs: readonly TabsWithCountsItem[];
  tab: string;
  /**
   * Choosing a tab. **The surface clears the search here**, because a suspended
   * tab that did nothing when pressed would be a dead control — and pressing
   * one asks for a state rather than for a match.
   */
  onTab: (tab: string) => void;
  /** The tab is bypassed by something else on the surface, and drawn set back. */
  suspended?: boolean;
  /** The key that focuses the field, drawn beside it. Omitted draws nothing. */
  searchKey?: string;
  /**
   * Which arrangement the list beneath is drawn in, and the control that
   * changes it. Omitted draws no control, which is what a surface offering one
   * arrangement passes.
   *
   * **Beside sort, not among the tabs.** A tab picks which Jobs exist and this
   * picks how the ones that exist are drawn — the same distinction that keeps
   * the text match off the tab line. Sort is its neighbour because the two are
   * the same kind of thing: neither changes the set.
   *
   * **Words, not glyphs.** A table and a card have no silhouette a person
   * reads at 13px without learning it first, and the icon registry has no row
   * for either. Two short words cost less than a wrong picture.
   */
  view?: "card" | "table";
  onView?: (view: "card" | "table") => void;
};

/** The two arrangements, in the order they are drawn. */
const VIEWS: readonly { id: "card" | "table"; label: string }[] = [
  { id: "card", label: "Cards" },
  { id: "table", label: "Table" },
];

/** What the field says when the surface does not say otherwise. */
const SEARCHES_EVERYTHING = "Search every job";

export function BoardControls({
  query,
  onQuery,
  placeholder = SEARCHES_EVERYTHING,
  searchRef,
  onLeaveSearch,
  sorts,
  sort,
  onSort,
  tabs,
  tab,
  onTab,
  suspended = false,
  searchKey,
  view,
  onView,
}: BoardControlsProps) {
  // Escape clears the field and gives the cursor back, in that order and always
  // both: a field that cleared but kept focus leaves the person somewhere no
  // single-key shortcut works, which is the state they pressed Escape to leave.
  function onSearchKey(event: KeyboardEvent<HTMLInputElement>): void {
    if (event.key !== "Escape") return;
    event.preventDefault();
    // Stopped, so an enclosing Escape — the one that closes a detail — does not
    // also fire on the press that was meant for this field.
    event.stopPropagation();
    onQuery("");
    onLeaveSearch?.();
  }

  return (
    <div className="armada-board-controls">
      <div className="armada-board-controls__line">
        <div className="armada-board-controls__search">
          <Input
            ref={searchRef}
            type="search"
            value={query}
            placeholder={placeholder}
            aria-label={placeholder}
            onChange={(event) => onQuery(event.target.value)}
            onKeyDown={onSearchKey}
            trailing={
              searchKey === undefined ? undefined : (
                <Kbd className="armada-board-controls__hint" aria-hidden>
                  {searchKey}
                </Kbd>
              )
            }
          />
        </div>
        {view === undefined || onView === undefined ? null : (
          <div className="armada-board-controls__view" role="group" aria-label="View">
            {VIEWS.map((one) => (
              <button
                type="button"
                key={one.id}
                className="armada-board-controls__view-option"
                aria-pressed={view === one.id}
                onClick={() => onView(one.id)}
              >
                {one.label}
              </button>
            ))}
          </div>
        )}
        <div className="armada-board-controls__sort">
          <Select aria-label="Sort" value={sort} onChange={(event) => onSort(event.target.value)}>
            {sorts.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))}
          </Select>
        </div>
      </div>
      <TabsWithCounts items={[...tabs]} value={tab} onChange={onTab} suspended={suspended} />
    </div>
  );
}
