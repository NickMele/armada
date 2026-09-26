// Every guide, in catalogue order. **This list is the only transcription.**
//
// One guide per file, so a change to one is a diff of one; one array here, so
// the catalogue, the marks and the first-contact bookkeeping all read the same
// set. Nothing globs the directory: the order is a reading order rather than
// an alphabet, and a glob would decide it by filename.
//
// **A guide inserted into a group renumbers the ones after it.** The catalogue
// reads down in number order and the groups are contiguous, so a new plan
// guide cannot take the next free number — `guides.test.ts` holds both rules.
// What is stable is the `piece`, which is what first contact is remembered by;
// a renumbered guide is the same guide and nobody meets it twice.

import type { Guide, GuideGroupId } from "./guide";
import { GUIDE_COMPLETION } from "./001-what-completes-a-job";
import { GUIDE_MEMBER_LINK } from "./002-how-a-member-reaches-the-one-before-it";
import { GUIDE_DISPATCH } from "./003-what-dispatch-sets-off";
import { GUIDE_GROUP_ORDER } from "./004-one-group-at-a-time";
import { GUIDE_GROUP_BOUNDARY } from "./005-a-groups-boundary";
import { GUIDE_PLAN_ASKS } from "./006-asking-the-plans-drone";
import { GUIDE_TIERS } from "./007-a-tier-picks-the-model";
import { GUIDE_STEP_BAR } from "./008-how-a-steps-bar-fills";
import { GUIDE_ALWAYS_LOOKS } from "./009-what-no-tick-turns-off";
import { GUIDE_CRITERIA } from "./010-what-the-judge-is-held-to";
import { GUIDE_GROUP_EDGES } from "./011-a-group-with-two-edges";
import { GUIDE_PROCESSES } from "./012-a-process-is-its-executable";
import { GUIDE_WORKTREE_SIZE } from "./013-size-on-disk";
import { GUIDE_LOOK } from "./014-looking-at-the-machine";

export * from "./guide";
export {
  GUIDE_COMPLETION,
  GUIDE_MEMBER_LINK,
  GUIDE_DISPATCH,
  GUIDE_GROUP_ORDER,
  GUIDE_GROUP_BOUNDARY,
  GUIDE_PLAN_ASKS,
  GUIDE_TIERS,
  GUIDE_STEP_BAR,
  GUIDE_ALWAYS_LOOKS,
  GUIDE_CRITERIA,
  GUIDE_GROUP_EDGES,
  GUIDE_PROCESSES,
  GUIDE_WORKTREE_SIZE,
  GUIDE_LOOK,
};

export const GUIDES: readonly Guide[] = [
  GUIDE_COMPLETION,
  GUIDE_MEMBER_LINK,
  GUIDE_DISPATCH,
  GUIDE_GROUP_ORDER,
  GUIDE_GROUP_BOUNDARY,
  GUIDE_PLAN_ASKS,
  GUIDE_TIERS,
  GUIDE_STEP_BAR,
  GUIDE_ALWAYS_LOOKS,
  GUIDE_CRITERIA,
  GUIDE_GROUP_EDGES,
  GUIDE_PROCESSES,
  GUIDE_WORKTREE_SIZE,
  GUIDE_LOOK,
];

/** The guides filed under one group, in number order. */
export function guidesIn(group: GuideGroupId): readonly Guide[] {
  return GUIDES.filter((guide) => guide.group === group);
}

/** The guide explaining a piece, or nothing where no guide claims it. */
export function guideForPiece(piece: string): Guide | undefined {
  return GUIDES.find((guide) => guide.piece === piece);
}
