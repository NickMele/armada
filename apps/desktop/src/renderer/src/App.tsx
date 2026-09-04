// Two screens and one piece of state between them: the board — what Fleet is, a
// form to propose a Job, and every Job — and one Job read whole. A row is the
// control that opens a detail; Escape and one button close it. No router: a
// list and a detail need which one is open and nothing else.
//
// Everything drawn comes from the state the main process publishes over the one
// connection. Nothing here fetches, and nothing here holds a copy of a Job that
// Fleet has not confirmed — a Job whose real state is not what the screen says
// is the failure that matters.
//
// # Nothing here fails silently
//
// Three failures, and they stay three. Fleet unreachable says which of the four
// answers the runtime file gave, a region that threw names itself and leaves
// the rest of the window usable, and a Job the store refused is one row rather
// than a board. Each points at its log, and the one that carries a run id — a
// refusal Fleet minted — says that the id names Fleet's run rather than that
// failure. **Nothing here mints one.** An id from a process that writes no log
// line joins to nothing, and a labelled blank is worse than an absent row.

import { useEffect, useRef, useState } from "react";
import { Alert, Button, Dialog, Textarea } from "@armada/components";

import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import type { Artifact, Draft, Outcome } from "@armada/protocol";
import type { FileReport, WorktreeReclaimed } from "@armada/protocol";
import { Boundary } from "@armada/shell";
import { CopiedToast, SaidToast, useCopied, useSaid } from "@armada/shell";
import { FailureBlock } from "@armada/shell";
import { fleetFailure, jobFailure, refusalFailure, transportFailure, uncaughtFailure } from "@armada/shell";
import { headOf } from "@armada/shell";
import { statementOf } from "@armada/shell";
import { Composer } from "@armada/screens";
import { DispatchJob } from "@armada/screens";
import type { Answered } from "@armada/screens";
import { watchOf } from "@armada/screens";
import { Reports } from "@armada/screens";
import { Worktrees } from "@armada/screens";
import { JobDetail, type ConfirmableAct } from "@armada/screens";
import { ACT_LABEL, CONFIRM, reclaimed, RESTART_NOTE, said } from "@armada/screens";
import { Jobs } from "@armada/screens";
import { BOARD_TABS, type BoardReach, type BoardTab } from "@armada/screens";
import { carryOut, dormantIn } from "./palette";
import { proposeRequest } from "./dispatch";
import { Palette, useCommandPalette } from "@armada/shell";
import { copyDebugInfoFor } from "@armada/shell";
import { Shell } from "@armada/shell";
import { SURFACE, SURFACES } from "@armada/shell";
import { watchUncaught } from "@armada/shell";
import type { Uncaught } from "@armada/shell";

/** How often the elapsed figures are redrawn. They are read, so they must move. */
const TICK_MS = 1000;

/** Re-exported so nothing importing it has to learn a new path. */
export const WAITING: BridgeState = NOTHING_YET;

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
const readDiff = (jobId: string | null): void => void window.armada.readDiff(jobId);
const readReports = (want: boolean): void => void window.armada.readReports(want);
const readHeld = (want: boolean): void => void window.armada.readHeld(want);
const reclaimOne = (jobId: string) => window.armada.reclaimWorktree(jobId);
const readEvidence = (jobId: string | null): void => void window.armada.readEvidence(jobId);
const readCall = (jobId: string, callId: string) => window.armada.readCall(jobId, callId);
const openArtifact = (jobId: string, what: Artifact) => window.armada.openArtifact(jobId, what);
const stageAttachment = (bytes: ArrayBuffer, filename: string, mimeType: string) =>
  window.armada.stageAttachment(bytes, filename, mimeType);

export function App() {
  const [state, setState] = useState<BridgeState>(WAITING);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // What has been read and acknowledged. The count itself belongs to the
  // connection and is never reset from here — a drop that happened, happened.
  const [acknowledged, setAcknowledged] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const [copied, setCopied] = useCopied();
  // What the app is telling somebody, as a sentence it already wrote. Today
  // that is only an open that did not happen; a click ending in nothing on
  // screen is the defect the openable records were added against.
  const [telling, setTelling] = useSaid();
  // What a boundary could never catch: a throw in a handler, and a rejected
  // promise from a `void`-ed preload call.
  const [uncaught, setUncaught] = useState<Uncaught | null>(null);
  // **The whole of navigation.** A list and a detail need one piece of state,
  // not a router: which Job is open, or none. The row is the control that sets
  // it and Escape is what clears it.
  const [openJob, setOpenJob] = useState<string | null>(null);
  // The tab a pressed notification asked for, until the Board is on screen to
  // take it. **Held for one render rather than set straight away**: the press
  // may have arrived over the composer or over a Job, so the surface it wants
  // is not mounted yet and `reach` is whatever the last one left.
  const [landing, setLanding] = useState<BoardTab | null>(null);
  // Whether the open Job's turns socket is held open. **Not navigation, and not
  // a control either.** It was navigation while watching swapped the surface
  // for a transcript, and then a tab; the turns are the open step's activity
  // log now, so this tracks which Job is open and nothing presses it.
  const [observing, setObserving] = useState(false);
  // Whether the composer is open. It used to sit permanently above the list;
  // `New job` is what opens it now, so the surface is the list until somebody
  // asks for the form.
  const [composing, setComposing] = useState(false);
  // What has been reported against the Judge. Its own view for the reason the
  // head gives: a report is filed about one Job and the rate is read across all
  // of them.
  const [auditing, setAuditing] = useState(false);
  // Whether the held worktrees are open. **Its own view for the reports' kind
  // of reason and not the same one**: what is decided there is which of a set
  // to give back, which no Job row can be asked, and putting a disk decision on
  // the Board would put a control nobody can act on beside rows that exist to
  // be acted on.
  const [clearing, setClearing] = useState(false);
  // The Manifest the rail names, and what a new Job is proposed against.
  // Bridge dispatches into the workspace it is pointed at, so this is one
  // value rather than a field on the form.
  const [scope, setScope] = useState("");
  // The row the cursor goes back to when the detail closes. A keyboard that
  // opened a row and came back to the top of the document has lost its place.
  const [returning, setReturning] = useState<string | null>(null);
  // Which Job an act is in flight on, and which act is waiting to be
  // confirmed. **Nothing destructive happens on one press** — every one of the
  // three ends something, so each states what happens and what survives first.
  const [acting, setActing] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<{ act: ConfirmableAct; jobId: string } | null>(
    null,
  );
  // What a person typed into the restart confirmation, which is the one
  // confirmation that collects anything. **Held beside `confirming` rather than
  // inside it**: the act being confirmed is what the palette and the step
  // header both set, and neither of them knows about a field.
  const [restartNote, setRestartNote] = useState("");
  // What the last reclaim answered. **Its own state and not `outcome`** — that
  // one draws refusals, and this is a success worth reading: the act asks for
  // two things, the halves can disagree, and a kept branch is something a
  // person has to go and deal with by hand.
  const [givenBack, setGivenBack] = useState<WorktreeReclaimed | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  // Which Job has a decision on its work in flight. Separate from `acting`,
  // the header's act set: sharing one flag would grey out the header's kills
  // while a review note was being sent.
  const [deciding, setDeciding] = useState<string | null>(null);
  // Whether the command palette is up, and where the Board's cursor is.
  // **The cursor is mirrored, not owned** — the Board holds it and reports it,
  // so the palette can title its context block with the job its acts would act
  // on. Two cursors would drift.
  const palette = useCommandPalette();
  const [cursor, setCursor] = useState<string | null>(null);
  // What the palette can reach on the Board: the state filter, and the search
  // field. Both belong to that surface and stay there — see `BoardReach`.
  const reach = useRef<BoardReach | null>(null);

  // The open Job, read out of the list rather than copied beside it. A Job that
  // leaves the list — superseded, or gone from a resync — closes its own detail
  // rather than leaving a row on screen that Fleet no longer has.
  const reading = openJob === null ? null : (state.jobs.find((job) => job.id === openJob) ?? null);
  useEffect(() => {
    void window.armada.state().then(setState);
    return window.armada.subscribe(setState);
  }, []);

  // Which Job main should read whole and keep current. The renderer says which
  // one is open and does no reading of its own — a component that fetched
  // would be Bridge's second connection.
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
  //
  // **A Job that is open is a Job being observed.** The turns are the step's
  // activity log rather than a screen of their own, and a log that filled only
  // after a press is the tab job detail removed. Closing the Job closes the
  // socket, so nothing is held for a Job nobody is reading.
  useEffect(() => setObserving(openJob !== null), [openJob]);

  // Which Job's turns main should hold a socket open for.
  useEffect(() => {
    void window.armada.observeJob(observing ? openJob : null);
  }, [observing, openJob]);

  useEffect(() => watchUncaught(setUncaught), []);

  // **Where a pressed notification says to go, and it always goes somewhere.**
  // A press that raised the window and left it on whatever it was last showing
  // is a press that did nothing, which is the one outcome that teaches somebody
  // to stop pressing them.
  //
  // One Job opens that Job. Several open the set they came from — the Needs-you
  // tab — because picking one of four for somebody is choosing on their behalf.
  // Either way the overlays come down first: the press asked for the Board or a
  // Job, not for the composer that happened to be up.
  useEffect(
    () =>
      window.armada.onSummoned((to) => {
        setComposing(false);
        setAuditing(false);
        setClearing(false);
        setOpenJob(to.jobId);
        if (to.jobId === null) setLanding("needs-you");
      }),
    [],
  );

  // The Board is mounted by the time an effect runs, so this is where the tab
  // asked for above is actually set — the handler that asked could only have
  // reached the surface it was leaving.
  useEffect(() => {
    if (landing === null) return;
    reach.current?.tab(landing);
    setLanding(null);
  }, [landing]);

  // The scope starts on the first Manifest Fleet names, and stays where a
  // person put it when the roster is re-read.
  useEffect(() => {
    const held = state.holds.manifests;
    if (scope === "" && held.length > 0) setScope(held[0]!.id);
  }, [state.holds.manifests, scope]);

  useEffect(() => {
    const tick = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(tick);
  }, []);

  // Escape closes the detail wherever the cursor is inside it, which is the
  // one thing every reader tries first. Bound while a Job is open and not
  // before, so nothing listens for a key that means nothing.
  useEffect(() => {
    if (openJob === null) return;
    const pressed = (event: KeyboardEvent): void => {
      // One view to leave, since the turns stopped being a screen of their own:
      // Escape returns to the list from anywhere inside a Job.
      if (event.key !== "Escape") return;
      close();
    };
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [openJob]);

  // The row is back in the document only after the list re-renders, so the
  // focus move is an effect rather than part of the click that closed it.
  useEffect(() => {
    if (returning === null) return;
    const row = document.querySelector<HTMLElement>(`[data-job-id="${CSS.escape(returning)}"]`);
    row?.focus();
    setReturning(null);
  }, [returning]);

  // The Job every palette act acts on: the one read whole, or the one under
  // the Board's cursor. In that order, because a Job open on screen is
  // unambiguously what is in front of you.
  const onWhat = reading ?? state.jobs.find((job) => job.id === cursor);
  const live = state.connection.state === "connected";
  const statement = statementOf(state.connection, now, state.readAt);
  const fleet = fleetFailure(state.connection, statement, state.bridge, now);
  // What the last command answered, where the answer was a failure rather than
  // guidance. **Two arms and not one**: a refusal is Fleet declining with an
  // envelope, and a transport failure is a command it did not answer at all —
  // which used to be a single line of copy with no code and nothing to copy.
  // Everything else `Outcome` carries is the form saying what it will not send,
  // which is guidance and takes the `Alert` below.
  const commandFailure =
    outcome === null || outcome.ok
      ? null
      : outcome.why === "refused"
        ? refusalFailure(outcome.error, state.bridge)
        : outcome.why === "transport"
          ? transportFailure(outcome, state.bridge)
          : null;
  const guarded = { bridge: state.bridge, onCopied: setCopied };
  // The failure Copy debug info would copy, where one is on screen. Fleet
  // being unreachable outranks a throw in a handler: it is the one that
  // explains every other symptom, so it is the one worth sending. A command
  // that failed sits between them — more specific than a stray throw, and
  // still explained by an unreachable Fleet where there is one.
  const failing =
    fleet ??
    commandFailure ??
    (uncaught === null ? null : uncaughtFailure(uncaught, state.bridge));

  async function propose(draft: Draft): Promise<void> {
    setOutcome(await window.armada.proposeJob(draft));
  }

  /**
   * The app's half of a proposer answer: a refusal the dispatch surface has no
   * drawing for goes to the same pipeline every other command failure uses.
   * `dispatch.ts` makes the call and decides which half an answer is.
   */
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

  async function proposeFrom(request: string): Promise<Answered> {
    const read = await proposeRequest(request, {
      workflows: state.holds.workflows,
      bridge: state.bridge,
    });
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
   */
  async function act(act: ConfirmableAct, jobId: string): Promise<void> {
    // Read before the state is cleared, and only the restart has one. A blank
    // field is not sent: `restartStep` drops it, so a person who opened the
    // dialog and typed nothing gets the restart they pressed for rather than
    // the 422 a blank note earns.
    const note = act === "restart_step" ? restartNote : undefined;
    setConfirming(null);
    setRestartNote("");
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
      if (answer.ok && answer.jobId !== undefined) setOpenJob(answer.jobId);
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
      setState(await window.armada.state());
    } finally {
      setRefreshing(false);
    }
  }

  function close(): void {
    setReturning(openJob);
    setOpenJob(null);
  }

  /**
   * Go to a place in the rail. **One function, because the rail and the palette
   * are two controls on one act** — a second copy is where one of them gets
   * left behind, which is how `auditing` came to survive a rail press.
   *
   * A clear-then-set rather than a branch per destination: a branch is where a
   * view gets left standing under the next one.
   */
  function goTo(surfaceId: string): void {
    setOpenJob(null);
    setComposing(false);
    setAuditing(false);
    setClearing(surfaceId === SURFACE.worktrees);
  }

  const scoped = state.holds.manifests.find((held) => held.id === scope);
  const head = headOf({
    reading: reading !== null,
    composing,
    auditing,
    clearing,
    live,
    refreshing,
    onCloseJob: close,
    onCloseComposer: () => setComposing(false),
    onCompose: () => setComposing(true),
    onCloseReports: () => setAuditing(false),
    onReadReports: () => setAuditing(true),
    onCloseWorktrees: () => setClearing(false),
    onReadWorktrees: () => setClearing(true),
    onRefresh: () => void refresh(),
  });

  return (
    <>
      <Shell
        connection={state.connection}
        statement={statement}
        manifests={state.holds.manifests}
        scope={scope}
        onScope={setScope}
        jobs={state.jobs}
        capacity={state.capacity}
        title={head.title}
        summary={head.summary}
        actions={head.actions}
        // Which row the rail marks. The held worktrees are the one surface
        // other than the Board that draws, so everything else — a Job, the
        // composer, the reports — is the Board with something over it.
        showing={clearing ? SURFACE.worktrees : SURFACE.board}
        onSurface={goTo}
      >
        <div className="flex flex-col gap-6">
          {/* Fleet, when the one connection is not one. The status bar keeps the
              single line; this is the same reading with the four runtime-file
              answers and the log under it. */}
          {fleet === null ? null : <FailureBlock failure={fleet} onCopied={setCopied} />}

          {/* What no boundary sees. A click that threw and a rejected preload
              call both look like a button that did nothing. */}
          {uncaught === null ? null : (
            <FailureBlock
              failure={uncaughtFailure(uncaught, state.bridge)}
              onCopied={setCopied}
              // Cleared by hand rather than on a timer: a failure that vanishes
              // while nobody is looking is the silence being repaired here.
              onDismiss={() => setUncaught(null)}
            />
          )}

          {state.missed <= acknowledged ? null : (
            <Alert
              tone="escalated"
              title="Events were dropped before Bridge saw them"
              action={
                <Button variant="ghost" size="sm" onClick={() => setAcknowledged(state.missed)}>
                  Noted
                </Button>
              }
            >
              {`${state.missed} events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.`}
            </Alert>
          )}

          {/* What a reclaim gave back. **Neutral, because nothing is wrong** —
              a branch kept for holding work nothing has taken is the safe
              setting working, and drawing it in the escalation hue would tell
              somebody the act failed when it did exactly what it promised.
              Dismissed by hand: a directory and a branch are what a person goes
              and looks at, and a notice that vanished while they did is one
              they cannot get back. */}
          {givenBack === null ? null : (
            <Alert
              tone="neutral"
              title="Worktree reclaimed"
              action={
                <Button variant="ghost" size="sm" onClick={() => setGivenBack(null)}>
                  Dismiss
                </Button>
              }
            >
              {reclaimed(givenBack)}
            </Alert>
          )}

          {/* A refusal Fleet named carries a `run_id`, its `fields` and its
              `chain`, so it is drawn whole rather than as one line of copy —
              its `message` names one problem even where several exist. A
              command Fleet did not answer carries no envelope and is drawn
              whole for the same reason: the code, the route and the wait are
              the whole of what a person has to hand on. Neither is reloadable,
              because a redraw re-runs no command. Everything else here is the
              form telling you what it will not send, which is guidance and not
              a failure. */}
          {commandFailure !== null ? (
            <FailureBlock
              failure={commandFailure}
              onCopied={setCopied}
              reloadable={false}
              onDismiss={() => setOutcome(null)}
            />
          ) : outcome === null || outcome.ok ? null : (
            <Alert
              tone="escalated"
              action={
                <Button variant="ghost" size="sm" onClick={() => setOutcome(null)}>
                  Dismiss
                </Button>
              }
            >
              {said(outcome)}
            </Alert>
          )}

          {/* One Job, read whole, in place of the board. Reviewing and deciding
              is one loop, so the detail is not a panel beside the list — and the
              list is what Escape and the control in the head both return to. */}
          {reading !== null ? (
            <Boundary region="the job detail" {...guarded}>
              <JobDetail
                job={reading}
                onReadDiff={readDiff}
                onOpenArtifact={openArtifact}
                onReadCall={readCall}
                onNeedMaterial={readEvidence}
                watched={state.watched}
                workflows={state.holds.workflows}
                manifests={state.holds.manifests}
                stale={!live}
                now={now}
                acting={acting === reading.id}
                approving={state.approving.includes(reading.id)}
                deciding={deciding === reading.id}
                observed={state.observed}
                journalled={state.journalled}
                resources={state.resources}
                examination={state.examination}
                // The one act here that changes nothing. It costs no model
                // call, and its answer arrives on the published state rather
                // than coming back — so a window reloaded mid-look still draws
                // what Fleet found.
                onExamine={(jobId) => void window.armada.examineJob(jobId)}
                recorded={{
                  footprint: state.footprint,
                  evidence: state.evidence,
                  diff: state.diff,
                }}
                onAct={(what, jobId) => setConfirming({ act: what, jobId })}
                onRedirect={(jobId, instruction) => void redirect(jobId, instruction)}
                onAnswer={(jobId, questionId, chose) => void answer(jobId, questionId, chose)}
                onOverrule={(jobId, reason) => void overrule(jobId, reason)}
                onRerun={(jobId) => void rerun(jobId)}
                onReport={report}
                onApprove={(jobId) => void approve(jobId)}
                onApproveReview={(jobId) => void decide(jobId, "approve")}
                onRequestChanges={(jobId, note) => void decide(jobId, "changes", note)}
                onReject={(jobId) => void decide(jobId, "reject")}
                onCopied={setCopied}
                onSaid={setTelling}
              />
            </Boundary>
          ) : auditing ? (
            /* Read across every Job rather than through one. The rate is the
               point, and a listing reached from a Job would show only the
               reports somebody already had reason to open. */
            <Boundary region="the filed reports" {...guarded}>
              <Reports reports={state.reports} onWant={readReports} onCopied={setCopied} />
            </Boundary>
          ) : clearing ? (
            /* What Fleet is holding disk for, read across every Job at once.
               The half of the reclaim rule that is a person's: Fleet has
               already taken back everything it could prove nobody needs, and
               this is where the rest is chosen from, item by item. */
            <Boundary region="the held worktrees" {...guarded}>
              <Worktrees
                held={state.held}
                onWant={readHeld}
                // The receipt belongs to the press that asked for it, so it is
                // answered to the surface rather than published: a reclaim
                // changes no row on the board, and a notice for one person's
                // gesture would outlive the screen they made it on.
                onReclaim={reclaimOne}
                // The same `now` every other elapsed figure in the window is
                // drawn from. Two clocks on one app drift, and this one is read
                // in days rather than seconds — but it is still the app's.
                now={now}
                onCopied={setCopied}
              />
            </Boundary>
          ) : composing ? (
            /* Describing the work is the path and the form is the override, so
               the composer is what `Enter by hand` swaps to rather than what
               opens. What Fleet holds is read over the one connection and not
               scraped off the Jobs already on the board, which is what this
               offered before `list_workflows` and `list_manifests` existed. */
            <Boundary region="the job composer" {...guarded}>
              <DispatchJob
                onPropose={proposeFrom}
                // What Fleet says the call is doing, against the same `now`
                // every other elapsed figure on screen is drawn from.
                watching={watchOf(state.proposing, now)}
                onStop={() => void stopProposal()}
                // A proposed Job is opened, never approved from here: approval
                // is a second act from detail, and this is the same signpost the
                // Board's own `awaiting_approval` row carries.
                onOpen={(jobId) => {
                  setComposing(false);
                  setOpenJob(jobId);
                }}
                disabled={!live}
                onCopied={setCopied}
                byHand={
                  <Composer
                    workflows={state.holds.workflows}
                    onStage={stageAttachment}
                    manifest={scoped}
                    models={state.holds.models}
                    disabled={!live}
                    onPropose={(draft) => {
                      void propose(draft);
                      setComposing(false);
                    }}
                  />
                }
              />
            </Boundary>
          ) : (
            <>
              {/* The boundary `docs/practices/react.md` names: a Job that cannot
                  be rendered must not blank the window, and the head above it
                  stays usable while the list says what it could not draw. */}
              <Boundary region="the job list" {...guarded}>
                <Jobs
                  onCursor={setCursor}
                  reach={reach}
                  jobs={state.jobs}
                  stale={!live}
                  now={now}
                  workflows={state.holds.workflows}
                  disconnected={live ? null : statement.headline}
                  selected={openJob}
                  onOpen={setOpenJob}
                  // The Board asks; this confirms. It is the same dialog the
                  // detail's own kill goes through, which is what keeps "Cancel
                  // holds initial focus" a rule with one implementation.
                  onKill={(jobId) => setConfirming({ act: "kill_job", jobId })}
                  onCompose={() => setComposing(true)}
                  onClearTerminal={(jobIds) => void clearTerminal(jobIds)}
                  onCopied={setCopied}
                />
              </Boundary>

              {/* Never merged into the list as a placeholder: a board that shows
                  nine of ten Jobs and says so is honest, one that shows nine is
                  not. One bad row is not a broken board, and hiding it is worse
                  than drawing it broken. */}
              {state.unreadable.map((row) => (
                <FailureBlock
                  key={row.job_id ?? row.fault}
                  failure={jobFailure(row, state.bridge)}
                  onCopied={setCopied}
                />
              ))}
            </>
          )}
        </div>
      </Shell>

      {/* Every destructive act confirms, and the confirmation states what
          happens and what survives rather than asking "are you sure". Cancel
          holds initial focus; the dialog owns that rule and this only supplies
          the words. */}
      {confirming === null ? null : (
        <Dialog
          open
          tone={CONFIRM[confirming.act].tone ?? "destructive"}
          title={CONFIRM[confirming.act].title}
          confirmLabel={ACT_LABEL[confirming.act]}
          onCancel={() => {
            setConfirming(null);
            setRestartNote("");
          }}
          onConfirm={() => void act(confirming.act, confirming.jobId)}
        >
          {CONFIRM[confirming.act].body}
          {/* The one confirmation that collects anything, and what it collects
              is optional — the button is never disabled on it, because leaving
              the field alone is the restart this dialog has always been. No
              `autoFocus`: the dialog puts initial focus on Cancel, and a
              second claim on it here would only lose to it. */}
          {confirming.act !== "restart_step" ? null : (
            <>
              <p>{RESTART_NOTE.says}</p>
              <Textarea
                label={RESTART_NOTE.label}
                rows={4}
                value={restartNote}
                onChange={(event) => setRestartNote(event.target.value)}
              />
            </>
          )}
        </Dialog>
      )}

      {/* The palette, over everything. It is the one surface present whatever
          else is — which is why it is a sibling of the shell rather than a
          child of a screen — and its context is the Job being read whole where
          there is one, and the Board where there is not. */}
      <Palette
        open={palette.open}
        onClose={palette.onClose}
        context={reading === null ? "board" : "detail"}
        on={onWhat === undefined ? null : `${onWhat.id} — ${onWhat.title}`}
        surfaces={SURFACES}
        filters={reading === null ? BOARD_TABS : []}
        jobs={state.jobs.map((job) => ({ id: job.id, label: `${job.id} — ${job.title}` }))}
        // Bridge serves no settings surface, so the section is empty and draws
        // no head. A head over nothing is the labelled blank this app refuses.
        settings={[]}
        dormant={dormantIn({
          reading: reading !== null,
          cursor,
          failing: failing !== null,
        })}
        onChoose={(choice) =>
          carryOut(choice, onWhat?.id ?? null, {
            openJob: setOpenJob,
            closeJob: close,
            compose: () => setComposing(true),
            surface: goTo,
            filter: (tabId) => reach.current?.tab(tabId as BoardTab),
            search: () => reach.current?.search(),
            copyDebugInfo: () => {
              if (failing !== null) copyDebugInfoFor(failing, setCopied);
            },
            confirm: (what, jobId) => setConfirming({ act: what, jobId }),
          })
        }
        // Every destructive act confirms, even from the palette. It hands the
        // act over and stays open behind the dialog, which is the way back.
        onConfirmAct={(id) => {
          const jobId = onWhat?.id;
          if (id === "kill" && jobId !== undefined) {
            setConfirming({ act: "kill_job", jobId });
          }
        }}
      />

      <CopiedToast copied={copied} />
      <SaidToast said={telling} />
    </>
  );
}
