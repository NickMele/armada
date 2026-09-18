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

import type { HelmContext, HelmScreen, JobSummary } from "@armada/protocol";
import { jobNumber } from "@armada/screens";

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
  studying: boolean;
  kitting: boolean;
  settling: boolean;
}): HelmScreen {
  if (where.reading) return "job_detail";
  if (where.clearing) return "cleanup";
  if (where.manifesting) return "manifest";
  if (where.overviewing) return "overview";
  if (where.studying) return "studio";
  if (where.kitting) return "kit";
  // **Last, and it was missing entirely until #1275.** A person on Settings
  // was told to Helm as being on the Board, which is the gap #1287 left when
  // it added `studio` and stopped.
  if (where.settling) return "settings";
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

/**
 * `AskHelm.context`, sent with every ask. Absent fields are left off the wire rather than sent `null`.
 * `studio` and `node` are the Studios surface's own — #1287 — and a node is sent only beside its Studio.
 */
export function contextOf(where: {
  screen: HelmScreen;
  picked: string | null;
  chip: string | null;
  cursor: string | null;
  studio?: string | null;
  node?: string | null;
}): HelmContext {
  const studio = where.screen === "studio" ? (where.studio ?? null) : null;
  const node = studio === null ? null : (where.node ?? null);
  return {
    screen: where.screen,
    ...(where.picked !== null ? { picked: where.picked } : {}),
    ...(where.chip !== null ? { chip: where.chip } : {}),
    ...(where.cursor !== null ? { cursor: where.cursor } : {}),
    ...(studio !== null ? { studio } : {}),
    ...(node !== null ? { node } : {}),
  };
}

/** How the open Studio and its selected node are named on the footer — `App.tsx` resolves the ids. */
export type StudioNamed = { name: string; node?: string };

/** Every screen but `job_detail`, which names itself off the Job it is reading rather than off this. */
const SCREEN_LABEL: Record<Exclude<HelmScreen, "job_detail">, string> = {
  overview: "Overview",
  board: "Job Board",
  manifest: "Manifest",
  cleanup: "Cleanup",
  studio: "Studios",
  kit: "Kit",
  settings: "Settings",
};

/**
 * The composer footer's one sentence — #1094. **Read off `context` itself**,
 * the same shape `contextOf` just built for the wire, so the words on screen
 * can never name a screen or a row other than the one actually sent. `jobs`
 * turns `cursor` and `chip`'s bare ids into the number Fleet's handle carries,
 * off the Board's own `jobNumber`.
 */
export function locationOf(context: HelmContext, jobs: readonly JobSummary[], studio?: StudioNamed): string {
  const numberOf = (jobId: string): string | undefined => {
    const job = jobs.find((one) => one.id === jobId);
    return job === undefined ? undefined : jobNumber(job);
  };
  if (context.screen === "job_detail") {
    const number = context.chip === undefined ? undefined : numberOf(context.chip);
    return number === undefined ? "Job's detail" : `Job ${number}'s detail (in context)`;
  }
  const label = SCREEN_LABEL[context.screen];
  // A Studio names itself, and the node selected on it, only where the wire carries the Studio.
  if (context.screen === "studio") {
    if (context.studio === undefined || studio === undefined) return label;
    const on = context.node === undefined || studio.node === undefined ? "" : ` · ${studio.node} selected`;
    return `${label} · ${studio.name}${on}`;
  }
  const number = context.cursor === undefined ? undefined : numberOf(context.cursor);
  return number === undefined ? label : `${label} · cursor on Job ${number}`;
}
