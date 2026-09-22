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

import type { CaseRunView, CaseView, ScopeRevisionView } from "./cases";
import type { CriterionView } from "./criterion";
import type { GroupView } from "./group";
import type { LandingRule } from "./landing";
import type { LedgerRow } from "./ledger";
import type { JobMembersView } from "./members";

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
  /** Every run of every case, with who ran it and what became of the run. */
  runs?: readonly CaseRunView[];
  /**
   * How this Job's work reaches the repository, and what completes it. Read by
   * Land, and by the landing-order region for its complete-when line.
   */
  landing?: LandingRule;
  /** The Record's rows. The Land board times a group by the ones inside it. */
  record?: readonly LedgerRow[];
  /**
   * The changes asked of the plan's Drone, in the order they were asked.
   *
   * **The ask, not the edit** (`#1552`). A plan is the Drone's record, so a
   * revision is a request whose answer is the Judge's on the step that
   * recorded it — `draft/revision.ts` pairs the two.
   */
  scope_revisions?: readonly ScopeRevisionView[];
  /**
   * The Jobs landing under this one, in the order they land. **Absent is a Job
   * that says nothing about members** — a board handed none derives what the
   * Board's own rows can answer (`jobMembersOf`), and an ordinary Job has no
   * members at all.
   */
  members?: JobMembersView;
};
