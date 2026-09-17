// The Studios surface's two reads, as the app holds them — one repository's list, and one Studio
// with its graph. No React, so main imports these shapes. #1287.

import type { Outcome, Studio, StudioList, StudioSummary } from "@armada/protocol";

/**
 * `GET /studios?manifest_id=`, held while the surface shows. **Named by its repository**, so an
 * answer for a repository the rail has since moved off is told apart from the one being read.
 */
export type StudiosRead =
  | { state: "none" }
  | { state: "reading"; manifestId: string }
  | { state: "read"; manifestId: string; list: StudioList }
  | { state: "failed"; manifestId: string; outcome: Outcome };

/**
 * `GET /studios/:studio_id`, held while that Studio is open, and replaced whole by every
 * `studio.changed` about it. `gone` is `studio.deleted`: the Studio this window had open is not
 * there any more, which is not the same as a read that failed.
 */
export type StudioRead =
  | { state: "none" }
  | { state: "reading"; studioId: string }
  | { state: "read"; studio: Studio }
  | { state: "gone"; studioId: string }
  | { state: "failed"; studioId: string; outcome: Outcome };

/** What an act on a Studio answers: the Studio whole, as Fleet wrote it, or why not. */
export type StudioAnswer = { ok: true; studio: Studio } | { ok: false; outcome: Outcome };

/**
 * The list main holds, with one Studio Fleet just wrote folded in: replaced where the list
 * has it, added where it does not, and last touched first — `list_studios`' own order.
 */
export function foldStudio(list: readonly StudioSummary[], studio: StudioSummary): StudioSummary[] {
  const summary: StudioSummary = {
    id: studio.id,
    manifest_id: studio.manifest_id,
    ...(studio.name === undefined ? {} : { name: studio.name }),
    created_at: studio.created_at,
    touched_at: studio.touched_at,
  };
  const rest = list.filter((one) => one.id !== studio.id);
  return [summary, ...rest].sort((a, b) => (a.touched_at < b.touched_at ? 1 : a.touched_at > b.touched_at ? -1 : 0));
}
