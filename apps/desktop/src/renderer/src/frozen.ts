// What a Job's status says about the step beneath it.
//
// `job-statuses.toml` gives `completed_failed` no `step_states` on purpose: the
// step keeps whatever state it had, and the Job being terminal is what says
// everything is over. `escalated` declares `["stopped"]` for the opposite
// reason — that stop is resumable, and a redirect or a restart resumes exactly
// that step. So a frozen step and a stopped one are two different facts, and a
// rail that drew them alike would invite a control that cannot work.
//
// **This is a rendering rule, not a seventh state.** The wire still says
// `running` under a failed Job and that is correct. Nothing here is sent, and
// nothing here is a state Fleet does not have.

import type { StepActivity } from "@armada/components";

import { JOB_LIFECYCLE, JOB_STATUS } from "../../shared/generated/vocabulary";

/**
 * The step states that claim the Job is still moving — being worked, or owed a
 * person. `step-states.toml` sees each of the three under `running` and
 * `awaiting_review` and under nothing else, and neither status is terminal, so
 * beneath a Job that is over none of the three can still be true.
 *
 * **`not_started` is not one of them.** A step the Job never reached is
 * honestly not started whatever ended the Job above it, and `advanced` and
 * `stopped` are already settled. Only a live claim is overridden.
 */
const LIVE: readonly string[] = ["running", "retrying", "awaiting_human"];

/**
 * The step activity a Job's status implies, and the one place it is written.
 * A list row's current segment takes it always; a rail takes it only where the
 * Job is over and the step still reads live.
 *
 * **Every row is the icon registry's borrowing convention** — a step carries
 * the Job glyph that means the same thing one level down, so `completed_failed`
 * lands on `failed` (`x`), `killed` on `killed` (`power`), and
 * `completed_success` on `advanced` (`check`). `rejected` (`ban`) and
 * `superseded` (`archive`) have no step counterpart in any roster and take
 * none; see `frozenBeneath` for what they draw instead. Reported.
 */
const ACTIVITY_FOR_STATUS: Readonly<Record<string, StepActivity>> = {
  running: "running",
  completed_failed: "failed",
  completed_success: "advanced",
  killed: "killed",
};

export function activityFor(status: string): StepActivity | undefined {
  return ACTIVITY_FOR_STATUS[status];
}

/** How a step reads once the Job above it is over. */
export type Frozen = {
  activity: StepActivity;
  /** The word at the row's trailing edge, in place of the step's own state. */
  word: string;
};

/**
 * What a terminal Job does to a step that still reads live, or nothing where
 * the step's own state stands.
 *
 * **Terminality is read, never listed.** `JOB_LIFECYCLE` carries it from
 * `job-statuses.toml`, so a status added there is terminal here the day it is
 * generated rather than the day somebody remembers this file.
 *
 * A terminal status with no step counterpart falls to `not_started`, the mark
 * that claims nothing and draws its own ordinal. That is wrong about the step
 * and right about what is known — and it still stops the pulse, the hue and
 * the clock, which is the whole of what a terminal Job owes.
 *
 * **The word is the Job's own verb**, borrowed from the generated vocabulary.
 * `enum-verbs.toml` has no `step_state` rows at all, and it is the Job's status
 * that carries the fact that the step is over, so the word comes from where
 * that fact is written rather than being chosen here.
 */
export function frozenBeneath(status: string, state: string): Frozen | undefined {
  if (JOB_LIFECYCLE[status]?.terminal !== true) return undefined;
  if (!LIVE.includes(state)) return undefined;
  return {
    activity: activityFor(status) ?? "not_started",
    word: JOB_STATUS[status]?.verb ?? status,
  };
}
