// What this app asks the host to do, and what it asks the host to hold open.
//
// **Its own file for the reason `dispatch.ts` and `palette.ts` are their own**:
// this is a subject rather than a line of wiring. The subject is the preload
// boundary — nothing here knows which surface is open or what is drawn, and
// nothing that draws knows how to reach the host.
//
// **The six pieces of state here are the ones a command in flight has**, and
// they are here for that reason and not because they were nearby: which Job an
// act is on, which Job a decision on its work is on, what the last command
// answered, what the last reclaim gave back, whether a re-read is out, and
// which Job is having its Checks run again. Every one of them is set by a call
// below and read by nothing else.
//
// # What is not decided here
//
// A command that ends somewhere on screen says so through a callback rather
// than by reaching for state it does not own: a redispatch's replacement is
// opened by `onOpen`, and a re-read's answer is published by `onRead`. Two
// commands need something the render knows — a proposal needs the workflow
// roster, and a confirmed restart needs the note the dialog collected — and
// both take it as an argument, because both are a function of the render.
//
// **The one piece of navigation state that is not navigation** is here too:
// whether the open Job's turns socket is held. It was navigation while watching
// swapped the surface for a transcript; the turns are the open step's activity
// log now, so it tracks which Job is open and nothing presses it.

import type { EditManifestProposal, WriteManifestProposal } from "@armada/protocol";
import { useEffect, useState } from "react";

import type { BridgeState } from "../../shared/bridge";
import type { EditManifest, SaveManifestFile } from "@armada/protocol";
import type {
  AddTask,
  Artifact,
  Draft,
  DropTask,
  FileReport,
  Outcome,
  StagedAttachment,
  WorktreeReclaimed,
} from "@armada/protocol";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";
import type {
  CommandAnswer,
  JudgeAnswer,
  SaveLimits,
  StartCheckoutRun,
  StartRun,
  WhenBlocked,
  WhenRefused,
} from "@armada/protocol";
import type { HelmContext, JobSummary } from "@armada/protocol";
import type { ActingAct, Answered, ConfirmableAct, DecidingAct, Taken, TakenAct } from "@armada/screens";
import { takenNotice, takenStands } from "@armada/screens";
import { proposeRequest } from "./dispatch";
import type { Proposing } from "./dispatch";

/* The host calls the screens make, bound once at module scope.
 *
 * **Stable on purpose.** Three of these are depended on by effects, and a
 * lambda rebuilt every render would open and close a read on a loop that feeds
 * itself — the reads publish state. Module scope is the cheapest guarantee
 * there is, and `window.armada` is itself fixed for the life of the window.
 *
 * They exist at all because a screen may not reach for the preload. A screen
 * that did could not be rendered outside the app, which is the whole of why the
 * screens are a layer. */
export const readDiff = (jobId: string | null): void => void window.armada.readDiff(jobId);
export const readReports = (want: boolean): void => void window.armada.readReports(want);
export const readHeld = (want: boolean): void => void window.armada.readHeld(want);
export const reclaimOne = (jobId: string) => window.armada.reclaimWorktree(jobId);
export const deleteBranchOne = (jobId: string, tip: string) => window.armada.deleteBranch(jobId, tip);
export const forgetOne = (jobId: string) => window.armada.forgetJob(jobId);
export const readEvidence = (jobId: string | null): void => void window.armada.readEvidence(jobId);
export const readRemarks = (jobId: string | null): void => void window.armada.readRemarks(jobId);
export const readCall = (jobId: string, callId: string) => window.armada.readCall(jobId, callId);
export const readCheckOutput = (jobId: string, kept: string) =>
  window.armada.readCheckOutput(jobId, kept);
/** New job's own reads for the repository its ask answered, on All — #959. */
export const readComposing = (repository: string) => window.armada.readComposing(repository);
export const followCheckOutput = (jobId: string | null, kept: string | null): void =>
  void window.armada.followCheckOutput(jobId, kept);

export const readFrame = (jobId: string, kept: string) => window.armada.readFrame(jobId, kept);
/** Where a recording streams from. Composed, not fetched — main answers it. */
export const frameSrc = (jobId: string, kept: string) => window.armada.frameStreamUrl(jobId, kept);
/** Ask a Job to show its work again. Answered to the screen that pressed it. */
export const showAgain = (jobId: string, spec?: string) => window.armada.showAgain(jobId, spec);
export const openArtifact = (jobId: string, what: Artifact) => window.armada.openArtifact(jobId, what);
export const openPullRequest = (jobId: string) => window.armada.openPullRequest(jobId);
export const openRemarkLink = (jobId: string, remarkId: string) =>
  window.armada.openRemarkLink(jobId, remarkId);

/** Open the issue a finding became. Main reads the address; the renderer sends none. #906. */
export const openFindingIssue = (jobId: string, finding: string) =>
  window.armada.openFindingIssue(jobId, finding);
export const examine = (jobId: string): void => void window.armada.examineJob(jobId);
// The run sheet — Journey 9. Opened by the sheet, not the Job.
export const watchRunSheet = (jobId: string | null): void => void window.armada.watchRunSheet(jobId);
export const observeRun = (jobId: string | null, runId: string | null): void =>
  void window.armada.observeRun(jobId, runId);
export const startRun = (jobId: string, body: StartRun) => window.armada.startRun(jobId, body);
export const stopRun = (jobId: string, runId: string) => window.armada.stopRun(jobId, runId);
export const undoRun = (jobId: string, runId: string) => window.armada.undoRun(jobId, runId);
export const listRuns = (jobId: string) => window.armada.listRuns(jobId);
export const getRunOutput = (jobId: string, runId: string) => window.armada.getRunOutput(jobId, runId);
// The same rehearsal in the main checkout — the Manifest surface. Held open
// by the app rather than by the screen, because the palette lists off the same
// reading and a read the screen owned would leave those rows missing
// everywhere but on that surface.
export const watchCheckoutRunSheet = (want: boolean): void =>
  void window.armada.watchCheckoutRunSheet(want);
export const observeCheckoutRun = (runId: string | null): void =>
  void window.armada.observeCheckoutRun(runId);
export const startCheckoutRun = (body: StartCheckoutRun) => window.armada.startCheckoutRun(body);
export const stopCheckoutRun = (runId: string) => window.armada.stopCheckoutRun(runId);
export const undoCheckoutRun = (runId: string) => window.armada.undoCheckoutRun(runId);
export const listCheckoutRuns = () => window.armada.listCheckoutRuns();
export const getCheckoutRunOutput = (runId: string) => window.armada.getCheckoutRunOutput(runId);
export const getCheckoutRunDiff = (runId: string) => window.armada.getCheckoutRunDiff(runId);
// Journey 9's *Verify*. Drift is held open by the surface; Verify is only ever
// pressed, so nothing here starts one on a read.
export const watchManifestDrift = (want: boolean): void =>
  void window.armada.watchManifestDrift(want);
// Overview's health and per-repository drift. Held open by the surface, `holding.ts`'s shape.
export const watchOverview = (want: boolean): void => void window.armada.watchOverview(want);
export const startCheckoutVerify = (workspace?: string) => window.armada.startCheckoutVerify(workspace);
export const pickRepository = (root: string | null): void => void window.armada.pickRepository(root);
// Helm's conversation — #944. The reply, and the thread it joins, arrive on
// `BridgeState.helm`. `context` names where the person is — #1075.
export const askHelm = (text: string, context?: HelmContext) => window.armada.askHelm(text, context);
export const startHelmFresh = () => window.armada.startHelmFresh();
export const pointHelm = (manifestId: string): void => void window.armada.pointHelm(manifestId);
// Locate: a folder from the OS dialog, and a repository added or cloned. The window picks what it located.
export const chooseFolder = () => window.armada.chooseFolder();
export const resolveFolder = (path: string) => window.armada.resolveFolder(path);
export const addRepository = (path: string) => window.armada.addRepository(path);
export const cloneRepository = (url: string, parent: string) => window.armada.cloneRepository(url, parent);
// The Manifest file. Held by the app, like the sheet above, so an unsaved edit
// outlives the surface it was made on.
export const readManifestFile = () => window.armada.readManifestFile();
export const saveManifestFile = (body: SaveManifestFile) => window.armada.saveManifestFile(body);
export const editManifest = (body: EditManifest) => window.armada.editManifest(body);
export const readManifestSpend = () => window.armada.readManifestSpend();
// Setup: Scan, the proposals, one edit, Write.
export const readRepositoryScan = () => window.armada.readRepositoryScan();
export const readManifestProposals = () => window.armada.readManifestProposals();
export const editManifestProposal = (body: EditManifestProposal) => window.armada.editManifestProposal(body);
export const writeManifestProposal = (body: WriteManifestProposal) => window.armada.writeManifestProposal(body);
// A repository-wide always-allow — Fleet's own table since protocol 13.5.
export const listRepositoryAllowedCommands = () => window.armada.listRepositoryAllowedCommands();
export const removeRepositoryAllowedCommand = (run: string) =>
  window.armada.removeRepositoryAllowedCommand(run);
export const startServer = (name: string, jobId?: string) => window.armada.startServer(name, jobId);
export const stopServer = (serverId: string) => window.armada.stopServer(serverId);
export const openServerLink = (serverId: string, url: string) =>
  window.armada.openServerLink(serverId, url);
export const stageAttachment = (bytes: ArrayBuffer, filename: string, mimeType: string) =>
  window.armada.stageAttachment(bytes, filename, mimeType);
/** Paths under the checkout narrowed against typed text, for the `@` mention popup. */
export const searchFiles = (query: string) => window.armada.searchFiles(query);
/** What one command does, for the person deciding whether to allow it. A read: nothing moves. */
export const explainCommand = (jobId: string, callId: string) =>
  window.armada.explainCommand(jobId, callId);

/** What a command needs from the render, and where two of them land. */
export type Sending = {
  /**
   * Where a redispatch's replacement goes. The Job that was open is over and
   * the one worth reading is the new one, which is a fact about the detail —
   * so it is answered to the render rather than decided here.
   */
  onOpen: (jobId: string) => void;
  /** Where a re-read's answer goes: the one place published state is held. */
  onRead: (state: BridgeState) => void;
  /** The rows as drawn, so a press taken at a frozen repository can say it waits. */
  jobs: readonly JobSummary[];
};

/**
 * Every command the window can send, and the state one in flight has.
 *
 * **Nothing memoises.** These are rebuilt every render, as they were when they
 * lived in `App`, and no effect depends on one — the bound host calls above are
 * the ones effects read, and those are module scope for exactly that reason.
 */
export function useCommands(sending: Sending) {
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // A press a freeze took and holds. Its own state: `outcome` draws refusals, and this is not one.
  const [taken, setTaken] = useState<Taken | null>(null);
  const rowOf = (jobId: string) => sending.jobs.find((job) => job.id === jobId);
  useEffect(() => {
    if (taken !== null && !takenStands(taken, rowOf(taken.jobId))) setTaken(null);
  }, [sending.jobs, taken]);
  function took(jobId: string, act: TakenAct, answer: Outcome): void {
    if (answer.ok) setTaken({ jobId, act, from: rowOf(jobId)?.status });
  }
  const notice = taken === null ? null : takenNotice(taken, rowOf(taken.jobId));
  // Which Job an act is in flight on. **Nothing destructive happens on one
  // press** — the dialog that collects the confirmation is the render's.
  const [acting, setActing] = useState<string | null>(null);
  // Which act that is, so the control that sent it is the one that waits. #1117.
  const [actingAct, setActingAct] = useState<ActingAct | null>(null);
  // Which Job has a decision on its work in flight. Separate from `acting`,
  // the header's act set: sharing one flag would grey out the header's kills
  // while a review note was being sent.
  const [deciding, setDeciding] = useState<string | null>(null);
  // Which of the four answers that is, so the control pressed is the one that waits. #1117.
  const [decidingAct, setDecidingAct] = useState<DecidingAct | null>(null);
  // Which Job is having a stopped step's Checks run again. Its own state
  // beside `acting`'s: a re-run can take minutes, and the step needs to keep
  // saying so for as long as it runs — `acting` alone cannot say which act
  // that grey covers, and every other one here answers in milliseconds.
  const [rerunningChecks, setRerunningChecks] = useState<string | null>(null);
  // What the last reclaim (or bulk clear) gave back. **Its own state and not
  // `outcome`** — that one draws refusals, and this is a success worth
  // reading: the act asks for two things, the halves can disagree, and a kept
  // branch is something a person has to go and deal with by hand.
  //
  // **An array, because the bulk act can keep more than one.** The per-Job
  // act always sets exactly one entry; `clearTerminal` below sets one per
  // branch a base cannot reach, which is the whole reason a person is told
  // rather than left to notice a branch nothing deleted.
  const [givenBack, setGivenBack] = useState<WorktreeReclaimed[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  // Which bulk sweep of finished Jobs is out, so its control waits and a second press sends nothing. #1117.
  const [sweeping, setSweeping] = useState<"clear" | "forget" | null>(null);

  async function propose(draft: Draft): Promise<void> {
    setOutcome(await window.armada.proposeJob(draft));
  }

  /**
   * Stop the proposal that is out. **Kills the call rather than stopping the
   * wait** — see `JobCommands.stopProposal`.
   *
   * A refusal goes to the same pipeline every other command failure uses. A
   * success says nothing: what a person sees is the wait ending, which the
   * event stream draws a beat later, and a toast on top of it would announce an
   * outcome they are already looking at.
   */
  async function stopProposal(): Promise<void> {
    const answer = await window.armada.stopProposal();
    if (!answer.ok) setOutcome(answer);
  }

  /**
   * The app's half of a proposer answer: a refusal the dispatch surface has no
   * drawing for goes to the same pipeline every other command failure uses.
   * `dispatch.ts` makes the call and decides which half an answer is.
   *
   * **What it is read against comes from the render.** The workflow roster and
   * Bridge's identity are published state, so they arrive as an argument rather
   * than being reached for here.
   *
   * `repository` is the root New job's own ask answered, on All — #959, so the
   * request names it rather than the pick, which stays on All while composing.
   */
  async function proposeFrom(
    request: string,
    attachments: readonly StagedAttachment[],
    proposing: Proposing,
    repository: string | null = null,
  ): Promise<Answered> {
    const read = await proposeRequest(request, attachments, proposing, repository);
    if (read.outcome !== null) setOutcome(read.outcome);
    return read;
  }

  /** Hold `acting` on a Job, with the act named, for as long as `work` is out. #1117. */
  async function acted<T>(jobId: string, act: ActingAct, work: () => Promise<T>): Promise<T> {
    setActing(jobId);
    setActingAct(act);
    try {
      return await work();
    } finally {
      setActing(null);
      setActingAct(null);
    }
  }

  /** `acted`, for the acts at the review gate that `deciding` guards. */
  async function decided<T>(jobId: string, act: DecidingAct, work: () => Promise<T>): Promise<T> {
    setDeciding(jobId);
    setDecidingAct(act);
    try {
      return await work();
    } finally {
      setDeciding(null);
      setDecidingAct(null);
    }
  }

  async function approve(jobId: string): Promise<Outcome> {
    const answer = await window.armada.approveDispatch(jobId);
    setOutcome(answer);
    took(jobId, "approve", answer);
    return answer;
  }

  /**
   * Reclaim every terminal Job's worktree and branch at once, keeping every
   * row. **One outcome shown, not a tally** — a failed reclaim is surfaced
   * through the same refusal pipeline every other command failure uses,
   * naming the first one that refused; the rest that succeeded already show
   * their disk back by the time this returns.
   *
   * **Every kept branch is named, not only the first.** A base that cannot
   * reach a branch is the safe setting working rather than a fault, and a
   * sweep that only ever showed the last one would leave every branch before
   * it silently kept.
   */
  async function clearTerminal(jobIds: readonly string[]): Promise<void> {
    setSweeping("clear");
    try {
      const result = await window.armada.clearTerminalJobs(jobIds);
      if (result.failed.length > 0) setOutcome(result.failed[0]!.outcome);
      const kept = result.reclaimed.filter((one) => one.branch.unmerged_commits != null);
      if (kept.length > 0) setGivenBack(kept);
    } finally {
      setSweeping(null);
    }
  }

  /**
   * Delete every terminal Job's whole record at once. **The bulk half of
   * `forget_job`**, for `clearTerminal`'s reason above — one outcome shown,
   * naming the first refusal, and the rows that succeeded are already gone
   * from the board.
   */
  async function forgetTerminal(jobIds: readonly string[]): Promise<void> {
    setSweeping("forget");
    try {
      const result = await window.armada.forgetTerminalJobs(jobIds);
      if (result.failed.length > 0) setOutcome(result.failed[0]!.outcome);
    } finally {
      setSweeping(null);
    }
  }

  /**
   * Do the confirmed act. **Six preload calls, not one with a discriminator**
   * — killing a Drone leaves the Job, killing the Job ends it, a redispatch
   * mints a replacement, a restart puts a fresh Drone on the same worktree at
   * the step that stopped, a reclaim takes the worktree away and leaves the
   * Job exactly where it is, and a forget takes the row.
   *
   * A redispatch answers with the replacement's id, and the detail follows it:
   * the Job that was open is over, and the one worth reading is the new one.
   *
   * **A reclaim answers with a receipt, which is shown rather than folded.**
   * Nothing on the board changes — the record survives, `forgetJob` is what
   * takes it — and the answer's two halves can disagree, so what happened is
   * stated instead of being left to a silent success.
   *
   * **A forget answers with nothing to fold either**, for the opposite
   * reason: there is no row left. `board.forget`, reached through
   * `Clearing.forget`, is what removes it from what is drawn.
   *
   * **The note arrives rather than being read here.** Only the restart has one,
   * and it is collected by the dialog that confirms — which is the render's, so
   * putting the dialog away and reading what it holds is the render's too.
   */
  async function act(act: ConfirmableAct, jobId: string, note?: string): Promise<void> {
    return acted(jobId, act, async () => {
      const answer =
        act === "redispatch"
          ? await window.armada.redispatchJob(jobId)
          : act === "kill_drone"
            ? await window.armada.killDrone(jobId)
            : act === "restart_step"
              ? await window.armada.restartStep(jobId, note)
              : act === "reclaim_worktree"
                ? await window.armada.reclaimWorktree(jobId)
                : act === "forget_job"
                  ? await window.armada.forgetJob(jobId)
                  : await window.armada.killJob(jobId);
      setOutcome(answer);
      if (act === "restart_step") took(jobId, "restart", answer);
      if (answer.ok && answer.reclaimed !== undefined) setGivenBack([answer.reclaimed]);
      if (answer.ok && answer.jobId !== undefined) sending.onOpen(answer.jobId);
    });
  }

  /**
   * Send a redirect. **Not through `act`** — the dialog that collected the
   * instruction already was the confirmation, so there is nothing left to
   * confirm here, only to send.
   */
  async function redirect(jobId: string, instruction: string): Promise<void> {
    return acted(jobId, "redirect", async () => {
      setOutcome(await window.armada.redirectDrone(jobId, instruction));
    });
  }

  /**
   * Answer the question a drone asked, with the label a person picked.
   *
   * **Not through `act`** for `redirect`'s reason and one of its own: there was
   * never a dialog, because there was never anything to confirm — the answer is
   * one of a closed set the drone itself offered, and it stops the drone
   * waiting rather than ending anything.
   */
  async function answer(jobId: string, questionId: string, chose: string): Promise<Outcome> {
    return acted(jobId, "answer", async () => {
      const answered = await window.armada.answerQuestion(jobId, questionId, chose);
      setOutcome(answered);
      return answered;
    });
  }

  /**
   * Allow or reject a command the drone was not given. **Not through `act`**,
   * for `answer`'s reason: the answers are the closed set Fleet offered for
   * that call, and none of them ends anything.
   */
  async function answerCommand(
    jobId: string,
    call: string,
    chose: CommandAnswer,
    note?: string,
    rule?: string,
  ): Promise<Outcome> {
    return acted(jobId, "answer_command", async () => {
      const answered = await window.armada.answerCommand(jobId, call, chose, note, rule);
      setOutcome(answered);
      return answered;
    });
  }

  /**
   * Change how the job meets the next command its drone was not given. **Not
   * through `act`** — a setting ends nothing, so there is nothing to confirm.
   * Under `acting` all the same, so the header's own control is off while it
   * is out rather than sending a second change over the first.
   */
  async function setWhenBlocked(jobId: string, whenBlocked: WhenBlocked): Promise<void> {
    return acted(jobId, "set_when_blocked", async () => {
      setOutcome(await window.armada.setWhenBlocked(jobId, whenBlocked));
    });
  }

  /**
   * Answer the question a judge refusal opened. **Not through `act`**, for
   * `answerCommand`'s reason: the answer is one of the closed set this design
   * offers, and none of them ends anything from the renderer's own view.
   */
  async function answerJudge(
    jobId: string,
    askedAt: string,
    answer: JudgeAnswer,
    note?: string,
  ): Promise<Outcome> {
    return acted(jobId, "answer_judge", async () => {
      const answered = await window.armada.answerJudge(jobId, askedAt, answer, note);
      setOutcome(answered);
      return answered;
    });
  }

  /** Change how the job meets the next judge criterion that refuses. On `setWhenBlocked`'s terms. */
  async function setWhenRefused(jobId: string, whenRefused: WhenRefused): Promise<void> {
    return acted(jobId, "set_when_refused", async () => {
      setOutcome(await window.armada.setWhenRefused(jobId, whenRefused));
    });
  }

  /**
   * Choose the model the job's next step starts on, or `null` for the
   * workflow's. On `setWhenBlocked`'s terms: nothing to confirm, and under
   * `acting` so the panel is off while it is out.
   */
  async function setModel(jobId: string, model: string | null): Promise<void> {
    return acted(jobId, "set_model", async () => {
      setOutcome(await window.armada.setModel(jobId, model));
    });
  }

  /** The review step's model, on `setModel`'s terms. #903. */
  async function setReviewModel(jobId: string, model: string | null): Promise<void> {
    return acted(jobId, "set_review_model", async () => {
      setOutcome(await window.armada.setReviewModel(jobId, model));
    });
  }

  /**
   * Take back a command allowed for this job. On `setWhenBlocked`'s terms — it
   * ends nothing, and the command can be allowed again from the next refusal.
   */
  async function removeAllowedCommand(jobId: string, run: string): Promise<void> {
    return acted(jobId, "remove_allowed_command", async () => {
      setOutcome(await window.armada.removeAllowedCommand(jobId, run));
    });
  }

  /**
   * Overrule a Judge that refused the work. **Not through `act`**, for
   * `redirect`'s reason: the dialog that collected the reason was the
   * confirmation. **And not through `decide`**, which answers a gate nothing
   * objected to — this one answers a gate that refused.
   */
  async function overrule(jobId: string, reason: string): Promise<void> {
    return acted(jobId, "override_verdict", async () => {
      setOutcome(await window.armada.overrideVerdict(jobId, reason));
    });
  }

  /**
   * Ask the gate again on a step it could not decide. **Not through `act`**,
   * which confirms first: nothing is destroyed, nothing is overruled and
   * nothing is advanced by pressing this, so there is nothing for a dialog to
   * state. **And not through `overrule`**, which answers a machine that ruled —
   * this answers one that could not.
   */
  async function rerun(jobId: string): Promise<void> {
    return acted(jobId, "rerun_gate", async () => {
      setOutcome(await window.armada.rerunGate(jobId));
    });
  }

  /**
   * Run a stopped step's Checks again. `rerun`'s shape, and `rerunningChecks`
   * beside `acting` for the wait's own reason: this can run for minutes, and
   * the step panel says so for as long as it does.
   */
  async function rerunChecks(jobId: string): Promise<void> {
    setActing(jobId);
    setActingAct("rerun_checks");
    setRerunningChecks(jobId);
    try {
      setOutcome(await window.armada.rerunChecks(jobId));
    } finally {
      setActing(null);
      setActingAct(null);
      setRerunningChecks(null);
    }
  }

  /**
   * Give one job a higher cost ceiling. **Not through `act`**, for `redirect`'s
   * reason: the dialog that collected the figure was the confirmation. It moves
   * no part of the job — what it changes is that the next dispatch is not
   * refused for money.
   */
  async function raiseCap(jobId: string, costCapMicros: number): Promise<void> {
    return acted(jobId, "raise_cost_cap", async () => {
      setOutcome(await window.armada.raiseCostCap(jobId, costCapMicros));
    });
  }

  /**
   * Let one job take more turns. **Not through `act`**, on `raiseCap`'s terms:
   * the dialog that collected the figure was the confirmation, and what changes
   * is that the next dispatch is not refused for turns.
   */
  async function raiseTurns(jobId: string, turnCap: number): Promise<void> {
    return acted(jobId, "raise_turn_cap", async () => {
      setOutcome(await window.armada.raiseTurnCap(jobId, turnCap));
    });
  }

  /**
   * File a report on a job that failed in error.
   *
   * **Not through `act`, and not like the others at all**: nothing about the
   * job changes, so there is nothing to fold and nothing to re-read. It returns
   * the outcome rather than only publishing one, because the record that comes
   * back is what the dialog shows next — held there rather than in app state,
   * where it would outlive the dialog that produced it.
   */
  /**
   * Change one or more of Fleet's three admission limits. **Not through
   * `setActing`**, `report`'s reason: there is no Job here for that state to
   * key on, and the sheet that sends this is off on its own while a save is
   * out.
   */
  async function saveLimits(values: SaveLimits): Promise<Outcome> {
    const answer = await window.armada.saveLimits(values);
    setOutcome(answer);
    return answer;
  }

  async function report(jobId: string, filing: FileReport): Promise<Outcome> {
    const answer = await window.armada.fileReport(jobId, filing);
    // Published as well as returned: a refusal belongs in the one place this
    // app says what a command answered, and a success is worth the same line.
    setOutcome(answer);
    return answer;
  }

  /**
   * Add a task to a job's plan. **Not through `act`**, `report`'s reason: what
   * comes back is the plan the add leaves rather than an `Outcome`, and the
   * dialog that collected the title is what needs it, to redraw at once and to
   * stay open on a refusal with what was typed.
   */
  async function addTask(jobId: string, add: AddTask): Promise<PlanEditAnswer> {
    const answer = await window.armada.addTask(jobId, add);
    setOutcome(answer.ok ? { ok: true } : answer.outcome);
    return answer;
  }

  /** Drop a task from a job's plan, with a reason. `addTask`'s own reason. */
  async function dropTask(jobId: string, drop: DropTask): Promise<PlanEditAnswer> {
    const answer = await window.armada.dropTask(jobId, drop);
    setOutcome(answer.ok ? { ok: true } : answer.outcome);
    return answer;
  }

  /**
   * Answer the review gate. **Four preload calls, not one with a
   * discriminator** — merging lands the branch and then takes the work,
   * approving takes it and leaves the pull request where it is, requesting
   * changes sends the drone back to the same step with the note, and rejecting
   * is terminal and ends the drone. **Nothing confirms here**: approving and
   * merging are the ordinary paths, and rejecting is confirmed by the review
   * render's own dialog, where the diff being decided on is still on screen.
   */
  async function decide(
    jobId: string,
    what: "approve" | "changes" | "reject" | "merge",
    note = "",
  ): Promise<void> {
    return decided(jobId, what, async () => {
      const answer =
        what === "merge"
          ? await window.armada.mergePullRequest(jobId)
          : what === "approve"
            ? await window.armada.approveReview(jobId)
            : what === "changes"
              ? await window.armada.requestChanges(jobId, note)
              : await window.armada.rejectWork(jobId);
      setOutcome(answer);
      if (what === "merge" || what === "approve") took(jobId, what, answer);
    });
  }

  /**
   * Hand the comments a person picked off the pull request to a drone.
   *
   * **Under `deciding`, with the four answers at the same gate.** It leaves
   * `awaiting_review` the way requesting changes does, so a second press aims
   * at a job that is no longer at the gate any of the five is legal on.
   *
   * **Nothing confirms.** What it commits to is on screen above the control —
   * a drone on the same branch, and the act keeps the worktree and every step
   * so far, like `changes` beside it.
   */
  async function takeUpRemarks(jobId: string, remarks: string[]): Promise<void> {
    return decided(jobId, "take_up_remarks", async () => {
      setOutcome(await window.armada.takeUpRemarks(jobId, remarks));
    });
  }

  /** Dismiss a finding the review raised, with the reason. #907. */
  async function dismissFinding(jobId: string, finding: string, reason: string): Promise<void> {
    return decided(jobId, "dismiss_finding", async () => {
      setOutcome(await window.armada.dismissFinding(jobId, finding, reason));
    });
  }

  /** Queue a Job after this one lands, from a For context finding. #906. */
  async function queueAfterFinding(jobId: string, finding: string): Promise<void> {
    return decided(jobId, "queue_after_finding", async () => {
      setOutcome(await window.armada.queueAfterFinding(jobId, finding));
    });
  }

  /** File the issue a person confirmed from a For context finding. #906. */
  async function fileFindingIssue(jobId: string, finding: string, title: string, body: string): Promise<void> {
    return decided(jobId, "file_finding_issue", async () => {
      setOutcome(await window.armada.fileFindingIssue(jobId, finding, title, body));
    });
  }

  /** Start the pull request's failed CI runs again. #905. */
  async function rerunFailedChecks(jobId: string): Promise<void> {
    return decided(jobId, "rerun_failed_checks", async () => {
      setOutcome(await window.armada.rerunFailedChecks(jobId));
    });
  }

  /** Send the branch back for a Drone to find out why CI failed. #905. */
  async function investigateFailedChecks(jobId: string): Promise<void> {
    return decided(jobId, "investigate_failed_checks", async () => {
      setOutcome(await window.armada.investigateFailedChecks(jobId));
    });
  }

  /**
   * Ask Fleet for current state over the connection Bridge already holds.
   *
   * **It re-reads; it does not reconnect.** The stream keeps the board current
   * on its own, so this is for the case where somebody wants to be sure rather
   * than for a connection that is broken — and dropping a working socket would
   * not fix one that is.
   */
  async function refresh(): Promise<void> {
    setRefreshing(true);
    try {
      sending.onRead(await window.armada.state());
    } finally {
      setRefreshing(false);
    }
  }

  return {
    outcome,
    setOutcome,
    taken: notice === null ? null : { ...notice, onDismiss: () => setTaken(null) },
    acting,
    actingAct,
    deciding,
    decidingAct,
    takeUpRemarks,
    dismissFinding,
    rerunFailedChecks,
    investigateFailedChecks,
    queueAfterFinding,
    fileFindingIssue,
    givenBack,
    setGivenBack,
    refreshing,
    sweeping,
    propose,
    stopProposal,
    proposeFrom,
    approve,
    clearTerminal,
    forgetTerminal,
    act,
    redirect,
    answer,
    answerCommand,
    answerJudge,
    setWhenRefused,
    setWhenBlocked,
    setModel,
    setReviewModel,
    removeAllowedCommand,
    overrule,
    rerun,
    rerunChecks,
    rerunningChecks,
    raiseCap,
    raiseTurns,
    saveLimits,
    report,
    addTask,
    dropTask,
    decide,
    refresh,
  };
}

/**
 * What main is asked to hold open for the Job being read, and nothing else.
 *
 * **Four reads and one flag, all of them a function of which Job is open.** The
 * renderer says which one that is and does no reading of its own — a component
 * that fetched would be Bridge's second connection.
 */
export function useWatching(openJob: string | null): void {
  // Whether the open Job's turns socket is held open. **Not navigation, and not
  // a control either.** A Job that is open is a Job being observed: the turns
  // are the open step's activity log rather than a screen of their own, and a
  // log that filled only after a press is the tab job detail removed.
  const [observing, setObserving] = useState(false);

  // Which Job main should read whole and keep current.
  useEffect(() => {
    void window.armada.watchJob(openJob);
  }, [openJob]);

  // What the open Job holds on this machine. **Opened with the Job**, like the
  // two sockets: the panel it draws answers *is this working*, which is the
  // question somebody opening a Job they suspect has wedged came with. Main
  // re-reads it on every event naming the Job and this says nothing about when.
  useEffect(() => {
    void window.armada.readResources(openJob);
  }, [openJob]);

  // The open Job's history, for the line of Pulse that says what a person last
  // did. Opened with the Job; main re-reads it on every move once it is asked.
  useEffect(() => {
    void window.armada.readHistory(openJob);
  }, [openJob]);

  // Opening another Job drops the socket, and does it before the one below is
  // reopened — the rows in hand belong to the Job that was open, and carrying
  // them into a different one would be a transcript under the wrong title.
  // Closing the Job closes the socket, so nothing is held for a Job nobody is
  // reading.
  useEffect(() => setObserving(openJob !== null), [openJob]);

  // Which Job's turns main should hold a socket open for.
  useEffect(() => {
    void window.armada.observeJob(observing ? openJob : null);
  }, [observing, openJob]);
}
