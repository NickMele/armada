// What can be done to one Job from its detail, and how the controls are
// arranged.
//
// Split out of `JobDetail.tsx` when that file grew past the gate's 500-line
// warning. It is one subject — the set of acts, which state offers which, and
// how they are arranged — and the screen beside it is another.
//
// **The controls that own a dialog are their own files**, for the same reason
// and at the same line: `Redirect.tsx`, `Overrule.tsx` and `Report.tsx` each
// hold one act and what it asks a person for before it sends. What stays here
// is which of them a state offers. The words on every button are `copy.ts`'s.

import { Button } from "@armada/components";

import { JOB_LIFECYCLE } from "../../shared/generated/vocabulary";
import type { Outcome } from "../../shared/bridge";
import type { FileReport, JobDetail as JobWhole, JobSummary } from "../../shared/protocol";
import { ACT_LABEL } from "./copy";
import { OverruleControl } from "./Overrule";
import { recourseOf } from "./recovery";
import { RedirectControl } from "./Redirect";
import type { Render } from "./render";
import { ReportControl } from "./Report";

/**
 * What the two kills, the redispatch, the two step-resuming acts and the two
 * answers to a gate that stopped the work are called.
 *
 * **`rerun_gate` is the seventh and it is not a widening of the sixth.** An
 * override answers a gate that ruled; this answers a gate that ruled on
 * nothing. `crates/fleet/src/regating.rs` admits exactly the trigger
 * `overrulable()` refuses, so the two names partition rather than overlap.
 */
export type JobAct =
  | "kill_drone"
  | "kill_job"
  | "redispatch"
  | "redirect"
  | "restart_step"
  | "override_verdict"
  | "rerun_gate";

/**
 * The acts that confirm through the shared dialog. **Redirect and the override
 * are not two of them** — each carries a required field in its own dialog, so
 * each is its own confirmation and neither also routes through this one.
 *
 * **Nor is the re-run, and for the opposite reason.** It has no dialog at all:
 * nothing is destroyed, nothing is overruled and nothing is committed, so there
 * is no responsibility for a person to take on the record. A confirmation here
 * would say a re-run costs something, which would be the screen inventing a
 * cost Fleet does not charge.
 */
export type ConfirmableAct = Exclude<
  JobAct,
  "redirect" | "override_verdict" | "rerun_gate"
>;

/**
 * What can be done to this Job from here.
 *
 * **Eight acts, and none of them collapses into another.** Killing the Drone
 * ends a process and leaves the Job open with its worktree held; killing the
 * Job ends the Job at `killed`, terminal; redispatch does the second and mints
 * a replacement; approving lets a Job at the gate run. Redirect and restart are
 * the two acts that put a person back on a Job rather than ending or replacing
 * it, and they are never offered together — a redirect wants a live session and
 * a restart wants the Drone gone.
 *
 * **Five of the eight are drawn on Fleet's answer and not on the row.**
 * `stuck.recourse` on `GET /jobs/:job_id` is the acts Fleet will take now, and
 * `#193` moved them here from a derivation that could not read the filesystem
 * and so offered a restart onto a worktree that had been reclaimed. The two
 * kills are the exception and stay derived: neither is recourse, and a Drone to
 * kill is presence on the row.
 *
 * | Act | Drawn on | Confirms |
 * |---|---|---|
 * | `approve` | `awaiting_approval` | no — see `onApprove` |
 * | `redispatch` | `recourse` names `redispatch_job` | yes |
 * | `kill_drone` | a Job holding an `assigned_drone` | yes |
 * | `kill_job` | every non-terminal status | yes |
 * | `redirect` | `recourse` names `redirect_drone` | its own dialog |
 * | `restart_step` | `recourse` names `restart_step` | yes |
 * | `override_verdict` | `recourse` names `override_verdict` | its own dialog |
 * | `rerun_gate` | `recourse` names `rerun_gate` | no — nothing is at stake |
 *
 * **The override is not a seventh variant of the six; it is the only one that
 * keeps the work the gate refused.** It sits with redirect and restart because
 * it ends nothing, and ahead of them because it takes nothing away —
 * `docs/concepts/job.md` orders the five acts on an escalated Job that way. It
 * is not offered beside `approve`, and never styled like it: approving says the
 * work was right, and this says a machine was wrong.
 *
 * **The three that end something are plain quiet buttons, not a red split.**
 * The drawing gives a running Job one control, `Kill job`, in the same neutral
 * treatment as every other button on the screen. A destructive red with a
 * caret made the header's only control the loudest thing on it, and hid two
 * acts behind a menu on a screen that is read rather than driven. Each is its
 * own button now, in menu order — mildest first — and each still confirms,
 * which is where what an act costs is stated.
 *
 * Redirect, restart, the override and the re-run sit outside that group: none
 * of them ends anything, so none belongs beside a control whose whole point is
 * announcing what does.
 *
 * **Which of the four is offered is `recourseOf`'s reading and not this
 * file's.** The stopped screen states in words what resumes this Job, and a
 * header that decided it a second time here could disagree with the sentence a
 * person just read — so the one reading lives in `recovery.ts` and both sides
 * take it, down to whether overruling this step commits the Job.
 */
export function Acts({
  job,
  whole,
  render,
  acting,
  approving,
  stale,
  onAct,
  onApprove,
  onReport,
  onCopied,
}: {
  job: JobSummary;
  /**
   * `GET /jobs/:job_id`, or `null` while it has not arrived. **Every act on a
   * Job that stopped reads it**, because `stuck` is there and on no Board row —
   * so the header offers none of them until the detail lands, rather than
   * offering one the row cannot decide.
   */
  whole: JobWhole | null;
  render: Render;
  acting: boolean;
  approving: boolean;
  stale: boolean;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onApprove: (jobId: string) => void;
  /**
   * Say this job failed in error. **Not one of the acts** — it moves nothing,
   * which is why it is not in `JobAct` and reaches no button here. It
   * answers with the outcome because the record that comes back is what the
   * dialog shows next.
   */
  onReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  onCopied: (value: string) => void;
}) {
  const life = JOB_LIFECYCLE[job.status];
  const over = life?.terminal ?? true;
  // What Fleet says it will do to this Job now — the acts, the words for them,
  // and the step each is about. **Only on the stopped render**, which is the
  // only one that reads a classification: nothing else on this header is an act
  // on a Job that stopped.
  const recourse = render === "stopped" ? recourseOf(job, whole) : undefined;
  // Mildest first, so the act that ends the Job is last and furthest from the
  // reading a person arrived to do.
  const acts: ConfirmableAct[] = [
    // Fleet's answer and not the status: a replacement also needs the workflow
    // this Job named to be one Fleet still holds, which no row carries.
    ...(recourse?.redispatch === true ? (["redispatch"] as ConfirmableAct[]) : []),
    // `assigned_drone` is presence rather than state: there is nothing to kill
    // without one.
    ...(job.assigned_drone === undefined ? [] : (["kill_drone"] as ConfirmableAct[])),
    ...(over ? [] : (["kill_job"] as ConfirmableAct[])),
  ];
  return (
    <>
      {/* Ghost, because it is not an act on the
          job: this records what a person concluded and leaves the job exactly
          where it was. Offered on every stopped job rather than only the ones
          something can still be done to — a job nothing can be done to is the
          one most likely to have failed wrongly and been left. */}
      {render === "stopped" ? (
        <ReportControl
          jobId={job.id}
          whole={whole}
          disabled={stale}
          onReport={onReport}
          onCopied={onCopied}
        />
      ) : null}
      {/* One button per act, in menu order. Neutral, because the confirmation
          is where a terminal act states what it costs — and a red control in a
          header a person is reading rather than driving reads as the screen's
          own alarm rather than as something they may press. */}
      {acts.map((act) => (
        <Button
          key={act}
          variant="secondary"
          disabled={acting || stale}
          onClick={() => onAct(act, job.id)}
        >
          {ACT_LABEL[act]}
        </Button>
      ))}
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
 * The four acts that change a step rather than the Job, in the panel header
 * beside the step they act on.
 *
 * **They were rendered at Job level and four of the eight do not act on the
 * Job.** Redirecting a Drone, restarting a step, overruling the verdict on a
 * step and asking a step's gate again all leave the Job exactly where it is —
 * only the step moves. Drawn in the Job header they read as peers of the two
 * kills, which is the reading job detail was redrawn to end.
 *
 * **The accent goes with them.** The object of attention on this screen is the
 * open step, and the Job header keeps only the acts that end or replace the
 * Job. Which one takes the fill is the caller's, because it depends on which
 * act the state calls for — nothing here decides emphasis for a state it cannot
 * see.
 *
 * **Which of the four is offered is `recovery.ts`'s reading, unchanged.** It is
 * the side that can see the worktree, and a restart offered without that answer
 * was refused on the press every time the worktree had been reclaimed. Moving
 * the controls did not move that decision.
 */
export function StepActs({
  job,
  whole,
  render,
  acting,
  stale,
  onAct,
  onRedirect,
  onOverrule,
  onRerun,
}: {
  job: JobSummary;
  whole: JobWhole | null;
  render: Render;
  acting: boolean;
  stale: boolean;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * Overrule the verdict, with the reason. Straight through like a redirect —
   * the dialog that collected the reason is the confirmation.
   */
  onOverrule: (jobId: string, reason: string) => void;
  /**
   * Ask the gate again on a step it could not decide. **Straight through like a
   * redirect, and for a different reason** — that one already confirmed in its
   * own dialog, and this one has nothing to confirm.
   */
  onRerun: (jobId: string) => void;
}) {
  // Fleet's answer, on the render that carries one. A Job nothing is wrong with
  // has no `stuck` and so offers none of these — which is the wire's reading
  // and not a rule written here.
  const recourse = render === "stopped" ? recourseOf(job, whole) : undefined;
  const canRedirect = recourse?.act === "redirect";
  const canRestart = recourse?.act === "restart_step";
  // Beside the two rather than instead of one: which trigger stopped the step
  // decides these, and whether a Drone is there decides those. **Never both**,
  // because the two triggers partition — `recovery.ts` says so.
  const overrule = recourse?.overrule;
  const reread = recourse?.reread;

  return (
    <>
      {/* First of the acts that resume, because it is the one that takes
          nothing away — the refused step's own work is kept. Secondary and not
          primary: an override that looked like an approval would be claiming
          the work was right rather than that the Judge was wrong. */}
      {overrule === undefined ? null : (
        <OverruleControl
          jobId={job.id}
          overrule={overrule}
          disabled={acting || stale}
          onOverrule={onOverrule}
        />
      )}
      {/* Where nothing ruled, in the place the override would be: the two are
          mutually exclusive, and both keep the step's work. **No dialog and no
          confirmation** — a re-run destroys nothing, overrules nothing and
          commits nothing, so stopping to ask would claim a cost Fleet does not
          charge. */}
      {reread === undefined ? null : (
        <Button variant="secondary" disabled={acting || stale} onClick={() => onRerun(job.id)}>
          {ACT_LABEL.rerun_gate}
        </Button>
      )}
      {/* Neither ends the Job, so neither is a plain-red act. The dialog a
          redirect opens is itself the confirmation — a person who cancels it
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
    </>
  );
}
