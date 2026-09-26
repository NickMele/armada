// Every guide, in catalogue order. **This list is the only transcription.**
//
// One guide per file, so a change to one is a diff of one; one array here, so
// the catalogue, the marks and the first-contact bookkeeping all read the same
// set. Nothing globs the directory: the order is a reading order rather than
// an alphabet, and a glob would decide it by filename.
//
// **A number is the guide's own for good.** A guide added takes the next
// number above the highest in use and is filed wherever its group wants it, so
// the catalogue reads in group order and not in number order. Guide 11 is
// retired and 11 is gone. `guides.test.ts` holds both rules.

import type { Guide, GuideGroupId } from "./guide";
import { GUIDE_COMPLETION } from "./001-when-is-a-job-done";
import { GUIDE_MEMBER_LINK } from "./002-how-do-jobs-land-in-order";
import { GUIDE_DISPATCH } from "./003-how-do-i-dispatch-a-job";
import { GUIDE_GROUP_ORDER } from "./004-why-does-only-one-group-run-at-a-time";
import { GUIDE_GROUP_BOUNDARY } from "./005-when-do-checks-run";
import { GUIDE_PLAN_ASKS } from "./006-how-do-i-change-the-plan";
import { GUIDE_TIERS } from "./007-which-model-does-each-task-get";
import { GUIDE_STEP_BAR } from "./008-what-does-the-progress-bar-show";
import { GUIDE_ALWAYS_LOOKS } from "./009-what-do-the-tick-boxes-on-a-step-do";
import { GUIDE_CRITERIA } from "./010-what-does-the-judge-check";
import { GUIDE_PROCESSES } from "./012-what-are-these-processes";
import { GUIDE_WORKTREE_SIZE } from "./013-how-much-disk-is-this-using";
import { GUIDE_LOOK } from "./014-what-does-a-look-do";
import { GUIDE_JOB } from "./015-what-is-a-job";
import { GUIDE_BEFORE_A_RUN } from "./016-what-can-i-change-before-a-job-runs";
import { GUIDE_PLAN } from "./017-what-is-a-plan";
import { GUIDE_DRONE } from "./018-what-is-a-drone";
import { GUIDE_WORKFLOW } from "./019-what-is-a-workflow";

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
  GUIDE_PROCESSES,
  GUIDE_WORKTREE_SIZE,
  GUIDE_LOOK,
  GUIDE_JOB,
  GUIDE_BEFORE_A_RUN,
  GUIDE_PLAN,
  GUIDE_DRONE,
  GUIDE_WORKFLOW,
};

/**
 * The numbers no guide may take. **A retired guide keeps its number.**
 *
 * 11 explained the second edge a group gets on the Workflow canvas. The plan's
 * graph moved to the Plan tab on 25 September 2026, so the guide had nothing
 * left to explain and was retired rather than rewritten.
 */
export const RETIRED_GUIDE_NUMBERS: readonly number[] = [11];

export const GUIDES: readonly Guide[] = [
  GUIDE_JOB,
  GUIDE_DISPATCH,
  GUIDE_BEFORE_A_RUN,
  GUIDE_COMPLETION,
  GUIDE_MEMBER_LINK,
  GUIDE_PLAN,
  GUIDE_GROUP_ORDER,
  GUIDE_GROUP_BOUNDARY,
  GUIDE_PLAN_ASKS,
  GUIDE_TIERS,
  GUIDE_DRONE,
  GUIDE_STEP_BAR,
  GUIDE_ALWAYS_LOOKS,
  GUIDE_CRITERIA,
  GUIDE_WORKFLOW,
  GUIDE_PROCESSES,
  GUIDE_WORKTREE_SIZE,
  GUIDE_LOOK,
];

/** The guides filed under one group, in catalogue order. */
export function guidesIn(group: GuideGroupId): readonly Guide[] {
  return GUIDES.filter((guide) => guide.group === group);
}

/** The guide explaining a piece, or nothing where no guide claims it. */
export function guideForPiece(piece: string): Guide | undefined {
  return GUIDES.find((guide) => guide.piece === piece);
}
