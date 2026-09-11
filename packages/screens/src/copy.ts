// What Bridge says when it refuses something, and what it says before it does
// something that ends something.
//
// Beside `App.tsx` rather than inside it because it is copy: every sentence
// here is read by a person and none of it is wiring. **No status label is
// among them** — those come from the generated vocabulary, which is what stops
// a second one being typed into a component.
//
// The act labels moved here from `Acts.tsx` when the controls that own a
// dialog were split out of it. They are the words on a button, which is this
// file's subject; which control draws them is that file's.

import type { DialogTone } from "@armada/components";

import type { CommandAnswer, Outcome, WhenBlocked, WorktreeReclaimed } from "@armada/protocol";
import type { ConfirmableAct, JobAct } from "./JobDetail";

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
    case "already_forgetting":
      return "That job is already being forgotten. It was not sent twice.";
    case "already_reclaiming":
      return "That worktree is already being reclaimed. It was not sent twice.";
    case "already_redirecting":
      return "That redirect is already in flight. It was not sent twice.";
    case "already_restarting":
      return "That restart is already in flight. It was not sent twice.";
    case "already_overruling":
      return "That override is already in flight. It was not sent twice.";
    case "already_rereading":
      return "That gate is already being re-run. It was not asked twice.";
    case "already_raising":
      return "That cost cap is already being raised. It was not sent twice.";
    case "already_raising_turns":
      return "That turn cap is already being raised. It was not sent twice.";
    case "cap_not_raised":
      return "A new cost cap has to be higher than the one in force. Nothing was sent, and the job is still held.";
    case "turn_cap_not_raised":
      return "A new turn cap has to be higher than the one in force. Nothing was sent, and the job is still held.";
    case "empty_report":
      return "A report needs what you know went wrong. The record on its own says nothing that was not already on the job.";
    case "already_reporting":
      return "That report is already being filed. It was not sent twice.";
    case "empty_instruction":
      return "A redirect needs an instruction. Nothing was sent.";
    case "empty_reason":
      return "An override needs a reason. Nothing was sent, and the judge's verdict stands.";
    case "no_remarks_chosen":
      return "Pick at least one comment. Nothing was sent, and the pull request is where it was.";
    case "already_deciding":
      return "A decision on that job's work is already in flight. It was not sent twice.";
    case "already_answering":
      return "That answer is already in flight. It was not sent twice.";
    case "already_setting":
      return "A change to this job's settings is already in flight. It was not sent twice.";
    case "already_showing":
      return "That job is already showing its work. It was not asked twice.";
    case "empty_note":
      return "Requesting changes needs a note. Nothing was sent, and the job is still waiting.";
    case "refused":
      // Drawn as a failure notice above, with everything it carries.
      return "";
    case "transport":
      // **The fallback and not the rendering.** `App.tsx` draws this through
      // `transportFailure`, with a code, the route, the wait and something to
      // copy — this is what is left for a surface that has only a sentence to
      // put it in, so it names the route rather than repeating the machine's
      // words on their own. "Fleet did not answer" is the same line whichever
      // route it was about, and that was the whole complaint.
      return `Fleet did not answer ${outcome.fault.method} ${outcome.fault.path}: ${outcome.detail}`;
  }
}

/**
 * What a reclaim answered, in two sentences: one per half.
 *
 * **Both halves are always stated, including the one that worked.** The act
 * asks for two things and a notice naming only the failure would leave a person
 * guessing whether the other happened — and the ordinary answer here is a
 * removed checkout beside a branch deliberately kept, which reads as a failure
 * unless the sentence says otherwise.
 *
 * **A kept branch is not an apology.** Fleet always runs this with the safe
 * setting and there is no force on this seam, so a branch holding work nothing
 * has taken surviving is the rule working, and the wording says what to do
 * about it rather than that something went wrong.
 */
export function reclaimed(answer: WorktreeReclaimed): string {
  const checkout = answer.worktree.removed
    ? `The worktree at ${answer.worktree.path} is gone.`
    : `The worktree at ${answer.worktree.path} is still there — ${answer.worktree.why ?? "no reason was given"}.`;
  const branch = answer.branch.deleted
    ? `Branch ${answer.branch.branch} was deleted${answer.branch.tip === undefined ? "" : `, at ${answer.branch.tip.slice(0, 12)}`}.`
    : answer.branch.unmerged_commits === undefined
      ? `Branch ${answer.branch.branch} was left alone — ${answer.branch.why ?? "no reason was given"}.`
      : `Branch ${answer.branch.branch} was kept: ${answer.branch.why}. Merge it or delete it by hand when you have taken what you want.`;
  return `${checkout} ${branch}`;
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
  // What survives is the whole subject here, and it is two different answers
  // for two different things — so the body says both rather than promising the
  // disk back and leaving a person to find the branch still there. Fleet always
  // keeps a branch nothing has merged and there is no force on this seam.
  reclaim_worktree: {
    title: "Give this job's worktree back?",
    body:
      "The checkout is deleted and the disk it was using comes back. The job stays on the " +
      "board with everything it recorded — this removes a directory, not the job. Its branch " +
      "is deleted only if the base branch already has every commit on it; one holding work " +
      "nothing has taken is kept, and the answer says so.",
  },
  restart_step: {
    title: "Restart this step?",
    tone: "neutral",
    body:
      "A fresh drone takes over on the same worktree, at the step the last one stopped at. " +
      "The toolset, model and environment are resolved again from scratch, so a widened scope " +
      "can only narrow. Fleet read the worktree before offering this, so there is one to take " +
      "over.",
  },
  // The one confirmation on this row for an act that cannot be undone. Its
  // worktree and branch go with it where a reclaim has not already taken
  // them — this is the whole record, not only the disk.
  forget_job: {
    title: "Delete this job's record?",
    body:
      "The job, its log, its checks and its judgments are removed from the board along with " +
      "its whole record. There is no undo, and a deleted job cannot be opened again.",
  },
};

/**
 * The field the restart confirmation carries, and what it promises.
 *
 * **The one confirmation that collects anything, and the field is optional.**
 * Every other dialog that takes a field is its own act — a redirect and an
 * override each need theirs, and neither reaches `CONFIRM`. This one is a
 * restart either way: leaving it alone sends exactly what a restart sent before
 * the field existed, which is why it is not a second control and not a second
 * step.
 *
 * **It says where the words go, because the answer is surprising.** There is no
 * drone to read them — that is what a restart is for — so they wait on the job
 * and open the brief of the drone this asks for. A person who was not told that
 * would expect something to happen to the drone they can see is gone.
 */
export const RESTART_NOTE = {
  label: "What to do differently — optional",
  says:
    "Anything you write here opens the new drone's brief before it starts. Leave it empty to " +
    "restart on what stopped the last one.",
};

/**
 * What each act is called on its button. **Redispatch does not say "retry" or
 * "run again"** — nothing resumes, and a label implying the same Job continues
 * would describe an act Fleet does not perform. The confirmation states the
 * rest; the button names the act.
 *
 * **The override says "overrule", never "approve" or "accept".** Approving is a
 * different act on a different status and means the work was right; this one
 * means a machine was wrong and a person is taking responsibility for going
 * past it.
 *
 * **The override's own control does not read this row.** Its wording changes
 * with what is being overruled — a Judge's verdict or a gaming flag — so it
 * comes from `OVERRULING` in `recovery.ts`, where the trigger is known. This
 * row is the act's name where no trigger is in hand, which is the shared
 * confirmation and the menu, and it keeps the record total over `JobAct`.
 *
 * **The re-run says "ask", not "approve", "accept", "retry" or "override".**
 * Nothing ruled on that step, so there is no verdict to accept and none to
 * overrule; and nothing the drone did is redone, so it is not a retry. What
 * happens is that a gate which could not answer is asked again, and the label
 * is that sentence with the words taken out.
 */
/**
 * Say this job failed in error. **Not a `JobAct`** — it moves nothing, so it is
 * outside the record every other label here is keyed by, and it still needs one
 * word wherever it is offered. `actions.toml` calls it `report_job` and binds
 * it to `b`.
 */
export const REPORT_LABEL = "Report this job";

/**
 * Give this job a higher cost ceiling. **Not a `JobAct`** for `REPORT_LABEL`'s
 * reason: it moves no status, no step and no drone, so it is outside the record
 * every other label here is keyed by. `actions.toml` calls it `raise_cost_cap`
 * and binds it to `B`.
 *
 * **"Cost cap", never "budget".** The lexicon gives budget to Machine — the
 * whole installation's resources, timing and money — and this act reaches one
 * job. A control called `Raise the budget` would read as loosening the setting
 * every other job is held to, which is the opposite of what it does.
 */
export const RAISE_CAP_LABEL = "Raise the cost cap";

/**
 * Let this job take more turns. **Not a `JobAct`** for `RAISE_CAP_LABEL`'s
 * reason, and it carries that label's rule too: "turn cap", never "budget".
 * `actions.toml` calls it `raise_turn_cap` and binds it to `T`.
 *
 * **Its own words beside the cost cap's, because the two ceilings are two.** A
 * job held on turns reads the same `over budget` label a job out of money
 * reads, and raising the cost cap does not start it.
 */
export const RAISE_TURN_CAP_LABEL = "Raise the turn cap";

/**
 * How a job meets a command its drone was not given, in the Job settings
 * panel's words, and the order it offers them. **The three are the whole set**
 * — a new job starts at the first, which asks nobody to be watching.
 *
 * The labels say what happens to the person, not to the call: *refuse and
 * hold* named Fleet's side of it, and what somebody choosing needs to know is
 * whether the job will stop for them, ask them, or not bother them at all.
 */
export const WHEN_BLOCKED: readonly WhenBlocked[] = ["refuse_and_hold", "ask_me", "allow_all"];

export const WHEN_BLOCKED_LABEL: Record<WhenBlocked, string> = {
  refuse_and_hold: "Stop and wait for me",
  ask_me: "Ask me first",
  allow_all: "Run it",
};

/**
 * What each choice commits to, drawn under it. **Run it names its two
 * exceptions** because they are what makes it safe to choose: a push stays
 * Armada's, and what armada.yml marks destructive still stops for a person.
 */
export const WHEN_BLOCKED_MEANS: Record<WhenBlocked, string> = {
  refuse_and_hold: "It's refused, and the job stops until you allow or reject it.",
  ask_me: "The drone waits while you decide, then carries on. The job shows under Needs you.",
  allow_all:
    "Any command runs without asking, so the job finishes however it can. Pushing stays " +
    "Armada's, and commands armada.yml marks destructive still stop for you.",
};

/**
 * The answers to a command a drone was not given: the words on each control,
 * and what it commits to in one line. **Fleet offers these three and no
 * others**, and which of them a command takes is Fleet's to say.
 *
 * `means` is drawn where a person picks before sending — a command the drone
 * is waiting on. A refused row draws the label alone, beside the sentence
 * under the list that says what allowing from a row does.
 */
export const COMMAND_ANSWER: Record<CommandAnswer, { label: string; means: string }> = {
  allow_for_job: {
    label: "Allow for this job",
    means: "The drone runs it now, and this job can run it again without asking.",
  },
  always_allow: {
    label: "Always allow in this repository",
    means:
      "The drone runs it now, and it is written into armada.yml under commands, as its own " +
      "commit on this job's branch.",
  },
  reject: {
    label: "Reject",
    means: "The drone is told no, and the command does not run.",
  },
};

/**
 * The answers one command offers, as words, in the order Fleet sent them. **An
 * answer from a Fleet ahead of this build is left out** rather than drawn as a
 * control with no words on it — one control fewer is honest, a blank one is not.
 */
export function offeredOf(
  offers: readonly CommandAnswer[],
): { offer: CommandAnswer; label: string; means: string }[] {
  const known: Partial<Record<string, { label: string; means: string }>> = COMMAND_ANSWER;
  return offers.flatMap((offer) => {
    const said = known[offer];
    return said === undefined ? [] : [{ offer, ...said }];
  });
}

/** An answer a control handed back, as the wire's word — or nothing, for one this build never drew. */
export function answerNamed(name: string): CommandAnswer | undefined {
  return (Object.keys(COMMAND_ANSWER) as CommandAnswer[]).find((answer) => answer === name);
}

export const ACT_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone",
  kill_job: "Kill job",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone",
  restart_step: "Restart step",
  override_verdict: "Overrule the verdict",
  rerun_gate: "Ask the gate again",
  reclaim_worktree: "Reclaim worktree",
  forget_job: "Delete record",
};

/**
 * The same acts inside the menu, where each says what survives it. A caret hides
 * the consequence that a button's own position states, so the label has to carry
 * it — `Kill drone` and `Kill job` differ by everything and by three characters.
 *
 * **Redirect, restart, the override and the re-run never reach a menu** — none
 * of them joins the split button, so those four entries exist only to keep the
 * record total over `JobAct` rather than for anything that reads them today.
 */
export const MENU_LABEL: Record<JobAct, string> = {
  kill_drone: "Kill drone, the job stays open",
  kill_job: "Kill job, it ends here",
  redispatch: "Redispatch as a new job",
  redirect: "Redirect drone, the job stays open",
  restart_step: "Restart the step, on the same worktree",
  override_verdict: "Overrule the verdict, the refused work stands",
  rerun_gate: "Ask the gate again, on the evidence already submitted",
  reclaim_worktree: "Reclaim worktree, the job stays on the board",
  forget_job: "Delete record, there is no undo",
};
