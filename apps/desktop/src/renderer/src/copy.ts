// What Bridge says when it refuses something, and what it says before it does
// something that ends something.
//
// Beside `App.tsx` rather than inside it because it is copy: every sentence
// here is read by a person and none of it is wiring. **No status label is
// among them** — those come from the generated vocabulary, which is what stops
// a second one being typed into a component.
//
// The act labels moved here from `Acts.tsx` when the controls that own a
// dialog were split out of it. They are the words on a button, which is this
// file's subject; which control draws them is that file's.

import type { DialogTone } from "@armada/components";

import type { Outcome } from "../../shared/bridge";
import type { ConfirmableAct, JobAct } from "./JobDetail";

/** What a refusal says. Every one names what happened and what to do. */
export function said(outcome: Outcome): string {
  if (outcome.ok) return "";
  switch (outcome.why) {
    case "empty_title":
      return "A job needs a title. Nothing was created.";
    case "empty_brief":
      return "A job needs a brief. Nothing was created.";
    case "no_workflow":
      return "A job needs a workflow Fleet holds. Nothing was sent.";
    case "no_manifest":
      return "A job needs a manifest Fleet holds. Nothing was sent.";
    case "not_connected":
      return "Fleet is not connected. Nothing was sent.";
    case "already_approving":
      return "That approval is already in flight. It was not sent twice.";
    case "already_redispatching":
      return "That redispatch is already in flight. It was not sent twice.";
    case "already_killing":
      return "That kill is already in flight. It was not sent twice.";
    case "already_forgetting":
      return "That job is already being forgotten. It was not sent twice.";
    case "already_redirecting":
      return "That redirect is already in flight. It was not sent twice.";
    case "already_restarting":
      return "That restart is already in flight. It was not sent twice.";
    case "already_overruling":
      return "That override is already in flight. It was not sent twice.";
    case "already_rereading":
      return "That gate is already being re-run. It was not asked twice.";
    case "empty_report":
      return "A report needs what you know went wrong. The record on its own says nothing that was not already on the job.";
    case "already_reporting":
      return "That report is already being filed. It was not sent twice.";
    case "empty_instruction":
      return "A redirect needs an instruction. Nothing was sent.";
    case "empty_reason":
      return "An override needs a reason. Nothing was sent, and the judge's verdict stands.";
    case "already_deciding":
      return "A decision on that job's work is already in flight. It was not sent twice.";
    case "already_answering":
      return "That answer is already in flight. It was not sent twice.";
    case "empty_note":
      return "Requesting changes needs a note. Nothing was sent, and the job is still waiting.";
    case "refused":
      // Drawn as a failure notice above, with everything it carries.
      return "";
    case "transport":
      return `Fleet did not answer: ${outcome.detail}`;
  }
}

/**
 * What each confirmation says. **What happens and what survives** — the two
 * halves the copy contract asks for, and the reason the two kills cannot share
 * one dialog: they survive differently.
 *
 * Neither the step nor the elapsed is named here. The design's sample line
 * carries both, and neither is a fact this dialog holds — the detail behind it
 * does, on screen while the dialog is open.
 *
 * `tone` defaults to destructive in `Dialog` itself; only `restart_step` says
 * otherwise, because it is a recovery, not an end. Redirect and the override
 * carry no entry — each collects a required field in its own dialog and is its
 * own confirmation, so neither reaches this one.
 */
export const CONFIRM: Record<ConfirmableAct, { title: string; body: string; tone?: DialogTone }> = {
  kill_drone: {
    title: "Kill the drone on this job?",
    body:
      "The process stops and the job stays open. Its worktree is held as the drone left it, " +
      "so the job can be redispatched from where it got to.",
  },
  kill_job: {
    title: "Kill this job?",
    body:
      "The job ends at killed. That is terminal and carries no verdict — nothing resumes it, " +
      "and anything the drone wrote stays on its branch.",
  },
  // Offered on three statuses, two of them already terminal — so the body can
  // neither call this "the failed job" nor promise a kill that already
  // happened. Fleet rewrites no terminal status.
  redispatch: {
    title: "Redispatch this job as a new one?",
    body:
      "A replacement is created carrying a reference back to this job, and this job does not " +
      "continue: one still open is killed, and one that already ended is left as it stands. " +
      "Nothing resumes — the new job starts at the approval gate and needs releasing, and this " +
      "job's worktree and branch stay as its drone left them.",
  },
  restart_step: {
    title: "Restart this step?",
    tone: "neutral",
    body:
      "A fresh drone takes over on the same worktree, at the step the last one stopped at. " +
      "The toolset, model and environment are resolved again from scratch, so a widened scope " +
      "can only narrow. Fleet read the worktree before offering this, so there is one to take " +
      "over.",
  },
};

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
 *
 * **The re-run says "ask", not "approve", "accept", "retry" or "override".**
 * Nothing ruled on that step, so there is no verdict to accept and none to
 * overrule; and nothing the drone did is redone, so it is not a retry. What
 * happens is that a gate which could not answer is asked again, and the label
 * is that sentence with the words taken out.
 */
export const ACT_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone",
  kill_job: "Kill job",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone",
  restart_step: "Restart step",
  override_verdict: "Overrule the verdict",
  rerun_gate: "Ask the gate again",
};

/**
 * The same acts inside the menu, where each says what survives it. A caret hides
 * the consequence that a button's own position states, so the label has to carry
 * it — `Kill drone` and `Kill job` differ by everything and by three characters.
 *
 * **Redirect, restart, the override and the re-run never reach a menu** — none
 * of them joins the split button, so those four entries exist only to keep the
 * record total over `JobAct` rather than for anything that reads them today.
 */
export const MENU_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone, the job stays open",
  kill_job: "Kill job, it ends here",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone, the job stays open",
  restart_step: "Restart the step, on the same worktree",
  override_verdict: "Overrule the verdict, the refused work stands",
  rerun_gate: "Ask the gate again, on the evidence already submitted",
};
