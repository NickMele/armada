// Every guide, in catalogue order. **This list is the only transcription.**
//
// One guide per file, so a change to one is a diff of one; one array here, so
// the catalogue, the marks and the first-contact bookkeeping all read the same
// set. Nothing globs the directory: the order is a reading order rather than
// an alphabet, and a glob would decide it by filename.
//
// A guide added here needs its number to be the next one and its group to
// already exist. `packages/screens/src/guides.test.ts` holds both to it.

import type { Guide, GuideGroupId } from "./guide";
import { GUIDE_COMPLETION } from "./001-what-completes-a-job";
import { GUIDE_GROUP_ORDER } from "./002-one-group-at-a-time";
import { GUIDE_GROUP_BOUNDARY } from "./003-a-groups-boundary";
import { GUIDE_STEP_BAR } from "./004-how-a-steps-bar-fills";
import { GUIDE_GROUP_EDGES } from "./005-a-group-with-two-edges";
import { GUIDE_PROCESSES } from "./006-a-process-is-its-executable";
import { GUIDE_WORKTREE_SIZE } from "./007-size-on-disk";

export * from "./guide";
export {
  GUIDE_COMPLETION,
  GUIDE_GROUP_ORDER,
  GUIDE_GROUP_BOUNDARY,
  GUIDE_STEP_BAR,
  GUIDE_GROUP_EDGES,
  GUIDE_PROCESSES,
  GUIDE_WORKTREE_SIZE,
};

export const GUIDES: readonly Guide[] = [
  GUIDE_COMPLETION,
  GUIDE_GROUP_ORDER,
  GUIDE_GROUP_BOUNDARY,
  GUIDE_STEP_BAR,
  GUIDE_GROUP_EDGES,
  GUIDE_PROCESSES,
  GUIDE_WORKTREE_SIZE,
];

/** The guides filed under one group, in number order. */
export function guidesIn(group: GuideGroupId): readonly Guide[] {
  return GUIDES.filter((guide) => guide.group === group);
}

/** The guide explaining a piece, or nothing where no guide claims it. */
export function guideForPiece(piece: string): Guide | undefined {
  return GUIDES.find((guide) => guide.piece === piece);
}
