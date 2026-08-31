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
 * **Two axes, and the Manifest is neither.** The Board is already scoped to one
 * Manifest, so scope is not a control here; origin was drawn as a filter and
 * rejected, because *what needs me*, *what is running* and *why has that not
 * started* are all state. What is left is state, and one text match.
 *
 * **The text match sits above the tabs, with sort.** A text match is not a
 * state, and putting it in the same row as the tabs would read as a sixth one.
 * The arrangement is the argument: what narrows by state is on the state line,
 * and what cuts across every state is above it.
 *
 * **Search reads every job whatever tab is set, and the tab is suspended while
 * it does.** The sentence is about what search reaches, not an instruction to
 * move the tab: the surface bypasses the tab, passes `suspended`, and the strip
 * steps back without losing the selection — so clearing the field gives the
 * person their filter back. Resetting the tab to `All` instead would spend a
 * choice to make a sentence true and then have nothing to restore.
 *
 * The tab counts a surface passes in should be counts of what the search
 * already matched rather than of the whole board, so a suspended strip is still
 * a breakdown of what is on screen.
 *
 * **Nothing here is bound to a key.** The tabs display their keys and this
 * displays the search's, because the surface is the only thing that knows
 * whether a text input holds focus — and a single-key shortcut that fires while
 * somebody is typing is the first failure the design contract's safety rules
 * name. `Esc` is the exception, and it is not a single-key action: it is the
 * field's own behaviour, so it lives here.
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
};

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
          />
        </div>
        {searchKey === undefined ? null : (
          <Kbd className="armada-board-controls__hint" aria-hidden>
            {searchKey}
          </Kbd>
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
