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

import { useEffect, useState } from "react";
import { Alert, Button, Dialog } from "@armada/components";

import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState, Draft, Outcome } from "../../shared/bridge";
import type { FileReport } from "../../shared/protocol";
import { Boundary } from "./Boundary";
import { CopiedToast, useCopied } from "./CopiedToast";
import { FailureBlock } from "./FailureSurface";
import { fleetFailure, jobFailure, refusalFailure, uncaughtFailure } from "./failures";
import { headOf } from "./Head";
import { statementOf } from "./fleet";
import { Composer } from "./Composer";
import { Reports } from "./Reports";
import { JobDetail, type ConfirmableAct } from "./JobDetail";
import { ACT_LABEL, CONFIRM, said } from "./copy";
import { Jobs } from "./Jobs";
import { Shell } from "./Shell";
import { watchUncaught } from "./uncaught";
import type { Uncaught } from "./uncaught";

/** How often the elapsed figures are redrawn. They are read, so they must move. */
const TICK_MS = 1000;

/** Re-exported so nothing importing it has to learn a new path. */
export const WAITING: BridgeState = NOTHING_YET;

export function App() {
  const [state, setState] = useState<BridgeState>(WAITING);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // What has been read and acknowledged. The count itself belongs to the
  // connection and is never reset from here — a drop that happened, happened.
  const [acknowledged, setAcknowledged] = useState(0);
  const [now, setNow] = useState(() => Date.now());
  const [copied, setCopied] = useCopied();
  // What a boundary could never catch: a throw in a handler, and a rejected
  // promise from a `void`-ed preload call.
  const [uncaught, setUncaught] = useState<Uncaught | null>(null);
  // **The whole of navigation.** A list and a detail need one piece of state,
  // not a router: which Job is open, or none. The row is the control that sets
  // it and Escape is what clears it.
  const [openJob, setOpenJob] = useState<string | null>(null);
  // Whether the open Job's turns socket is held open. **Not navigation any
  // more.** It was, while watching swapped the whole surface for a transcript;
  // job detail holds the turns in its own record at every state now, so this
  // says only that a tab is open and no screen turns on it.
  const [observing, setObserving] = useState(false);
  // Whether the composer is open. It used to sit permanently above the list;
  // `New job` is what opens it now, so the surface is the list until somebody
  // asks for the form.
  const [composing, setComposing] = useState(false);
  // What has been reported against the Judge. Its own view for the reason the
  // head gives: a report is filed about one Job and the rate is read across all
  // of them.
  const [auditing, setAuditing] = useState(false);
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
  const [refreshing, setRefreshing] = useState(false);
  // Which Job has a decision on its work in flight. Separate from `acting`,
  // the header's act set: sharing one flag would grey out the header's kills
  // while a review note was being sent.
  const [deciding, setDeciding] = useState<string | null>(null);

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

  // Opening another Job drops the socket, and does it before the one below is
  // reopened — the rows in hand belong to the Job that was open, and carrying
  // them into a different one would be a transcript under the wrong title.
  useEffect(() => setObserving(false), [openJob]);

  // Which Job's turns main should hold a socket open for. Closed the moment the
  // tab is: the subscription exists only while somebody is reading it, and
  // nothing about it is written onto the Job.
  useEffect(() => {
    void window.armada.observeJob(observing ? openJob : null);
  }, [observing, openJob]);

  useEffect(() => watchUncaught(setUncaught), []);

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

  const live = state.connection.state === "connected";
  const statement = statementOf(state.connection, now, state.readAt);
  const fleet = fleetFailure(state.connection, statement, state.bridge, now);
  const refused = outcome !== null && !outcome.ok && outcome.why === "refused" ? outcome.error : null;
  const guarded = { bridge: state.bridge, onCopied: setCopied };

  async function propose(draft: Draft): Promise<void> {
    setOutcome(await window.armada.proposeJob(draft));
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
   * Do the confirmed act. **Four preload calls, not one with a discriminator**
   * — killing a Drone leaves the Job, killing the Job ends it, a redispatch
   * mints a replacement, and a restart puts a fresh Drone on the same worktree
   * at the step that stopped.
   *
   * A redispatch answers with the replacement's id, and the detail follows it:
   * the Job that was open is over, and the one worth reading is the new one.
   */
  async function act(act: ConfirmableAct, jobId: string): Promise<void> {
    setConfirming(null);
    setActing(jobId);
    try {
      const answer =
        act === "redispatch"
          ? await window.armada.redispatchJob(jobId)
          : act === "kill_drone"
            ? await window.armada.killDrone(jobId)
            : act === "restart_step"
              ? await window.armada.restartStep(jobId)
              : await window.armada.killJob(jobId);
      setOutcome(answer);
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

  const scoped = state.holds.manifests.find((held) => held.id === scope);
  const head = headOf({
    reading: reading !== null,
    composing,
    auditing,
    live,
    refreshing,
    onCloseJob: close,
    onCloseComposer: () => setComposing(false),
    onCompose: () => setComposing(true),
    onCloseReports: () => setAuditing(false),
    onReadReports: () => setAuditing(true),
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
        // The rail returns to the list from wherever you are. Both views it
        // closes are single pieces of state, so there is no history to unwind.
        onSurface={() => {
          setOpenJob(null);
          setComposing(false);
        }}
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

          {/* A refusal Fleet named carries a `run_id`, its `fields` and its
              `chain`, so it is drawn whole rather than as one line of copy —
              its `message` names one problem even where several exist.
              Everything else here is the form telling you what it will not send,
              which is guidance and not a failure. */}
          {refused !== null ? (
            <FailureBlock
              failure={refusalFailure(refused, state.bridge)}
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
                watched={state.watched}
                workflows={state.holds.workflows}
                manifests={state.holds.manifests}
                stale={!live}
                now={now}
                acting={acting === reading.id}
                approving={state.approving.includes(reading.id)}
                deciding={deciding === reading.id}
                observed={state.observed}
                recorded={{
                  footprint: state.footprint,
                  history: state.history,
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
                onObserve={setObserving}
                onCopied={setCopied}
              />
            </Boundary>
          ) : auditing ? (
            /* Read across every Job rather than through one. The rate is the
               point, and a listing reached from a Job would show only the
               reports somebody already had reason to open. */
            <Boundary region="the filed reports" {...guarded}>
              <Reports reports={state.reports} onCopied={setCopied} />
            </Boundary>
          ) : composing ? (
            /* What Fleet holds, read over the one connection. Not scraped off
               the Jobs already on the board, which is what this offered before
               `list_workflows` and `list_manifests` existed. */
            <Boundary region="the job composer" {...guarded}>
              <Composer
                workflows={state.holds.workflows}
                manifest={scoped}
                models={state.holds.models}
                disabled={!live}
                onPropose={(draft) => {
                  void propose(draft);
                  setComposing(false);
                }}
              />
            </Boundary>
          ) : (
            <>
              {/* The boundary `docs/practices/react.md` names: a Job that cannot
                  be rendered must not blank the window, and the head above it
                  stays usable while the list says what it could not draw. */}
              <Boundary region="the job list" {...guarded}>
                <Jobs
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
          onCancel={() => setConfirming(null)}
          onConfirm={() => void act(confirming.act, confirming.jobId)}
        >
          {CONFIRM[confirming.act].body}
        </Dialog>
      )}

      <CopiedToast copied={copied} />
    </>
  );
}
