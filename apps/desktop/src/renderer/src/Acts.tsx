// What can be done to one Job from its detail, and the two dialogs one of the
// acts opens.
//
// Split out of `JobDetail.tsx` when that file grew past the gate's 500-line
// warning. It is one subject — the set of acts, which state offers which, and
// how they are arranged — and the screen beside it is another.

import { useState } from "react";
import { Button, Dialog, SplitButton, Textarea, type SplitButtonItem } from "@armada/components";

import { JOB_LIFECYCLE } from "../../shared/generated/vocabulary";
import type { JobSummary } from "../../shared/protocol";
import type { Render } from "./render";

/** What the two kills, the redispatch and the two step-resuming acts are called. */
export type JobAct = "kill_drone" | "kill_job" | "redispatch" | "redirect" | "restart_step";

/**
 * The acts that confirm through the shared dialog. **Redirect is not one** —
 * its dialog carries the instruction itself, so it is its own confirmation
 * and does not also route through this one.
 */
export type ConfirmableAct = Exclude<JobAct, "redirect">;

/**
 * The statuses a redispatch is offered on. Three, and **`rejected` is not one**:
 * a rejected Job never ran, so it has no Facts and no Evidence to carry
 * forward, and redispatching it would only be proposing a new Job — which the
 * composer already does.
 *
 * Written here rather than read from the generated vocabulary because no
 * registry file carries the set: `job-fields.toml` still asks it as an open
 * question on `redispatched_from`. Fleet's route is the authority and refuses
 * anything else; this only keeps a button off the screen that would be.
 */
const REDISPATCHABLE: ReadonlySet<string> = new Set([
  "escalated",
  "completed_failed",
  "killed",
]);

/**
 * What can be done to this Job from here.
 *
 * **Six acts, and none of them collapses into another.** Killing the Drone
 * ends a process and leaves the Job open with its worktree held; killing the
 * Job ends the Job at `killed`, terminal; redispatch does the second and mints
 * a replacement; approving lets a Job at the gate run. Redirect and restart are
 * the two acts that resume a stopped step rather than ending or replacing it —
 * `crates/api/src/routes.rs` decides which applies by whether the Job still
 * holds a Drone, and the two are never offered together for that reason.
 *
 * | Act | Drawn on | Confirms |
 * |---|---|---|
 * | `approve` | `awaiting_approval` | no — see `onApprove` |
 * | `redispatch` | `escalated`, `completed_failed`, `killed` | yes |
 * | `kill_drone` | a Job holding an `assigned_drone` | yes |
 * | `kill_job` | every non-terminal status | yes |
 * | `redirect` | escalated, holding an `assigned_drone` | its own dialog |
 * | `restart_step` | escalated, no `assigned_drone` | yes |
 *
 * **The three that end something are one split button, not a row of red.** Two
 * outlined reds side by side read as one control with two labels, which is the
 * thing they are least like. What is on the face is the act that state calls
 * for; the rest sit in the menu and each one's label says what survives it, so
 * the caret never turns a terminal act into a variant of a milder one.
 *
 * Redirect and restart sit outside that group: neither ends anything, so
 * neither belongs beside a control whose whole point is announcing what does.
 */
export function Acts({
  job,
  render,
  acting,
  approving,
  stale,
  onAct,
  onRedirect,
  onApprove,
  onObserve,
}: {
  job: JobSummary;
  render: Render;
  acting: boolean;
  approving: boolean;
  stale: boolean;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  onApprove: (jobId: string) => void;
  /**
   * Open this Job's turns as a view of their own. **Omitted on the finished
   * render**, where the turns are a section of the record on the page rather
   * than a screen reached from the header — one route to a thing, not two.
   */
  onObserve?: () => void;
}) {
  const life = JOB_LIFECYCLE[job.status];
  const over = life?.terminal ?? true;
  // Menu order, mildest first — the split button puts destructive last.
  const acts: ConfirmableAct[] = [
    // Only where Fleet accepts one; anything else is its 409, and this does not
    // offer a button that is refused on press.
    ...(render === "stopped" && REDISPATCHABLE.has(job.status)
      ? (["redispatch"] as ConfirmableAct[])
      : []),
    // `assigned_drone` is presence rather than state: there is nothing to kill
    // without one.
    ...(job.assigned_drone === undefined ? [] : (["kill_drone"] as ConfirmableAct[])),
    ...(over ? [] : (["kill_job"] as ConfirmableAct[])),
  ];
  // Which of redirect and restart applies. Decided by the Drone's presence, the
  // same signal `kill_drone` reads — a surface that offered both regardless
  // would offer one Fleet always refuses.
  const canRedirect = render === "stopped" && job.assigned_drone !== undefined;
  const canRestart = render === "stopped" && job.assigned_drone === undefined;
  // What the state calls for goes on the face: replacing a Job that stopped, and
  // otherwise the kill that ends it. Never the milder kill — the act with the
  // larger consequence does not hide behind a caret.
  const face = FACE.find((act) => acts.includes(act)) ?? acts[0];
  const menu: SplitButtonItem[] = acts
    .filter((act) => act !== face)
    .map((act) => ({
      label: MENU_LABEL[act],
      danger: act === "kill_job",
      onSelect: () => onAct(act, job.id),
    }));

  return (
    <>
      {/* Ghost, and first: watching is not one of the acts. It ends nothing
          and confirms nothing. It is offered wherever the turns are not already
          on the page, because the transcript is the Job's history across every
          Drone it has had — a Job that never had one says so in the pane rather
          than by having no control. */}
      {onObserve === undefined ? null : (
        <Button variant="ghost" disabled={stale} onClick={onObserve}>
          Watch the turns
        </Button>
      )}
      {/* Neither ends the Job, so neither is a plain-red act. The dialog it
          opens is itself the confirmation — a person who cancels the dialog
          has sent nothing. */}
      {canRedirect ? (
        <RedirectControl jobId={job.id} disabled={acting || stale} onRedirect={onRedirect} />
      ) : null}
      {canRestart ? (
        <Button
          variant="secondary"
          disabled={acting || stale}
          onClick={() => onAct("restart_step", job.id)}
        >
          {ACT_LABEL.restart_step}
        </Button>
      ) : null}
      {face === undefined ? null : menu.length === 0 ? (
        // A split button with nothing in its menu is a button. Outlined, because
        // a solid red control reads as an error state rather than as an act.
        <Button
          variant="destructive"
          disabled={acting || stale}
          onClick={() => onAct(face, job.id)}
        >
          {ACT_LABEL[face]}
        </Button>
      ) : (
        <SplitButton
          variant="destructive"
          disabled={acting || stale}
          menuLabel="What else ends this job"
          items={menu}
          onAction={() => onAct(face, job.id)}
        >
          {ACT_LABEL[face]}
        </SplitButton>
      )}
      {/* The one primary this header ever carries, and the only forward act in
          the set. Last, where the shell head puts its own primary — the accent
          fill and the distance are what keep it from reading as a peer of the
          red group. Approving a Job you opened in order to read it is the whole
          point of the gate; going back to the list to say yes is not. */}
      {job.status === "awaiting_approval" ? (
        <Button
          variant="primary"
          disabled={approving || stale}
          onClick={() => onApprove(job.id)}
        >
          {approving ? "Approving" : "Approve dispatch"}
        </Button>
      ) : null}
    </>
  );
}

/**
 * The button that opens the redirect dialog, and the dialog itself.
 *
 * **The dialog is the confirmation.** There is no second "are you sure" after
 * it, because sending is the one thing the dialog's own button does — closing
 * it any other way sends nothing. `confirmDisabled` keeps the send control off
 * while the field is blank, matching the 422 Fleet would give it, rather than
 * letting the press round-trip to Fleet to learn that.
 */
function RedirectControl({
  jobId,
  disabled,
  onRedirect,
}: {
  jobId: string;
  disabled: boolean;
  onRedirect: (jobId: string, instruction: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [instruction, setInstruction] = useState("");

  function close() {
    setOpen(false);
    setInstruction("");
  }

  return (
    <>
      <Button variant="secondary" disabled={disabled} onClick={() => setOpen(true)}>
        {ACT_LABEL.redirect}
      </Button>
      <Dialog
        open={open}
        tone="neutral"
        title="Redirect the drone on this job?"
        confirmLabel={ACT_LABEL.redirect}
        confirmDisabled={instruction.trim() === ""}
        onCancel={close}
        onConfirm={() => {
          const sent = instruction;
          close();
          onRedirect(jobId, sent);
        }}
      >
        <p>
          The instruction is sent to the drone as a new turn. The job stays at the same step, with
          the same session — nothing is spawned and nothing already done is thrown away.
        </p>
        {/* No `autoFocus`: the dialog's own contract puts initial focus on
            Cancel, and a second claim on it here would only lose to it. */}
        <Textarea
          label="Instruction"
          rows={4}
          value={instruction}
          onChange={(event) => setInstruction(event.target.value)}
        />
      </Dialog>
    </>
  );
}

/**
 * What each act is called on its button. **Redispatch does not say "retry" or
 * "run again"** — nothing resumes, and a label implying the same Job continues
 * would describe an act Fleet does not perform. The confirmation states the
 * rest; the button names the act.
 */
export const ACT_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone",
  kill_job: "Kill job",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone",
  restart_step: "Restart step",
};

/**
 * The same acts inside the menu, where each says what survives it. A caret hides
 * the consequence that a button's own position states, so the label has to carry
 * it — `Kill drone` and `Kill job` differ by everything and by three characters.
 *
 * **Redirect and restart never reach a menu** — neither joins the split
 * button, so these two entries exist only to keep the record total over
 * `JobAct` rather than for anything that reads them today.
 */
const MENU_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone, the job stays open",
  kill_job: "Kill job, it ends here",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone, the job stays open",
  restart_step: "Restart the step, on the same worktree",
};

/** Which act takes the split button's face, in preference order. */
const FACE: readonly ConfirmableAct[] = ["redispatch", "kill_job"];
