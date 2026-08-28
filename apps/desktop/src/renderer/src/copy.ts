// What Bridge says when it refuses something, and what it says before it does
// something that ends something.
//
// Beside `App.tsx` rather than inside it because it is copy: every sentence
// here is read by a person and none of it is wiring. **No status label is
// among them** — those come from the generated vocabulary, which is what stops
// a second one being typed into a component.

import type { DialogTone } from "@armada/components";

import type { Outcome } from "../../shared/bridge";
import type { ConfirmableAct } from "./JobDetail";

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
    case "already_redirecting":
      return "That redirect is already in flight. It was not sent twice.";
    case "already_restarting":
      return "That restart is already in flight. It was not sent twice.";
    case "already_overruling":
      return "That override is already in flight. It was not sent twice.";
    case "empty_instruction":
      return "A redirect needs an instruction. Nothing was sent.";
    case "empty_reason":
      return "An override needs a reason. Nothing was sent, and the judge's verdict stands.";
    case "already_deciding":
      return "A decision on that job's work is already in flight. It was not sent twice.";
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
      "can only narrow — and where the worktree itself is gone, Fleet refuses this and names a " +
      "redispatch instead.",
  },
};
