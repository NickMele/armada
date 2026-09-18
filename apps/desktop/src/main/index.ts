import { app, BrowserWindow, dialog, ipcMain, nativeImage, net, Notification, protocol } from "electron";
import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import tokens from "@armada/tokens/tokens.json";
import { STUDIO_PROMOTIONS } from "@armada/protocol";
import { CHANNELS, NOTHING_YET } from "../shared/bridge";
import type { BridgeState, PickedView, Summons } from "../shared/bridge";
import type { StudioPromotion } from "@armada/protocol";
import type { Draft, HelmContext, StagedAttachment } from "@armada/protocol";
import type { AddTask, DropTask, FileReport } from "@armada/protocol";
import type {
  Artifact,
  CommandAnswer,
  HelmCallAnswer,
  JudgeAnswer,
  SaveLimits,
  SavePreference,
  WhenBlocked,
  WhenRefused,
} from "@armada/protocol";
import type { EditManifest, SaveManifestFile, StartCheckoutRun, StartRun } from "@armada/protocol";
import type { EditManifestProposal, WriteManifestProposal } from "@armada/protocol";
import type { StagedFrame, StudioCapture, StudioNodeByHand, StudioPosition } from "@armada/protocol";
import { ANNOTATE_FLAG } from "../shared/annotations";
import { handleAnnotations } from "./annotations";
import { FleetConnection } from "./connection";
import { handleTaps } from "./haptics";
import { installSounds } from "./dev-sounds";
import { resolvedFolder } from "./locating";
import { openArtifact } from "./open";
import { openFindingIssue, openPullRequest, openRemarkLink } from "./forge";
import { RemarksPoll } from "./remarks-poll";
import { openServerLink } from "./servers";
import { frameStream, FRAME_SCHEME } from "./streaming";
import { Attention, soundOf } from "./telling";

// Bridge's window, and the one connection under it.
//
// Bridge and Fleet have independent lifetimes: Jobs keep progressing with the
// window closed, and opening it reconnects to whatever Fleet is already running
// rather than starting one. Nothing here spawns Fleet.

/**
 * The mark, as the macOS app tile.
 *
 * Emitted beside this bundle by `electron.vite.config.ts` — see the note there
 * for why it is a file on disk rather than an import. **The accent-filled
 * variant is sanctioned for the app tile and nowhere else**, which is what this
 * carries and why nothing in the renderer may reach for it.
 *
 * **A PNG, and the `.icns` beside it is not what this reads.**
 * `nativeImage.createFromPath` returns an empty image for `.icns` — measured
 * against Electron 40.10.6, for the checked-in file and for one rebuilt with
 * `iconutil`, while the same iconset's PNG loads at 512×512. This line said
 * `AppIcon.icns` for a fortnight and the tile stayed Electron's the whole time.
 */
const APP_ICON = join(__dirname, "AppIcon.png");

// **Before the app is ready, because that is the only moment it can be.** A
// scheme a window may load has to be privileged before any window exists;
// `protocol.handle` below fills in what answers it, once there is a connection
// to forward to. `streaming.ts` says why a recording is not read whole.
protocol.registerSchemesAsPrivileged([
  {
    scheme: FRAME_SCHEME,
    privileges: {
      // `standard` gives the scheme an origin, which is what lets the policy in
      // `index.html` name it. `secure` keeps a page loaded from `file:` from
      // treating it as mixed content and refusing it on those grounds instead.
      standard: true,
      secure: true,
      // What a `<video>` needs: a partial answer, streamed. Without `stream`
      // the body is buffered whole, which is the thing this exists to stop.
      stream: true,
      supportFetchAPI: true,
      // **Never `bypassCSP`.** The policy admits this scheme by name; one that
      // ignored the policy would be the same hole in a different shape.
      bypassCSP: false,
      corsEnabled: false,
    },
  },
]);

/**
 * Wear the mark. **Runtime, not packaging** — `BrowserWindow`'s `icon` option
 * is ignored on macOS, and a packaged `Armada.app` already carries the `.icns`
 * through its own `Info.plist`. This is what makes the dock tile right when
 * Bridge is run unpackaged, which is how it is run all day: `electron-vite
 * preview` launches Electron's own bundle, and that bundle's icon is
 * Electron's.
 *
 * A failed read leaves Electron's own icon rather than stopping the app: a
 * window that will not open because of a picture is the wrong trade. **It says
 * so now.** Silence is what let a tile nobody had looked at go two builds
 * wearing Electron's mark, with every gate green — this is the only line main
 * writes, and it earns it by being the failure that has already happened.
 */
function wearTheMark(): void {
  if (process.platform !== "darwin" || app.dock === undefined) return;
  const icon = nativeImage.createFromPath(APP_ICON);
  if (icon.isEmpty()) {
    console.warn(`the app tile did not load, so the dock keeps Electron's: ${APP_ICON}`);
    return;
  }
  app.dock.setIcon(icon);
}

/** The hard window floor, from the token that exists to be read here. */
function floor(name: string): number {
  const found = tokens.tokens.find((token) => token.name === name);
  return found === undefined ? 0 : Number.parseInt(found.value, 10);
}

/**
 * Write pasted or picked bytes to a fresh staging directory, before any Job
 * exists to key storage on. One directory per file rather than one per
 * batch, keyed by a fresh uuid, so two attachments sharing a filename in one
 * paste never collide on disk.
 */
async function stageAttachment(
  bytes: ArrayBuffer,
  filename: string,
  _mimeType: string,
): Promise<{ path: string }> {
  const dir = join(app.getPath("temp"), "armada-attachments", randomUUID());
  await mkdir(dir, { recursive: true });
  const path = join(dir, filename);
  await writeFile(path, Buffer.from(bytes));
  return { path };
}

/** Enough of a capture to be worth sending. The rest is Fleet's to refuse. */
function isCapture(value: unknown): value is StudioCapture {
  if (typeof value !== "object" || value === null) return false;
  const capture = value as Record<string, unknown>;
  return typeof capture["selector"] === "string" && typeof capture["markup"] === "string";
}

/**
 * A PNG of the whole window the capture came from, written where an attachment
 * is staged. **The window, not the element**: a Note keeps what was on screen,
 * and the element's box within it says which part to look at.
 */
async function stagedFrame(event: Electron.IpcMainInvokeEvent): Promise<StagedFrame | null> {
  const image = await event.sender.capturePage();
  const png = image.toPNG();
  if (png.byteLength === 0) return null;
  const dir = join(app.getPath("temp"), "armada-frames", randomUUID());
  await mkdir(dir, { recursive: true });
  const staged = join(dir, "frame.png");
  await writeFile(staged, png);
  const size = image.getSize();
  return { staged_path: staged, width: size.width, height: size.height };
}

let connection: FleetConnection | null = null;

/** Whether any window is on screen and not minimized. A closed one is neither. */
function anyWindowShown(): boolean {
  return BrowserWindow.getAllWindows().some(
    (window) => !window.isDestroyed() && window.isVisible() && !window.isMinimized(),
  );
}

function createWindow(): BrowserWindow {
  const window = new BrowserWindow({
    width: 1280,
    height: 800,
    // Every layout is designed for resize rather than for the size it was built
    // at, and the floor is where that stops being the layout's problem.
    minWidth: floor("--window-floor"),
    show: false,
    // Frameless: the title row Bridge draws itself — #1087 — replaces macOS's
    // grey bar, which said only "Armada" while the app's own controls sat
    // lower. `x, y` is the top-left corner of the button cluster, not its
    // centre — a 12px-diameter button 22px down a 44px row sits low, which is
    // the correction pass's #1106 finding. 16 centres it: (44 - 12) / 2. The
    // row's own height and `TitleBar.css`'s left gutter are read against this
    // figure together, same gap that css already reports.
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 18, y: 16 },
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
      // The dev-only annotation layer, #1226: the preload exposes it on this flag alone.
      additionalArguments: app.isPackaged ? [] : [ANNOTATE_FLAG],
    },
  });

  window.on("ready-to-show", () => window.show());

  // The on-screen comments poll, #667: backgrounded or minimized stops the
  // 20 s timer, and any of these bringing a window back on screen resumes it.
  // `closed` is here rather than left to `window-all-closed`, because on
  // macOS the app and the connection both outlive a closed window.
  const tellVisibility = (): void => remarksPoll.shown(anyWindowShown());
  window.on("show", tellVisibility);
  window.on("hide", tellVisibility);
  window.on("minimize", tellVisibility);
  window.on("restore", tellVisibility);
  window.on("closed", tellVisibility);
  // This window's own pick and its per-window commands go with it.
  window.on("closed", () => {
    pickedViews.delete(window.id);
    connection?.dropWindow(window.id);
  });

  // **This window goes nowhere.** It loads one file and stays on it for the
  // life of the process, so every navigation and every new window is refused
  // outright rather than filtered — there is no address this app is meant to
  // reach, which makes an allowlist a thing that could be wrong and a refusal a
  // thing that cannot.
  //
  // Here because job detail now draws a real `<a href>` for a Job's pull
  // request. That anchor cancels its own default and hands the click to main,
  // but the middle click, the modifier click and the `target` a future one
  // forgets to cancel all route around a click handler — and `default-src
  // 'self'` does not cover top-level navigation. A window that navigated to a
  // forge would be a window with no rail, no shell and no way back: Electron's
  // version of the frozen surface this app was built to escape.
  //
  // `openExternal` is `forge.ts`'s, on a channel, from an address main read
  // off its own state. Nothing the renderer initiates reaches it.
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());

  // **Always the built renderer, and `dev` builds it first.** electron-vite
  // serves a dev server and publishes `ELECTRON_RENDERER_URL`; loading it would
  // be the obvious branch and it does not work here, because the React plugin
  // injects an inline module preamble in dev and `default-src 'self'` refuses
  // it. Relaxing the CSP is a security review rather than a local convenience,
  // so the build is what moves. Reported.
  void window.loadFile(join(__dirname, "../renderer/index.html"));
  return window;
}

/**
 * What main last published.
 *
 * Kept here rather than read back off the connection, because the one caller
 * needs a Job and its Manifest and nothing else — and `FleetConnection.state`
 * is five round trips to Fleet, which is the wrong price for a click on a path.
 */
let published: BridgeState = NOTHING_YET;

/**
 * The comments timer for whichever Job's review panel is on screen. #667.
 *
 * **Reads the port off `published` rather than holding one**, `review.ts`'s
 * reason for taking a port on every call rather than keeping it: this is
 * built once, long before a connection exists to read it from. `again` is
 * `ReviewMaterial.remarksChanged`, the route `job.remarks_changed` already
 * calls, so the sweep and this timer refresh the same way.
 */
const remarksPoll = new RemarksPoll({
  port: () => (published.connection.state === "connected" ? published.connection.fleet.port : null),
  again: (port, jobId) => connection?.material.remarksChanged(port, jobId) ?? Promise.resolve(),
});

/** Every window's own `PickedView`, kept apart from `published` and from every other window's. */
const pickedViews = new Map<number, PickedView>();

const NO_PICK: PickedView = {
  repository: null,
  manifestReading: null,
  health: { state: "none" },
  drifts: { state: "none" },
  checkoutRunSheet: { state: "none" },
  checkoutRunFollowed: { state: "none" },
  manifestDrift: { state: "none" },
};

function viewFor(windowId: number): PickedView {
  return pickedViews.get(windowId) ?? NO_PICK;
}

/** `state` with one window's own view folded in — `leftOut` nests under `holds`, everything else is top-level. */
function withView(state: BridgeState, view: PickedView): BridgeState {
  const { leftOut, ...rest } = view;
  return { ...state, ...rest, holds: { ...state.holds, leftOut } };
}

/** Every window sees the same state, save its own pick — `pickedViews`, overlaid here. */
function publish(state: BridgeState): void {
  published = state;
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send(CHANNELS.changed, withView(state, viewFor(window.id)));
  }
  // **Here rather than on chosen events**, because what decides a notification
  // is the needs-you set changing and this is the one funnel every change to
  // the list passes through. `readAt` is what tells a publish that carries a
  // reading from one that carries only a connection state.
  attention.read(state.jobs, state.readAt);
}

/** One window's own pick moved. Told to that window alone — nothing else here reaches the rest. */
function publishToWindow(windowId: number, change: Partial<PickedView>): void {
  const view = { ...viewFor(windowId), ...change };
  pickedViews.set(windowId, view);
  const window = BrowserWindow.getAllWindows().find((one) => one.id === windowId);
  if (window !== undefined && !window.isDestroyed()) {
    window.webContents.send(CHANNELS.changed, withView(published, view));
  }
}

/** Every window open right now — `connection.ts`'s `Wiring.windowIds`. */
function windowIds(): readonly number[] {
  return BrowserWindow.getAllWindows().map((window) => window.id);
}

/** The window an IPC call arrived from, or `0` where Electron cannot say — never a real window's id. */
function windowIdOf(event: Electron.IpcMainInvokeEvent): number {
  return BrowserWindow.fromWebContents(event.sender)?.id ?? 0;
}

/**
 * A press on a notification, waiting for a window to hand it to.
 *
 * **The press outlives the absence of a window, and that is the point.** The
 * whole feature is being told with Bridge closed, so a click that arrived with
 * nothing on screen has to open one and land in the right place rather than
 * open one at whatever it was last showing.
 */
let summoned: Summons | null = null;

/**
 * Go where a pressed notification says.
 *
 * A window that exists is raised and told. With none, one is opened and the
 * press is held until that window asks for state — which every window does on
 * mount, having already installed its listener, so nothing races.
 */
function summon(to: Summons): void {
  summoned = to;
  const [window] = BrowserWindow.getAllWindows();
  if (window === undefined || window.isDestroyed()) {
    createWindow();
    return;
  }
  if (window.isMinimized()) window.restore();
  window.show();
  window.focus();
  hand(window);
}

/** Hand over a held press, if there is one. Nothing is handed twice. */
function hand(window: BrowserWindow): void {
  if (summoned === null || window.isDestroyed()) return;
  window.webContents.send(CHANNELS.summoned, summoned);
  summoned = null;
}

/**
 * What Armada says when a Job starts waiting, and what the dock says while one
 * is. The rule and the words are `telling.ts`'s; these are the two effects.
 *
 * **Nothing here asks for permission.** The first `show()` is what makes macOS
 * ask, once, and a refusal turns this into a no-op with the rest of Bridge
 * untouched — see `telling.ts`. `isSupported` is the platform question rather
 * than the person's, and it is the only one this process can actually answer.
 */
const attention = new Attention({
  show: (told) => {
    if (!Notification.isSupported()) return;
    const banner = new Notification({ title: told.title, body: told.body, sound: soundOf(told.tone) });
    banner.on("click", () => summon({ jobId: told.jobId }));
    banner.show();
  },
  // The standing count, and the one signal that survives a refused permission.
  // `dock` is macOS's; nothing else in this workspace has a tile to badge.
  count: (waiting) => app.dock?.setBadge(waiting === 0 ? "" : String(waiting)),
  now: () => Date.now(),
});

void app.whenReady().then(() => {
  wearTheMark();
  if (!app.isPackaged) {
    // A sent note's screenshot: the part of the window the note points at, #1250.
    handleAnnotations(ipcMain, app.getAppPath(), async (event, box) => {
      const image = await (event as Electron.IpcMainInvokeEvent).sender.capturePage({
        x: Math.max(0, Math.round(box.x)),
        y: Math.max(0, Math.round(box.y)),
        width: Math.max(1, Math.round(box.width)),
        height: Math.max(1, Math.round(box.height)),
      });
      const png = image.toPNG();
      return png.buffer.slice(png.byteOffset, png.byteOffset + png.byteLength) as ArrayBuffer;
    });
  }

  // A recording plays from here, forwarded to the port main already holds.
  // **The port is read at request time and never captured**, for the reason
  // the comments timer reads it that way: this is built once, and a connection
  // comes and goes under it. `net.fetch` is Chromium's own stack, which is what
  // streams a span rather than buffering one.
  protocol.handle(
    FRAME_SCHEME,
    frameStream(
      () => (published.connection.state === "connected" ? published.connection.fleet.port : null),
      (url, init) => net.fetch(url, init),
    ),
  );

  connection = new FleetConnection({
    home: process.env["HOME"],
    publish,
    publishToWindow,
    windowIds,
    now: () => Date.now(),
  });
  handleTaps({ ipc: ipcMain, app });
  if (!app.isPackaged) installSounds(join(app.getAppPath(), "sounds"), join(app.getPath("home"), "Library", "Sounds"));

  // The renderer initiates exactly these things and no others. There is no
  // arbitrary-channel invoke, which is what keeps the surface readable.
  // Awaited, because `state()` brings the connection current first — a window
  // reload is a fresh reader and gets what exists rather than what main last
  // heard about. See `FleetConnection.state`.
  ipcMain.handle(CHANNELS.state, async (event) => {
    const state = await connection?.state();
    // A fresh window asks for state on mount with its listener already
    // installed, so this is the moment a press held while there was no window
    // can be handed over — and the only one that needs no second channel to
    // announce readiness.
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window !== null) hand(window);
    // This window's own pick, overlaid — a reload is a fresh reader, not a reset of it.
    return state === undefined ? state : withView(state, viewFor(windowIdOf(event)));
  });
  // Every act on a Job is reached through `commands` — see `command.ts`, which
  // holds them because they are HTTP and the connection is a socket.
  ipcMain.handle(CHANNELS.proposeJob, (_event, draft: Draft) =>
    connection?.commands.proposeJob(draft),
  );
  // The other way a Job reaches the same gate. Its own channel and not a mode
  // on the one above: that one carries a workflow the person chose and this
  // one carries the sentence they wrote, and one channel taking which would
  // make the model call a flag.
  ipcMain.handle(
    CHANNELS.proposeFromRequest,
    (event, request: string, attachments: StagedAttachment[], repository: string | null) =>
      connection?.commands.proposeFromRequest(
        request,
        attachments,
        repository,
        connection.repositories.pickedByWindow.of(windowIdOf(event)),
      ),
  );
  // No argument: what may be stopped is what this window started. See
  // `JobCommands.stopProposal`.
  ipcMain.handle(CHANNELS.stopProposal, () => connection?.commands.stopProposal());
  // Staging happens before a Job exists — there is no id yet to key storage
  // on, and one is minted at `propose` time. Fleet is not involved here at
  // all; this only writes bytes to a temp file `proposeJob` later names.
  ipcMain.handle(
    CHANNELS.stageAttachment,
    (_event, bytes: ArrayBuffer, filename: string, mimeType: string) =>
      stageAttachment(bytes, filename, mimeType),
  );
  ipcMain.handle(CHANNELS.searchFiles, (event, query: string) =>
    connection?.commands.searchFiles(query, connection.repositories.pickedByWindow.of(windowIdOf(event))),
  );
  ipcMain.handle(CHANNELS.approveDispatch, (_event, jobId: string) =>
    connection?.commands.approveDispatch(jobId),
  );
  ipcMain.handle(CHANNELS.redispatchJob, (_event, jobId: string) =>
    connection?.commands.redispatchJob(jobId),
  );
  // Two channels, because they are two acts: one ends a process and one ends
  // the unit of work. Collapsing them here would make the difference a flag.
  ipcMain.handle(CHANNELS.killDrone, (_event, jobId: string) =>
    connection?.commands.killDrone(jobId),
  );
  ipcMain.handle(CHANNELS.killJob, (_event, jobId: string) =>
    connection?.commands.killJob(jobId),
  );
  // The disk rather than the record, and the one act here `armada clean` could
  // already do — but only with Fleet stopped, which is never when a person
  // wants the space back. Every row stays on the board afterwards, under
  // `Cleared`. One call per id, in main, because main is what holds the board
  // these ids came off of — the renderer decides which ids are terminal and
  // sends the set once.
  // The re-read afterwards is not bookkeeping: whether the row is still held is
  // Fleet's reading, and a checkout that would not go has to stay on the list.
  ipcMain.handle(CHANNELS.clearTerminalJobs, async (_event, jobIds: string[]) => {
    const outcome = await connection?.commands.clearTerminalJobs(jobIds);
    await connection?.rereadHeld();
    return outcome;
  });
  // Real deletion, and the only channel here where that is true. One call per
  // id, in main, for `clearTerminalJobs`'s reason above.
  ipcMain.handle(CHANNELS.forgetTerminalJobs, (_event, jobIds: string[]) =>
    connection?.commands.forgetTerminalJobs(jobIds),
  );
  // The disk rather than the record, and the one act here `armada clean`
  // could already do — but only with Fleet stopped, which is never when a
  // person wants the space back. The Job stays on the board afterwards,
  // under `Cleared`.
  // The re-read afterwards is not bookkeeping: whether the row is still held is
  // Fleet's reading, and a checkout that would not go has to stay on the list.
  // Folding the receipt in instead would be Bridge deciding a worktree is gone.
  ipcMain.handle(CHANNELS.reclaimWorktree, async (_event, jobId: string) => {
    const outcome = await connection?.commands.reclaimWorktree(jobId);
    await connection?.rereadHeld();
    return outcome;
  });
  // A force, unlike the reclaim above. The re-read afterwards is the same
  // reason: whether the branch still stands is Fleet's reading, and a delete
  // that refused has to stay on the list exactly as it was.
  ipcMain.handle(CHANNELS.deleteBranch, async (_event, jobId: string, tip: string) => {
    const outcome = await connection?.commands.clearing.deleteBranch(jobId, tip);
    await connection?.rereadHeld();
    return outcome;
  });
  // The per-Job half of `forgetTerminalJobs` above. Real deletion, and there
  // is no undo. The re-read afterwards keeps Cleanup's own list honest: a
  // forgotten Job's row is gone from `worktrees_held`, which walks Job
  // records rather than directories.
  ipcMain.handle(CHANNELS.forgetJob, async (_event, jobId: string) => {
    const outcome = await connection?.commands.forgetJob(jobId);
    await connection?.rereadHeld();
    return outcome;
  });
  // The two acts that resume a step without redispatching. Which applies is
  // decided by whether the Job still holds a Drone; Fleet is the authority
  // and refuses the wrong one rather than Bridge picking silently.
  ipcMain.handle(CHANNELS.answerQuestion, (_event, jobId: string, questionId: string, chose: string) =>
    connection?.commands.answerQuestion(jobId, questionId, chose),
  );
  // A command the drone was not given, answered where it waits or where it was
  // refused. The note rides with a reject, and the rule with an always-allow —
  // nothing else reads either.
  ipcMain.handle(
    CHANNELS.answerCommand,
    (
      _event,
      jobId: string,
      call: string,
      answer: CommandAnswer,
      note?: string,
      rule?: string,
    ) => connection?.commands.answerCommand(jobId, call, answer, note, rule),
  );
  // One call helm was held on, answered in the dock. No job id: the session is
  // waiting inside its own tool call, and nothing on the board moves. #1389.
  ipcMain.handle(
    CHANNELS.answerHelmCall,
    (_event, call: string, answer: HelmCallAnswer, note?: string) =>
      connection?.commands.answerHelmCall(call, answer, note),
  );
  // What that command does, read for the person deciding. It moves nothing and
  // decides nothing — the answers above stay live while it is out.
  ipcMain.handle(CHANNELS.explainCommand, (_event, jobId: string, callId: string) =>
    connection?.commands.explainCommand(jobId, callId),
  );
  // How the job meets the next such command. Moves nothing on the job.
  ipcMain.handle(CHANNELS.setWhenBlocked, (_event, jobId: string, whenBlocked: WhenBlocked) =>
    connection?.commands.setWhenBlocked(jobId, whenBlocked),
  );
  // The question a judge refusal opened, answered: agree, disagree once, or
  // disagree always.
  ipcMain.handle(
    CHANNELS.answerJudge,
    (_event, jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) =>
      connection?.commands.answerJudge(jobId, askedAt, answer, note),
  );
  // How the job meets the next such refusal. Moves nothing on the job.
  ipcMain.handle(CHANNELS.setWhenRefused, (_event, jobId: string, whenRefused: WhenRefused) =>
    connection?.commands.setWhenRefused(jobId, whenRefused),
  );
  // The model the job's next step starts on, and an allow taken back. Neither
  // moves anything on the job.
  ipcMain.handle(CHANNELS.setModel, (_event, jobId: string, model: string | null) =>
    connection?.commands.setModel(jobId, model),
  );
  ipcMain.handle(CHANNELS.setReviewModel, (_event, jobId: string, model: string | null) =>
    connection?.commands.setReviewModel(jobId, model),
  );
  ipcMain.handle(CHANNELS.removeAllowedCommand, (_event, jobId: string, run: string) =>
    connection?.commands.removeAllowedCommand(jobId, run),
  );
  ipcMain.handle(CHANNELS.redirectDrone, (_event, jobId: string, instruction: string) =>
    connection?.commands.redirectDrone(jobId, instruction),
  );
  ipcMain.handle(CHANNELS.restartStep, (_event, jobId: string, note?: string) =>
    connection?.commands.restartStep(jobId, note),
  );
  // The third act on an escalated Job, and the only one that keeps the work the
  // gate refused. Its own channel rather than a flag on `approveReview`: that
  // answers a gate nothing objected to, and this answers one that refused.
  ipcMain.handle(CHANNELS.overrideVerdict, (_event, jobId: string, reason: string) =>
    connection?.commands.overrideVerdict(jobId, reason),
  );
  // The answer at a gate that could not rule, which is a different place again:
  // the override lifts a decision and this asks for one. Its own channel for
  // the reason the two routes are two — the triggers partition, and neither act
  // is legal where the other one is.
  ipcMain.handle(CHANNELS.rerunGate, (_event, jobId: string) =>
    connection?.commands.rerunGate(jobId),
  );
  // A stopped step's own Checks, asked again. Its own channel beside the
  // gate re-run's: `#1105`, and the two triggers partition the same way.
  ipcMain.handle(CHANNELS.rerunChecks, (_event, jobId: string) =>
    connection?.commands.rerunChecks(jobId),
  );
  ipcMain.handle(CHANNELS.showAgain, (_event, jobId: string, spec?: string) =>
    connection?.commands.showAgain(jobId, spec),
  );
  // More money for one job, on a channel of its own because nothing else here
  // sets a value on a job. It moves no status and asks for no drone: what it
  // stops is the next dispatch being refused for money.
  ipcMain.handle(CHANNELS.raiseCostCap, (_event, jobId: string, costCapMicros: number) =>
    connection?.commands.raiseCostCap(jobId, costCapMicros),
  );
  // More turns for one job, on a channel of its own beside the cost cap's. Two
  // routes on the wire and two channels here: a job held on turns is not
  // started by more money, so one channel carrying either would report a
  // success that left the job exactly where it was.
  ipcMain.handle(CHANNELS.raiseTurnCap, (_event, jobId: string, turnCap: number) =>
    connection?.commands.raiseTurnCap(jobId, turnCap),
  );
  // Fleet's three admission limits. Fleet-wide, so no Job id rides this
  // channel — the only one among the acts above that names none.
  ipcMain.handle(CHANNELS.saveLimits, (_event, values: SaveLimits) =>
    connection?.commands.saveLimits(values),
  );
  // A person's Bridge preferences. Fleet-wide, so no Job id rides this
  // channel either.
  ipcMain.handle(CHANNELS.savePreference, (_event, save: SavePreference) =>
    connection?.commands.savePreference(save),
  );
  // Saying a job failed in error. Its own channel beside the override rather
  // than a flag on it: the override moves the job past a verdict and this moves
  // nothing, and the two would otherwise be one press meaning either.
  ipcMain.handle(CHANNELS.fileReport, (_event, jobId: string, filing: FileReport) =>
    connection?.commands.fileReport(jobId, filing),
  );
  // A person's own add or drop of a task on the plan — `connection.planEdits`,
  // `plan-edits.ts`. Neither is `commands`' — the answer is the plan the
  // change leaves rather than an `Outcome`, and that plan is folded straight
  // into the open Job's detail rather than through `command.ts`'s board.
  ipcMain.handle(CHANNELS.addTask, (_event, jobId: string, add: AddTask) =>
    connection?.planEdits.add(jobId, add),
  );
  ipcMain.handle(CHANNELS.dropTask, (_event, jobId: string, drop: DropTask) =>
    connection?.planEdits.drop(jobId, drop),
  );
  // Which Job is open. Main does the reading and republishes it as events
  // arrive, so the detail moves without the renderer asking again.
  ipcMain.handle(CHANNELS.watchJob, (_event, jobId: string | null) =>
    connection?.watchJob(jobId),
  );
  // Which Job's turns are open. A second socket to Fleet, carrying rows only:
  // there is nothing to send up it, which is what keeps observing read-only.
  ipcMain.handle(CHANNELS.observeJob, (_event, jobId: string | null) =>
    connection?.observeJob(jobId),
  );
  // Which running Check's log is open. A fourth socket, carrying lines only,
  // and read-only for `observeJob`'s reason.
  ipcMain.handle(
    CHANNELS.followCheckOutput,
    (_event, jobId: string | null, kept: string | null) =>
      connection?.followCheckOutput(jobId, kept),
  );
  // Which Job's transition history is unfolded. One HTTP read, kept current
  // while it is open, and dropped when the section closes — a history is its
  // own operation precisely so a Job opened does not pay for it.
  ipcMain.handle(CHANNELS.readHistory, (_event, jobId: string | null) =>
    connection?.readHistory(jobId),
  );
  // What the open Job holds on this machine. Opened with the Job and re-read
  // on every event naming it — its own channel because it is its own operation
  // and because it walks a process table, which a Job's detail does not.
  ipcMain.handle(CHANNELS.readResources, (_event, jobId: string | null) =>
    connection?.readResources(jobId),
  );
  // The run sheet, Journey 9. Opened by the sheet rather than by the Job.
  // Every act below is `connection.rehearsal`'s — see `rehearsal.ts`.
  ipcMain.handle(CHANNELS.watchRunSheet, (_event, jobId: string | null) =>
    connection?.rehearsal.watchRunSheet(jobId),
  );
  // One run's output, `followCheckOutput`'s reason and its own socket for it.
  ipcMain.handle(CHANNELS.observeRun, (_event, jobId: string | null, runId: string | null) =>
    connection?.rehearsal.observeRun(jobId, runId),
  );
  // A rehearsal in this Job's own worktree — no Evidence, nothing on the Job
  // moves. Opens `observeRun` for the caller the moment the run exists.
  ipcMain.handle(CHANNELS.startRun, (_event, jobId: string, body: StartRun) =>
    connection?.rehearsal.startRun(jobId, body),
  );
  ipcMain.handle(CHANNELS.stopRun, (_event, jobId: string, runId: string) =>
    connection?.rehearsal.stopRun(jobId, runId),
  );
  ipcMain.handle(CHANNELS.undoRun, (_event, jobId: string, runId: string) =>
    connection?.rehearsal.undoRun(jobId, runId),
  );
  ipcMain.handle(CHANNELS.listRuns, (_event, jobId: string) => connection?.rehearsal.listRuns(jobId));
  ipcMain.handle(CHANNELS.getRunOutput, (_event, jobId: string, runId: string) =>
    connection?.rehearsal.getRunOutput(jobId, runId),
  );
  // The same rehearsal in this window's own checkout — the Manifest surface. Held open
  // while that surface is showing or the palette is up, since the palette
  // lists one row per Check and Command off this reading.
  ipcMain.handle(CHANNELS.watchCheckoutRunSheet, (event, want: boolean) =>
    connection?.rehearsal.watchCheckoutRunSheet(windowIdOf(event), want),
  );
  ipcMain.handle(CHANNELS.observeCheckoutRun, (event, runId: string | null) =>
    connection?.rehearsal.observeCheckoutRun(windowIdOf(event), runId),
  );
  // A run in the tree a person is working in. **A name and nothing else** —
  // there is no frozen Manifest to choose against and no diff to narrow to.
  ipcMain.handle(CHANNELS.startCheckoutRun, (event, body: StartCheckoutRun) =>
    connection?.rehearsal.startCheckoutRun(windowIdOf(event), body),
  );
  ipcMain.handle(CHANNELS.stopCheckoutRun, (event, runId: string) =>
    connection?.rehearsal.stopCheckoutRun(windowIdOf(event), runId),
  );
  ipcMain.handle(CHANNELS.undoCheckoutRun, (event, runId: string) =>
    connection?.rehearsal.undoCheckoutRun(windowIdOf(event), runId),
  );
  ipcMain.handle(CHANNELS.listCheckoutRuns, (event) => connection?.rehearsal.listCheckoutRuns(windowIdOf(event)));
  ipcMain.handle(CHANNELS.getCheckoutRunOutput, (event, runId: string) =>
    connection?.rehearsal.getCheckoutRunOutput(windowIdOf(event), runId),
  );
  // What one run changed, against the snapshot it took — never `HEAD`. A read.
  ipcMain.handle(CHANNELS.getCheckoutRunDiff, (event, runId: string) =>
    connection?.rehearsal.getCheckoutRunDiff(windowIdOf(event), runId),
  );
  // Drift, held open by the Manifest surface; Verify, only ever pressed there.
  ipcMain.handle(CHANNELS.watchManifestDrift, (event, want: boolean) =>
    connection?.rehearsal.watchManifestDrift(windowIdOf(event), want),
  );
  // Overview's health and per-repository drift, held open by that surface.
  ipcMain.handle(CHANNELS.watchOverview, (event, want: unknown) =>
    connection?.overviewFor(windowIdOf(event)).watch(want === true),
  );
  // Helm's conversation: say something, forget it, and point it without moving the rail's own
  // pick. Fleet-wide, unlike the window's own reads above: one conversation per repository,
  // whichever window's dock is open on it.
  ipcMain.handle(CHANNELS.askHelm, (_event, text: string, context?: HelmContext) =>
    connection?.askHelm(text, context),
  );
  ipcMain.handle(CHANNELS.startHelmFresh, () => connection?.startHelmFresh());
  ipcMain.handle(CHANNELS.pointHelm, (_event, manifestId: string) => connection?.pointHelm(manifestId));
  ipcMain.handle(CHANNELS.startCheckoutVerify, (event, workspace: unknown) =>
    connection?.rehearsal.startCheckoutVerify(windowIdOf(event), typeof workspace === "string" ? workspace : undefined),
  );
  // The Manifest file, read and saved, and Setup below it — each window's own, `connection.ts`'s
  // `editingFor`: Fleet resolves the path and guards the write against a file that moved; nothing
  // here composes either.
  ipcMain.handle(CHANNELS.readManifestFile, (event) => connection?.editingFor(windowIdOf(event)).readFile());
  ipcMain.handle(CHANNELS.saveManifestFile, (event, body: SaveManifestFile) =>
    connection?.editingFor(windowIdOf(event)).saveFile(body),
  );
  ipcMain.handle(CHANNELS.editManifest, (event, body: EditManifest) =>
    connection?.editingFor(windowIdOf(event)).edit(body),
  );
  ipcMain.handle(CHANNELS.readManifestSpend, (event) => connection?.editingFor(windowIdOf(event)).readSpend());
  ipcMain.handle(CHANNELS.readRepositoryScan, (event) => connection?.editingFor(windowIdOf(event)).setup.readScan());
  ipcMain.handle(CHANNELS.readManifestProposals, (event) =>
    connection?.editingFor(windowIdOf(event)).setup.readProposals(),
  );
  ipcMain.handle(CHANNELS.editManifestProposal, (event, body: EditManifestProposal) =>
    connection?.editingFor(windowIdOf(event)).setup.edit(body),
  );
  ipcMain.handle(CHANNELS.writeManifestProposal, (event, body: WriteManifestProposal) =>
    connection?.editingFor(windowIdOf(event)).setup.write(body),
  );
  // `null` is All repositories. Anything but a string or `null` is dropped here; a root Fleet does not list, in `Picked.pick`.
  ipcMain.handle(CHANNELS.pickRepository, (event, root: unknown) =>
    typeof root === "string" || root === null ? connection?.repositories.pick(windowIdOf(event), root) : undefined,
  );
  // Locate. The folder dialog is sheeted to the window that asked, so it cannot be left behind it.
  ipcMain.handle(CHANNELS.chooseFolder, async (event) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    const options: Electron.OpenDialogOptions = { properties: ["openDirectory", "createDirectory"] };
    const chosen = window === null ? await dialog.showOpenDialog(options) : await dialog.showOpenDialog(window, options);
    return chosen.canceled ? null : (chosen.filePaths[0] ?? null);
  });
  // A path in, its canonical folder out: the clone preview names what Fleet will. Reads nothing inside it.
  ipcMain.handle(CHANNELS.resolveFolder, (_event, path: unknown) => (typeof path === "string" ? resolvedFolder(path) : null));
  ipcMain.handle(CHANNELS.addRepository, (_event, path: unknown) =>
    typeof path === "string" ? connection?.repositories.locating.add(path) : undefined,
  );
  ipcMain.handle(CHANNELS.cloneRepository, (_event, url: unknown, parent: unknown) =>
    typeof url === "string" && typeof parent === "string"
      ? connection?.repositories.locating.clone(url, parent)
      : undefined,
  );
  // A repository-wide always-allow — Fleet's own table since protocol 13.5.
  // Neither takes a path or a job id: Fleet names the repository this window picked.
  ipcMain.handle(CHANNELS.listRepositoryAllowedCommands, (event) =>
    connection?.repositoryAllowsFor(windowIdOf(event)).list(),
  );
  ipcMain.handle(CHANNELS.removeRepositoryAllowedCommand, (event, run: string) =>
    connection?.repositoryAllowsFor(windowIdOf(event)).remove(run),
  );
  // A declared server, for this Job's worktree or the main checkout where no
  // Job is named. `servers` on the published state is what keeps a *Serving*
  // row on screen after the sheet that started it closes.
  ipcMain.handle(CHANNELS.startServer, (event, name: string, jobId?: string) =>
    connection?.rehearsal.startServer(name, jobId, connection.repositories.pickedByWindow.of(windowIdOf(event))),
  );
  ipcMain.handle(CHANNELS.stopServer, (_event, serverId: string) =>
    connection?.rehearsal.stopServer(serverId),
  );
  // The third channel that leaves this machine. The address is checked against
  // what main published for that server before anything is handed to the OS.
  ipcMain.handle(CHANNELS.openServerLink, (_event, serverId: string, url: string) =>
    openServerLink(published, serverId, url),
  );
  // The act above that read. It moves nothing, costs no model call, and the
  // answer it publishes is also written into the Job's own log.
  ipcMain.handle(CHANNELS.examineJob, (_event, jobId: string) =>
    connection?.examineJob(jobId),
  );
  // The three reads a review is made of. Three channels because they are three
  // operations: the claims are four lines a step, the diff is the patch, and
  // the conversation is a forge. Each is read only where somebody is looking.
  ipcMain.handle(CHANNELS.readEvidence, (_event, jobId: string | null) =>
    connection?.readEvidence(jobId),
  );
  ipcMain.handle(CHANNELS.readDiff, (_event, jobId: string | null) =>
    connection?.readDiff(jobId),
  );
  // The panel's own 20 s timer, #667 — same Job as the read this triggers,
  // so the two can never end up watching different pull requests.
  ipcMain.handle(CHANNELS.readRemarks, (_event, jobId: string | null) => {
    remarksPoll.watch(jobId);
    return connection?.readRemarks(jobId);
  });
  // The rest of one cut row, fetched by the person who opened it. Its own
  // channel and not part of `observeJob`: the socket is bounded on purpose, and
  // an argument big enough to need this is the payload that would evict the
  // rows somebody is reading. It answers rather than publishing, so nothing on
  // the board re-renders because one reader opened a row.
  ipcMain.handle(CHANNELS.readCall, (_event, jobId: string, callId: string) =>
    connection?.readCall(jobId, callId),
  );
  ipcMain.handle(CHANNELS.readFrame, (_event, jobId: string, kept: string) =>
    connection?.readFrame(jobId, kept),
  );
  ipcMain.handle(CHANNELS.readCheckOutput, (_event, jobId: string, kept: string) =>
    connection?.readCheckOutput(jobId, kept),
  );
  // New job's own reads for the repository its ask answered, on All — #959.
  ipcMain.handle(CHANNELS.readComposing, (event, repository: string) =>
    connection?.readComposing(repository, connection.repositories.pickedByWindow.of(windowIdOf(event))),
  );
  // Every report a person has filed, and the counts they are read beside. The
  // one read here that names no Job: a report outlives the Job it is about, so
  // a listing reached through one would lose the reports that most need
  // reading. Read-only, and nothing on this channel can file or withdraw one.
  ipcMain.handle(CHANNELS.readReports, (_event, want: boolean) =>
    connection?.readReports(want),
  );
  // What fleet is holding disk for, while somebody is deciding about it. The
  // second read here that names no Job, and read-only: the act beside it is
  // `reclaimWorktree`, which already has its own channel and takes one id.
  ipcMain.handle(CHANNELS.readHeld, (_event, want: boolean) =>
    connection?.readHeld(want),
  );
  // A repository's Studios and the one open — #1287. An id that is not a string, or a position that
  // is not two whole numbers, is not put on a route: the call answers nothing, as a typo would.
  const text = (value: unknown): value is string => typeof value === "string" && value !== "";
  const unsent = { ok: false, why: "not_connected" } as const;
  // #1291: one channel across six operations, so the tag is what is checked —
  // it picks the route. The body is Fleet's to decode and refuse, as every
  // other act's body already is.
  const promoted = (value: unknown): value is StudioPromotion =>
    typeof value === "object" &&
    value !== null &&
    (STUDIO_PROMOTIONS as readonly string[]).includes((value as { act?: unknown }).act as string);
  /** A position in whole canvas units, or `null` where it is not one. */
  const whole = (value: unknown): StudioPosition | null => {
    const at = (value ?? {}) as { x?: unknown; y?: unknown };
    return Number.isInteger(at.x) && Number.isInteger(at.y)
      ? { x: at.x as number, y: at.y as number }
      : null;
  };
  /** One of the three kinds a person adds by hand, with its own field filled. */
  const byHand = (value: unknown): value is StudioNodeByHand => {
    const node = (value ?? {}) as { kind?: unknown; said?: unknown; address?: unknown; body?: unknown };
    if (node.kind === "note") return text(node.said);
    if (node.kind === "link") return text(node.address);
    return node.kind === "sketch" && text(node.body);
  };
  ipcMain.handle(CHANNELS.watchStudios, (_event, manifestId: unknown) =>
    text(manifestId) || manifestId === null ? connection?.studios.watchList(manifestId) : undefined,
  );
  ipcMain.handle(CHANNELS.watchStudio, (_event, studioId: unknown) =>
    text(studioId) || studioId === null ? connection?.studios.watchStudio(studioId) : undefined,
  );
  ipcMain.handle(CHANNELS.createStudio, async (_event, manifestId: unknown) =>
    text(manifestId)
      ? ((await connection?.studios.create(manifestId)) ?? { ok: false, outcome: unsent })
      : undefined,
  );
  ipcMain.handle(CHANNELS.renameStudio, async (_event, studioId: unknown, name: unknown) =>
    text(studioId) && text(name) ? ((await connection?.studios.rename(studioId, name)) ?? unsent) : undefined,
  );
  // A node by hand — #1364. **The kind is checked here, not only typed**: the
  // preload is the boundary, and a renderer that sent `finding` would otherwise
  // reach a route Fleet refuses rather than one Bridge never offered.
  ipcMain.handle(CHANNELS.addStudioNode, async (_event, studioId: unknown, node: unknown, position: unknown) => {
    const at = whole(position);
    if (!text(studioId) || at === null || !byHand(node)) return undefined;
    return (await connection?.studios.addNode(studioId, node, at)) ?? unsent;
  });
  ipcMain.handle(CHANNELS.moveStudioNode, async (_event, studioId: unknown, nodeId: unknown, position: unknown) => {
    const at = whole(position);
    if (!text(studioId) || !text(nodeId) || at === null) return undefined;
    return (await connection?.studios.moveNode(studioId, nodeId, at)) ?? unsent;
  });
  ipcMain.handle(CHANNELS.removeStudioNode, async (_event, studioId: unknown, nodeId: unknown) =>
    text(studioId) && text(nodeId) ? ((await connection?.studios.removeNode(studioId, nodeId)) ?? unsent) : undefined,
  );
  // Studio capture — #1290. **Main takes the frame, of the sender's own window
  // and no other**, so the one capability the preload gains is a Note on a
  // Studio rather than a screenshot the renderer could ask for and keep.
  ipcMain.handle(CHANNELS.captureStudioNote, async (event, studioId: unknown, said: unknown, capture: unknown) => {
    if (!text(studioId) || !text(said) || !isCapture(capture)) return undefined;
    const frame = await stagedFrame(event as Electron.IpcMainInvokeEvent).catch(() => null);
    return (await connection?.studios.captureNote(studioId, said, capture, frame)) ?? unsent;
  });
  // The other half of the capture: the bytes of the picture one Note kept, read
  // by main and handed over for a `blob:`. **No new scheme and no CSP change** —
  // `img-src 'self' blob:` already draws one — and no path crosses either way.
  ipcMain.handle(CHANNELS.readStudioFrame, async (_event, studioId: unknown, nodeId: unknown) =>
    text(studioId) && text(nodeId)
      ? ((await connection?.studios.frameOf(studioId, nodeId)) ?? { ok: false, outcome: unsent })
      : undefined,
  );
  ipcMain.handle(CHANNELS.promoteOnStudio, async (_event, studioId: unknown, promotion: unknown) => {
    // The tag is checked here because it picks the route; the body Fleet
    // decodes and refuses on its own, as every other act's body is.
    if (!text(studioId) || !promoted(promotion)) return undefined;
    return (await connection?.studios.promote(studioId, promotion)) ?? unsent;
  });
  ipcMain.handle(CHANNELS.decideStudioEdge, async (_event, studioId: unknown, edgeId: unknown, accepted: unknown) =>
    text(studioId) && text(edgeId) && typeof accepted === "boolean"
      ? ((await connection?.studios.decideEdge(studioId, edgeId, accepted)) ?? unsent)
      : undefined,
  );
  // The four decisions on the work, and they stay four channels. Merging lands
  // the branch and then takes the work, approving takes it and leaves the pull
  // request open, requesting changes sends the drone back to the same step, and
  // rejecting is terminal and ends the drone — a single channel taking which
  // one as an argument would make that difference a flag.
  ipcMain.handle(CHANNELS.approveReview, (_event, jobId: string) =>
    connection?.commands.approveReview(jobId),
  );
  ipcMain.handle(CHANNELS.mergePullRequest, (_event, jobId: string) =>
    connection?.commands.mergePullRequest(jobId),
  );
  ipcMain.handle(CHANNELS.rerunFailedChecks, (_event, jobId: string) =>
    connection?.commands.rerunFailedChecks(jobId),
  );
  ipcMain.handle(CHANNELS.investigateFailedChecks, (_event, jobId: string) =>
    connection?.commands.investigateFailedChecks(jobId),
  );
  ipcMain.handle(CHANNELS.queueAfterFinding, (_event, jobId: string, finding: string) =>
    connection?.commands.queueAfterFinding(jobId, finding),
  );
  ipcMain.handle(
    CHANNELS.fileFindingIssue,
    (_event, jobId: string, finding: string, title: string, body: string) =>
      connection?.commands.fileFindingIssue(jobId, finding, title, body),
  );
  ipcMain.handle(CHANNELS.requestChanges, (_event, jobId: string, note: string) =>
    connection?.commands.requestChanges(jobId, note),
  );
  ipcMain.handle(CHANNELS.rejectWork, (_event, jobId: string) =>
    connection?.commands.rejectWork(jobId),
  );
  // The fifth act at the same gate, and a channel of its own for the four
  // above's reason: it carries a set of handles off a forge rather than a note,
  // and what it does to the Job is what `requestChanges` does.
  ipcMain.handle(CHANNELS.takeUpRemarks, (_event, jobId: string, remarks: string[]) =>
    connection?.commands.takeUpRemarks(jobId, remarks),
  );
  // A person ruling on the review. It moves nothing. #907.
  ipcMain.handle(
    CHANNELS.dismissFinding,
    (_event, jobId: string, finding: string, reason: string) =>
      connection?.commands.dismissFinding(jobId, finding, reason),
  );
  // The one channel that reaches the OS, and the only one carrying no Fleet
  // request at all. **The path is built here** from the Job and the repository
  // its Manifest was read from; what crosses is a Job id and one of three
  // words, so no string the renderer composed reaches `shell.openPath`.
  ipcMain.handle(CHANNELS.openArtifact, (_event, jobId: string, what: Artifact) =>
    openArtifact(published, jobId, what),
  );
  // The second channel that reaches the OS, and the only one that leaves this
  // machine. **The address is read here** off what main published, and checked
  // to be a web address before it is handed over; what crosses is a Job id, so
  // no string the renderer composed reaches `shell.openExternal`. `forge.ts`.
  ipcMain.handle(CHANNELS.openPullRequest, (_event, jobId: string) =>
    openPullRequest(published, jobId),
  );
  // The third channel that leaves this machine, `openPullRequest`'s reason:
  // the comment's own address is read here off the remarks reading main
  // published, never off a string the renderer sent.
  ipcMain.handle(CHANNELS.openRemarkLink, (_event, jobId: string, remarkId: string) =>
    openRemarkLink(published, jobId, remarkId),
  );
  ipcMain.handle(CHANNELS.openFindingIssue, (_event, jobId: string, finding: string) =>
    openFindingIssue(published, jobId, finding),
  );

  createWindow();
  connection.start();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on("window-all-closed", () => {
  // **On macOS the connection stays up with the window closed, and this feature
  // rests on that.** Fleet is a daemon rather than a subprocess of the window,
  // so Jobs progress either way — but a Bridge that stopped reading would have
  // nothing left to notice a Job starting to wait, and telling somebody who is
  // away from the app is the whole point. The app is still running; what it
  // costs is one socket.
  //
  // Everywhere else the app quits, which stops the connection on its way out.
  if (process.platform === "darwin") return;
  connection?.stop();
  attention.close();
  app.quit();
});
