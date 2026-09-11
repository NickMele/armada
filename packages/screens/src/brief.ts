// What chapter one's `steps` and `checks` sections read, off data Bridge
// already holds rather than off Fleet's own prose.
//
// **Three positions, because that is what Fleet's own rail ever says to a
// Drone.** A step is behind the one it is on, the one it is on, or ahead of
// it — `briefing.rs` never tells a Drone a step retried or which Check
// failed, so a structured reading that claimed more would answer a question
// the brief itself never raises. `run.ts`'s richer `StepActivity` answers a
// different question — where the work is, for a person watching — and reads
// off `job_steps.state` rather than off a step's position in the order.

import type { BriefStep } from "@armada/components";
import type { StepDetail } from "@armada/protocol";
import { nameOf } from "./declared";

/**
 * The Job's steps, read against the one a Drone is working now.
 *
 * **Position, not state.** `whole.steps` arrives "one entry per step of the
 * frozen WorkflowDef, in order" — Fleet's own guarantee — so ordinal
 * comparison against `currentStepId` is enough to say what Fleet's own rail
 * says: done, here, or not yet.
 */
export function briefStepsOf(steps: readonly StepDetail[], currentStepId: string): BriefStep[] {
  const at = steps.findIndex((step) => step.step_id === currentStepId);
  return steps.map((step, i) => ({
    id: step.step_id,
    label: step.label,
    position: at === -1 ? "not_yours" : i < at ? "done" : i === at ? "current" : "not_yours",
  }));
}

/**
 * This step's declared Checks, by the name a chip already reads for one.
 *
 * **`undefined` covers two different absences, deliberately.** `checks`
 * missing is "Fleet cannot say"; empty is a step that gates on nothing, and
 * `briefing.rs` writes no `checks` heading for one — so a `checks` section on
 * screen at all already implies a non-empty list, and either absence falls
 * back to the section's own words rather than an empty row of chips.
 */
export function briefChecksOf(step: StepDetail): readonly string[] | undefined {
  const checks = step.checks;
  return checks === undefined || checks.length === 0 ? undefined : checks.map(nameOf);
}
