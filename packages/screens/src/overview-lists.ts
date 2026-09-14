// What Overview's lists draw, from the Jobs in scope. #920. `OverviewLists.tsx` draws them; this
// decides them, so the arithmetic is tested as arithmetic.
//
// **Nothing here is a second rule.** `sectionsOf` in `board.ts` already says which section a Job
// is drawn under, and `foldLineages` in `lineage.ts` already says a redispatch chain is one row.
// This only scopes the board to the pick, applies both in the order the Board itself applies them
// — fold, then the Board's own default sort, then section — and leaves Done off: that list stays
// the Board's.
//
// **A Job the row shape cannot draw is not a row in any section.** `readingOf` in `reading.ts` is
// the same test `Jobs.tsx` runs before a badge is drawn; a Job that fails it is named instead,
// which is what `undrawable` is for. Named rather than silently dropped — the same choice the
// Board makes for the same case.

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { DEFAULT_SORT, ofPicked, sectionsOf, sorted } from "./board";
import type { BoardSection } from "./board";
import { foldLineages } from "./lineage";
import type { Dispatch } from "./lineage";
import { readingOf } from "./reading";

export type OverviewSection = { id: BoardSection; label: string; jobs: JobSummary[] };

export type OverviewListsRead = {
  /**
   * Needs you, Running, Queued, Recently ended and Other — Done left off, and
   * a section with nothing in it left off too. Recently ended joined the set
   * in Overview 28 (#1092): `sectionsOf` already carves it out of Done, so
   * excluding only `"done"` here is what lets it through without a second rule.
   */
  sections: OverviewSection[];
  /** Which dispatch of its lineage each folded-in Job is, keyed by id — `headlineOf`'s second argument. */
  dispatch: ReadonlyMap<string, Dispatch>;
  /** Jobs in scope the row shape cannot draw, oldest first — named beneath the lists, drawn by neither. */
  undrawable: JobSummary[];
};

/**
 * Overview's lists, scoped to the pick and ready to draw as the Board's own rows.
 *
 * `picked` is `null` for All repositories, and `ofPicked`'s own term otherwise — the same term
 * `OverviewSummary` resolves before reading this, so the strip's counts and the panels below it
 * never drift apart.
 */
export function overviewListsOf(jobs: readonly JobSummary[], picked: RepositorySummary | null): OverviewListsRead {
  const board = foldLineages(ofPicked(jobs, picked));
  const shown = sorted(board.shown, DEFAULT_SORT);
  const drawn = shown.filter((job) => readingOf(job).as === "badge");
  const undrawable = shown.filter((job) => readingOf(job).as !== "badge");
  const sections = sectionsOf(drawn).filter((section) => section.id !== "done");
  return { sections, dispatch: board.dispatch, undrawable };
}
