// What Bridge says when it refuses something, and what it says before it does
// something that ends something.
//
// Beside `App.tsx` rather than inside it because it is copy: every sentence
// here is read by a person and none of it is wiring. **No status label is
// among them** — those come from the generated vocabulary, which is what stops
// a second one being typed into a component.

import type { Outcome } from "../../shared/bridge";
import type { JobAct } from "./JobDetail";

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
 */
export const CONFIRM: Record<JobAct, { title: string; body: string }> = {
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
  redispatch: {
    title: "Redispatch this job as a new one?",
    body:
      "This job is killed and a replacement is created carrying a reference back to it. " +
      "Nothing resumes: the new job starts at the approval gate and needs releasing. " +
      "The failed job's worktree and branch are left as its drone left them.",
  },
};
