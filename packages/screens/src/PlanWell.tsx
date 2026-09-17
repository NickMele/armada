// The Plan region's well, and the two acts a person takes on it: adding a
// task from the region's own head, and dropping one from the row a cursor or
// the keyboard is on. `#897`; `docs/concepts/plan.md`.
//
// Split out of `InsideAJob.tsx` for `Redirect.tsx`'s own reason — one file
// per control that owns a dialog or a field — now that the well owns two.

import { useState } from "react";
import {
  Button,
  Clamped,
  Dialog,
  Input,
  patternFor,
  StepBar,
  TaskMark,
  Textarea,
  useHaptics,
  type ButtonAnswer,
} from "@armada/components";

import type { Outcome } from "@armada/protocol";
import { Eyebrow, type PlanRegionData, type PlanTaskRow } from "./InsideAJob";
import { ADD_TASK_LABEL, DROP_TASK_LABEL, said } from "./copy";
import type { PlanEditAnswer } from "./plan-edits";

/**
 * What a refusal says, inline. **`said` alone is not enough**: it reads empty
 * for `why: "refused"` on purpose — `copy.ts` draws that one as a failure
 * notice elsewhere — and a dialog that stays open on a refusal has nowhere
 * else to put Fleet's own sentence. `ReportControl` reads the same way.
 */
function refusalSaid(outcome: Outcome): string {
  if (outcome.ok) return "";
  return outcome.why === "refused" ? outcome.error.message : said(outcome);
}

/**
 * The button that opens the add-task dialog, and the dialog itself.
 *
 * **`RedirectControl`'s own shape**, with one difference it does not need:
 * Fleet refuses more here than a blank field — no plan recorded, or `after`
 * naming a task the plan does not hold — so a refusal cannot close the dialog
 * blind the way a redirect does. It stays open, with what was typed, and
 * says why. `ReportControl` is the precedent for that half.
 */
function AddTaskControl({
  onAddTask,
  onSaid,
}: {
  onAddTask: (title: string, detail: string) => Promise<PlanEditAnswer>;
  onSaid?: (sentence: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [detail, setDetail] = useState("");
  const [adding, setAdding] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);

  function close(): void {
    setOpen(false);
    setTitle("");
    setDetail("");
    setRefused(null);
  }

  async function add(): Promise<void> {
    setAdding(true);
    setRefused(null);
    try {
      const answer = await onAddTask(title, detail);
      if (answer.ok) {
        onSaid?.("Added");
        close();
        return;
      }
      setRefused(refusalSaid(answer.outcome));
    } finally {
      setAdding(false);
    }
  }

  return (
    <>
      <button type="button" className="armada-screen__eyebrow-act" onClick={() => setOpen(true)}>
        {ADD_TASK_LABEL}
      </button>
      <Dialog
        open={open}
        tone="neutral"
        title="Add a task to this job's plan?"
        confirmLabel={adding ? "Adding…" : ADD_TASK_LABEL}
        confirmDisabled={title.trim() === "" || adding}
        // Refuses a second press: the confirm is already out, so nothing
        // here abandons it. #1117.
        onCancel={adding ? undefined : close}
        onConfirm={() => void add()}
      >
        {/* One column at the dialog's own rhythm — `--space-4`, `.armada-dialog`'s
            own top-level gap — because a `<p>` carries no margin under this app's
            reset and would otherwise run straight into the field below it. */}
        <div className="armada-plan-add-task-body">
          <p>
            The task goes at the end of the plan. If a drone is working on this job, it&rsquo;s
            told now; otherwise the next drone sees it in the plan.
          </p>
          {/* No `autoFocus`: the dialog's own contract puts initial focus on
              Cancel, and a second claim on it here would only lose to it. */}
          <Input label="Title" value={title} onChange={(event) => setTitle(event.target.value)} />
          <Textarea
            label="Detail — optional"
            rows={3}
            value={detail}
            onChange={(event) => setDetail(event.target.value)}
          />
          {refused === null ? null : <p>{refused}</p>}
        </div>
      </Dialog>
    </>
  );
}

/**
 * One task row, with the drop control a cursor or the keyboard is on.
 *
 * **Offered only on `open` or `working`.** `docs/concepts/plan.md`: a `done`
 * or `dropped` task cannot be dropped — the first has nothing left to drop,
 * the second already is one.
 *
 * **The reason field is inline, not a dialog** — the owner's design: a drop
 * is a short act on the row it is about, and a modal over a list a person is
 * still reading would hide the very row the reason is about.
 */
function TaskRow({
  task,
  onDropTask,
  onSaid,
}: {
  task: PlanTaskRow;
  onDropTask?: (taskId: string, reason: string) => Promise<PlanEditAnswer>;
  onSaid?: (sentence: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [dropping, setDropping] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  // Only a refusal is drawn on the control: an accepted drop closes the form it sits in.
  const [dropAnswer, setDropAnswer] = useState<ButtonAnswer>();
  const tap = useHaptics();
  // Whether an empty reason has actually been submitted. **A pristine field
  // is a hint, never an error** — the other forms here draw the same
  // distinction, and a red border on a field nobody has touched yet reads as
  // a mistake that already happened.
  const [attemptedEmpty, setAttemptedEmpty] = useState(false);
  const canDrop =
    onDropTask !== undefined && (task.state === "open" || task.state === "working");
  const blank = reason.trim() === "";

  function close(): void {
    setOpen(false);
    setReason("");
    setRefused(null);
    setAttemptedEmpty(false);
  }

  async function drop(): Promise<void> {
    if (blank) {
      setAttemptedEmpty(true);
      return;
    }
    if (onDropTask === undefined) return;
    setDropping(true);
    setRefused(null);
    setDropAnswer(undefined);
    try {
      const answer = await onDropTask(task.id, reason);
      // The tap answers the press, and a dropped task closes its dialog with no
      // control left to draw one. #1326.
      tap(patternFor(answer.ok ? "accepted" : "refused"));
      if (answer.ok) {
        onSaid?.("Dropped");
        close();
        return;
      }
      setRefused(refusalSaid(answer.outcome));
      setDropAnswer("refused");
    } finally {
      setDropping(false);
    }
  }

  return (
    <li className="armada-inside__plan-task" data-state={task.state}>
      <TaskMark state={task.state} />
      <span className="armada-inside__plan-task-id">{task.id}</span>
      <span className="armada-inside__plan-task-body">
        <span className="armada-inside__plan-task-title">{task.title}</span>
        {task.state === "dropped" && task.reason !== undefined ? (
          <span className="armada-inside__plan-task-reason">{task.reason}</span>
        ) : null}
        {open ? (
          <span className="armada-inside__plan-task-drop-form">
            <Input
              label="Reason"
              value={reason}
              onChange={(event) => setReason(event.target.value)}
              invalid={attemptedEmpty && blank}
              onKeyDown={(event) => {
                if (event.key === "Escape") close();
                if (event.key === "Enter") void drop();
              }}
            />
            {/* Shown for as long as the field is blank — a hint before a
                submit is tried empty, an error from that point on. Input's
                own `message` only ever draws the error half, so this is its
                own line rather than that prop. */}
            {blank ? (
              <span
                className="armada-inside__plan-task-hint"
                data-tone={attemptedEmpty ? "error" : "muted"}
              >
                A reason is needed.
              </span>
            ) : null}
            {refused === null ? null : (
              <span className="armada-inside__plan-task-refused">{refused}</span>
            )}
            <span className="armada-inside__plan-task-drop-acts">
              <Button
                variant="secondary"
                size="sm"
                ground="sunken"
                disabled={dropping}
                onClick={close}
              >
                Cancel
              </Button>
              <Button
                variant="secondary"
                size="sm"
                ground="sunken"
                pending={dropping}
                answer={dropAnswer}
                disabled={blank}
                onClick={() => void drop()}
              >
                {dropping ? "Dropping…" : DROP_TASK_LABEL}
              </Button>
            </span>
          </span>
        ) : null}
      </span>
      {canDrop && !open ? (
        <button type="button" className="armada-inside__plan-task-drop" onClick={() => setOpen(true)}>
          {DROP_TASK_LABEL}…
        </button>
      ) : null}
    </li>
  );
}

/**
 * The Plan region before a plan is recorded — a step on the workflow declares
 * `plan_recorded` and has not run yet. One muted line naming that step, and
 * nothing else: no task bar, no figure, no approach, no `Add task`. `#1007`;
 * `docs/journeys/monitor-active-work.md`, Plan.
 *
 * **The same wrapper `PlanWell` opens with.** Reusing `.armada-inside__pulse-head`
 * keeps the eyebrow at the region's usual place rather than drawing a second
 * shape a reader has to recognise as the same region.
 */
export function PlanPending({ stepLabel }: { stepLabel: string }) {
  return (
    <>
      <div className="armada-inside__pulse-head">
        <Eyebrow>Plan</Eyebrow>
      </div>
      <p className="armada-inside__absent" role="note">
        No plan yet — {stepLabel} records it.
      </p>
    </>
  );
}

/**
 * The Plan region's well — the task bar, the figure, the approach and one row
 * per task. `StepBar`'s segment grammar, extended to draw tasks rather than
 * steps; `docs/journeys/monitor-active-work.md`, Plan.
 */
export function PlanWell({
  approach,
  tasks,
  onAddTask,
  onDropTask,
  onSaid,
}: PlanRegionData & {
  onAddTask?: (title: string, detail: string) => Promise<PlanEditAnswer>;
  onDropTask?: (taskId: string, reason: string) => Promise<PlanEditAnswer>;
  onSaid?: (sentence: string) => void;
}) {
  const notDropped = tasks.filter(
    (task): task is PlanTaskRow & { state: "open" | "working" | "done" } => task.state !== "dropped",
  );
  const done = notDropped.filter((task) => task.state === "done").length;

  return (
    <>
      <div className="armada-inside__pulse-head">
        <Eyebrow>Plan</Eyebrow>
        {onAddTask === undefined ? null : (
          <AddTaskControl onAddTask={onAddTask} onSaid={onSaid} />
        )}
      </div>
      <div className="armada-inside__plan">
        <div className="armada-inside__plan-progress">
          <StepBar
            tasks={notDropped.map((task) => task.state)}
            label={`${done} of ${notDropped.length} tasks`}
          />
          <span className="armada-inside__plan-figure">
            {done} of {notDropped.length}
          </span>
        </div>
        <Clamped lines={2}>{approach}</Clamped>
        <ul className="armada-inside__plan-tasks">
          {tasks.map((task) => (
            <TaskRow key={task.id} task={task} onDropTask={onDropTask} onSaid={onSaid} />
          ))}
        </ul>
      </div>
    </>
  );
}
