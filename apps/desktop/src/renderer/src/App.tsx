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
// # What this file is, now that three subjects have left it
//
// What is left is the window: which surface is open, what keeps that consistent
// as events arrive, and what is drawn. Three things that are not the window are
// beside it, each because it is a subject rather than a line of wiring —
// `commands.ts` for what the host is asked to do, `failing.ts` for which
// failure is on screen, and `palette.ts` for what the palette can reach.

import { useEffect, useRef, useState } from "react";
import { Dialog, Textarea } from "@armada/components";

import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import { Boundary } from "@armada/shell";
import { Standing } from "./Standing";
import { CopiedToast, SaidToast, useCopied, useSaid } from "@armada/shell";
import { FailureBlock } from "@armada/shell";
import { jobFailure } from "@armada/shell";
import { headOf } from "@armada/shell";
import { Composer } from "@armada/screens";
import { DispatchJob } from "@armada/screens";
import { watchOf } from "@armada/screens";
import { Reports } from "@armada/screens";
import { Worktrees } from "@armada/screens";
import { JobDetail, type ConfirmableAct } from "@armada/screens";
import { ACT_LABEL, CONFIRM, RESTART_NOTE } from "@armada/screens";
import { Jobs } from "@armada/screens";
import { BOARD_TABS, type BoardReach, type BoardTab } from "@armada/screens";
import { carryOut, dormantIn } from "./palette";
import { failingIn } from "./failing";
import {
  examine,
  openArtifact,
  openPullRequest,
  openRemarkLink,
  readCall,
  readCheckOutput,
  followCheckOutput,
  readFrame,
  readDiff,
  readEvidence,
  readRemarks,
  readHeld,
  readReports,
  reclaimOne,
  showAgain,
  stageAttachment,
  useCommands,
  useWatching,
} from "./commands";
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

export function App() {
  const [state, setState] = useState<BridgeState>(WAITING);
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
  // Which act is waiting to be confirmed. **Nothing destructive happens on one
  // press** — every one of them ends something, so each states what happens and
  // what survives first.
  const [confirming, setConfirming] = useState<{ act: ConfirmableAct; jobId: string } | null>(
    null,
  );
  // What a person typed into the restart confirmation, which is the one
  // confirmation that collects anything. **Held beside `confirming` rather than
  // inside it**: the act being confirmed is what the palette and the step
  // header both set, and neither of them knows about a field.
  const [restartNote, setRestartNote] = useState("");
  /** The Manifest reading a person put away. A later read draws again. */
  const [readingSeen, setReadingSeen] = useState<string | null>(null);
  // Whether the command palette is up, and where the Board's cursor is.
  // **The cursor is mirrored, not owned** — the Board holds it and reports it,
  // so the palette can title its context block with the job its acts would act
  // on. Two cursors would drift.
  const palette = useCommandPalette();
  const [cursor, setCursor] = useState<string | null>(null);
  // What the palette can reach on the Board: the state filter, and the search
  // field. Both belong to that surface and stay there — see `BoardReach`.
  const reach = useRef<BoardReach | null>(null);

  // Every command the window can send, and what it holds while one is out.
  // Two of them end somewhere this file owns, so both are answered to rather
  // than reached for: a redispatch opens its replacement, and a re-read
  // publishes what came back.
  const commands = useCommands({ onOpen: setOpenJob, onRead: setState });

  // The open Job, read out of the list rather than copied beside it. A Job that
  // leaves the list — superseded, or gone from a resync — closes its own detail
  // rather than leaving a row on screen that Fleet no longer has.
  const reading = openJob === null ? null : (state.jobs.find((job) => job.id === openJob) ?? null);
  useEffect(() => {
    void window.armada.state().then(setState);
    return window.armada.subscribe(setState);
  }, []);

  // What main is asked to hold open for the Job being read: the Job itself, what
  // it holds on this machine, and its turns.
  useWatching(openJob);

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
  // Which failure is on screen, and which one `Copy debug info` would copy.
  // The order between them, and the reason there is one, are `failing.ts`.
  const { statement, fleet, commandFailure, failing } = failingIn({
    connection: state.connection,
    bridge: state.bridge,
    readAt: state.readAt,
    outcome: commands.outcome,
    uncaught,
    now,
  });
  const guarded = { bridge: state.bridge, onCopied: setCopied };

  function close(): void {
    setReturning(openJob);
    setOpenJob(null);
  }

  /**
   * Do the act the dialog collected, and put the dialog away first.
   *
   * **The note is read here rather than in the command.** Only the restart has
   * one, and the field that holds it belongs to the dialog below. A blank field
   * is not sent: `restartStep` drops it, so a person who opened the dialog and
   * typed nothing gets the restart they pressed for rather than the 422 a blank
   * note earns.
   */
  function confirmed(what: ConfirmableAct, jobId: string): void {
    const note = what === "restart_step" ? restartNote : undefined;
    setConfirming(null);
    setRestartNote("");
    void commands.act(what, jobId, note);
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
    refreshing: commands.refreshing,
    onCloseComposer: () => setComposing(false),
    onCompose: () => setComposing(true),
    onCloseReports: () => setAuditing(false),
    onReadReports: () => setAuditing(true),
    onCloseWorktrees: () => setClearing(false),
    onReadWorktrees: () => setClearing(true),
    onRefresh: () => void commands.refresh(),
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
        title={head?.title}
        summary={head?.summary}
        actions={head?.actions}
        // Which row the rail marks. The held worktrees are the one surface
        // other than the Board that draws, so everything else — a Job, the
        // composer, the reports — is the Board with something over it.
        showing={clearing ? SURFACE.worktrees : SURFACE.board}
        onSurface={goTo}
      >
        {/* Real CSS, not utilities: nothing Tailwind spells emits a rule in
            this app, so the class that bounds this box lives in the app's own
            stylesheet where it can be read, and in the components' one so a
            story can check it. `.armada-screen__mounted` says why. */}
        <div className="armada-screen__mounted">
          <Standing
            fleet={fleet}
            manifestReading={state.manifestReading}
            readingSeen={readingSeen}
            onReadingSeen={setReadingSeen}
            uncaught={uncaught}
            onUncaught={setUncaught}
            bridge={state.bridge}
            onCopied={setCopied}
            missed={state.missed}
            acknowledged={acknowledged}
            onAcknowledged={setAcknowledged}
            givenBack={commands.givenBack}
            onGivenBack={commands.setGivenBack}
            commandFailure={commandFailure}
            outcome={commands.outcome}
            onOutcome={commands.setOutcome}
          />

          {/* One Job, read whole, in place of the board. Reviewing and deciding
              is one loop, so the detail is not a panel beside the list — and the
              list is what Escape and the control in the head both return to. */}
          {reading !== null ? (
            <Boundary region="the job detail" {...guarded}>
              <JobDetail
                job={reading}
                onReadDiff={readDiff}
                onOpenArtifact={openArtifact}
                onOpenPullRequest={openPullRequest}
                onReadCall={readCall}
                onReadCheckOutput={readCheckOutput}
                onReadFrame={readFrame}
                onNeedMaterial={readEvidence}
                onNeedRemarks={readRemarks}
                watched={state.watched}
                workflows={state.holds.workflows}
                manifests={state.holds.manifests}
                stale={!live}
                now={now}
                acting={commands.acting === reading.id}
                approving={state.approving.includes(reading.id)}
                deciding={commands.deciding === reading.id}
                observed={state.observed}
                journalled={state.journalled}
                followed={state.followed}
                onFollowCheckOutput={followCheckOutput}
                resources={state.resources}
                history={state.history}
                examination={state.examination}
                // The one act here that changes nothing. It costs no model
                // call, and its answer arrives on the published state rather
                // than coming back — so a window reloaded mid-look still draws
                // what Fleet found.
                onExamine={examine}
                recorded={{
                  footprint: state.footprint,
                  evidence: state.evidence,
                  diff: state.diff,
                  remarks: state.remarks,
                }}
                onAct={(what, jobId) => setConfirming({ act: what, jobId })}
                onRedirect={(jobId, instruction) => void commands.redirect(jobId, instruction)}
                onAnswer={(jobId, questionId, chose) =>
                  void commands.answer(jobId, questionId, chose)
                }
                onOverrule={(jobId, reason) => void commands.overrule(jobId, reason)}
                onRaiseCap={(jobId, micros) => void commands.raiseCap(jobId, micros)}
                onRaiseTurnCap={(jobId, turns) => void commands.raiseTurns(jobId, turns)}
                onRerun={(jobId) => void commands.rerun(jobId)}
                onReport={commands.report}
                onShowAgain={showAgain}
                onApprove={(jobId) => void commands.approve(jobId)}
                onMergePullRequest={(jobId) => void commands.decide(jobId, "merge")}
                onApproveReview={(jobId) => void commands.decide(jobId, "approve")}
                onRequestChanges={(jobId, note) => void commands.decide(jobId, "changes", note)}
                onReject={(jobId) => void commands.decide(jobId, "reject")}
                onTakeUpRemarks={(jobId, remarks) => void commands.takeUpRemarks(jobId, remarks)}
                onOpenRemarkLink={(jobId, remarkId) => void openRemarkLink(jobId, remarkId)}
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
                // What the reading is read against is published state, so it is
                // handed over at the press rather than held by the command.
                onPropose={(request) =>
                  commands.proposeFrom(request, {
                    workflows: state.holds.workflows,
                    bridge: state.bridge,
                  })
                }
                // What Fleet says the call is doing, against the same `now`
                // every other elapsed figure on screen is drawn from.
                watching={watchOf(state.proposing, now)}
                onStop={() => void commands.stopProposal()}
                // A proposed Job is opened where somebody wants to read it
                // first, which is the same signpost the Board's own
                // `awaiting_approval` row carries.
                onOpen={(jobId) => {
                  setComposing(false);
                  setOpenJob(jobId);
                }}
                // And released without leaving, on the head of the proposal.
                // The same command the detail's own gate calls, so a second
                // approval is refused by the one guard rather than by two.
                onApprove={(jobId) => void commands.approve(jobId)}
                approving={state.approving}
                // What the board says each proposed Job is at now. The fold
                // `approveDispatch` does is what moves the row off its gate.
                statusOf={(jobId) => state.jobs.find((job) => job.id === jobId)?.status}
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
                      void commands.propose(draft);
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
                  onClearTerminal={(jobIds) => void commands.clearTerminal(jobIds)}
                  onForgetTerminal={(jobIds) => void commands.forgetTerminal(jobIds)}
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
          onConfirm={() => confirmed(confirming.act, confirming.jobId)}
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
