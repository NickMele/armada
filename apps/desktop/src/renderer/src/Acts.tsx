// What can be done to one Job from its detail, and the two dialogs one of the
// acts opens.
//
// Split out of `JobDetail.tsx` when that file grew past the gate's 500-line
// warning. It is one subject — the set of acts, which state offers which, and
// how they are arranged — and the screen beside it is another.

import { useState } from "react";
import { Button, Dialog, SplitButton, Textarea, type SplitButtonItem } from "@armada/components";

import { JOB_LIFECYCLE } from "../../shared/generated/vocabulary";
import type { JobDetail as JobWhole, JobSummary } from "../../shared/protocol";
import { onwards, OVERRULING, recourseOf, REDISPATCHABLE, type Overrule } from "./recovery";
import type { Render } from "./render";

/**
 * What the two kills, the redispatch, the two step-resuming acts and the answer
 * to a gate that refused are called.
 */
export type JobAct =
  | "kill_drone"
  | "kill_job"
  | "redispatch"
  | "redirect"
  | "restart_step"
  | "override_verdict";

/**
 * The acts that confirm through the shared dialog. **Redirect and the override
 * are not two of them** — each carries a required field in its own dialog, so
 * each is its own confirmation and neither also routes through this one.
 */
export type ConfirmableAct = Exclude<JobAct, "redirect" | "override_verdict">;

/**
 * What can be done to this Job from here.
 *
 * **Seven acts, and none of them collapses into another.** Killing the Drone
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
 * | `redirect` | escalated, a step stopped, holding an `assigned_drone` | its own dialog |
 * | `restart_step` | escalated, a step stopped, no `assigned_drone` | yes |
 * | `override_verdict` | escalated, the stopped step refused by the judge | its own dialog |
 *
 * **The override is not a seventh variant of the six; it is the only one that
 * keeps the work the gate refused.** It sits with redirect and restart because
 * it ends nothing, and ahead of them because it takes nothing away —
 * `docs/concepts/job.md` orders the five acts on an escalated Job that way. It
 * is not offered beside `approve`, and never styled like it: approving says the
 * work was right, and this says a machine was wrong.
 *
 * **The three that end something are one split button, not a row of red.** Two
 * outlined reds side by side read as one control with two labels, which is the
 * thing they are least like. What is on the face is the act that state calls
 * for; the rest sit in the menu and each one's label says what survives it, so
 * the caret never turns a terminal act into a variant of a milder one.
 *
 * Redirect, restart and the override sit outside that group: none of them ends
 * anything, so none belongs beside a control whose whole point is announcing
 * what does.
 *
 * **Which of the three is offered is `recourseOf`'s answer and not this
 * file's.** The stopped screen states in words what resumes this Job, and a
 * header that decided it a second time here could disagree with the sentence a
 * person just read — so the predicate lives in `recovery.ts` and both sides
 * read it, down to whether overruling this step commits the Job.
 */
export function Acts({
  job,
  whole,
  render,
  acting,
  approving,
  stale,
  onAct,
  onRedirect,
  onOverrule,
  onApprove,
  onObserve,
}: {
  job: JobSummary;
  /**
   * `GET /jobs/:job_id`, or `null` while it has not arrived. **Only the
   * override reads it** — which trigger stopped the step and whether that step
   * is the workflow's last are on the detail and on no Board row.
   */
  whole: JobWhole | null;
  render: Render;
  acting: boolean;
  approving: boolean;
  stale: boolean;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Overrule the verdict, with the reason. Straight through like a redirect —
   * the dialog that collected the reason is the confirmation.
   */
  onOverrule: (jobId: string, reason: string) => void;
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
  // Which of redirect and restart applies, or neither. Four of Fleet's five
  // refusals are decidable from what the wire serves, and this is the surface
  // that has to decide them: a `completed_failed` Job with no Drone on it used
  // to be offered a restart, which is `NotResumable` on press every time.
  const recourse = render === "stopped" ? recourseOf(job, whole) : undefined;
  const canRedirect = recourse?.act === "redirect";
  const canRestart = recourse?.act === "restart_step";
  // Beside the two rather than instead of one: which trigger stopped the step
  // decides this, and whether a Drone is there decides those.
  const overrule = recourse?.overrule;
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
      {/* First of the acts that resume, because it is the one that takes
          nothing away — the refused step's own work is kept. Secondary and not
          primary: the one accent fill this header carries belongs to approving
          a dispatch, and an override that looked like an approval would be
          claiming the work was right rather than that the judge was wrong. */}
      {overrule === undefined ? null : (
        <OverruleControl
          jobId={job.id}
          overrule={overrule}
          disabled={acting || stale}
          onOverrule={onOverrule}
        />
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
 * The button that opens the override dialog, and the dialog itself.
 *
 * **The dialog is the confirmation, and the reason is why there is one.** A
 * person is recording that a verifier was wrong and that they took
 * responsibility for going past it, and `#154` will read those reasons to learn
 * whether the Judge or the criterion was at fault — so the send control stays
 * off while the field is blank, matching the 422 Fleet would answer, and there
 * is no path through this that produces an unexplained override.
 *
 * **Neutral tone, not destructive.** Nothing is destroyed: the work the gate
 * refused is exactly what survives. What the dialog owes instead is the cost —
 * the refusal stays on the record, the reason is written to the log beside it,
 * and the log is append-only.
 *
 * **It says which of the two things is about to happen.** Overruling a middle
 * step advances it and the Job carries on; overruling the last one makes Fleet
 * commit and deliver. `recourseOf` decided which, once, and `onwards` is the
 * sentence the screen behind this dialog already said about it.
 *
 * **And which of the two decisions is being overruled.** A Judge that refused a
 * criterion and a gaming check that called the evidence suspect are different
 * machines saying different things, so every word here comes from `OVERRULING`
 * keyed by the trigger rather than being written once for the refusal and
 * reused. The flag's case carries one thing the refusal's does not: what was
 * flagged, and where — a person taking responsibility for evidence a machine
 * distrusted must not have to leave the dialog to find out what it distrusted.
 * The same weight either way: the dialog and the required reason are what make
 * this a decision taken rather than a button pressed.
 */
function OverruleControl({
  jobId,
  overrule,
  disabled,
  onOverrule,
}: {
  jobId: string;
  overrule: Overrule;
  disabled: boolean;
  onOverrule: (jobId: string, reason: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const words = OVERRULING[overrule.trigger];
  // Only a flag has these, and a step can trip more than one pattern. Read
  // rather than counted: what was cited is the whole value of a flag, exactly
  // as a citation is the whole value of a refusal.
  const flagged = overrule.trigger === "evidence_suspect" ? overrule.step.flagged : [];

  function close() {
    setOpen(false);
    setReason("");
  }

  return (
    <>
      <Button variant="secondary" disabled={disabled} onClick={() => setOpen(true)}>
        {words.label}
      </Button>
      <Dialog
        open={open}
        tone="neutral"
        title={words.asks}
        confirmLabel={words.label}
        confirmDisabled={reason.trim() === ""}
        onCancel={close}
        onConfirm={() => {
          const said = reason;
          close();
          onOverrule(jobId, said);
        }}
      >
        {/* What the act is, said before what it does, and what it is not said
            beside it. The step is named because the person reading this came
            from a rail with several rows on it. */}
        <p>{words.dialog(overrule.step.label)}</p>
        {/* What the check actually found, on the flag's case only. The pattern
            is a name a workflow chose and the citation is a place in the work,
            so both are mono — machine-derived, and neither is a sentence. A
            flagged step that carries none is a Fleet that flagged without
            citing, and drawing nothing is the honest render of that. */}
        {flagged.length === 0 ? null : (
          <p>
            {"It flagged "}
            {flagged.map((flag, at) => (
              <span key={`${flag.pattern}-${flag.cited}`}>
                {at === 0 ? null : ", "}
                <span className="mono">{flag.pattern}</span>
                {" in "}
                <span className="mono">{flag.cited}</span>
              </span>
            ))}
            {"."}
          </p>
        )}
        {/* What happens next, and what it costs. `onwards` is the same sentence
            the screen behind this one already said, so the two cannot differ
            about whether this job is about to land. */}
        <p>
          {`${onwards(overrule)} Your reason is written to this job's log and stays there — the ` +
            "log is append-only, and nothing takes an override back. It is not sent to the drone, " +
            "which did nothing wrong and is told only that the step was accepted."}
        </p>
        {/* No `autoFocus`, for `RedirectControl`'s reason: the dialog's own
            contract puts initial focus on Cancel. */}
        <Textarea
          label={words.field}
          rows={4}
          value={reason}
          onChange={(event) => setReason(event.target.value)}
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
 *
 * **The override says "overrule", never "approve" or "accept".** Approving is a
 * different act on a different status and means the work was right; this one
 * means a machine was wrong and a person is taking responsibility for going
 * past it.
 *
 * **The override's own control does not read this row.** Its wording changes
 * with what is being overruled — a Judge's verdict or a gaming flag — so it
 * comes from `OVERRULING` in `recovery.ts`, where the trigger is known. This
 * row is the act's name where no trigger is in hand, which is the shared
 * confirmation and the menu, and it keeps the record total over `JobAct`.
 */
export const ACT_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone",
  kill_job: "Kill job",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone",
  restart_step: "Restart step",
  override_verdict: "Overrule the verdict",
};

/**
 * The same acts inside the menu, where each says what survives it. A caret hides
 * the consequence that a button's own position states, so the label has to carry
 * it — `Kill drone` and `Kill job` differ by everything and by three characters.
 *
 * **Redirect, restart and the override never reach a menu** — none of them
 * joins the split button, so these three entries exist only to keep the record
 * total over `JobAct` rather than for anything that reads them today.
 */
const MENU_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone, the job stays open",
  kill_job: "Kill job, it ends here",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone, the job stays open",
  restart_step: "Restart the step, on the same worktree",
  override_verdict: "Overrule the verdict, the refused work stands",
};

/** Which act takes the split button's face, in preference order. */
const FACE: readonly ConfirmableAct[] = ["redispatch", "kill_job"];
