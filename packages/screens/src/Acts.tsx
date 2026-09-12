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
import { ACT_LABEL, MENU_LABEL, RAISE_CAP_LABEL, RAISE_TURN_CAP_LABEL, REPORT_LABEL } from "./copy";
import { RaiseCapControl } from "./RaiseCap";
import { RaiseTurnCapControl } from "./RaiseTurnCap";
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
  | "reclaim_worktree"
  // The ninth, and the one act on this header that cannot be undone. It takes
  // the row `reclaim_worktree` leaves — `crates/ipc/operations.toml` argues
  // the split on the same row that argues this one's — so a person wanting
  // both sends both, and this is never the act a stray `Enter` reaches.
  | "forget_job";

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
 * **Ten acts, and none of them collapses into another.** Killing the Drone
 * ends a process and leaves the Job open with its worktree held; killing the
 * Job ends the Job at `killed`, terminal; redispatch does the second and mints
 * a replacement; approving lets a Job at the gate run. Redirect and restart are
 * the two acts that put a person back on a Job rather than ending or replacing
 * it, and they are never offered together — a redirect wants a live session and
 * a restart wants the Drone gone. Reclaiming the worktree is the one act on
 * disk rather than on the Job: the record stays exactly where it is. Forgetting
 * the Job is the one that takes the record the reclaim leaves — its bulk twin
 * is the board's own `Clear`/`Delete record` acts, and this is the per-Job door
 * to the same act.
 *
 * **Six of the ten are drawn on Fleet's answer and not on the row.**
 * `stuck.recourse` on `GET /jobs/:job_id` is the acts Fleet will take now, and
 * `#193` moved them here from a derivation that could not read the filesystem
 * and so offered a restart onto a worktree that had been reclaimed. The two
 * kills, the reclaim and the forget are the exceptions and stay derived: none
 * of them is recourse — recourse is how a Job goes forward and these four do
 * not carry one forward — and a Drone to kill is presence on the row.
 *
 * | Act | Drawn on | Confirms |
 * |---|---|---|
 * | `approve` | `awaiting_approval` | no — see `onApprove` |
 * | `redispatch` | `recourse` names `redispatch_job` | yes |
 * | `kill_drone` | a Job holding an `assigned_drone` | yes |
 * | `kill_job` | every non-terminal status | yes |
 * | `reclaim_worktree` | every terminal status | yes |
 * | `forget_job` | every terminal status | yes |
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
  onRaiseCap,
  raising,
  onRaising,
  onRaiseTurnCap,
  raisingTurns,
  onRaisingTurns,
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
  /**
   * Give this job a higher cost ceiling. **Not one of the acts** — it moves no
   * status, no step and no drone, which is why it is not in `JobAct` and does
   * not reach the split button. It carries the figure because the dialog that
   * collected it is the confirmation.
   */
  onRaiseCap: (jobId: string, costCapMicros: number) => void;
  /** Whether the cost-cap dialog is up. Held by the screen; `B` opens it too. */
  raising: boolean;
  onRaising: (up: boolean) => void;
  /**
   * Let this job take more turns. **Not one of the acts**, on `onRaiseCap`'s
   * terms and for its reason. The figure is a plain turn count — the other
   * ceiling reads in millionths of a dollar, and neither converts to the other.
   */
  onRaiseTurnCap: (jobId: string, turnCap: number) => void;
  /** Whether the turn-cap dialog is up. Held by the screen; `T` opens it too. */
  raisingTurns: boolean;
  onRaisingTurns: (up: boolean) => void;
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
    // The same predicate as the reclaim, for the same reason — there is
    // nothing to delete while a Job is still in flight — and last, because it
    // is the one act here that cannot be undone.
    ...(over ? (["forget_job"] as ConfirmableAct[]) : []),
    // `assigned_drone` is presence rather than state: there is nothing to kill
    // without one.
    ...(job.assigned_drone === undefined ? [] : (["kill_drone"] as ConfirmableAct[])),
    ...(over ? [] : (["kill_job"] as ConfirmableAct[])),
  ];
  // The lead, on the render that has one. Redispatch where Fleet offers it,
  // and the report where it does not — the two are the only non-destructive
  // acts this header holds until `Pilot` lands, and the lead is never
  // destructive.
  // Every act this header offers, in the order one would lead. **One control
  // carries them all**: the first is its face and the rest are its menu, so the
  // header never shows two buttons side by side. The face can be an act that
  // ends something, because every act here asks for a confirmation before it
  // does anything, and a stray Enter lands on that dialog's Cancel.
  //
  // Approving leads where a Job waits for it, redispatch leads a stopped Job
  // that can be redispatched, and the report leads one that cannot. After those,
  // the acts in `acts`' own order, then raising a ceiling, which is a decision
  // about spending and never the face while anything else is offered.
  const hasSpend = whole?.spend !== undefined;
  const forMoney = heldForMoney(job) && hasSpend;
  const forTurns = heldForTurns(job) && hasSpend;
  const act = (name: ConfirmableAct): Entry => ({
    face: ACT_LABEL[name],
    label: MENU_LABEL[name],
    danger: true,
    onSelect: () => onAct(name, job.id),
  });
  const entries: Entry[] = [
    ...(job.status === "awaiting_approval"
      ? [{ face: approving ? "Approving" : APPROVE_LABEL, label: APPROVE_LABEL, onSelect: () => onApprove(job.id) }]
      : []),
    ...(acts.includes("redispatch") ? [act("redispatch")] : []),
    ...(render === "stopped"
      ? [{ face: REPORT_LABEL, label: REPORT_LABEL, shortcut: REPORT_KEY, onSelect: () => onReporting(true) }]
      : []),
    ...acts.filter((name) => name !== "redispatch").map(act),
    ...(forMoney ? [{ face: RAISE_CAP_LABEL, label: RAISE_CAP_LABEL, onSelect: () => onRaising(true) }] : []),
    ...(forTurns
      ? [{ face: RAISE_TURN_CAP_LABEL, label: RAISE_TURN_CAP_LABEL, onSelect: () => onRaisingTurns(true) }]
      : []),
  ];
  const [lead, ...behind] = entries;
  // The accent says a person is waited on and nothing else does. A terminal
  // Job's control is quiet, because there is nobody it is waiting for.
  const variant = job.status === "awaiting_approval" || life?.whoIsActing === "Person" ? "primary" : "secondary";
  const busy = acting || stale || approving;
  return (
    <>
      {/* The ceilings' dialogs, on a job held for money or for turns. Their
          entries are in the one control below; these draw no button. */}
      {forMoney && whole?.spend !== undefined ? (
        <RaiseCapControl
          trigger={false}
          jobId={job.id}
          spend={whole.spend}
          disabled={acting || stale}
          open={raising && !stale}
          onOpen={onRaising}
          onRaise={onRaiseCap}
        />
      ) : null}
      {forTurns && whole?.spend !== undefined ? (
        <RaiseTurnCapControl
          trigger={false}
          jobId={job.id}
          spend={whole.spend}
          disabled={acting || stale}
          open={raisingTurns && !stale}
          onOpen={onRaisingTurns}
          onRaise={onRaiseTurnCap}
        />
      ) : null}
      {/* The report's dialog, on every stopped job. A job nothing can be done
          to is the one most likely to have failed wrongly and been left. */}
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
      {/* A split button with nothing in its menu is a button: a caret over an
          empty menu is a control that does not answer. */}
      {lead === undefined ? null : behind.length === 0 ? (
        <Button variant={variant} disabled={busy} onClick={lead.onSelect}>
          {lead.face}
        </Button>
      ) : (
        <SplitButton
          variant={variant}
          items={behind.map(({ face: _face, ...item }) => item)}
          disabled={busy}
          menuLabel="Everything else this job can do"
          onAction={lead.onSelect}
        >
          {lead.face}
        </SplitButton>
      )}
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
 * The one reason a queued job carries that does not clear on its own —
 * `enum-verbs.toml`'s spelling, named once rather than typed at the comparison.
 * Rename it there and the raise controls stop being offered, which is visible
 * rather than silent.
 */
const OVER_BUDGET = "over_budget";

/** `budget_hold`'s two spellings, named here for `OVER_BUDGET`'s reason. */
const COST_CAP = "cost_cap";
const TURN_CAP = "turn_cap";

/**
 * Whether a queued job is over budget at all, on either ceiling.
 *
 * **The wire's answer and not a comparison made here.** `queued_reason` is
 * computed by Fleet from the same predicate admission asks, and a screen that
 * compared the spend to the cap for itself would be a second reading — right
 * until the two disagreed on a job Fleet was already starting.
 */
function overBudget(job: JobSummary): boolean {
  return job.status === "queued" && job.queued_reason === OVER_BUDGET;
}

/**
 * Whether this job is stopped for money.
 *
 * **Not every over-budget job.** A job at its turn cap carries the same
 * `over_budget` reason but is not started by more money — `budget_hold` tells
 * the two apart.
 *
 * **A Fleet that sends no `budget_hold` falls here** — the cost cap had a
 * route before the field existed, and an older Fleet refuses
 * `raise_turn_cap`. A spelling this build has never heard of falls to
 * neither control: a third ceiling would have a third act.
 *
 * Exported: `B` opens this dialog from `JobDetail.tsx`, and a binding offered
 * where the control is not would answer nothing.
 */
export function heldForMoney(job: JobSummary): boolean {
  return overBudget(job) && (job.budget_hold === COST_CAP || job.budget_hold === undefined);
}

/**
 * Whether this job is stopped for turns.
 *
 * **It reads the field and never its absence**, which is `heldForMoney`'s rule
 * turned around: a Fleet that does not say which ceiling caught the job has no
 * route to raise this one either, and a control offered there would be a press
 * answered with a 404.
 *
 * Exported for `heldForMoney`'s reason — `T` opens the dialog from
 * `JobDetail.tsx`.
 */
export function heldForTurns(job: JobSummary): boolean {
  return overBudget(job) && job.budget_hold === TURN_CAP;
}

/** One act the header offers: its face as the lead, its line in the menu. */
type Entry = SplitButtonItem & { face: string };

/** The approval act, which is the screen's own and not a `JobAct`. */
const APPROVE_LABEL = "Approve dispatch";

