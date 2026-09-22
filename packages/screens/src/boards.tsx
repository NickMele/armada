// The draft half of a board's reading, while no operation answers it.
//
// The wire half arrives as props; this half arrives as context, because there
// is no read to hang it on. A real Fleet hands in nothing and each board falls
// back to the `…Of(detail)` derivation its own draft module carries.
//
// Context rather than props: a promoted shape (#1545) moves onto the Job's own
// read and loses its key here, and a prop chain built for it would have to be
// unpicked in the same change. Not exported from `index.ts` — the draft
// schema's rule, which `xtask` enforces on the main process.

import { createContext, useContext, type ReactNode } from "react";

import type {
  CaseRunView,
  CaseView,
  CriterionView,
  GroupView,
  JobMembersView,
  LandingRule,
  LedgerRow,
} from "./draft";

/**
 * What a moment's boards draw that the wire cannot carry — the fixtures'
 * `ArcDraft` minus what no board reads from context, so a moment is handed
 * straight in.
 */
export type BoardDraft = {
  landing?: LandingRule;
  criteria?: CriterionView[];
  groups?: GroupView[];
  cases?: CaseView[];
  runs?: CaseRunView[];
  record?: LedgerRow[];
  members?: JobMembersView;
};

/** A window with no mock behind it. Every board falls back to the wire. */
const NOTHING_DRAFTED: BoardDraft = {};

const Drafted = createContext<BoardDraft>(NOTHING_DRAFTED);

/** Hand a moment's draft to every board under it. */
export function BoardDrafts({ draft, children }: { draft?: BoardDraft; children: ReactNode }) {
  return <Drafted.Provider value={draft ?? NOTHING_DRAFTED}>{children}</Drafted.Provider>;
}

/** The draft this window was handed, read at the point of use. */
export function useBoardDraft(): BoardDraft {
  return useContext(Drafted);
}
