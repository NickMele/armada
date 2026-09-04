// One Job, as a row on the Board.
//
// Split out of `Jobs.tsx` when that file grew past the gate's 500-line warning,
// the same way `Acts.tsx` and `Head.tsx` came out of the files they were in. It
// is one subject — what a row draws, which verb its state calls for, and what
// the wire does not serve it — and the board's filter set and keyboard beside
// it are another.
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
// The list built an eight-column `Table` because the Bridge shell was written
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
// # The field run is four of the drawing's six, and the two missing are left
// out rather than drawn empty
//
// The drawing's row is the branch or the workflow, the step bar, the step,
// elapsed, spend, origin. Four of those reach here.
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
// **The same track carries when a Job was created, not just how long.** While
// working, elapsed stays the drawn value and the creation time rides along as
// a hover title — the glance-value people scan for stays "is this stuck," and
// the exact instant is a hover away. Once terminal, elapsed has nothing left
// to answer, and that slot — blank until now — draws the creation time itself
// rather than staying empty.
//
// **Spend stays out of the row entirely.** Nothing measures it — not on the
// wire, not in the store, not computed — and a labelled gap on every row reads
// as a value that failed to load rather than one nothing serves.
//
// **Origin stays out too, and for a nearer miss.** `origin` is on `JobSummary`
// and its five values are real; what is missing is the word. `enum-verbs.toml`
// has no `origin` entry, so there is nothing mapping `auto_detected` to `Found
// by Fleet` — and writing the five sentences here would be a second place they
// live and the one nothing checks. The screen stories draw them as literals, so
// **the track is drawn there and empty here**. Issue #234.
//
// **The step reads its name**, since `StepDetail` carries a label Fleet fills
// from the frozen workflow. A list row holds `JobSummary` and not the steps, so
// what it has is `current_step_id` — the id, in mono. The name is on the detail
// one click away, where the rail draws it.

import { Button, JobRowStacked, SplitButton, StepBar } from "@armada/components";
import type { JobRowField } from "@armada/components";
import { GitBranch, GitPullRequest, Layers } from "lucide-react";

import { JOB_LIFECYCLE } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import type { WorkflowSummary } from "@armada/protocol";
import { absoluteOf, span } from "./duration";
import { activityFor } from "./frozen";
import { ROW_VERBS, verbOf } from "./keys";
import { leading, readingOf } from "./reading";

/** Whether the Job is over, from the registry that says so. */
export function isTerminal(job: JobSummary): boolean {
  return JOB_LIFECYCLE[job.status]?.terminal === true;
}

/**
 * What a settled pull request reads as. **Written here rather than generated**,
 * unlike every status verb: `Settled` is a wire set of `crates/ipc`'s and not a
 * row in `crates/core-model/domain/`, so `enum-verbs.toml` has nothing to say
 * about it. A registry row would be the better home the day a third state
 * exists, and there is no third state — a pull request merges or it does not.
 *
 * The same two words are used on the detail, imported from here rather than
 * written twice.
 *
 * **Spelled mid-sentence, and capitalised by whoever opens a line with it.**
 * The detail continues the pull request's own fact with this — `Pull request
 * #4711, merged` — and this row opens a field with it. `leading` is what turns
 * one reading into the other, exactly as it does for the registry's verbs,
 * which are spelled lowercase for the same reason. The alternative was a second
 * roster of the same two words in mid-sentence case, which is two spellings of
 * a set that has one owner.
 */
export const LANDED: Record<string, string | undefined> = {
  merged: "merged",
  closed_unmerged: "closed without merging",
};

export function Row({
  job,
  headline,
  stale,
  now,
  workflows,
  selected,
  focused,
  onOpen,
  onKill,
  onCopied,
}: {
  job: JobSummary;
  /** The title, plus which dispatch of the work this is where there is more than one. */
  headline: string;
  stale: boolean;
  now: number;
  workflows: readonly WorkflowSummary[];
  selected: boolean;
  /** The cursor is on this row, so this row draws its key. */
  focused: boolean;
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  onCopied: (value: string) => void;
}) {
  const reading = readingOf(job);
  // Every row reaching here is renderable — the list filtered the rest out and
  // names them, rather than this picking a glyph the registry does not have.
  if (reading.as !== "badge") return null;
  // **The row's one control names the act its state calls for, and opens the
  // Job.** Nothing on the Board approves, attests or redirects: approval is a
  // second act from detail by rule, a redirect is offered on `stuck.recourse`
  // which is a `JobDetail` field a list row has never read, and nothing serves
  // an attestation at all. So the verb says why you are being sent to detail,
  // which is exactly what Review has always meant on an `awaiting_review` row.
  //
  // It replaced `Approve dispatch`, which contradicted the rule the whole time
  // it was on screen — settled 2026-08-31, and `a` left the keyboard map with
  // it. See `docs/concepts/job-board.md`, Dispatch flow.
  const verb = verbOf(job, isTerminal(job));
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
          // The Job's status, one level down. The rail reads the same mapping
          // for a terminal Job — a row and a rail saying different things about
          // one step is the drift keeping it in two files would produce.
          // Anything the mapping does not name leaves the bar unhued, which is
          // what the component does for `killed` and `retrying` everywhere else.
          activity={activityFor(job.status)}
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

  // **Did this land**, appended rather than always drawn, for track four's
  // reason. It is absent on every Job that has not opened a pull request and
  // on every one nobody has merged yet — which are one absence on the wire,
  // because neither is news. Drawing "not merged yet" on every finished row
  // would put a label on the state a pull request is in from the moment it
  // exists.
  //
  // `git-pull-request` for exactly what the icon registry reserves it for.
  // The word carries the fact and the glyph says which fact it is about; the
  // address is on the detail, where a click can reach it.
  const landed = LANDED[job.landed ?? ""];
  if (landed !== undefined) {
    fields.push({ icon: GitPullRequest, value: leading(landed) });
  }

  // Track four, appended rather than always drawn: a Job with neither an
  // elapsed reading nor a readable `created_at` loses the field, for the
  // reason spend is not on the row at all. An empty slot in a shared column
  // reads as a value that failed to load.
  //
  // While working, the track keeps answering "is this stuck" — elapsed stays
  // the value — with the Job's actual creation time on hover, since that is
  // the one instant elapsed cannot show. Once terminal there is no more
  // elapsed to climb, and the slot that left blank now carries the same
  // creation time outright rather than nothing.
  const elapsed = elapsedOf(job, now);
  const created = absoluteOf(job.created_at) ?? undefined;
  if (elapsed !== undefined) {
    fields.push({
      value: <span title={created ? `Created ${created}` : undefined}>{elapsed}</span>,
      mono: true,
      quiet: true,
    });
  } else if (created !== undefined) {
    fields.push({ value: created, mono: true, quiet: true });
  }

  return (
    <JobRowStacked
      onCopied={onCopied}
      onOpen={() => onOpen(job.id)}
      selected={selected}
      status={reading.status}
      statusIcon={reading.icon}
      statusLabel={reading.verb}
      headline={headline}
      jobId={job.id}
      fields={fields}
      // **Every running row, and the row applies the ceiling.** Two Jobs run
      // at once now, so this alone would breathe twice on one board — which
      // Motion forbids and then names: the pulse rides the focused row. The
      // row knows where the cursor is and this does not, so the rule is its.
      pulsing={job.status === "running" && !stale}
      dimmed={stale}
      focused={focused || undefined}
      action={
        // **One control, and it is secondary.** A list row never takes a
        // primary action — fourteen rows offering a decision would be fourteen
        // accent blocks. Kill is in the menu rather than beside it, because two
        // buttons on a row is two controls whatever they are called, and the
        // menu is where the drawing already put it.
        //
        // A job that is over gets the plain button: there is nothing to kill,
        // and `Split button` draws its caret whether or not the menu has
        // anything in it — so a caret over an empty menu is a control that does
        // not respond.
        isTerminal(job) ? (
          <Button size="sm" onClick={() => onOpen(job.id)} disabled={stale}>
            {ROW_VERBS[verb].label}
          </Button>
        ) : (
          <SplitButton
            ground="card"
            disabled={stale}
            onAction={() => onOpen(job.id)}
            menuLabel={`More for ${job.title}`}
            // The binding is displayed here and bound in `keys.ts`, which is
            // the only way a person finds `x` without reading a contract.
            items={[{ label: "Kill", shortcut: "x", danger: true, onSelect: () => onKill(job.id) }]}
          >
            {ROW_VERBS[verb].label}
          </SplitButton>
        )
      }
      // The key that fires the verb, drawn on the cursor's row only. The
      // component holds that rule; this only says which key.
      actionKey={ROW_VERBS[verb].key}
    />
  );
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
