// The workflow a proposal names, drawn before anything runs.
//
// **This is the same rail `rail.ts` builds, one moment earlier.** After the
// fact the rail says what happened; before it, this is what somebody is
// agreeing to when they press Propose — a workflow that will stop and wait for
// them at `handoff`, and will spend two Judge calls at `implement`, previewed
// as neither until `judge_checks` and `advance_gate` crossed on `WorkflowStep`.
//
// Every word on a row comes from `declared.ts`, so the two moments cannot say
// different things about one declaration.
//
// # Nothing here has a result, and no row pretends to
//
// The running rail's gate rows carry a glyph and an outcome, and its
// declaration rows carry `not reached`. **A preview carries neither**: no step
// exists yet, so "not reached" would be a state about a Job nobody has created,
// and `shield-*` is the Checks' verdict family — spending one on a Check that
// has not been asked to run would draw a result where there is only a promise.
// Label-only, which is the treatment #164 settled for declaration rows and the
// same reasoning one step further.

import type { WorkflowRailStep } from "@armada/components";

import type { WorkflowStep, WorkflowSummary } from "../../shared/setup";
import { advanceOf, commandOf, judgeOf } from "./declared";

/**
 * One workflow's steps, as a rail a proposal can be read against.
 *
 * **Every step is `not_started`, because none of them has been.** That is the
 * mark that draws its own ordinal and claims nothing, and it is the honest
 * activity for a workflow with no Job behind it — nothing here derives a state,
 * a duration or a current row, all three of which belong to a Job that exists.
 */
export function previewOf(workflow: WorkflowSummary | undefined): WorkflowRailStep[] {
  return (workflow?.steps ?? []).map((step) => ({
    id: step.step_id,
    // Fleet substitutes the `step_id` where the definition declares no label,
    // so what arrived is drawn and nothing composes a name — in mono only where
    // what arrived *is* the id, which is the one case a reader needs told apart.
    label: step.label,
    labelIsAnIdentifier: step.label === step.step_id || undefined,
    activity: "not_started",
    gates: gatesOf(step),
    declarations: declarationsOf(step),
  }));
}

/**
 * The Checks the step declares, or none.
 *
 * **Empty is answered by the rail's own ungated sentence, not by an empty
 * list.** `WorkflowStep.checks` is always a list and never absent — a workflow
 * Fleet is serving is one Fleet holds — so unlike the running rail there is no
 * "Fleet cannot say" case here, and `undefined` means the step gates on nothing.
 */
function gatesOf(step: WorkflowStep): WorkflowRailStep["gates"] {
  if (step.checks.length === 0) return undefined;
  return step.checks.map((check) => ({ command: commandOf(check) }));
}

/**
 * What will look at the step beyond its Checks: the Judge it declares, and the
 * gate it advances through.
 *
 * **A step carrying one of these is not ungated and must not say it is.** The
 * rail counts declarations with the Check rows when it decides whether to fall
 * back to "no check on this step" — which is what `handoff`, `human_always` in
 * six of the seven shipped workflows, read as before either field crossed.
 *
 * **`auto` draws no row**, per `declared.ts`: it says the mechanical tier is
 * the whole gate, and a row on every step of every workflow would bury the two
 * values a person is actually deciding on.
 */
function declarationsOf(step: WorkflowStep): WorkflowRailStep["declarations"] {
  const declared = step.judge_checks.map((judge) => ({ label: judgeOf(judge) }));
  const advance = advanceOf(step.advance_gate);
  return advance === undefined ? declared : [...declared, advance];
}
