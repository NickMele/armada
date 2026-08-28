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
//
// # A third act, and it is not one of the two
//
// Overruling a verdict is decided here too, and for the same reason: the screen
// says what a person may do and the header offers it, and those two must agree.
// It is not a third value of `act`, because it is not exclusive with either —
// `crates/fleet/src/overruling.rs` says a Drone being there decides only how the
// Job carries on, never whether the act applies. What decides that is the
// trigger on the stopped step, and it is read off `last_verdict` because that is
// where Fleet reads it.

import { JOB_STATUS } from "../../shared/generated/vocabulary";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "../../shared/protocol";
import { ordered } from "./facts";

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
 *
 * **`overrule` is beside `act` and not one of its values.** Redirect and
 * restart are mutually exclusive because the Drone decides which one Fleet will
 * take; overruling a verdict is the same act whether or not a process is still
 * holding the session, so it is offered alongside whichever of the two applies
 * rather than instead of it. `docs/concepts/job.md` says so in the table of
 * five.
 */
export type Recourse = {
  act?: "redirect" | "restart_step";
  overrule?: Overrule;
  note: string;
};

/**
 * The verdict a person may overrule, and what overruling it is about to do.
 *
 * `commits` is the difference between the two outcomes `overruling.rs` produces
 * and is not a nicety: overruling a middle step advances it and the Job carries
 * on at the next one, and overruling the last step makes Fleet commit and
 * deliver. A control that said only "overrule" would be the same button for
 * both.
 */
export type Overrule = {
  /** The step the Judge refused. What the confirmation names. */
  step: StepDetail;
  /** Whether that step is the workflow's last, so the Job lands rather than runs on. */
  commits: boolean;
};

/**
 * What is left to do with a Job that stopped.
 *
 * **Every branch says what applies and what does not.** A sentence that named
 * only the act on offer would leave a person wondering whether the other one
 * was hidden or refused, which is the question this exists to close.
 *
 * The detail is a second argument because the override needs it and the two
 * resume acts do not: which trigger stopped the step, and whether that step is
 * the last one, are on `GET /jobs/:job_id` and on no Board row. It is `null`
 * while the read is in flight, and the override is not offered then — a button
 * that could not yet say whether it commits the Job is the one thing this act
 * must not be.
 */
export function recourseOf(job: JobSummary, whole: JobWhole | null): Recourse {
  // First and outside what follows: the override is legal on `escalated` alone,
  // so a Job that is not there reaches none of the three.
  if (job.status !== ESCALATED) {
    return { note: `${notResumable(job)} ${replacement(job)}` };
  }
  const overrule = overruleOf(whole);
  // Overruling leads wherever it applies, because it takes nothing away and the
  // other two do — `docs/concepts/job.md` orders the five acts that way.
  const said = overrule === undefined ? "" : `${overruling(overrule)} `;
  if (job.current_step_id === undefined) {
    return { note: `${said}${NO_STEP_STOPPED} ${replacement(job)}` };
  }
  if (job.assigned_drone !== undefined) {
    return { act: "redirect", overrule, note: `${said}${REDIRECT} ${replacement(job)}` };
  }
  return { act: "restart_step", overrule, note: `${said}${RESTART} ${replacement(job)}` };
}

/**
 * The one trigger a person may overrule. `crates/fleet/src/overruling.rs` holds
 * it as a constant for the reason it is one here: the whole scope of the act is
 * this value, and a list is where a second entry gets added without anybody
 * arguing for it. `gate_undecided` means the gate never weighed the work and
 * `evidence_suspect` is a claim about the evidence rather than an opinion about
 * it, so neither is a verdict to disagree with.
 */
const OVERRULABLE = "gate_failure";

/** The step state Fleet reads the overrulable verdict off. */
const STOPPED = "stopped";

/**
 * Whether this Job's stopped step is one a person may overrule. The status is
 * the caller's guard, so this only asks about the step.
 *
 * **Read off the step's own `last_verdict`, which is what Fleet reads.** The
 * Job's escalation reason carries the same spelling, and taking it from there
 * would be a second path to one answer — `overridable()` in `overruling.rs`
 * finds the stopped step and looks at the verdict on it, and so does this.
 *
 * The Check guard that function also makes is not restated: a failed mechanical
 * Check ends the Job at `completed_failed`, which stops no step and never
 * reaches this screen. Fleet keeps the guard because it must hold the day the
 * tier ordering moves; Bridge only keeps a button off a screen, and a button
 * Fleet refuses is a 409 a person reads.
 */
function overruleOf(whole: JobWhole | null): Overrule | undefined {
  if (whole === null) return undefined;
  const steps = ordered(whole);
  const stopped = steps.find((step) => step.state === STOPPED);
  if (stopped === undefined || stopped.last_verdict?.trigger !== OVERRULABLE) return undefined;
  return { step: stopped, commits: steps[steps.length - 1]?.step_id === stopped.step_id };
}

/**
 * What overruling this verdict does, in the two shapes it has.
 *
 * **Both halves are said, and the second is the one that changes.** Overruling
 * a middle step continues the Job and overruling the last one commits and
 * delivers, so a sentence that stopped at "the job carries on" would be wrong
 * on exactly the case where being wrong costs the most.
 */
function overruling(overrule: Overrule): string {
  return (
    "Overrule the verdict. The judge refused this step and a person may disagree: the step " +
    "advances still carrying the refusal, so what the judge said stays beside the fact that it " +
    `did not stand. ${onwards(overrule)} The reason given is written to the job's log, which is ` +
    "append-only."
  );
}

/**
 * What the Job does after the refused step advances. **The one fact the two
 * outcomes differ by**, written once: the screen's sentence carries it and the
 * confirmation carries it, and a person must not read one of them and press the
 * other.
 */
export function onwards(overrule: Overrule): string {
  return overrule.commits
    ? "It is the last step of this workflow, so overruling it commits the work and delivers it."
    : "It is not the last step, so the job carries on at the next one.";
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
