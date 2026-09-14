// Where Helm's dock says the person is — #1075. No window, no Fleet: the two
// decisions a rendering cannot show on its own, folded here and tested on
// their own.
//
// **The chip and the point are one event.** Opening a Job's detail both
// chips it and points Helm at its repository (`HelmConnection.point()`,
// wired in `App.tsx`) — `opened` answers both at once so the two can never
// drift apart. Leaving the Job (`opened` with `null`) drops the chip; its own
// `×` (`dismissed`) drops it early, for a question that isn't about this Job,
// and reopening the same Job (`opened` again) restores it.

import type { HelmContext, HelmScreen } from "@armada/protocol";

/** The chip's own state: which Job opened it, and whether its `×` was pressed since. */
export type ChipState = { openJobId: string | null; dismissed: boolean };

export const NO_CHIP: ChipState = { openJobId: null, dismissed: false };

/** What `opened` answers with: the chip's next state, and the repository to point Helm at, if any. */
export type Opened = { state: ChipState; point: string | null };

/**
 * A Job's detail opened, or closed (`job: null`). A reopen of the Job the
 * chip already names is a no-op — nothing to point at twice.
 */
export function opened(state: ChipState, job: { id: string; manifestId: string } | null): Opened {
  if (job !== null && job.id === state.openJobId && !state.dismissed) {
    return { state, point: null };
  }
  const next: ChipState = job === null ? NO_CHIP : { openJobId: job.id, dismissed: false };
  return { state: next, point: job === null ? null : job.manifestId };
}

/** The `×`. The chip drops from the next ask's context; the Job stays open. */
export function dismissed(state: ChipState): ChipState {
  return { ...state, dismissed: true };
}

/** The chipped Job's id, only while the chip stands. */
export function chippedJobId(state: ChipState): string | null {
  return state.dismissed ? null : state.openJobId;
}

/** Which screen `AskHelm.context` names, in the precedence `App.tsx` draws by. */
export function screenOf(where: {
  reading: boolean;
  clearing: boolean;
  manifesting: boolean;
  overviewing: boolean;
}): HelmScreen {
  if (where.reading) return "job_detail";
  if (where.clearing) return "cleanup";
  if (where.manifesting) return "manifest";
  if (where.overviewing) return "overview";
  return "board";
}

/**
 * Which cursor `AskHelm.context` carries — the Board's or Overview's,
 * whichever screen is showing. Neither is a fact off the other: the Board's
 * `cursor` is stale once Overview is what's on screen, and Overview has its
 * own roving row (`OverviewLists`, reported the same way `Jobs.tsx` reports
 * the Board's).
 */
export function cursorRowFor(where: {
  screen: HelmScreen;
  board: string | null;
  overview: string | null;
}): string | null {
  if (where.screen === "board") return where.board;
  if (where.screen === "overview") return where.overview;
  return null;
}

/** `AskHelm.context`, sent with every ask. Absent fields are left off the wire rather than sent `null`. */
export function contextOf(where: {
  screen: HelmScreen;
  picked: string | null;
  chip: string | null;
  cursor: string | null;
}): HelmContext {
  return {
    screen: where.screen,
    ...(where.picked !== null ? { picked: where.picked } : {}),
    ...(where.chip !== null ? { chip: where.chip } : {}),
    ...(where.cursor !== null ? { cursor: where.cursor } : {}),
  };
}
