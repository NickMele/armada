// One thing a Job is held to, and where it came from. Draft, for
// `crates/ipc/src/detail.rs`.
//
// Source of truth today: `Criterion` on `JobDetail.acceptance_criteria` —
// `criterion_id`, `text` and `source`, where `source` is a `CriterionSource`
// (`check`, `judge` or `attested`, `crates/core-model/src/job/fields.rs`).
//
// **`verified_by`, never a second `source`** (#1532, 22 Sep). The wire's
// `source` already means *how this criterion is answered*, and using the same
// word for *where the words came from* would give one name two meanings on one
// row. So the wire's field is renamed on the way in, and the new fact gets a
// name of its own.

import type { Criterion, JobDetail } from "@armada/protocol";

/** How a criterion is answered. The wire's `CriterionSource`, renamed. */
export type VerifiedBy = "check" | "judge" | "attested";

/**
 * Where the words came from.
 *
 * `issue` carries the reference so a surface can say the issue has moved since
 * the Job froze its words — the Job keeps what it froze (#1530, 22 Sep).
 */
export type CriterionOrigin =
  | { origin: "issue"; ref: string }
  | { origin: "prompt" }
  | { origin: "person" };

/** One acceptance criterion, with its provenance. */
export type CriterionView = {
  /** Absent on a criterion nothing has minted an id for yet. */
  criterion_id?: string;
  text: string;
  verified_by: VerifiedBy;
  origin: CriterionOrigin;
  /**
   * When the issue these words came from was last edited, where that is after
   * the Job froze them. **Absent is the ordinary case** — it is present only
   * to say the source has moved, never to date the freeze.
   */
  origin_moved_at?: string;
};

/**
 * Today's wire carries the text, the id and how it is verified, and says
 * nothing about where the words came from — so every criterion derives as
 * `prompt`, which is where a requester's words are typed.
 */
export function criterionViewOf(criterion: Criterion): CriterionView {
  return {
    criterion_id: criterion.criterion_id,
    text: criterion.text,
    verified_by: verifiedByOf(criterion.source),
    origin: { origin: "prompt" },
  };
}

/** Every criterion a Job is held to, in the order it was given. */
export function criterionViewsOf(detail: JobDetail): CriterionView[] {
  return detail.acceptance_criteria.map(criterionViewOf);
}

// The wire leaves every closed set as `string`, so an unrecognised spelling is
// possible. It reads as `judge` — the one that declines to refuse and never
// grants — rather than as `check`, which would claim something mechanical
// answered a criterion nothing ran for.
function verifiedByOf(source: string): VerifiedBy {
  switch (source) {
    case "check":
    case "attested":
      return source;
    default:
      return "judge";
  }
}
