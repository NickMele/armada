// Which render a Job takes, and the one reason it takes the dead-end one.
//
// Split out of `JobDetail.tsx` so that the three renders, the acts and the
// screen that chooses between them can each import this without importing one
// another. The choice is the one thing all three agree on, so it lives where
// none of them owns it.

import { ESCALATION_REASON, JOB_LIFECYCLE, JOB_STATUS } from "@armada/components";
import type { JobSummary } from "@armada/protocol";

/**
 * Which render a Job takes. Four, because a Job waiting on a person to take its
 * work is a fourth thing to draw.
 *
 * **The choice reads the registries, not a list of statuses typed here.**
 * `job-statuses.toml` says whether a Job is over and what it is doing, and
 * `enum-verbs.toml` says which token a status carries — both arrive through
 * the generated module. A Job that stopped and asked takes the dead-end render
 * whatever its status says, because that screen is the one built to state why
 * something stopped and where the work was left.
 *
 * **`reviewing` is decided before `working`, and that is the whole of it.**
 * `awaiting_review` is non-terminal, so without this it takes the running
 * render — a live rail and a per-step elapsed on a Job that is stopped and
 * waiting on a person, answering a question nobody is asking. And the finished
 * render is reached only once the Job is over, by which point all three review
 * acts are refused: the decision has to be drawn on the status it is legal on.
 */
export type Render = "working" | "reviewing" | "finished" | "stopped" | "unrenderable";

/**
 * The token the one successful terminal status carries. Named once: rename it
 * in `enum-verbs.toml` and a finished Job falls to the dead-end render, which
 * is visible rather than silent.
 */
const SUCCEEDED = "--status-completed-success";

/**
 * The token the one status a review is legal on carries. Named the same way and
 * for the same reason: rename it in `enum-verbs.toml` and the review surface
 * falls back to the running render, which is visible rather than silent.
 *
 * **Read off the token rather than `JOB_LIFECYCLE.mode`**, which says `Waited
 * on` for the approval gate too — and those two gates are at opposite ends of a
 * Job, with different acts and different consequences.
 */
const AT_REVIEW = "--status-awaiting-review";

/**
 * The status a spent retry budget holds at. **A wire value, not a token** —
 * see the rule that reads it.
 */
const HELD_FOR_REPAIR = "awaiting_repair";

export function renderFor(job: JobSummary): Render {
  const base = JOB_STATUS[job.status];
  const life = JOB_LIFECYCLE[job.status];
  if (base === undefined || life === undefined) return "unrenderable";
  // An escalated Job has stopped and is waiting on a person, whatever its
  // reason says. **`escalated` has its own verb and glyph** —
  // `docs/contracts/iconography.md`, "escalated's reasons — the same
  // handover, one status over" — and a reason's replaces both only where
  // one is both present and known to this build's `ESCALATION_REASON`.
  // Where it is absent, or names a spelling this build has no row for, the
  // status's own badge still stands: not every surface is served a reason
  // (the same doc section's held-worktrees example), and a blank cannot be
  // told from a finished Job. Falling through to `working` would still be
  // wrong — a live rail and a running clock over a Job that has stopped —
  // which is the one thing this status must never draw as, known reason or
  // not.
  if (job.status === "escalated") return "stopped";
  // **`awaiting_repair` is the line above one status over**, and it is here
  // rather than a row down because the failure is identical: a step spent its
  // retry budget, the Job is waiting on a person, and falling through to
  // `working` would draw the same live rail and running clock over it. #208.
  //
  // This status has a verb and a glyph of its own — `needs repair`, `wrench`
  // — the same way `escalated` above does, and what stopped the work is on
  // the stopped step's `last_verdict` rather than on the Job's transition,
  // which `escalation` deliberately does not read.
  //
  // Keyed on the status and not on a token, unlike the two rules below: the
  // registry key is the wire value, and a token is a rendering choice that
  // could be renamed by somebody thinking about colour.
  if (job.status === HELD_FOR_REPAIR) return "stopped";
  if (base.statusToken === AT_REVIEW) return "reviewing";
  if (!life.terminal) return "working";
  return base.statusToken === SUCCEEDED ? "finished" : "stopped";
}

/** The escalation reason a Job carries, where the registry has that spelling. */
export function escalation(job: JobSummary) {
  const named = job.reason?.named;
  if (named === undefined || job.status !== "escalated") return undefined;
  return ESCALATION_REASON[named];
}
