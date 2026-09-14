// Which act is out on a Job, named, so the control that sent it is the one that
// waits rather than one grey among its siblings. #1117.

import type { DecisionAct } from "@armada/components";
import type { JobAct } from "./Acts";

/** Every act Bridge sends on a Job under `acting`. */
export type ActingAct =
  | JobAct
  | "answer"
  | "answer_command"
  | "answer_judge"
  | "set_when_blocked"
  | "set_when_refused"
  | "set_model"
  | "set_review_model"
  | "remove_allowed_command"
  | "raise_cost_cap"
  | "raise_turn_cap";

/** Every act sent under `deciding`: the four review answers, and the rest at that gate. */
export type DecidingAct =
  | DecisionAct
  | "take_up_remarks"
  | "dismiss_finding"
  | "queue_after_finding"
  | "file_finding_issue"
  | "resolve_conflict"
  | "rerun_failed_checks"
  | "investigate_failed_checks";

const DECISIONS: readonly string[] = ["merge", "approve", "changes", "reject"];

/** One of the four answers `ReviewDecision` draws, rather than another act at the gate. */
export function isDecision(act: DecidingAct | undefined): act is DecisionAct {
  return act !== undefined && DECISIONS.includes(act);
}
