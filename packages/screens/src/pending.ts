// Which act is out on a Job, named, so the control that sent it is the one that
// waits rather than one grey among its siblings. #1117.

import type { ButtonAnswer, DecisionAct } from "@armada/components";
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
  | "rerun_failed_checks"
  | "investigate_failed_checks";

const DECISIONS: readonly string[] = ["merge", "approve", "changes", "reject"];

/** One of the four answers `ReviewDecision` draws, rather than another act at the gate. */
export function isDecision(act: DecidingAct | undefined): act is DecisionAct {
  return act !== undefined && DECISIONS.includes(act);
}

/**
 * What Fleet said to the last act sent on this Job, and which act that was, so
 * the control that sent it is the one whose edge answers. The app clears it as
 * the next act goes out, and once the line's own hold is over.
 */
export type ActAnswer = { act: ActingAct | DecidingAct; answer: ButtonAnswer };

/** The answer a control shows: set only where `act` is the one that was answered. */
export function answerTo(answered: ActAnswer | undefined, act: ActingAct | DecidingAct): ButtonAnswer | undefined {
  return answered?.act === act ? answered.answer : undefined;
}
