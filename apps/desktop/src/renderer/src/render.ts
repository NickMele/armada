// Which render a Job takes, and the one reason it takes the dead-end one.
//
// Split out of `JobDetail.tsx` so that the three renders, the acts and the
// screen that chooses between them can each import this without importing one
// another. The choice is the one thing all three agree on, so it lives where
// none of them owns it.

import { ESCALATION_REASON, JOB_LIFECYCLE, JOB_STATUS } from "../../shared/generated/vocabulary";
import type { JobSummary } from "../../shared/protocol";

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
 * The renders that hold the Drone's turns as a section of their own record.
 *
 * **Every one of them, since job detail became one arrangement.** It held two —
 * the finished and stopped renders folded the turns into a record and the other
 * two sent you to a screen of your own. That was the fifth arrangement job
 * detail had, and it is gone: the record region is in the same place on a
 * running Job as on a dead one, and the step's activity log is in the panel.
 *
 * **Two things follow from it, and they are in two files, which is why it is in
 * neither.** The header does not offer "Watch the turns", because one route to
 * a thing is enough; and the board does not put a turns screen up when the
 * socket opens, because the socket being open means a tab was pressed on the
 * page a person is reading. A screen that swapped away under them is the
 * surface this app was built to escape.
 */
export const RECORDS_ITS_OWN_TURNS: ReadonlySet<Render> = new Set<Render>([
  "working",
  "reviewing",
  "finished",
  "stopped",
]);

export function renderFor(job: JobSummary): Render {
  const base = JOB_STATUS[job.status];
  const life = JOB_LIFECYCLE[job.status];
  if (base === undefined || life === undefined) return "unrenderable";
  if (escalation(job) !== undefined) return "stopped";
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
