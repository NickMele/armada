// What still resumes a Job that stopped, and the sentence that says so.
//
// # Over 500 lines, and left whole
//
// Almost all of it is the sentences, and they are the reason to keep them here.
// Each one states an act and its precondition in the same breath — "the drone
// is gone, so the job goes back in the queue" — and the classification above
// them is what decides which act applies. Split the copy out and the two halves
// can disagree silently, which is the exact failure the section below says this
// file exists to prevent, one level up: a screen offering a restart beside a
// header with no restart button. The reading and the words go together.
//
// # One answer, two consumers
//
// The stopped screen states which act applies and the header offers it, and
// those two must not be able to disagree — a screen that says "restart the
// step" beside a header with no restart button is worse than either alone. So
// the reading is here, `Acts.tsx` reads it for which control to draw, and
// `Stopped.tsx` reads it for the words.
//
// # Fleet decides which acts apply, and this draws them
//
// This file used to derive them from `status`, `current_step_id` and
// `assigned_drone`, reaching four of the five refusals
// `crates/fleet/src/adrift.rs` carries. The fifth is whether the worktree is
// still on disk: a renderer reads no filesystem, so a restart was offered on a
// Job whose worktree had been reclaimed and the refusal arrived on the press.
// `stuck` on `GET /jobs/:job_id` is Fleet's own answer — read off the disk, the
// store and the slot this Job's own Drone holds, never a neighbour's — and
// `stuck.recourse` names each act as the operation that performs it.
//
// **There is no second path to it.** Nothing re-derives an act where `stuck` is
// absent: absent is Fleet classifying nothing, and a fallback would resurrect
// the bug for exactly the Jobs it was written for. **And the set is not
// closed** — `rerun_gate` arrived after `docs/concepts/job.md` wrote its table
// of five, so an act this Bridge has no control for is named to the person
// rather than dropped.
//
// What is still decided here is words, not acts: which sentence an act is
// offered in, which step it names, and whether overruling that step commits the
// Job. The first two are `OVERRULING` keyed by `stuck.stopped_by`; the third is
// the frozen step list's own shape, and Fleet serves no field for it.
//
// # The two acts beside `act`, and why they are beside it
//
// Overruling a verdict is not exclusive with either resume act —
// `crates/fleet/src/overruling.rs` says a Drone being there decides only how
// the Job carries on. **Two triggers reach it and the words change with
// which**: a Judge's refusal overruled and a gaming flag overruled are
// different things a person is doing, so the sentences are one record and
// admitting a trigger costs the words to describe it.
//
// `gate_undecided` is the gate declining to rule in either direction. There is
// nothing to overrule, so what is left is to ask again —
// `crates/fleet/src/regating.rs` — and that runs out of the Job's own slot, so
// it co-occurs with a redirect rather than replacing it.

import { JOB_STATUS } from "@armada/components";
import type { JobDetail as JobWhole, JobSummary, RedirectInFlight, StepDetail, Stuck } from "@armada/protocol";
import type { JobAct } from "./Acts";
import { clock } from "./duration";
import { ordered } from "./facts";

/**
 * The one status the two resume acts are legal on. **Kept for the words and no
 * longer for the acts**: `stuck.recourse` says which apply, and this only
 * chooses which sentence explains an absence.
 */
const ESCALATED = "escalated";

/**
 * The acts, spelled as `crates/ipc/operations.toml` keys the operation that
 * performs each — which is how `stuck.recourse` names them. **Five names and
 * not the set**: Fleet declares the set by the acts it implements, so it may
 * name one this Bridge was built before, which `unreachable` says out loud.
 */
const OVERRIDE_VERDICT = "override_verdict";
const RERUN_GATE = "rerun_gate";
const REDIRECT_DRONE = "redirect_drone";
const RESTART_STEP = "restart_step";
const REDISPATCH_JOB = "redispatch_job";

/**
 * Which of the two resume acts applies to this Job, and what a person reads
 * about it. `act` absent is a Job neither one reaches.
 *
 * **`overrule` and `reread` are beside `act` and not values of it.** Redirect
 * and restart are exclusive because the Drone decides which one Fleet will
 * take; the other two turn on the trigger instead, so each is offered alongside
 * whichever of the two applies. `docs/concepts/job.md` says so in its table.
 *
 * **`overrule` and `reread` are never both present**, because the two triggers
 * partition: `overrulable()` refuses what `undecided_step()` admits.
 */
export type Recourse = {
  act?: "redirect" | "restart_step";
  overrule?: Overrule;
  reread?: Reread;
  /**
   * Whether Fleet will mint a replacement. **A field rather than a status test
   * here**: two of the three things it turns on are on no row — whether Fleet
   * still holds the workflow, and whether a person raised the Job.
   */
  redispatch: boolean;
  /**
   * One sentence per act on offer, keyed by the act — **for that act's own
   * tooltip, on hover and on focus**, which is where the journey puts a step's
   * help text.
   *
   * They were concatenated into one ninety-word paragraph above the controls,
   * describing four acts in the imperative — *Overrule the verdict.* *Redirect
   * the drone.* — to a person who then had to find the matching button
   * elsewhere on the screen. The sentences are good and were in the wrong
   * place. Which acts apply is still decided here; what changed is that the
   * reading reaches a person through the control it is about.
   */
  says: Partial<Record<JobAct, string>>;
  /**
   * Why an act a person is looking for is not here. **One line, and only where
   * the absence is the interesting fact** — a restart withheld because the
   * Drone is alive, or an act this Fleet offers and this Bridge has no control
   * for.
   */
  withheld?: string;
  /** Where the step stands, in the panel's own voice. Never a menu. */
  stands: string;
};

/**
 * The step whose gate could not decide, and so the step a re-run reads again.
 *
 * **A record and not a boolean**, for `Overrule`'s reason: the act is about one
 * step and "yes" would leave the screen unable to name which. Nothing beyond it
 * is carried, because a re-run takes no reason and has no second outcome.
 */
export type Reread = {
  /** The step that stopped, undecided. What the sentence is about. */
  step: StepDetail;
};

/**
 * The decision a person may overrule, and what overruling it is about to do.
 * **A decision and not a verdict**: a Judge's refusal is one, and a gaming
 * check calling the evidence suspect is the other, which is never a verdict.
 *
 * `commits` is the difference between the two outcomes `overruling.rs`
 * produces: a middle step advances and the Job runs on, and the last one makes
 * Fleet commit and deliver. "Overrule" alone is one button for both.
 */
export type Overrule = {
  /** The step that stopped. What the confirmation names. */
  step: StepDetail;
  /**
   * Which machine's decision is being overruled. **Not a detail of the act but
   * the subject of it** — a refusal overruled and a flag overruled are two
   * different things a person is doing, and every word on the screen and in the
   * confirmation is chosen off this.
   */
  trigger: Overruled;
  /** Whether that step is the workflow's last, so the Job lands rather than runs on. */
  commits: boolean;
};

/** The triggers this file has words for. Fleet decides which are overrulable. */
export type Overruled = "gate_failure" | "evidence_suspect";

/**
 * What is left to do with a Job that stopped.
 *
 * **Every branch says what applies and what does not.** A sentence that named
 * only the act on offer would leave a person wondering whether the other one
 * was hidden or refused, which is the question this exists to close.
 *
 * The detail is a second argument because the answer is on it and on no Board
 * row. It is `null` while the read is in flight, and nothing is offered then —
 * naming a Job's acts off the row is the derivation this stopped doing.
 */
export function recourseOf(job: JobSummary, whole: JobWhole | null): Recourse {
  if (whole === null) return { redispatch: false, says: {}, stands: READING };
  const stuck = whole.stuck;
  if (stuck === undefined) return { redispatch: false, says: {}, stands: UNCLASSIFIED };
  const offered = new Set(stuck.recourse);
  const overrule = offered.has(OVERRIDE_VERDICT) ? overruleOf(whole, stuck) : undefined;
  const reread = offered.has(RERUN_GATE) ? rereadOf(whole, stuck) : undefined;
  // Exclusive, and Fleet is what made them exclusive: a redirect wants a live
  // session and a restart wants the Drone gone, so no classification carries
  // both. Read in that order anyway, because a Job holding a Drone is a Job a
  // restart would throw a session away on.
  const act: Recourse["act"] = offered.has(REDIRECT_DRONE)
    ? "redirect"
    : offered.has(RESTART_STEP)
      ? "restart_step"
      : undefined;
  const redispatch = offered.has(REDISPATCH_JOB);
  // Overruling and asking again both lead wherever they apply, because both
  // take nothing away and the other two do — `docs/concepts/job.md` orders the
  // acts on an escalated Job that way. **Never both**: the two triggers
  // partition, so at most one of these two sentences is here.
  const says: Partial<Record<JobAct, string>> = {
    ...(overrule === undefined ? {} : { override_verdict: overruling(overrule) }),
    ...(reread === undefined ? {} : { rerun_gate: REREAD }),
    ...(act === "redirect" ? { redirect: REDIRECT } : {}),
    ...(act === "restart_step" ? { restart_step: RESTART } : {}),
  };
  const drew: Recourse = { act, overrule, reread, redispatch, says, stands: "" };
  // **The answer to the last press leads.** A redirect that is waiting and one
  // that never arrived are the same escalated Job, and the person reading this
  // is usually the person who just sent one — so what happened to it comes
  // before where the step stands.
  const sent =
    act === "redirect" && whole.redirecting !== undefined ? `${waiting(whole.redirecting)} ` : "";
  return {
    ...drew,
    withheld: withheldBy(stuck, drew),
    stands: act === undefined ? `${sent}${stalled(job, stuck)}` : `${sent}${HOLDING}`,
  };
}

/**
 * What the step is doing while it waits for a person, where something can still
 * be done to it. **One sentence about the state**, because what each act does
 * is on that act's own tooltip and a paragraph naming all of them above the
 * buttons is the block this replaced.
 */
const HOLDING =
  "The drone is holding at this step. Nothing advances until you decide what happens next.";

/**
 * Why an act a person is looking for is not on offer. **At most one line**: an
 * absence a reader can see for themselves needs no sentence, and the two worth
 * stating are the restart Fleet refuses while a Drone is alive, and an act this
 * Fleet offers that this Bridge cannot draw.
 */
function withheldBy(stuck: Stuck, made: Recourse): string | undefined {
  const cannot = unreachable(stuck, made);
  if (cannot !== undefined) return cannot;
  return made.act === "redirect" ? RESTART_WITHHELD : undefined;
}

/**
 * `DroneStillThere`, from the other side: why the act a person may be reaching
 * for is not here. It was the fourth sentence of a paragraph about Redirect,
 * which is where a reason for an absent act is least likely to be read.
 */
const RESTART_WITHHELD =
  "Restart is not offered while the drone is alive: a restart throws that session away.";

/**
 * The words one trigger's override is offered in. **Five, because five places
 * say it** — the button, the confirmation's question, its reason field, the
 * screen's sentence and the confirmation's first paragraph — held together so
 * no two of them describe different acts.
 */
export type Overruling = {
  /** The button, and the confirmation's confirm control. **Never "approve"** —
   * approving means the work was right, and this means a machine was wrong. */
  label: string;
  /** The confirmation's question. Sentence case, and it names what happens. */
  asks: string;
  /** The label over the required reason. It asks what is wrong with the
   * decision, never what is right about the work. */
  field: string;
  /** What the act is, on the stopped screen, before what it does. */
  screen: string;
  /** The same, in the confirmation, with the step named — a person arrives here
   * from a rail with several rows on it. */
  dialog: (step: string) => string;
};

/**
 * What one trigger's override is called and what it says, for the two triggers
 * an override reaches.
 *
 * **The scope is Fleet's now and the words are still here.** A trigger Fleet
 * admits and this record has no sentence for draws no button and is named in
 * words, rather than a screen offering a Judge's words for a check that is not
 * the Judge — which is how `evidence_suspect` would have arrived.
 *
 * **Two, since Fleet admitted the second.** `gate_failure` is the Judge
 * refusing a criterion — a judgement about the work. `evidence_suspect` is the
 * gaming check reading a diff and inferring intent — a claim about the
 * evidence, and the owner's rule is that anything a machine decides a person
 * can overrule. `gate_undecided` is absent and that is not an omission: the
 * gate never weighed the work, so there is no ruling to disagree with.
 */
export const OVERRULING: Record<Overruled, Overruling> = {
  gate_failure: {
    label: "Overrule the verdict",
    asks: "Overrule the judge on this step?",
    field: "Why the judge is wrong",
    screen:
      "Overrule the verdict. The judge refused this step and a person may disagree: the step " +
      "advances still carrying the refusal, so what the judge said stays beside the fact that " +
      "it did not stand.",
    dialog: (step) =>
      `The judge refused ${step}. Overruling says the judge was wrong — not that the work was ` +
      "approved. The step advances still recorded as failed, so what the judge said stays " +
      "beside the fact that it did not stand.",
  },
  evidence_suspect: {
    label: "Overrule the flag",
    asks: "Overrule the gaming flag on this step?",
    field: "Why the flag is wrong",
    screen:
      "Overrule the flag. The gaming check called this step's evidence suspect and a person may " +
      "disagree: the step advances still carrying the flag, so what the check found stays " +
      "beside the fact that it did not stand.",
    dialog: (step) =>
      `The gaming check flagged the evidence for ${step}. It did not refuse the work — it says ` +
      "the evidence for it is not to be trusted. Overruling says a person has read that " +
      "evidence and takes responsibility for it; the step advances still recorded as failed " +
      "against the flag.",
  },
};

/**
 * The override Fleet offered, in the words this build has for it. Fleet's
 * answer is the caller's guard, so this reads only what the sentence needs.
 *
 * **The trigger is `stuck.stopped_by` and not the step's own verdict.** They
 * carry the same spelling, because Fleet reads one off the other; taking the
 * published one is one answer where reading both would be two.
 */
function overruleOf(whole: JobWhole, stuck: Stuck): Overrule | undefined {
  const held = stoppedIn(whole, stuck);
  const trigger = stuck.stopped_by;
  if (held === undefined || !worded(trigger)) return undefined;
  return {
    step: held.stopped,
    trigger,
    commits: held.steps[held.steps.length - 1]?.step_id === held.stopped.step_id,
  };
}

/**
 * The re-run Fleet offered, against the step it is about. **Only the step**,
 * because a re-run takes no reason and has no second outcome; `REREAD` says
 * what is left for the press to answer.
 */
function rereadOf(whole: JobWhole, stuck: Stuck): Reread | undefined {
  const held = stoppedIn(whole, stuck);
  return held === undefined ? undefined : { step: held.stopped };
}

/**
 * The step that stopped, and the steps it stopped among.
 *
 * **`stuck.step_id` rather than a search for a stopped state**: which step
 * stopped is one fact and Fleet published it. Absent is every Job-level
 * escalation, which names no step at all.
 */
function stoppedIn(
  whole: JobWhole,
  stuck: Stuck,
): { steps: StepDetail[]; stopped: StepDetail } | undefined {
  if (stuck.step_id === undefined) return undefined;
  const steps = ordered(whole);
  const stopped = steps.find((step) => step.step_id === stuck.step_id);
  return stopped === undefined ? undefined : { steps, stopped };
}

/**
 * Whether this build has words for the trigger Fleet named. **The record above
 * is what admits it**, rather than a second list here that could disagree with
 * it.
 */
function worded(trigger: string | undefined): trigger is Overruled {
  return trigger !== undefined && Object.hasOwn(OVERRULING, trigger);
}

/**
 * The acts Fleet named that nothing here drew, in the wire's own spelling.
 *
 * **Named rather than dropped.** `recourse` is declared by the acts Fleet
 * implements and by no registry, so a Bridge older than its Fleet meets one it
 * has no control for — `rerun_gate` was that act on the day it landed — and an
 * override whose trigger `OVERRULING` cannot word is the same hole from the
 * other side. The spelling arrives as it is, because a label invented for an
 * act this build does not know would be inventing what it does.
 */
function unreachable(stuck: Stuck, made: Recourse): string | undefined {
  const drawn = new Set<string>();
  if (made.overrule !== undefined) drawn.add(OVERRIDE_VERDICT);
  if (made.reread !== undefined) drawn.add(RERUN_GATE);
  if (made.act === "redirect") drawn.add(REDIRECT_DRONE);
  if (made.act === "restart_step") drawn.add(RESTART_STEP);
  if (made.redispatch) drawn.add(REDISPATCH_JOB);
  const undrawn = stuck.recourse.filter((named) => !drawn.has(named));
  return undrawn.length === 0
    ? undefined
    : `This fleet also offers ${undrawn.join(", ")}, which this bridge has no control for. ` +
        "The act is fleet's and it stands; a bridge built from the same commit draws it.";
}

/**
 * What overruling this decision does — **two triggers by two outcomes, and the
 * sentence carries both.** What is overruled is the trigger's words, because a
 * refusal and a flag are different acts; what happens next is `onwards`, which
 * would be wrong on the last step if it stopped at "the job carries on". The
 * cost is the same either way and is said once, here.
 */
function overruling(overrule: Overrule): string {
  return (
    `${OVERRULING[overrule.trigger].screen} ${onwards(overrule)} The reason given is written to ` +
    "the job's log, which is append-only."
  );
}

/**
 * What the Job does after the stopped step advances. **The one fact the two
 * outcomes differ by**, written once: a person must not read it on the screen
 * and press the other one in the confirmation.
 */
export function onwards(overrule: Overrule): string {
  return overrule.commits
    ? "It is the last step of this workflow, so overruling it commits the work and delivers it."
    : "It is not the last step, so the job carries on at the next one.";
}

/**
 * Why neither resume act is on offer, in the words that fit the reason. **Four
 * and not one**, because a person reading an absence asks which absence it is —
 * and the third is the one Bridge could not see before, which is what
 * `stuck.worktree_on_disk` rides beside the acts to say. The fourth is reached
 * by no classification Fleet makes today, and is here because the alternative
 * is one of the other three said where it is false.
 */
function stalled(job: SummaryStatus, stuck: Stuck): string {
  if (job.status !== ESCALATED) return notResumable(job);
  if (stuck.step_id === undefined) return NO_STEP_STOPPED;
  if (!stuck.worktree_on_disk) return WORKTREE_GONE;
  return NOTHING_STANDS;
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

/** `WorktreeGone`, stated as a fact and not a risk: Fleet read the disk. */
const WORKTREE_GONE =
  "Nothing resumes this job. Its drone is gone and so is the worktree it was working in — " +
  "fleet read the disk, and there is nothing left for a fresh drone to take over.";

/** Escalated, a step stopped, a worktree on disk, and Fleet offers neither. */
const NOTHING_STANDS =
  "Nothing resumes this job. Its step stopped and its worktree is still there, and fleet " +
  "offers neither a redirect nor a restart on it.";

/** What the screen says while the Job's own read is still in flight. */
const READING =
  "Fleet has not answered yet. What can be done to a job that stopped is fleet's reading of " +
  "the job, and nothing is offered here until it arrives.";

/**
 * What the screen says where Fleet served no classification at all.
 *
 * **Never a Fleet that predates the field**: one behind this Bridge is refused
 * at the socket, since `connects()` in `@armada/protocol` admits `same`
 * and `fleet_ahead` and nothing else. What is left is a Job Fleet classifies
 * none of — `superseded` is the one it serves, where the work landed elsewhere.
 */
const UNCLASSIFIED =
  "Nothing resumes this job and nothing replaces it. Fleet classifies a job that stopped and " +
  "asked, and a job that ended without landing; it says nothing about one whose work landed " +
  "somewhere other than in it. Proposing a new job is what is left.";

/**
 * The redirect that is out, in the owner's words: *sent, waiting for the drone*.
 *
 * **It is not a status and it does not claim delivery.** The job stays
 * escalated by design and comes back to `running` on the drone's next turn,
 * which is evidence it resumed where sending is not. Fleet knows it wrote to
 * the session, and this says that and no more.
 *
 * **The time is a clock, not an age.** Nothing ticks on the wire and nothing
 * counts up here — the same bargain the transcript and the history make.
 */
function waiting(sent: RedirectInFlight): string {
  return (
    `Sent, waiting for the drone. The instruction went into its session at ${clock(sent.sent_at)}, ` +
    "and this job stays escalated until the drone takes a turn — a turn is the evidence it " +
    "resumed, and sending is not. Redirecting again replaces what is outstanding."
  );
}

/**
 * What asking again is, in the same shape the other acts are stated in: what it
 * does, what it costs, and the answer that only comes on the press.
 *
 * **Never "approve" and never "overrule".** Nothing ruled on this step, so
 * nothing is being disagreed with and nothing is being let through — the gate
 * is being asked the question it could not answer, on evidence that has not
 * changed.
 *
 * **The press still answers one thing.** The classification says Fleet's slot
 * holds this job, not which step it is standing at, and `regating.rs` refuses a
 * re-run of any other one.
 */
const REREAD =
  "The gate could not decide, so there is nothing to overrule. Asking again runs it over the " +
  "evidence already submitted — no drone works, nothing is redone, and no retry is spent.";

/** `DroneStillThere` stated as the act it points at rather than as a refusal. */
const REDIRECT =
  "Its session, its worktree and every step so far are still held, so an instruction reaches " +
  "it as a new turn at the step above.";

/**
 * `NoDroneToRedirect` stated the same way, and the worktree stated as the
 * settled fact it now is: Bridge read no filesystem and let the refusal arrive
 * on the press, and Fleet reads the disk before naming the act.
 *
 * **It no longer promises a drone immediately either.** A restart takes
 * `escalated -> queued` and admission starts it, bounded by the concurrency cap
 * and the machine as an approval is — `crates/fleet/src/readmitting.rs`. The
 * act is never refused for that; it stopped claiming the drone is already there.
 */
const RESTART =
  "The drone is gone, so the job goes back in the queue and a fresh one takes over at the step " +
  "above when there is room, resolving its toolset, model and environment again.";

/** What replaces a Job that nothing resumes, or that nothing replaces either. */
function replacement(redispatch: boolean): string {
  return redispatch
    ? "A redispatch mints a new job from the approval gate and carries none of the work over."
    : "Nothing replaces it either: a redispatch takes an escalated, failed or killed job whose " +
        "workflow and request fleet still holds, and this is not one. Proposing a new job is " +
        "what is left.";
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
