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
// A fifth joins them where somebody has merged or closed the pull request,
// which is the one question a finished Job gets asked and is absent on every
// Job that has not reached one.
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

import type { JobDetailField } from "@armada/components";

import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { WorkflowSummary } from "@armada/protocol";
import { span } from "./duration";
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
  return [
    ...workflowFact(job, workflow),
    ...branchFact(job),
    ...landedFact(whole),
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
 * Whether the pull request landed, beside the branch it was opened from.
 *
 * **Absent on nearly every Job, which is why it is beside the branch and not a
 * fact of its own line.** It appears the moment there is something to say and
 * takes no room until then — a Job with no remote, one still running, and one
 * whose pull request nobody has merged yet all draw nothing here, because
 * "nobody has merged it yet" is the state a pull request is in from the moment
 * it exists and is not news about this Job.
 *
 * The address itself is not drawn. `JobDelivery.pull_request` is what a person
 * clicks and there is nowhere in a field run for a link; this says which of the
 * two things happened, and the *did this land* question is answered by the word
 * alone. Reported: the header wants the pull request as a link.
 */
function landedFact(whole: JobWhole | null): JobDetailField[] {
  const landed = LANDED[whole?.delivery?.landed ?? ""];
  return landed === undefined ? [] : [{ label: landed }];
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
