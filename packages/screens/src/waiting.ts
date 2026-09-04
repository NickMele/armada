// When a job *starts* waiting on a person, and what one notification says about
// the ones that just did.
//
// **The arithmetic only. Nothing here posts anything** — `Notification`, the
// dock and the window live in `apps/desktop/src/main/telling.ts`, and this is
// what that file is a shell around, so the rules below are unit-tested in node
// rather than inferred from a screenshot of a banner.
//
// # Entering the set is the event, and being in it is not
//
// `needsYou` says which jobs are waiting on a person right now. The thing worth
// interrupting somebody for is a job that was not in that set and now is — so
// what is held is the set of ids, and every reading is a diff against it.
//
// **A job already waiting when Bridge started is not news.** It is why the set
// is filled by `waitingIn` before anything is diffed: the first reading tells
// nobody, and every reading after it is a diff. Without that, opening Bridge on
// a Monday would post one notification per job that had sat there all weekend —
// which is the failure below, on the worst possible morning.
//
// **A job that leaves and comes back is news again.** `awaiting_review` →
// changes requested → `running` → back to `awaiting_review` is a second thing
// to look at, not a repeat of the first.
//
// # Five at once is one notification's worth of attention
//
// **Notifications that cry wolf get turned off, permanently, by the person they
// were built for**, and the cheapest way to cry wolf is to be correct five
// times in one second. A dispatch of five jobs that all reach their review gate
// together is one event to a person and five to the machine.
//
// So entries are collected and told once. This module says what the one
// telling *is*; `telling.ts` holds the interval it is collected over. A batch
// of one names the job. A batch of more says how many and lists them, in the
// order the Board would list them — oldest first, because the thing waiting
// longest is the thing most likely to have gone wrong.

import type { JobSummary } from "@armada/protocol";
import { instant } from "./duration";
import { needsYou, needsYouClause, oldest } from "./needs-you";

/** The ids waiting on a person, in a reading. */
export function waitingIn(jobs: readonly JobSummary[]): Set<string> {
  return new Set(jobs.filter(needsYou).map((job) => job.id));
}

/**
 * The jobs that were not waiting on a person and now are.
 *
 * `held` is what the last reading left. A job absent from `jobs` altogether —
 * forgotten, or gone from a resync — is simply not in the answer and drops out
 * of the next `held`, which is what stops a cleared board notifying about
 * anything.
 */
export function entering(
  held: ReadonlySet<string>,
  jobs: readonly JobSummary[],
): JobSummary[] {
  return jobs.filter((job) => needsYou(job) && !held.has(job.id));
}

/** How many notifications the body of one names before it says "and 3 more". */
const NAMED = 3;

/** One notification: what it says, and where pressing it lands. */
export type Telling = {
  title: string;
  body: string;
  /**
   * The one job it is about, or `null` where it is about several.
   *
   * **`null` is not "nowhere".** A telling about four jobs cannot open one of
   * them without picking for somebody, so it opens the set instead — which is
   * the Needs-you tab, the same set this module derived. The host decides how
   * to land there; what is decided here is that there is no single job to land
   * on.
   */
  jobId: string | null;
};

/**
 * What to say about a batch of jobs that just started waiting, or `null` for a
 * batch of none.
 *
 * **Identity and verb first, then location, then elapsed** — the register the
 * design system sets for an alert, on the reasoning that a notification's line
 * gets cut and what survives has to be worth having survived.
 *
 * `now` is passed rather than read. A module that calls the clock cannot be
 * replayed, and the elapsed figure is the one thing here that moves on its own.
 */
export function telling(entered: readonly JobSummary[], now: number): Telling | null {
  if (entered.length === 0) return null;
  const ordered = [...entered].sort(oldest);
  const first = ordered[0]!;
  if (ordered.length === 1) {
    return { title: `${quoted(first.title)} needs you.`, body: whereItIs(first, now), jobId: first.id };
  }
  return {
    title: needsYouClause(ordered.length),
    // One job a line, because a notification body wraps and a comma-separated
    // run of titles is one paragraph nobody finishes reading.
    body: [
      ...ordered.slice(0, NAMED).map((job) => job.title),
      ...(ordered.length > NAMED ? [`and ${ordered.length - NAMED} more`] : []),
    ].join("\n"),
    jobId: null,
  };
}

/**
 * Where the job is and how long it has been going, for the second line.
 *
 * **Every part is dropped rather than filled in.** A step nothing named and a
 * date that will not parse each leave the line shorter; neither invents a
 * value, because a notification is read once and a wrong figure in it is not
 * corrected by anything.
 */
function whereItIs(job: JobSummary, now: number): string {
  const started = instant(job.created_at);
  return [
    job.id,
    job.current_step_id === undefined ? null : `step ${job.current_step_id}`,
    started === null ? null : `${Math.max(0, Math.round((now - started) / 60000))} min`,
  ]
    .filter((part): part is string => part !== null)
    .join(" · ");
}

/** Curly, because these are quotation marks in a sentence and not code. */
function quoted(text: string): string {
  return `“${text}”`;
}
