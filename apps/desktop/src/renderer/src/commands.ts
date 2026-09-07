// What this app asks the host to do, and what it asks the host to hold open.
//
// **Its own file for the reason `dispatch.ts` and `palette.ts` are their own**:
// this is a subject rather than a line of wiring. The subject is the preload
// boundary — nothing here knows which surface is open or what is drawn, and
// nothing that draws knows how to reach the host.
//
// **The five pieces of state here are the ones a command in flight has**, and
// they are here for that reason and not because they were nearby: which Job an
// act is on, which Job a decision on its work is on, what the last command
// answered, what the last reclaim gave back, and whether a re-read is out.
// Every one of them is set by a call below and read by nothing else.
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

import { useEffect, useState } from "react";

import type { BridgeState } from "../../shared/bridge";
import type { Artifact, Draft, FileReport, Outcome, WorktreeReclaimed } from "@armada/protocol";
import type { Answered, ConfirmableAct } from "@armada/screens";
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
export const readEvidence = (jobId: string | null): void => void window.armada.readEvidence(jobId);
export const readCall = (jobId: string, callId: string) => window.armada.readCall(jobId, callId);
export const openArtifact = (jobId: string, what: Artifact) => window.armada.openArtifact(jobId, what);
export const openPullRequest = (jobId: string) => window.armada.openPullRequest(jobId);
export const examine = (jobId: string): void => void window.armada.examineJob(jobId);
export const stageAttachment = (bytes: ArrayBuffer, filename: string, mimeType: string) =>
  window.armada.stageAttachment(bytes, filename, mimeType);

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
  // Which Job an act is in flight on. **Nothing destructive happens on one
  // press** — the dialog that collects the confirmation is the render's.
  const [acting, setActing] = useState<string | null>(null);
  // Which Job has a decision on its work in flight. Separate from `acting`,
  // the header's act set: sharing one flag would grey out the header's kills
  // while a review note was being sent.
  const [deciding, setDeciding] = useState<string | null>(null);
  // What the last reclaim answered. **Its own state and not `outcome`** — that
  // one draws refusals, and this is a success worth reading: the act asks for
  // two things, the halves can disagree, and a kept branch is something a
  // person has to go and deal with by hand.
  const [givenBack, setGivenBack] = useState<WorktreeReclaimed | null>(null);
  const [refreshing, setRefreshing] = useState(false);

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
   */
  async function proposeFrom(request: string, proposing: Proposing): Promise<Answered> {
    const read = await proposeRequest(request, proposing);
    if (read.outcome !== null) setOutcome(read.outcome);
    return read;
  }

  async function approve(jobId: string): Promise<void> {
    setOutcome(await window.armada.approveDispatch(jobId));
  }

  /**
   * Clear every terminal Job at once. **One outcome shown, not a tally** — a
   * failed forget is surfaced through the same refusal pipeline every other
   * command failure uses, naming the first one that refused; the rest that
   * succeeded are already gone from the board by the time this returns.
   */
  async function clearTerminal(jobIds: readonly string[]): Promise<void> {
    const result = await window.armada.clearTerminalJobs(jobIds);
    if (result.failed.length > 0) setOutcome(result.failed[0]!.outcome);
  }

  /**
   * Do the confirmed act. **Five preload calls, not one with a discriminator**
   * — killing a Drone leaves the Job, killing the Job ends it, a redispatch
   * mints a replacement, a restart puts a fresh Drone on the same worktree at
   * the step that stopped, and a reclaim takes the worktree away and leaves
   * the Job exactly where it is.
   *
   * A redispatch answers with the replacement's id, and the detail follows it:
   * the Job that was open is over, and the one worth reading is the new one.
   *
   * **A reclaim answers with a receipt, which is shown rather than folded.**
   * Nothing on the board changes — the record survives, `clearTerminalJobs` is
   * what takes it — and the answer's two halves can disagree, so what happened
   * is stated instead of being left to a silent success.
   *
   * **The note arrives rather than being read here.** Only the restart has one,
   * and it is collected by the dialog that confirms — which is the render's, so
   * putting the dialog away and reading what it holds is the render's too.
   */
  async function act(act: ConfirmableAct, jobId: string, note?: string): Promise<void> {
    setActing(jobId);
    try {
      const answer =
        act === "redispatch"
          ? await window.armada.redispatchJob(jobId)
          : act === "kill_drone"
            ? await window.armada.killDrone(jobId)
            : act === "restart_step"
              ? await window.armada.restartStep(jobId, note)
              : act === "reclaim_worktree"
                ? await window.armada.reclaimWorktree(jobId)
                : await window.armada.killJob(jobId);
      setOutcome(answer);
      if (answer.ok && answer.reclaimed !== undefined) setGivenBack(answer.reclaimed);
      if (answer.ok && answer.jobId !== undefined) sending.onOpen(answer.jobId);
    } finally {
      setActing(null);
    }
  }

  /**
   * Send a redirect. **Not through `act`** — the dialog that collected the
   * instruction already was the confirmation, so there is nothing left to
   * confirm here, only to send.
   */
  async function redirect(jobId: string, instruction: string): Promise<void> {
    setActing(jobId);
    try {
      setOutcome(await window.armada.redirectDrone(jobId, instruction));
    } finally {
      setActing(null);
    }
  }

  /**
   * Answer the question a drone asked, with the label a person picked.
   *
   * **Not through `act`** for `redirect`'s reason and one of its own: there was
   * never a dialog, because there was never anything to confirm — the answer is
   * one of a closed set the drone itself offered, and it stops the drone
   * waiting rather than ending anything.
   */
  async function answer(jobId: string, questionId: string, chose: string): Promise<void> {
    setActing(jobId);
    try {
      setOutcome(await window.armada.answerQuestion(jobId, questionId, chose));
    } finally {
      setActing(null);
    }
  }

  /**
   * Overrule a Judge that refused the work. **Not through `act`**, for
   * `redirect`'s reason: the dialog that collected the reason was the
   * confirmation. **And not through `decide`**, which answers a gate nothing
   * objected to — this one answers a gate that refused.
   */
  async function overrule(jobId: string, reason: string): Promise<void> {
    setActing(jobId);
    try {
      setOutcome(await window.armada.overrideVerdict(jobId, reason));
    } finally {
      setActing(null);
    }
  }

  /**
   * Ask the gate again on a step it could not decide. **Not through `act`**,
   * which confirms first: nothing is destroyed, nothing is overruled and
   * nothing is advanced by pressing this, so there is nothing for a dialog to
   * state. **And not through `overrule`**, which answers a machine that ruled —
   * this answers one that could not.
   */
  async function rerun(jobId: string): Promise<void> {
    setActing(jobId);
    try {
      setOutcome(await window.armada.rerunGate(jobId));
    } finally {
      setActing(null);
    }
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
  async function report(jobId: string, filing: FileReport): Promise<Outcome> {
    const answer = await window.armada.fileReport(jobId, filing);
    // Published as well as returned: a refusal belongs in the one place this
    // app says what a command answered, and a success is worth the same line.
    setOutcome(answer);
    return answer;
  }

  /**
   * Answer the review gate. **Three preload calls, not one with a
   * discriminator** — approving takes the work, requesting changes sends the
   * drone back to the same step with the note, and rejecting is terminal and
   * ends the drone. **Nothing confirms here**: approving is the ordinary path,
   * and rejecting is confirmed by the review render's own dialog, where the
   * diff being decided on is still on screen.
   */
  async function decide(jobId: string, what: "approve" | "changes" | "reject", note = ""): Promise<void> {
    setDeciding(jobId);
    try {
      setOutcome(
        what === "approve"
          ? await window.armada.approveReview(jobId)
          : what === "changes"
            ? await window.armada.requestChanges(jobId, note)
            : await window.armada.rejectWork(jobId),
      );
    } finally {
      setDeciding(null);
    }
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
    acting,
    deciding,
    givenBack,
    setGivenBack,
    refreshing,
    propose,
    stopProposal,
    proposeFrom,
    approve,
    clearTerminal,
    act,
    redirect,
    answer,
    overrule,
    rerun,
    report,
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
