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

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { dockQuestionsOf, jobNumber, ofPicked } from "@armada/screens";
import type { Outstanding } from "@armada/screens";
import type { HelmContext, JobSummary } from "@armada/protocol";
import { useDockAnswering } from "./dock-answering";
import { HelmDock } from "./HelmDock";
import { chippedJobId, contextOf, cursorRowFor, dismissed, NO_CHIP, opened, screenOf } from "./helm-context";
import type { ChipState } from "./helm-context";
import { Dialog, Textarea } from "@armada/components";

import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import { Boundary } from "@armada/shell";
import { Standing } from "./Standing";
import { CopiedToast, SaidToast, useCopied, useSaid } from "@armada/shell";
import { FailureBlock } from "@armada/shell";
import { jobFailure } from "@armada/shell";
import { BoardActions } from "@armada/shell";
import { AskRepository } from "@armada/screens";
import { BridgeSettings } from "@armada/screens";
import { Reports } from "@armada/screens";
import { Composing } from "./Composing";
import { Overview } from "./Overview";
import { CaptureLayer, type CaptureAim } from "./capture/Layer";
import { StudiosSurface } from "./StudiosSurface";
import { nodeNamed, studioName, type OpenStudio } from "@armada/screens";
import { Worktrees } from "@armada/screens";
import { Manifest, checkoutRunnablesOf, useManifestEditing, useManifestForm } from "@armada/screens";
import { Setup, useSetup } from "@armada/screens";
import { Locate, LocatedNotice, useLocate } from "@armada/screens";
import { JobDetail, type ConfirmableAct } from "@armada/screens";
import { ACT_LABEL, CONFIRM, RESTART_NOTE } from "@armada/screens";
import { Jobs } from "@armada/screens";
import { BOARD_TABS, type BoardReach, type BoardTab } from "@armada/screens";
import { absentIn, carryOut, dormantIn } from "./palette";
import { failingIn } from "./failing";
import {
  examine,
  openArtifact,
  openPullRequest,
  openFindingIssue,
  openRemarkLink,
  openServerLink,
  observeRun,
  observeCheckoutRun,
  pickRepository,
  chooseFolder,
  resolveFolder,
  addRepository,
  cloneRepository,
  askHelm,
  startHelmFresh,
  pointHelm,
  getRunOutput,
  getCheckoutRunOutput,
  getCheckoutRunDiff,
  readManifestFile,
  saveManifestFile,
  editManifest,
  readManifestSpend,
  readRepositoryScan,
  readManifestProposals,
  editManifestProposal,
  writeManifestProposal,
  listRepositoryAllowedCommands,
  removeRepositoryAllowedCommand,
  listRuns,
  listCheckoutRuns,
  undoRun,
  undoCheckoutRun,
  explainCommand,
  readCall,
  readCheckOutput,
  followCheckOutput,
  readFrame,
  frameSrc,
  readDiff,
  readEvidence,
  readRemarks,
  deleteBranchOne,
  forgetOne,
  captureStudioNote,
  readHeld,
  readReports,
  reclaimOne,
  showAgain,
  startRun,
  startCheckoutRun,
  startCheckoutVerify,
  startServer,
  stopRun,
  stopCheckoutRun,
  stopServer,
  useCommands,
  useWatching,
  watchRunSheet,
  watchCheckoutRunSheet,
  watchManifestDrift,
  watchOverview,
} from "./commands";
import { useWhereOpen } from "./where-open";
import { usePanelOpen } from "./panel-open";
import { statsOf, fleetPanelOf } from "./left-column";
import { Palette, useCommandPalette } from "@armada/shell";
import { copyDebugInfoFor } from "@armada/shell";
import { Shell } from "@armada/shell";
import { SURFACE, SURFACES, useSurfaceKeys } from "@armada/shell";
import { useAtFloor } from "@armada/shell";
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
  // What has been reported against the Judge. Its own view: a report is filed
  // about one Job and the rate is read across all of them.
  const [auditing, setAuditing] = useState(false);
  // Whether the held worktrees are open. **Its own view for the reports' kind
  // of reason and not the same one**: what is decided there is which of a set
  // to give back, which no Job row can be asked, and putting a disk decision on
  // the Board would put a control nobody can act on beside rows that exist to
  // be acted on.
  const [clearing, setClearing] = useState(false);
  // Whether Settings is open — a rail surface since #1089, the sheet it
  // replaced having lost its own opener when the status bar went (#1088).
  const [settingsShowing, setSettingsShowing] = useState(false);
  // Whether the Manifest surface is open — Journey 9's *Running one*. **Its
  // own view, and it needs no Job to draw**: it is read off the file Fleet
  // already holds, which is what lets a person run this project's lint with
  // the Board empty.
  const [manifesting, setManifesting] = useState(false);
  // Whether Overview is open. **True from the first render**: Overview is
  // where Bridge opens (#921), so the Board is what a press away from it
  // reaches rather than the surface a fresh window starts on.
  const [overviewing, setOverviewing] = useState(true);
  // Whether the Studios surface is open, which Studio is open on it, and the node selected there —
  // #1287. The last two are Helm's context as well as the screen's.
  const [studying, setStudying] = useState(false);
  const [openStudio, setOpenStudio] = useState<OpenStudio | null>(null);
  const [studioNode, setStudioNode] = useState<string | null>(null);
  // What Studio capture lands on — #1290. **It outlives the surface**: what is
  // wrong is on the Board or Overview, not on the whiteboard, so the aim is the
  // Studio last open and continued rather than the one a surface is drawing.
  const [captureAim, setCaptureAim] = useState<CaptureAim>(null);
  // The Check or Command the palette picked, or `null`. **It selects rather
  // than runs**, which is what Journey 9's own table says the palette does.
  const [picked, setPicked] = useState<string | null>(null);
  // Whether Setup is the Manifest surface's view. Its own flag: the Manifest surface's own views are three.
  const [settingUp, setSettingUp] = useState(false);
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
  // Overview's own cursor, mirrored the same way — `OverviewLists` holds it
  // and reports it up. #1075.
  const [overviewCursor, setOverviewCursor] = useState<string | null>(null);
  // The Job chipped above Helm's message box — #1075. Opening a Job's detail
  // chips it and points Helm at its repository; leaving the Job or its own ×
  // drops the chip, and reopening the Job restores it. `helm-context.ts` is
  // the fold, tested on its own.
  const [chip, setChip] = useState<ChipState>(NO_CHIP);
  // What the palette can reach on the Board: the state filter, and the search
  // field. Both belong to that surface and stay there — see `BoardReach`.
  const reach = useRef<BoardReach | null>(null);

  // Every command the window can send, and what it holds while one is out.
  // Two of them end somewhere this file owns, so both are answered to rather
  // than reached for: a redispatch opens its replacement, and a re-read
  // publishes what came back.
  const commands = useCommands({ onOpen: setOpenJob, onRead: setState, jobs: state.jobs });
  // Whether the window is at `--window-floor`, `JobDetail`'s own reading —
  // Fleet settings is the same trailing layer and takes it the same way.
  const floor = useAtFloor();
  // Where things are' own open choice — held locally so a press moves it at
  // once, `#927`'s round trip off the critical path of a toggle.
  const [whereOpen, pressWhereOpen] = useWhereOpen(state.preferences.where_things_are_open);
  // The left column's own fold, remembered across a restart — Bridge/1088.
  const [statsOpen, setStatsOpen] = usePanelOpen("stats");
  const [fleetOpen, setFleetOpen] = usePanelOpen("fleet");

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

  // Opening or leaving a Job's detail folds the chip and, on opening, points
  // Helm at that Job's repository without moving the rail's own pick — #1075.
  // **Keyed on `openJob` alone.** `chip` is read as of the render this ran
  // in, not listed as a dependency: the chip's own × must never re-run this
  // and re-point Helm or un-dismiss what was just dismissed.
  useEffect(() => {
    const target = reading === null ? null : { id: reading.id, manifestId: reading.owner_manifest_id };
    const { state: next, point } = opened(chip, target);
    setChip(next);
    if (point !== null) pointHelm(point);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openJob]);

  // `⌘1`…`⌘n`, the binding the contract publishes and nothing answered until
  // the Manifest surface needed `⌘5`. One roster, read by the rail, the
  // palette and now the keyboard.
  useSurfaceKeys(goTo);

  // What this repository's Manifest declares, held open while the surface that
  // draws it is showing **or the palette is up**. The palette lists one row
  // per Check and Command off the same reading, so a read scoped to the
  // surface alone would leave those rows missing everywhere a person would
  // think to look for them.
  useEffect(() => {
    watchCheckoutRunSheet(manifesting || palette.open);
  }, [manifesting, palette.open]);

  // Drift is the surface's own free read on opening, and the palette lists
  // nothing off it. Verify is not here: it is only ever pressed.
  useEffect(() => {
    watchManifestDrift(manifesting);
  }, [manifesting]);

  // Fleet's health and every repository's drift in scope. Held for the life
  // of the window rather than only while Overview is showing — Bridge/1088's
  // Stats and Fleet panels draw the same two reads on every surface now.
  useEffect(() => {
    watchOverview(true);
    return () => watchOverview(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The Manifest file and an edit of it. **Held here rather than by the
  // screen**, which unmounts whenever the rail moves: an unsaved correction the
  // screen owned would be gone after one look at the Board.
  // The repository the rail picked. Main holds it, so every read below is taken after main moved.
  const repositories = state.holds.repositories ?? [];
  const repository = state.repository ?? undefined;
  // Whether Fleet has answered the listing. `models` is what says the holdings were read.
  const listed = state.connection.state === "connected" && state.holds.models !== null;
  // Locate, held here so a clone outlives its dialog, and opened by itself on a rail Fleet listed empty.
  const locate = useLocate({
    onChooseFolder: chooseFolder,
    onResolveFolder: resolveFolder,
    onAdd: addRepository,
    onClone: cloneRepository,
    nothingServed: listed && repositories.length === 0,
    landed: state.located,
    onLocated: (one) => {
      pickRepository(one.root);
      goTo(SURFACE.manifest);
      setSettingUp(true);
    },
  });
  const editing = useManifestEditing({
    repository,
    showing: manifesting,
    reading: state.manifestReading,
    onReadFile: readManifestFile,
    onSaveFile: saveManifestFile,
  });
  // The forms beside it, held here for the same reason.
  const form = useManifestForm({
    repository,
    showing: manifesting,
    reading: state.manifestReading,
    onReadFile: readManifestFile,
    onEditManifest: editManifest,
    onReadSpend: readManifestSpend,
  });

  // Setup, held here for the forms' reason: ticks and the open proposal outlive a look away.
  const setting = useSetup({
    repository,
    showing: manifesting && settingUp,
    onReadScan: readRepositoryScan,
    onReadProposals: readManifestProposals,
    onEditProposal: editManifestProposal,
    onWriteProposal: writeManifestProposal,
  });

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
      // A press a layer above already answered — the palette, a sheet — is not a second exit.
      if (event.key !== "Escape" || event.defaultPrevented) return;
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
    setManifesting(surfaceId === SURFACE.manifest);
    setOverviewing(surfaceId === SURFACE.overview);
    setSettingsShowing(surfaceId === SURFACE.settings);
    setStudying(surfaceId === SURFACE.studios);
    setOpenStudio(null);
    setStudioNode(null);
    if (surfaceId !== SURFACE.manifest) setPicked(null);
    if (surfaceId !== SURFACE.manifest) setSettingUp(false);
  }

  /** A repository nobody set up opens on Setup when picked: there is nothing else to do with it yet. `null` is All. */
  function pick(root: string | null): void {
    pickRepository(root);
    if (root === null || repositories.find((one) => one.root === root)?.manifest !== undefined) return;
    goTo(SURFACE.manifest);
    setSettingUp(true);
  }

  // What a new Job is proposed against: the picked repository's Manifest, absent until it has one.
  const pickedRepository = repositories.find((one) => one.root === state.repository) ?? null;
  const scoped = pickedRepository?.manifest;
  // All repositories: no one picked from a listing that has any. New job and the Manifest ask which.
  const all = pickedRepository === null && repositories.length > 0;
  // The Board's Jobs follow the pick. The status bar, the palette and held worktrees read every Job.
  const boardJobs = useMemo(() => ofPicked(state.jobs, pickedRepository), [state.jobs, pickedRepository]);
  // Where the person is, for Helm — #1075. `cursorRowFor` picks the Board's
  // or Overview's row by which screen is showing, so neither's stale row
  // reaches an ask made on the other.
  const helmScreen = screenOf({ reading: reading !== null, clearing, manifesting, overviewing, studying });
  const chippedJob = state.jobs.find((job) => job.id === chippedJobId(chip));
  const helmContext: HelmContext = contextOf({
    screen: helmScreen,
    picked: scoped?.id ?? null,
    chip: chippedJob?.id ?? null,
    cursor: cursorRowFor({ screen: helmScreen, board: cursor, overview: overviewCursor }),
    studio: openStudio?.id ?? null,
    node: studioNode,
  });
  const shownStudio = state.studio.state === "read" && state.studio.studio.id === openStudio?.id ? state.studio.studio : null;
  // Aimed during render rather than in an effect: it is a value this render
  // already knows, and an effect would leave capture a frame behind the Studio.
  const continued =
    openStudio?.editable === true && shownStudio !== null
      ? { id: shownStudio.id, name: studioName(shownStudio) }
      : null;
  if (continued !== null && (continued.id !== captureAim?.id || continued.name !== captureAim.name)) {
    setCaptureAim(continued);
  }
  // Helm's dock lists every repository's questions, whatever the pick. Answering is #936, Helm #944.
  const dockAnswering = useDockAnswering(commands);
  // "Discuss with Helm" points it at the card's own repository. The picker never moves for it.
  const onDiscussHelm = useCallback(
    (_question: Outstanding, job: JobSummary) => pointHelm(job.owner_manifest_id),
    [],
  );
  const questions = useMemo(
    () => dockQuestionsOf(state.questions, state.jobs, repositories, now, { ...dockAnswering, onDiscuss: onDiscussHelm }),
    [state.questions, state.jobs, repositories, now, dockAnswering, onDiscussHelm],
  );
  // The Board's own menu, drawn at the top of its content now that #1090
  // removed the page head it used to sit in — `BoardActions` unchanged, only
  // where it mounts.
  const boardActions = (
    <BoardActions
      jobs={boardJobs}
      live={live}
      refreshing={commands.refreshing}
      onCompose={() => setComposing(true)}
      onRefresh={() => void commands.refresh()}
      onReadReports={() => setAuditing(true)}
      onReadWorktrees={() => setClearing(true)}
      onOpenSettings={() => goTo(SURFACE.settings)}
      onClearTerminal={(jobIds) => void commands.clearTerminal(jobIds)}
      onForgetTerminal={(jobIds) => void commands.forgetTerminal(jobIds)}
      sweeping={commands.sweeping}
    />
  );

  return (
    <>
      <Shell
        connection={state.connection}
        repositories={repositories}
        listed={listed}
        scope={state.repository}
        onScope={pick}
        onAddRepository={locate.onOpen}
        onCompose={() => setComposing(true)}
        onSearch={palette.onOpen}
        boardJobs={boardJobs}
        questions={questions}
        helm={
          <HelmDock
            helm={state.helm}
            repositories={repositories}
            jobs={state.jobs}
            workflows={state.holds.workflows}
            live={live}
            chip={chippedJob === undefined ? undefined : { jobHandle: jobNumber(chippedJob), title: chippedJob.title }}
            onRemoveChip={() => setChip(dismissed)}
            onAsk={(text, context) => void askHelm(text, context)}
            context={helmContext}
            studio={
              shownStudio === null
                ? undefined
                : {
                    name: studioName(shownStudio),
                    ...(studioNode === null ? {} : { node: nodeNamed(shownStudio, studioNode, state.jobs) }),
                  }
            }
            onStartFresh={() => void startHelmFresh()}
            onSwitch={(manifestId) => pointHelm(manifestId)}
            onApprove={commands.approve}
          />
        }
        stats={{
          rows: statsOf(state.connection, state.jobs, state.capacity, repositories, state.repository, state.drifts),
          open: statsOpen,
          onOpenChange: setStatsOpen,
        }}
        fleet={{
          ...fleetPanelOf(state.connection, statement, state.health, now, state.readAt),
          open: fleetOpen,
          onOpenChange: setFleetOpen,
        }}
        // Which row the rail marks. The held worktrees are the one surface
        // other than the Board that draws, so everything else — a Job, the
        // composer, the reports — is the Board with something over it.
        showing={
          clearing
            ? SURFACE.worktrees
            : manifesting
              ? SURFACE.manifest
              : overviewing
                ? SURFACE.overview
                : settingsShowing
                  ? SURFACE.settings
                  : studying
                    ? SURFACE.studios
                    : SURFACE.board
        }
        onSurface={goTo}
      >
        {/* Real CSS, not utilities: nothing Tailwind spells emits a rule in
            this app, so the class that bounds this box lives in the app's own
            stylesheet where it can be read, and in the components' one so a
            story can check it. `.armada-screen__mounted` says why. */}
        <div className="armada-screen__mounted">
          <Standing
            fleet={fleet}
            // **Not while the file is on screen**, which draws the same
            // reading beside the text it is about. Twice at once is two places
            // to read one refusal and one to dismiss while the other stands.
            manifestReading={
              manifesting && editing.view === "file" ? null : state.manifestReading
            }
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
            taken={commands.taken}
            located={<LocatedNotice locating={locate} repositories={repositories} />}
          />

          {/* One Job, read whole, in place of the board. Reviewing and deciding
              is one loop, so the detail is not a panel beside the list — and the
              list is what Escape and the rail's own Job Board row both return to. */}
          {reading !== null ? (
            // Keyed by the Job, so a render that threw on one Job is not the
            // failure notice drawn over the next one opened.
            <Boundary key={reading.id} region="the job detail" {...guarded}>
              <JobDetail
                job={reading}
                onReadDiff={readDiff}
                onOpenArtifact={openArtifact}
                onOpenPullRequest={openPullRequest}
                onReadCall={readCall}
                onReadCheckOutput={readCheckOutput}
                onReadFrame={readFrame}
                onFrameSrc={frameSrc}
                onNeedMaterial={readEvidence}
                onNeedRemarks={readRemarks}
                watched={state.watched}
                workflows={state.holds.workflows}
                manifests={state.holds.manifests}
                stale={!live}
                now={now}
                acting={commands.acting === reading.id}
                actingAct={commands.acting === reading.id ? (commands.actingAct ?? undefined) : undefined}
                rerunningChecks={commands.rerunningChecks === reading.id}
                approving={state.approving.includes(reading.id)}
                deciding={commands.deciding === reading.id}
                decidingAct={
                  commands.deciding === reading.id ? (commands.decidingAct ?? undefined) : undefined
                }
                answered={commands.answeredOn(reading.id)}
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
                  handed: state.handed,
                  evidence: state.evidence,
                  diff: state.diff,
                  remarks: state.remarks,
                }}
                onAct={(what, jobId) => setConfirming({ act: what, jobId })}
                // Held on the header, so already confirmed: it sends what the dialog's own confirm sends.
                onActHeld={(what, jobId) => void commands.act(what, jobId)}
                onRedirect={(jobId, instruction) => void commands.redirect(jobId, instruction)}
                onAnswer={(jobId, questionId, chose) =>
                  void commands.answer(jobId, questionId, chose)
                }
                onAnswerCommand={(jobId, call, chose, note, rule) =>
                  void commands.answerCommand(jobId, call, chose, note, rule)
                }
                // A read beside the act it informs. It moves nothing, so it
                // goes straight through rather than under `acting`.
                onExplainCommand={explainCommand}
                onAnswerJudge={(jobId, askedAt, answer, note) =>
                  void commands.answerJudge(jobId, askedAt, answer, note)
                }
                onSetWhenBlocked={(jobId, whenBlocked) =>
                  void commands.setWhenBlocked(jobId, whenBlocked)
                }
                onSetWhenRefused={(jobId, whenRefused) =>
                  void commands.setWhenRefused(jobId, whenRefused)
                }
                onSetModel={(jobId, model) => void commands.setModel(jobId, model)}
                onSetReviewModel={(jobId, model) => void commands.setReviewModel(jobId, model)}
                onRemoveAllowedCommand={(jobId, run) =>
                  void commands.removeAllowedCommand(jobId, run)
                }
                models={state.holds.models}
                onOverrule={(jobId, reason) => void commands.overrule(jobId, reason)}
                // The card's Send it back: the restart act, with the note typed there.
                onSendBack={(jobId, note) => void commands.act("restart_step", jobId, note)}
                onRaiseCap={(jobId, micros) => void commands.raiseCap(jobId, micros)}
                onRaiseTurnCap={(jobId, turns) => void commands.raiseTurns(jobId, turns)}
                onRerun={(jobId) => void commands.rerun(jobId)}
                onRerunChecks={(jobId) => void commands.rerunChecks(jobId)}
                onReport={commands.report}
                onAddTask={commands.addTask}
                onDropTask={commands.dropTask}
                onShowAgain={showAgain}
                onApprove={(jobId) => void commands.approve(jobId)}
                onMergePullRequest={(jobId) => void commands.decide(jobId, "merge")}
                onRerunFailedChecks={(jobId) => void commands.rerunFailedChecks(jobId)}
                onInvestigateFailedChecks={(jobId) => void commands.investigateFailedChecks(jobId)}
                onQueueAfterFinding={(jobId, finding) => void commands.queueAfterFinding(jobId, finding)}
                onFileFindingIssue={(jobId, finding, title, body) => void commands.fileFindingIssue(jobId, finding, title, body)}
                onApproveReview={(jobId) => void commands.decide(jobId, "approve")}
                onRequestChanges={(jobId, note) => void commands.decide(jobId, "changes", note)}
                onReject={(jobId) => void commands.decide(jobId, "reject")}
                onTakeUpRemarks={(jobId, remarks) => void commands.takeUpRemarks(jobId, remarks)}
                onDismissFinding={(jobId, finding, reason) => void commands.dismissFinding(jobId, finding, reason)}
                onOpenRemarkLink={(jobId, remarkId) => void openRemarkLink(jobId, remarkId)}
                onOpenFindingIssue={(jobId, finding) => void openFindingIssue(jobId, finding)}
                onCopied={setCopied}
                onSaid={setTelling}
                whereOpen={whereOpen}
                onOpenWhere={pressWhereOpen}
                // `n` — the same composer every contextual surface opens.
                onCompose={() => setComposing(true)}
                // The run sheet — Journey 9 — and the servers it starts.
                rehearsal={{
                  runSheet: state.runSheet,
                  runFollowed: state.runFollowed,
                  servers: state.servers,
                  onWatchRunSheet: watchRunSheet,
                  onObserveRun: observeRun,
                  onStartRun: startRun,
                  onStopRun: stopRun,
                  onUndoRun: undoRun,
                  onListRuns: listRuns,
                  onGetRunOutput: getRunOutput,
                  onStartServer: startServer,
                  onStopServer: stopServer,
                  onOpenServerLink: openServerLink,
                }}
              />
            </Boundary>
          ) : auditing ? (
            /* Read across every Job rather than through one. The rate is the
               point, and a listing reached from a Job would show only the
               reports somebody already had reason to open. */
            <Boundary region="the filed reports" {...guarded}>
              <Reports
                reports={state.reports}
                onWant={readReports}
                onClose={() => setAuditing(false)}
                onCopied={setCopied}
              />
            </Boundary>
          ) : clearing ? (
            /* What Fleet is holding disk for, read across every Job at once.
               The half of the reclaim rule that is a person's: Fleet has
               already taken back everything it could prove nobody needs, and
               this is where the rest is chosen from, item by item. */
            <Boundary region="Cleanup" {...guarded}>
              <Worktrees
                held={state.held}
                // Read for the handle a `depended_on` reason names its
                // blocker by — the only fact this screen borrows from the
                // board rather than from `held` itself.
                jobs={state.jobs}
                onWant={readHeld}
                // Each receipt is answered to the press that asked for it: a
                // published notice would outlive the screen it was made on.
                onReclaim={reclaimOne}
                onDeleteBranch={deleteBranchOne}
                onForget={forgetOne}
                // The app's one `now`, because two clocks in one window drift.
                now={now}
                onClose={() => setClearing(false)}
                onCopied={setCopied}
              />
            </Boundary>
          ) : manifesting && all ? (
            <AskRepository
              repositories={repositories}
              title="Pick a repository to open its Manifest"
              next="The Board lists every repository's Jobs. A Manifest belongs to one, and picking it focuses the Board there."
              onPick={pick}
            />
          ) : manifesting ? (
            /* Everything this repository's Manifest declares, and one press
               that runs one of them in the checkout as it is on disk. No Job
               exists and none is created: the whole point of the surface is
               that a person can run this project's lint without one. */
            <Boundary region="the manifest" {...guarded}>
              <Manifest
                // Another repository is another page: its runs, its allows and a dismissed Verify are not this one's.
                key={state.repository ?? ""}
                sheet={state.checkoutRunSheet}
                followed={state.checkoutRunFollowed}
                picked={picked}
                now={now}
                onSaid={setTelling}
                editing={editing}
                form={form}
                onObserveRun={observeCheckoutRun}
                onStartRun={startCheckoutRun}
                drift={state.manifestDrift}
                onStartVerify={startCheckoutVerify}
                onStopRun={stopCheckoutRun}
                onUndoRun={undoCheckoutRun}
                onListRuns={listCheckoutRuns}
                onGetRunOutput={getCheckoutRunOutput}
                onGetRunDiff={getCheckoutRunDiff}
                onListRepositoryAllowedCommands={listRepositoryAllowedCommands}
                onRemoveRepositoryAllowedCommand={removeRepositoryAllowedCommand}
                // The one call this surface shares with a Job's own sheet:
                // `start_server` has taken an optional Job since it landed,
                // and no Job means the main checkout.
                onStartServer={(name) => startServer(name)}
                onStopServer={stopServer}
                onOpenServerLink={openServerLink}
                settingUp={settingUp}
                onSettingUp={setSettingUp}
                setUp={repository === undefined || scoped !== undefined}
                setup={
                  <Setup
                    setting={setting}
                    now={now}
                    sheet={state.checkoutRunSheet}
                    onStartVerify={startCheckoutVerify}
                    onStopRun={stopCheckoutRun}
                    onOpenEdit={() => {
                      setSettingUp(false);
                      editing.onView("form");
                    }}
                    floor={floor}
                  />
                }
              />
            </Boundary>
          ) : composing ? (
            <Composing
              state={state}
              commands={commands}
              now={now}
              live={live}
              all={all}
              repositories={repositories}
              scoped={scoped}
              onPick={pick}
              onOpen={setOpenJob}
              onClose={() => setComposing(false)}
              onCopied={setCopied}
            />
          ) : overviewing ? (
            <Overview
              state={state}
              now={now}
              live={live}
              repositories={repositories}
              disconnected={live ? null : statement.headline}
              selected={openJob}
              onOpen={setOpenJob}
              onKill={(jobId) => setConfirming({ act: "kill_job", jobId })}
              // Recently ended's own two, reusing the same confirmation
              // `JobDetail`'s header already goes through for both acts —
              // `reclaim_worktree` is that header's own word for Clear.
              onRedispatch={(jobId) => setConfirming({ act: "redispatch", jobId })}
              onClear={(jobId) => setConfirming({ act: "reclaim_worktree", jobId })}
              onCompose={() => setComposing(true)}
              onCopied={setCopied}
              onCursor={setOverviewCursor}
            />
          ) : studying ? (
            <StudiosSurface
              state={state}
              live={live}
              repositories={repositories}
              manifestId={scoped?.id}
              all={all}
              onPick={pick}
              open={openStudio}
              onOpenChange={setOpenStudio}
              selectedNode={studioNode}
              onSelectNode={setStudioNode}
              onCopied={setCopied}
            />
          ) : settingsShowing ? (
            <Boundary region="Settings" {...guarded}>
              <BridgeSettings
                limits={state.limits}
                live={live}
                health={state.health}
                onSave={commands.saveLimits}
              />
            </Boundary>
          ) : (
            <>
              {/* The boundary `docs/practices/react.md` names: a Job that cannot
                  be rendered must not blank the window, and the board's own
                  actions above it stay usable while the list says what it
                  could not draw. */}
              <Boundary region="the job list" {...guarded}>
                <Jobs
                  onCursor={setCursor}
                  reach={reach}
                  jobs={boardJobs}
                  stale={!live}
                  now={now}
                  workflows={state.holds.workflows}
                  served={listed ? repositories : null}
                  all={all}
                  disconnected={live ? null : statement.headline}
                  selected={openJob}
                  onOpen={setOpenJob}
                  // The Board asks; this confirms. It is the same dialog the
                  // detail's own kill goes through, which is what keeps "Cancel
                  // holds initial focus" a rule with one implementation.
                  onKill={(jobId) => setConfirming({ act: "kill_job", jobId })}
                  // Recently ended's own two, `Overview`'s own reason: one
                  // shared confirmation, whichever screen a row is asked from.
                  onRedispatch={(jobId) => setConfirming({ act: "redispatch", jobId })}
                  onClear={(jobId) => setConfirming({ act: "reclaim_worktree", jobId })}
                  onCompose={() => setComposing(true)}
                  actions={boardActions}
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

      {/* Studio capture — #1290. Last, so its layer paints over every surface
          and every overlay the shell draws under it. */}
      <CaptureLayer aim={captureAim} onCapture={captureStudioNote} />

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

      <Locate locating={locate} />

      {/* The palette, over everything. It is the one surface present whatever
          else is — which is why it is a sibling of the shell rather than a
          child of a screen — and its context is the Job being read whole where
          there is one, and the Board where there is not. */}
      <Palette
        open={palette.open}
        onClose={palette.onClose}
        // Three places, not two: a Studio open on its whiteboard is neither
        // the Board nor a job read whole, and the acts scoped to it act on the
        // board rather than on anything focused — #1364.
        context={reading !== null ? "detail" : shownStudio === null ? "board" : "studio"}
        // The block is titled with what its rows act on, which on a Studio is
        // the Studio: the acts scoped there put a node on the board.
        on={
          shownStudio !== null && reading === null
            ? `Studio — ${studioName(shownStudio)}`
            : onWhat === undefined
            ? null
            : `${onWhat.id} — ${onWhat.title}`
        }
        surfaces={SURFACES}
        filters={reading === null ? BOARD_TABS : []}
        // One row per Check and Command, off the same reading the Manifest
        // surface draws from — Journey 9's own table. Empty until that read
        // has answered, which is what the effect above holds open.
        runnables={checkoutRunnablesOf(state.checkoutRunSheet)}
        jobs={state.jobs.map((job) => ({ id: job.id, label: `${job.handle} — ${job.title}` }))}
        // Fleet settings is the section's first row. It carries no value,
        // because choosing it opens the sheet rather than stating a field.
        settings={[{ id: "fleet_settings", label: "Fleet settings" }]}
        dormant={dormantIn({
          reading: reading !== null,
          cursor,
          failing: failing !== null,
        })}
        absent={absentIn({ reading: reading !== null, cursor })}
        onChoose={(choice) =>
          carryOut(choice, onWhat?.id ?? null, {
            openJob: setOpenJob,
            closeJob: close,
            compose: () => setComposing(true),
            surface: goTo,
            run: (entryId) => {
              goTo(SURFACE.manifest);
              setPicked(entryId);
            },
            filter: (tabId) => reach.current?.tab(tabId as BoardTab),
            search: () => reach.current?.search(),
            copyDebugInfo: () => {
              if (failing !== null) copyDebugInfoFor(failing, setCopied);
            },
            confirm: (what, jobId) => setConfirming({ act: what, jobId }),
            openSetting: (id) => {
              if (id === "fleet_settings") goTo(SURFACE.settings);
            },
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
