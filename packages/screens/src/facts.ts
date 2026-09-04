// The header's field run on job detail: four facts on one line, and a fifth
// on the few Jobs that have one.
//
// It is here rather than in `JobDetail.tsx` because it is one job — turning a
// served Job into a list of labelled facts — and the screen beside it is
// another. Nothing in this file decides how a fact looks; `JobDetailField` is
// the design system's shape and this only fills it in.
//
// # Four, and it used to be ten
//
// The workflow, the branch, how long it has been alive and what it has spent.
// A fifth joins them the moment Fleet opens a pull request, and says what
// became of it once somebody has merged or closed it.
// Step, Manifest, Model, Origin, Urgency, Drone, Scope and the write-scope
// overlap were all in here as well, over two lines, and none of them is in the
// drawing's header. The ones worth reaching are under *Where things are*, which
// is the region for a value you want rather than one you are reading — and the
// step is the panel's whole subject, so naming it again above it said the same
// thing twice.
//
// **The branch is bare.** `Branch armada/01K…` was a label in front of a
// 26-character identifier; the branch is the one value here that reads as
// itself, so it takes the line the drawing gives it and no label.
//
// # The pull request is one fact, and what became of it continues it
//
// `Pull request #4711` while it is open, `Pull request #4711, merged` once
// somebody has taken it. Two facts with the run's gap between them would read
// as two things to know; they are one thing — what the branch came to — said
// to whatever depth the record can say it, so the second continues the first.
//
// **A Job with no pull request draws neither**, and that is most Jobs: one
// still running, one in a repository with no remote, one that stopped before
// it delivered. The rule `branchFact` and `landedFact` already keep.

import type { JobDetailField } from "@armada/components";

import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { WorkflowSummary } from "@armada/protocol";
import { span } from "./duration";
import { leading } from "./reading";
import { LANDED } from "./Row";

/**
 * The run, in the order the drawing runs it: what workflow this is, what branch
 * it writes to, how long it has been alive, and what it has cost.
 *
 * Elapsed is read off the row rather than off the detail. Both carry it and
 * cannot disagree — the detail's is built from the same record — and the row is
 * already in hand, so the fact is there on the first frame instead of appearing
 * when `GET /jobs/:job_id` lands.
 */
export function factsOf(
  job: JobSummary,
  whole: JobWhole | null,
  workflow: WorkflowSummary | undefined,
  now: number,
): JobDetailField[] {
  const address = whole?.delivery?.pull_request;
  return [
    ...workflowFact(job, workflow),
    ...branchFact(job),
    ...pullRequestFact(address),
    ...landedFact(whole, address !== undefined),
    ...elapsedFact(job, now),
    ...spendFact(whole),
  ];
}

/**
 * Which workflow this Job is running, as a phrase rather than a labelled value:
 * `Bug workflow`, not `Workflow bug`. The name where Fleet holds it, the id
 * where it does not — which after the refusal at creation means a Job older
 * than the check.
 */
function workflowFact(job: JobSummary, workflow: WorkflowSummary | undefined): JobDetailField[] {
  if (workflow === undefined) {
    return [{ value: job.workflow_id, mono: true, suffix: " workflow", copyValue: job.workflow_id }];
  }
  return [{ value: workflow.name, suffix: " workflow", copyValue: job.workflow_id }];
}

/**
 * The branch, unlabelled, or the reason there is none. **Absent is a fact
 * here**, not a blank: a Job at the approval gate has no worktree, and saying
 * so is different from leaving the value out and letting somebody wonder.
 */
function branchFact(job: JobSummary): JobDetailField[] {
  if (job.branch === undefined) return [{ label: "No branch yet" }];
  return [{ value: job.branch, mono: true, copyValue: job.branch }];
}

/**
 * The pull request Fleet opened, by its number, clickable.
 *
 * **News from the moment it exists.** Every Job that finishes is in this state
 * — open, waiting on a reviewer — so a fact that waited for a merge would be
 * absent exactly when a person is looking for it, which is what sent the owner
 * to the forge to find a branch by hand. `#422`.
 *
 * **The number, never the address.** A forge address is sixty characters of
 * which a person reads four, and the run is a line of short readings. The whole
 * of it is on the link's `title`, which is where somebody who wants to copy it
 * can still get at it.
 *
 * **The number is read off the address rather than served.** Nothing on the
 * wire carries it: `JobDelivery` has the address and no id beside it, so the
 * choice was to parse or to draw something longer. A forge that numbers its
 * pull requests some other way falls back to the words alone, still linked —
 * see `pullRequestNumber`.
 */
function pullRequestFact(address: string | undefined): JobDetailField[] {
  if (address === undefined) return [];
  const number = pullRequestNumber(address);
  if (number === null) return [{ value: "Pull request", href: address }];
  return [{ label: "Pull request", value: number, mono: true, href: address }];
}

/**
 * The number out of a pull request address, or `null` where there is not one.
 *
 * **The last all-digit segment of the path, and that is the whole rule.**
 * `…/pull/4711` and `…/-/merge_requests/12` both answer, without this file
 * holding a list of forges — a roster of URL shapes would be a second statement
 * of something Fleet already resolved, and it would be wrong for the first
 * forge nobody thought of. Last rather than first, so an organisation or a
 * repository named in digits does not win over the number at the end.
 *
 * **The query and the fragment are cut before anything is read, and that is
 * not tidying.** A forge address that arrives with a line anchor on it ends
 * `#3000`, which is all digits and sits after the number — so reading them as
 * segments draws a line number where the pull request goes.
 *
 * `null` is a real answer and not a failure: a forge that addresses a pull
 * request by a slug is a forge whose pull requests have no number, and the fact
 * draws the words alone rather than inventing one.
 */
export function pullRequestNumber(address: string): string | null {
  const last = (address.split(/[?#]/)[0] ?? "")
    .split("/")
    .filter((part) => /^\d+$/.test(part))
    .at(-1);
  return last === undefined ? null : `#${last}`;
}

/**
 * What became of that pull request — continuing the fact that names it, where
 * there is one to continue.
 *
 * **Absent on nearly every Job, which is why it is beside the branch and not a
 * fact of its own line.** It appears the moment there is something to say and
 * takes no room until then: a Job with no remote, one still running, and one
 * whose pull request nobody has merged yet all draw nothing here, because
 * "nobody has merged it yet" is the state a pull request is in from the moment
 * it exists and is not news about this Job. The address above it is.
 *
 * **`continues` where the address is drawn, standalone where it is not.** With
 * one, this is the second half of a sentence and reads mid-line: `Pull request
 * #4711, merged`. Without one — a Job old enough that Fleet recorded the
 * verdict and not the address — it opens a fact of its own and takes a capital.
 */
function landedFact(whole: JobWhole | null, linked: boolean): JobDetailField[] {
  const landed = LANDED[whole?.delivery?.landed ?? ""];
  if (landed === undefined) return [];
  return linked ? [{ label: landed, continues: true }] : [{ label: leading(landed) }];
}

/** How long the Job has been alive, from `created_at`. */
function elapsedFact(job: JobSummary, now: number): JobDetailField[] {
  const alive = span(job.created_at, now);
  return alive === null ? [] : [{ label: "Elapsed", value: alive, mono: true }];
}

/**
 * What the Job has cost so far.
 *
 * **Hedged, always.** The design contract spells an estimated value `~$2.40`
 * and never `$2.40`, and the figure is notional besides — it is what the run
 * would have cost at list price, which is not what a subscription account is
 * billed. Absent on a Fleet that does not count, which draws nothing rather
 * than a zero: a Job that cost nothing and a Fleet with no figure are two
 * different facts.
 */
function spendFact(whole: JobWhole | null): JobDetailField[] {
  const spend = whole?.spend;
  if (spend === undefined) return [];
  return [{ label: "Spend", value: money(spend.cost_micros), mono: true }];
}

/**
 * A spend, from millionths of a dollar. **Four places under a cent**, so a Job
 * that has cost a tenth of a penny does not round to `~$0.00` and read as free.
 */
export function money(micros: number): string {
  const dollars = micros / 1_000_000;
  return `~$${dollars.toFixed(dollars > 0 && dollars < CENT ? 4 : 2)}`;
}

/** A cent, in dollars. Below this two decimal places round to nothing. */
const CENT = 0.01;

/** The steps in the order they were frozen. `ordinal` is the order, not the array. */
export function ordered(whole: JobWhole | null): StepDetail[] {
  return whole === null ? [] : [...whole.steps].sort((a, b) => a.ordinal - b.ordinal);
}

/**
 * The rows of a step's own list — `check_runs` or `judged` — that belong to
 * its current attempt.
 *
 * **Since 7.0 both lists hold every attempt's rows, oldest first.** A screen
 * reading either as "where does this step stand right now" has to narrow to
 * the live attempt first — the current attempt is the last one `attempts`
 * names, or every row where nothing has run yet.
 */
export function onlyCurrentAttempt<T extends { attempt: number }>(
  step: StepDetail,
  rows: T[],
): T[] {
  const attempt = step.attempts.at(-1)?.attempt;
  return attempt === undefined ? rows : rows.filter((row) => row.attempt === attempt);
}
