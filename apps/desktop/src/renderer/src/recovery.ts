// What still resumes a Job that stopped, and the sentence that says so.
//
// # One answer, two consumers
//
// The stopped screen states which act applies and the header offers it, and
// those two must not be able to disagree — a screen that says "restart the
// step" beside a header with no restart button is worse than either alone. So
// the predicate is here, `Acts.tsx` reads it for which control to draw, and
// `Stopped.tsx` reads it for the words.
//
// # Read off `adrift.rs`, which is the authority
//
// `crates/fleet/src/adrift.rs` carries five refusals for the two resume acts,
// and four of them are decidable from what the wire already serves:
//
// | Refusal | What Fleet is refusing | What Bridge reads |
// |---|---|---|
// | `NotResumable` | a Job that is not `escalated` | `job.status` |
// | `NoStepStopped` | an escalation that named no step | `job.current_step_id` |
// | `NoDroneToRedirect` | a redirect with the Drone gone | `job.assigned_drone` |
// | `DroneStillThere` | a restart with the Drone alive | `job.assigned_drone` |
// | `WorktreeGone` | a restart onto a worktree that is not there | nothing |
//
// **The fifth is named rather than predicted.** Bridge does not read the
// filesystem, so whether the worktree survived is an answer that only comes on
// the press — and the copy says that instead of implying a restart is certain.

import { JOB_STATUS } from "../../shared/generated/vocabulary";
import type { JobSummary } from "../../shared/protocol";

/**
 * The one status the two resume acts are legal on. Written here rather than
 * read from the generated vocabulary because no registry file carries the
 * answer: `job-statuses.toml` says which statuses are terminal, and
 * `escalated` is not terminal for reasons that have nothing to do with this.
 * Fleet's route is the authority and refuses anything else.
 */
const ESCALATED = "escalated";

/**
 * The statuses a redispatch is offered on. Three, and **`rejected` is not one**:
 * a rejected Job never ran, so it has no Facts and no Evidence to carry
 * forward, and redispatching it would only be proposing a new Job — which the
 * composer already does.
 *
 * Written here rather than read from the generated vocabulary because no
 * registry file carries the set: `job-fields.toml` still asks it as an open
 * question on `redispatched_from`. Fleet's route is the authority and refuses
 * anything else; this only keeps a button off the screen that would be.
 */
export const REDISPATCHABLE: ReadonlySet<string> = new Set([
  "escalated",
  "completed_failed",
  "killed",
]);

/**
 * Which of the two resume acts applies to this Job, and what a person reads
 * about it. `act` absent is a Job neither one reaches.
 */
export type Recourse = {
  act?: "redirect" | "restart_step";
  note: string;
};

/**
 * What is left to do with a Job that stopped.
 *
 * **Every branch says what applies and what does not.** A sentence that named
 * only the act on offer would leave a person wondering whether the other one
 * was hidden or refused, which is the question this exists to close.
 */
export function recourseOf(job: JobSummary): Recourse {
  if (job.status !== ESCALATED) {
    return { note: `${notResumable(job)} ${replacement(job)}` };
  }
  if (job.current_step_id === undefined) {
    return { note: `${NO_STEP_STOPPED} ${replacement(job)}` };
  }
  if (job.assigned_drone !== undefined) {
    return { act: "redirect", note: `${REDIRECT} ${replacement(job)}` };
  }
  return { act: "restart_step", note: `${RESTART} ${replacement(job)}` };
}

/**
 * `NotResumable`, in Bridge's words. **The status is named**, because "this job
 * is not escalated" is a sentence a person has to translate and "this one is
 * killed" is one they can read off the badge above it.
 */
function notResumable(job: SummaryStatus): string {
  return (
    "Nothing resumes this job. Redirect and restart both take a job a person is holding, " +
    `which is an escalated one, and this job is ${named(job)}.`
  );
}

/** `NoStepStopped`. The Job is held, and no step of it is. */
const NO_STEP_STOPPED =
  "Nothing resumes this job. It escalated without stopping a step, so redirect and restart " +
  "have no step to land on.";

/** `DroneStillThere` stated as the act it points at rather than as a refusal. */
const REDIRECT =
  "Redirect the drone. Its session, its worktree and every step so far are still held, so an " +
  "instruction reaches it as a new turn at the step above. Fleet refuses a restart while a " +
  "drone is alive, because a restart throws that session away.";

/**
 * `NoDroneToRedirect` stated the same way, and `WorktreeGone` named as the one
 * answer that only arrives on the press.
 */
const RESTART =
  "Restart the step. The drone is gone, so a fresh one takes over the worktree at the step " +
  "above, resolving its toolset, model and environment again. Fleet refuses this where the " +
  "worktree is no longer on disk, and Bridge does not read the filesystem, so that answer " +
  "comes on the press.";

/** What replaces a Job that nothing resumes, or that nothing replaces either. */
function replacement(job: SummaryStatus): string {
  return REDISPATCHABLE.has(job.status)
    ? "A redispatch mints a new job from the approval gate and carries none of the work over."
    : "Nothing replaces it either: a redispatch takes an escalated, failed or killed job. " +
        "Proposing a new job is what is left.";
}

/** Only the status is read, so only the status is asked for. */
type SummaryStatus = Pick<JobSummary, "status">;

/**
 * The Job's status in the registry's word. **Never a word chosen here** — the
 * wire spelling stands in where the registry carries none, the same fallback
 * every other surface takes rather than inventing a second vocabulary.
 */
function named(job: SummaryStatus): string {
  return JOB_STATUS[job.status]?.verb ?? job.status;
}
