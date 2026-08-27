// The list. **Not the Job Board** — that is the Surface milestone's, with its
// own journey and its own component inventory. This is every Job, what it is
// called, its state, the step it is on, and the one decision a person can make
// from here.
//
// # A row is a control, not a div that happens to answer a click
//
// Every drawn row opens that Job's detail, so the frame is a listbox and its
// rows are options: Tab reaches the list once, Up and Down rove within it,
// Enter and Space open a row, and the open one carries `aria-selected` as well
// as the accent fill. A listitem with an `onClick` looks identical and is
// reachable by the mouse alone.
//
// # It was a table, and the table is the thing the row shape replaced
//
// This file built an eight-column `Table` because the Bridge shell was written
// before the compositions existed and nothing went back. That is the shape
// `Job row (stacked)` exists to have retired: the design contract is explicit
// that the Job row is **one shape at every width** — a badge leading, the
// headline beside it, a labelled field run beneath — and that it replaced an
// eight-column table because the Board and Alerts disagreed about what a job
// looks like. Bridge had re-created the thing that was replaced.
//
// So the rows are `JobRowStacked` inside `ActiveJobsList`, and the screen story
// `Screens/The list — six states, one row shape` is what they are measured
// against. Nothing here draws a cell, a column or a border.
//
// # The field run is four of the drawing's five, and the fifth is left out
// rather than drawn empty
//
// The drawing's row is the branch or the workflow, the step bar, the step,
// elapsed, spend. Four of those reach here now.
//
// **Track one switches from the workflow to the branch the moment a worktree
// exists**, which is the drawing's own rule and was unreachable while `branch`
// was a `JobDetail` field: reading it per row would have been one request per
// row, the failure `docs/practices/bridge.md` names first. It is on
// `JobSummary` now, so the switch is a field the row already holds. A Job at
// the approval gate has no worktree and keeps the workflow.
//
// **Elapsed is measured from `created_at`**, also on the summary now. It is the
// track that answers "is this stuck" without opening the Job, which was the
// whole reason it was drawn. It runs to now while the Job is working and stops
// at the Job's own last movement once it is over — a terminal Job whose elapsed
// kept climbing would read as still running.
//
// **Spend stays out of the row entirely.** Nothing measures it — not on the
// wire, not in the store, not computed — and a labelled gap on every row reads
// as a value that failed to load rather than one nothing serves.
//
// **The step reads its name**, since `StepDetail` carries a label Fleet fills
// from the frozen workflow. A list row holds `JobSummary` and not the steps, so
// what it has is `current_step_id` — the id, in mono. The name is on the detail
// one click away, where the rail draws it.
//
// # One state the row shape cannot draw, said out loud
//
// `JobRowStacked` requires a `statusIcon`, "required on every state, from the
// icon registry" — and the registry has states with no glyph. `escalated`
// carries none, and six of the escalation reasons carry none either, so a Job
// Fleet actually produces (`interrupted` aside, which does have one) can reach
// a status this row physically cannot render.
//
// **No glyph is invented for it.** `octagon-alert` is reserved to `stalled`,
// `triangle-alert` is reserved to Doctor, and there is no registry row meaning
// "unspecified". So those Jobs are named beneath the list rather than drawn in
// a second row shape: a missing thing that renders as a finding is a finding,
// and one that is silently absent is a gap nobody sees. Reported against the
// composition, not worked around with a prop only this app would use.
//
// A row the store refused is a different thing and is not here. It is a failure
// with a fault and a log, so `App` draws it as a failure notice — a Job that
// will not render and a Job that will not load are told apart.

import { ActiveJobsList, BoardEmptyState, Button, JobRowStacked, StepBar } from "@armada/components";
import type { JobRowField } from "@armada/components";
import { GitBranch, Layers } from "lucide-react";

import { JOB_LIFECYCLE } from "../../shared/generated/vocabulary";
import type { JobSummary, WorkflowSummary } from "../../shared/protocol";
import { instant, span } from "./duration";
import { readingOf } from "./reading";

/**
 * How many rows are drawn. Nothing renders an unbounded list directly, and no
 * virtualization library has been chosen — so the list is bounded and says what
 * it left out, which is the honest half of the same rule.
 */
const DRAWN = 200;

/**
 * Newest first. A created Job is folded onto whichever end the caller put it
 * — the store's own order is an implementation detail, not a promise — so the
 * list orders itself rather than trusting arrival order to stay right.
 *
 * A Job whose `created_at` will not parse sorts last, not first: a corrupt
 * date must not shove real Jobs off the bounded window above.
 */
export function newestFirst(jobs: readonly JobSummary[]): JobSummary[] {
  return [...jobs].sort((a, b) => {
    const left = instant(a.created_at);
    const right = instant(b.created_at);
    if (left === null) return right === null ? 0 : 1;
    if (right === null) return -1;
    return right - left;
  });
}

export type JobsProps = {
  jobs: readonly JobSummary[];
  /** Jobs with an approval in flight. A second click on one does not dispatch twice. */
  approving: readonly string[];
  /** True while what is shown is not live. Every row reads as de-emphasised. */
  stale: boolean;
  /** Now, injected. Elapsed on a working Job runs to it, so it has to move. */
  now: number;
  /** What Fleet holds, so a row can say `bug` where it carried a ULID. */
  workflows: readonly WorkflowSummary[];
  /** The whole reading of the connection, for the empty state that is a fault. */
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  /** Open a Job. Every row is a control, so every row calls this. */
  onOpen: (jobId: string) => void;
  onApprove: (jobId: string) => void;
  /** A clipboard write is silent, so the surface confirms every one with a toast. */
  onCopied: (value: string) => void;
};

export function Jobs({
  jobs,
  approving,
  stale,
  now,
  workflows,
  disconnected,
  selected,
  onOpen,
  onApprove,
  onCopied,
}: JobsProps) {
  const bounded = newestFirst(jobs).slice(0, DRAWN);
  const drawn = bounded.filter((job) => readingOf(job).as === "badge");
  const undrawable = bounded.filter((job) => readingOf(job).as !== "badge");


  return (
    <div className="flex flex-col gap-2">
      <ActiveJobsList
        // No heading and no summary: the panel head above the list carries
        // both, and the same sentence in two places is two chances to disagree.
        // Every drawn row opens a Job, so the frame is a listbox and its rows
        // are options — which is what lets "this one is open" be a state a
        // screen reader can read rather than a fill only a sighted eye catches.
        selectable
        label="Active jobs"
        empty={
          disconnected === null ? (
            <BoardEmptyState quiet>No jobs. Propose one above.</BoardEmptyState>
          ) : (
            // The two empty states differ because the two situations do: a
            // Fleet that is up with no work is a null result, and one that is
            // not running is a fault Bridge cannot fix.
            <BoardEmptyState command="armada fleet start" note={disconnected}>
              Fleet is not connected, so there is nothing to show.
            </BoardEmptyState>
          )
        }
      >
        {drawn.map((job) => (
          <Row
            key={job.id}
            job={job}
            approving={approving.includes(job.id)}
            stale={stale}
            now={now}
            workflows={workflows}
            selected={job.id === selected}
            onOpen={onOpen}
            onApprove={onApprove}
            onCopied={onCopied}
          />
        ))}
      </ActiveJobsList>

      {jobs.length > bounded.length ? (
        <p className="text-fg-muted">
          {`${bounded.length} of ${jobs.length} jobs drawn. The rest are on Fleet and not on screen.`}
        </p>
      ) : null}

      {/* The registry has no glyph for this state, so the row shape cannot draw
          it and nothing here invents one. Named rather than dropped. */}
      {undrawable.map((job) => {
        const reading = readingOf(job);
        if (reading.as === "badge") return null;
        return (
          <p key={job.id} className="text-fg-muted">
            {`${job.title} — `}
            <span className="mono">{reading.wire}</span>
            {`. The registry carries no ${reading.missing.join(" and no ")} for it, so the row shape cannot draw it.`}
          </p>
        );
      })}
    </div>
  );
}

/**
 * "6 jobs. 1 awaiting approval." Lowercase anything countable.
 *
 * The panel head above the list draws this, and the status bar draws the count
 * on its own — so both read the one function rather than two spellings of the
 * same plural.
 */
export function summaryOf(total: number, atTheGate: number): string {
  const jobs = `${jobCount(total)}.`;
  return atTheGate === 0 ? jobs : `${jobs} ${atTheGate} awaiting approval.`;
}

/** "6 jobs", on its own. */
export function jobCount(total: number): string {
  return `${total} ${total === 1 ? "job" : "jobs"}`;
}

/** How many Jobs are at the approval gate. What the sentence above counts. */
export function atTheGate(jobs: readonly JobSummary[]): number {
  return jobs.filter((job) => job.status === "awaiting_approval").length;
}

function Row({
  job,
  approving,
  stale,
  now,
  workflows,
  selected,
  onOpen,
  onApprove,
  onCopied,
}: {
  job: JobSummary;
  approving: boolean;
  stale: boolean;
  now: number;
  workflows: readonly WorkflowSummary[];
  selected: boolean;
  onOpen: (jobId: string) => void;
  onApprove: (jobId: string) => void;
  onCopied: (value: string) => void;
}) {
  const reading = readingOf(job);
  // Every row reaching here is renderable — the list filtered the rest out and
  // names them, rather than this picking a glyph the registry does not have.
  if (reading.as !== "badge") return null;
  // The gate is the status, not the button. A Job that has moved off
  // `awaiting_approval` has no approve control at all, so the second click has
  // nothing to hit even before the guard in the main process refuses it.
  const waiting = job.status === "awaiting_approval";
  const workflow = workflows.find((held) => held.id === job.workflow_id);
  const steps = workflow?.steps ?? [];
  // Matched on `step_id`, because a workflow's steps are objects carrying their
  // Checks since protocol 3. Compared against the whole step this silently
  // never matched, and every bar drew its first segment as the current one.
  const at = steps.findIndex((step) => step.step_id === job.current_step_id);

  const fields: JobRowField[] = [
    // Track one, the drawing's switch: the branch the moment a worktree exists,
    // and the workflow until then. Two glyphs because they are two different
    // facts — a branch is where the work is and a workflow is what it will do —
    // and the row never draws both, so the track stays one column wide.
    job.branch === undefined
      ? {
          // The workflow's name where Fleet holds it, the id where it does not,
          // which after the refusal at creation means a Job older than the check.
          icon: Layers,
          value:
            workflow === undefined ? job.workflow_id : `${workflow.name}, ${steps.length} steps`,
          mono: workflow === undefined,
          copyValue: job.workflow_id,
        }
      : { icon: GitBranch, value: job.branch, mono: true, copyValue: job.branch },
    {
      // An empty bar, never no bar. A Job at the gate has its ordinals and no
      // progress, and a row that dropped the bar there would read as a workflow
      // with no steps rather than as one that has not started.
      value: (
        <StepBar
          total={Math.max(steps.length, 1)}
          current={at + 1}
          activity={activityOf(job.status)}
          label={
            job.current_step_id === undefined
              ? `Not started, ${steps.length} steps`
              : `Step ${at + 1} of ${steps.length}`
          }
        />
      ),
    },
    // The step's id, because nothing serves its name (#109). `Not started` is
    // the same sentence at the gate and in the queue: the queued row's reason
    // is already the badge's verb, and the status grammar puts it there rather
    // than in the run — so repeating it here would say one thing twice.
    job.current_step_id === undefined
      ? { value: "Not started", quiet: true }
      : { value: job.current_step_id, mono: true, emphasis: true },
  ];

  // Track four, appended rather than always drawn: a Job with no elapsed to
  // state loses the field, for the reason spend is not on the row at all. An
  // empty slot in a shared column reads as a value that failed to load.
  const elapsed = elapsedOf(job, now);
  if (elapsed !== undefined) fields.push({ value: elapsed, mono: true, quiet: true });

  return (
    <JobRowStacked
      onCopied={onCopied}
      onOpen={() => onOpen(job.id)}
      selected={selected}
      status={reading.status}
      statusIcon={reading.icon}
      statusLabel={reading.verb}
      headline={job.title}
      jobId={job.id}
      fields={fields}
      pulsing={job.status === "running" && !stale}
      dimmed={stale}
      action={
        waiting ? (
          // Secondary, and one. **A list row never takes a primary action** —
          // fourteen rows offering a decision would be fourteen accent blocks.
          <Button size="sm" onClick={() => onApprove(job.id)} disabled={approving || stale}>
            {approving ? "Approving" : "Approve dispatch"}
          </Button>
        ) : null
      }
    />
  );
}

/**
 * The step bar's activity, from the Job's status.
 *
 * **Not a second vocabulary**: the four values are `StepBar`'s own prop, and
 * the mapping is the one sentence the status grammar states about a list row —
 * the current segment takes the Job's hue, and everything else is the bar's
 * own. Anything not one of these four leaves the bar unhued, which is what the
 * component does for `killed` and `retrying` everywhere else.
 */
function activityOf(status: string): "running" | "failed" | "advanced" | "killed" | undefined {
  if (status === "running") return "running";
  if (status === "completed_failed") return "failed";
  if (status === "completed_success") return "advanced";
  if (status === "killed") return "killed";
  return undefined;
}

/**
 * How long this Job has been alive.
 *
 * **A working Job runs to now; a Job that is over stops.** `JobSummary` carries
 * no ended-at, so a terminal Job would otherwise keep counting and read as
 * still running. There is nothing on the row to stop it against, so a terminal
 * Job shows no elapsed at all rather than a figure that is wrong every second
 * after it is drawn. Reported: the row wants the instant the Job stopped.
 */
function elapsedOf(job: JobSummary, now: number): string | undefined {
  return JOB_LIFECYCLE[job.status]?.terminal === false
    ? (span(job.created_at, now) ?? undefined)
    : undefined;
}
