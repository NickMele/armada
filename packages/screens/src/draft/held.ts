// What one Job's boards draw that the wire cannot carry, held together.
//
// **This is the only file here that is not a wire shape**, and it names no
// `crates/ipc` module because it is not meant for one. It is the carrier: the
// mock puts a moment's draft on it, the real Fleet puts nothing, and a board
// that is handed nothing derives what it can from `JobDetail` instead. So the
// prop exists at both ends of `#1545` and only its source moves.
//
// **It stops at the renderer.** `apps/desktop/src/renderer/src/App.tsx` reads
// it by this path, never through `@armada/screens`, so the rule that keeps the
// drafts out of the main process can still see every reach for one.

import type { CaseView } from "./cases";
import type { CriterionView } from "./criterion";
import type { GroupView } from "./group";
import type { LandingRule } from "./landing";
import type { ProposalView } from "./proposal";

/**
 * A Job's draft reading. **Every field is absent by default**, and absent
 * means this Job says nothing about that — never zero, and never a board
 * drawing an empty state it was not told to draw.
 */
export type JobDraft = {
  /** The plan, as groups of tasks. The Plan board's whole input. */
  groups?: readonly GroupView[];
  /** The cases the plan owes, and what became of each. */
  cases?: readonly CaseView[];
  /** What the Job is held to, with where each criterion's words came from. */
  criteria?: readonly CriterionView[];
  /**
   * The proposal this Job is at, where it is still at one.
   *
   * **The same shape the dispatch form holds one moment earlier** — a title,
   * a gate per step, the tier map and the two caps — and its `approved_at` is
   * the whole difference between "yours to change" and "frozen at the press".
   */
  proposal?: ProposalView;
  /** How the work reaches the repository. Set here, frozen with everything else. */
  landing?: LandingRule;
};
