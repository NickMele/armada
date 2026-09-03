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

import { Button, SplitButton } from "@armada/components";
import type { SplitButtonItem } from "@armada/components";

import { JOB_LIFECYCLE } from "@armada/components";
import type { Outcome } from "@armada/protocol";
import type { FileReport, JobDetail as JobWhole, JobSummary } from "@armada/protocol";
import { ACT_LABEL, MENU_LABEL, REPORT_LABEL } from "./copy";
import { recourseOf } from "./recovery";
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
  | "rerun_gate"
  // The eighth, and the only one that acts on disk rather than on the record
  // or the machine. `armada clean` could already do it and refuses while Fleet
  // is running, which is exactly when a person wants the space back.
  | "reclaim_worktree";

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
 * **Nine acts, and none of them collapses into another.** Killing the Drone
 * ends a process and leaves the Job open with its worktree held; killing the
 * Job ends the Job at `killed`, terminal; redispatch does the second and mints
 * a replacement; approving lets a Job at the gate run. Redirect and restart are
 * the two acts that put a person back on a Job rather than ending or replacing
 * it, and they are never offered together — a redirect wants a live session and
 * a restart wants the Drone gone. Reclaiming the worktree is the one act on
 * disk rather than on the Job: the record stays exactly where it is, and
 * clearing the row is the board's own act.
 *
 * **Five of the nine are drawn on Fleet's answer and not on the row.**
 * `stuck.recourse` on `GET /jobs/:job_id` is the acts Fleet will take now, and
 * `#193` moved them here from a derivation that could not read the filesystem
 * and so offered a restart onto a worktree that had been reclaimed. The two
 * kills and the reclaim are the exceptions and stay derived: none of them is
 * recourse — recourse is how a Job goes forward and these three do not carry
 * one forward — and a Drone to kill is presence on the row.
 *
 * | Act | Drawn on | Confirms |
 * |---|---|---|
 * | `approve` | `awaiting_approval` | no — see `onApprove` |
 * | `redispatch` | `recourse` names `redispatch_job` | yes |
 * | `kill_drone` | a Job holding an `assigned_drone` | yes |
 * | `kill_job` | every non-terminal status | yes |
 * | `reclaim_worktree` | every terminal status | yes |
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
 * **On a Job that stopped, the header carries one split button.** Its lead is
 * the act the state calls for, its divided segment is the caret and nothing
 * else, and every other Job-level act sits behind it. Three controls in a row —
 * `Report this job`, `Redispatch as a new job`, `Kill job` on a finished Job
 * with disputed evidence — collapsed the title's column and wrapped a
 * seven-word title to three lines, because the header's width was a function of
 * how many acts the state offered. It is not a function of that any more.
 * `docs/journeys/monitor-active-work.md`, Acts.
 *
 * **This is not the red split the previous pass removed.** That one made the
 * header's only control the loudest thing on a screen that is read rather than
 * driven, and it led with an act that ends the Job. This one is never
 * destructive on the face and never red: the fill is the state and the lead is
 * the way forward.
 *
 * **The fill is the state, not the act.** Accent where the Job is waiting on a
 * person, which the registry answers — `whoIsActing` is `Person` on `escalated`
 * and `None` on every terminal status. Secondary where it is not. Same height
 * in both cases: emphasis comes from fill, never from size, and nothing here
 * types a list of statuses the registry already holds.
 *
 * **The lead is never destructive.** Both kills sit in the menu, because the
 * lead segment is what a stray `Enter` hits. Each still confirms, which is
 * where what an act costs is stated.
 *
 * **Two of the drawing's five leads are not here yet.** A running Job and a
 * plain escalated one lead with `Pilot`, which `actions.toml` carries as
 * `unbuilt = "#250"`; a Job awaiting review leads with `Review`, which on this
 * screen is the decision block under the story rather than a header act. Until
 * those land there is no non-destructive lead on those renders, and a split
 * button with no legal lead is not a split button — so they keep the buttons
 * they had. The drawing is ahead of the code here, and says which issue closes
 * the gap rather than inventing a control to fill it.
 *
 * **`Copy debug info` and `Observe` are in the drawing's menu and not in
 * this one.** Copy debug info is an error surface's act and composes no
 * Job-level payload yet; observing is not a control at all — `App.tsx` opens
 * the socket because a Job is open. Neither is invented here to match a
 * picture.
 *
 * Redirect, restart, the override and the re-run sit outside all of this: none
 * of them acts on the Job, so none reaches this header at all, and since the
 * split they are not even in this file — `StepActs.tsx` draws them.
 *
 * **What this header does read of that answer is `redispatch`**, which is
 * `recourseOf`'s and not this file's. The stopped screen states in words what
 * replaces this Job, and a header that decided it a second time here could
 * disagree with the sentence a person just read.
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
  reporting,
  onReporting,
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
  /**
   * Whether the report dialog is up. **Held by the screen and not here**,
   * because `b` opens it and the keyboard is bound one level up — see
   * `detail-keys.ts`.
   */
  reporting: boolean;
  onReporting: (up: boolean) => void;
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
    // Terminal, and the row is what says so — no recourse names this act,
    // because reclaiming is not a way to carry the Job forward. Fleet refuses
    // it on anything still in flight, so this is the same predicate rather
    // than a second one: there is no disk to give back while a Drone might
    // still write to it.
    ...(over ? (["reclaim_worktree"] as ConfirmableAct[]) : []),
    // `assigned_drone` is presence rather than state: there is nothing to kill
    // without one.
    ...(job.assigned_drone === undefined ? [] : (["kill_drone"] as ConfirmableAct[])),
    ...(over ? [] : (["kill_job"] as ConfirmableAct[])),
  ];
  // The lead, on the render that has one. Redispatch where Fleet offers it,
  // and the report where it does not — the two are the only non-destructive
  // acts this header holds until `Pilot` lands, and the lead is never
  // destructive.
  const lead: Lead | undefined =
    render !== "stopped"
      ? undefined
      : acts.includes("redispatch")
        ? { act: "redispatch", label: ACT_LABEL.redispatch }
        : { act: "report", label: REPORT_LABEL };
  const behind: SplitButtonItem[] =
    lead === undefined
      ? []
      : [
          // Never a repeat of the lead: a menu that offers the label again is
          // one entry a person reads twice and can act on once.
          ...(lead.act === "report"
            ? []
            : [{ label: REPORT_LABEL, shortcut: REPORT_KEY, onSelect: () => onReporting(true) }]),
          ...acts
            .filter((act) => act !== lead.act)
            .map((act) => ({
              label: MENU_LABEL[act],
              // Both kills end something and the reclaim removes a directory,
              // and the menu draws that rather than the control announcing it
              // in red on the face.
              danger: true,
              onSelect: () => onAct(act, job.id),
            })),
        ];
  return (
    <>
      {/* The dialog with no button. Offered on every stopped job rather than
          only the ones something can still be done to — a job nothing can be
          done to is the one most likely to have failed wrongly and been left. */}
      {render === "stopped" ? (
        <ReportControl
          jobId={job.id}
          whole={whole}
          open={reporting && !stale}
          onClose={() => onReporting(false)}
          onReport={onReport}
          onCopied={onCopied}
        />
      ) : null}
      {/* One control, whatever the state offers. The accent says the Job is
          waiting on a person and nothing else does — a terminal Job's control
          is quiet, because there is nobody it is waiting for.

          **A split button with nothing in its menu is a button**, which is the
          primitive's own rule and not a shortcut taken here: a killed Job with
          no Drone and no replacement offers the report and nothing else, and a
          caret over an empty menu is a control that does not answer. */}
      {lead === undefined ? null : behind.length === 0 ? (
        <Button
          variant={life?.whoIsActing === "Person" ? "primary" : "secondary"}
          disabled={acting || stale}
          onClick={() => (lead.act === "report" ? onReporting(true) : onAct(lead.act, job.id))}
        >
          {lead.label}
        </Button>
      ) : (
        <SplitButton
          variant={life?.whoIsActing === "Person" ? "primary" : "secondary"}
          items={behind}
          disabled={acting || stale}
          menuLabel="Everything else this job can do"
          onAction={() => (lead.act === "report" ? onReporting(true) : onAct(lead.act, job.id))}
        >
          {lead.label}
        </SplitButton>
      )}
      {/* The renders the split button has no legal lead on. Both of the
          drawing's leads there are unbuilt — `Pilot` is #250, and `Review` is
          the decision block under the story rather than a header act — and a
          lead is never destructive, so until one lands each act keeps its own
          quiet button. Neutral, because the confirmation is where a terminal
          act states what it costs. */}
      {lead !== undefined
        ? null
        : acts.map((act) => (
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
 * The binding the report entry displays, from `actions.toml` — `report_job`,
 * scope `detail`. **Shown because this surface answers it**: `detail-keys.ts`
 * binds `b`, and a menu chip promising a key nothing answers is worse than a
 * menu with no chips at all.
 *
 * **The kills carry none here for exactly that reason.** `x` is `kill` in the
 * registry and it is bound on the Board, not on job detail — so the entries
 * that end this Job state what they cost and no key beside it.
 */
const REPORT_KEY = "b";

/**
 * The act on the face of the split button. Two, and never a third: the lead is
 * never destructive, and these are the only acts this header holds that are
 * not.
 */
type Lead = { act: "redispatch"; label: string } | { act: "report"; label: string };

