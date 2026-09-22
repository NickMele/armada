// Plan — how the work is split, read before anybody approves it.
//
// **The split is the reading, not the task list.** Job 3's plan ran past a
// Judge that had refused it because nothing on screen said which tasks ran
// together, which agent each got, or what would be run once they stopped. So
// the card is the group, the boundary is under it, and what a Drone will be
// told is one press away.
//
// What this file holds is the open state of one reading — which task the
// inspector is on. What the board says is `tab-plan-read.ts`, and what it
// looks like is `PlanBoard`. Changing a plan is `#1552` and is not here.

import { Button, PlanBoard, PlanTaskSheet } from "@armada/components";
import { useState } from "react";

import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { Eyebrow } from "./InsideAJob";
import { RedirectControl } from "./Redirect";
import {
  casesOf,
  criteriaOf,
  groupsOf,
  planBoardOf,
  taskSheetOf,
  touchedByOf,
} from "./tab-plan-read";
import type { JobDraft } from "./draft/held";
import type { CriterionView } from "./draft/criterion";

/** The `DeclaredCheck.kind` a step recording a plan declares. `plan.ts`'s own read. */
const PLAN_RECORDED = "plan_recorded";

export type PlanTabProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** What this moment's boards draw that Fleet cannot serve yet. */
  draft?: JobDraft;
  /** The window is at `--window-floor`, so the inspector goes flush. */
  floor: boolean;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  /** An act on this Job is already out. */
  acting: boolean;
  deciding: boolean;
  onApproveReview: (jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
};

/**
 * Which step recorded the plan, where one did. **Read off the declared checks,
 * never the workflow's name** — `plan.ts`'s rule, and `#1006` lets any step
 * declare `plan_recorded`.
 */
function planStepOf(whole: JobWhole | null): StepDetail | undefined {
  return whole?.steps.find((step) =>
    (step.checks ?? []).some((check) => check.kind === PLAN_RECORDED),
  );
}

/** Where a criterion's words came from, as one line. */
function originSaid(criterion: CriterionView): string {
  const from =
    criterion.origin.origin === "issue"
      ? `from ${criterion.origin.ref}`
      : criterion.origin.origin === "person"
        ? "written by you"
        : "from the prompt";
  return `${from} · answered by the ${criterion.verified_by}`;
}

/**
 * What the Job is held to, with where each line came from.
 *
 * **Not the planner's expectation**, which is per task and sits in the
 * inspector. `#1274` is why the two are never one list: a Drone never chooses
 * the cases it is held to, and a task's `expects` is the planner's word.
 */
function HeldTo({ criteria }: { criteria: readonly CriterionView[] }) {
  if (criteria.length === 0) return null;
  return (
    <section className="armada-detail-tab__region" aria-label="What this Job is held to">
      <Eyebrow>What this Job is held to</Eyebrow>
      <ul className="armada-plan-tab__criteria">
        {criteria.map((criterion, at) => (
          <li key={criterion.criterion_id ?? at}>
            <span className="armada-plan-tab__criterion-text">{criterion.text}</span>
            <span className="armada-plan-tab__criterion-origin">{originSaid(criterion)}</span>
            {/* The Job keeps the words it froze and says the issue has moved
                since — `#1530`, 22 Sep. It never re-reads them. */}
            {criterion.origin_moved_at === undefined ? null : (
              <span className="armada-plan-tab__criterion-moved">
                The issue has been edited since these words were frozen.
              </span>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * The two acts at a plan gate. **Approve it, or tell the Drone that wrote it
 * what to change** — the owner's call of 22 Sep 2026. Changing the plan
 * without sending it back is `#1552`.
 *
 * Drawn only while the step that recorded the plan is waiting on a person, so
 * a Job already implementing offers nothing here to press.
 */
function PlanGate({
  job,
  step,
  stale,
  acting,
  deciding,
  onApproveReview,
  onRedirect,
}: {
  job: JobSummary;
  step: StepDetail | undefined;
  stale: boolean;
  acting: boolean;
  deciding: boolean;
  onApproveReview: (jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
}) {
  if (step === undefined || step.state !== "awaiting_human") return null;
  return (
    <section className="armada-detail-tab__region" aria-label="This plan is waiting on you">
      <Eyebrow>This plan is waiting on you</Eyebrow>
      <div className="armada-plan-tab__acts">
        <Button
          variant="primary"
          size="sm"
          disabled={stale || deciding}
          pending={deciding}
          onClick={() => onApproveReview(job.id)}
        >
          Approve the plan
        </Button>
        <RedirectControl
          jobId={job.id}
          drone="holding"
          disabled={stale || acting}
          onRedirect={onRedirect}
        />
      </div>
    </section>
  );
}

export function PlanTab({
  job,
  whole,
  draft,
  floor,
  stale,
  acting,
  deciding,
  onApproveReview,
  onRedirect,
}: PlanTabProps) {
  // Which task the inspector is on. **This tab's own state, not the screen's**
  // — the sheet is contained by the destination, so a reader who leaves and
  // comes back lands on the board rather than inside one task.
  const [openTask, setOpenTask] = useState<string | null>(null);

  const groups = groupsOf(whole, draft);
  const cases = casesOf(whole, draft);
  const touchedBy = touchedByOf(groups);
  const board = planBoardOf(whole, draft, setOpenTask, openTask ?? undefined);
  const step = planStepOf(whole);
  const reading = openTask === null ? undefined : taskSheetOf(openTask, groups, cases, touchedBy);

  return (
    /* The sheet is the panel's sibling and not its child: the panel is the box
       that scrolls, and a layer positioned inside it slides out of the window
       with the reading — the defect `.armada-screen__detail` already carries
       the whole argument for. */
    <>
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.plan}>
      {board === undefined ? (
        <p className="armada-inside__absent" role="note">
          {step === undefined
            ? "This workflow records no plan, so there is no split to read."
            : `No plan yet — ${step.label} records it.`}
        </p>
      ) : (
        <PlanBoard {...board} />
      )}
      <HeldTo criteria={criteriaOf(whole, draft)} />
      <PlanGate
        job={job}
        step={step}
        stale={stale}
        acting={acting}
        deciding={deciding}
        onApproveReview={onApproveReview}
        onRedirect={onRedirect}
      />
    </div>
    {reading === undefined ? null : (
      <PlanTaskSheet {...reading} open floor={floor} onClose={() => setOpenTask(null)} />
    )}
    </>
  );
}
