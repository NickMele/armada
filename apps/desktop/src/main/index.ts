import { app, BrowserWindow, ipcMain, nativeImage, Notification } from "electron";
import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import tokens from "@armada/tokens/tokens.json";
import { CHANNELS, NOTHING_YET } from "../shared/bridge";
import type { BridgeState, Summons } from "../shared/bridge";
import type { Draft } from "@armada/protocol";
import type { FileReport } from "@armada/protocol";
import type { Artifact } from "@armada/protocol";
import { FleetConnection } from "./connection";
import { openArtifact } from "./open";
import { Attention } from "./telling";

// Bridge's window, and the one connection under it.
//
// Bridge and Fleet have independent lifetimes: Jobs keep progressing with the
// window closed, and opening it reconnects to whatever Fleet is already running
// rather than starting one. Nothing here spawns Fleet.

/**
 * The Countersign mark, as the macOS app tile.
 *
 * Emitted beside this bundle by `electron.vite.config.ts` — see the note there
 * for why it is a file on disk rather than an import. **The accent-filled
 * variant is sanctioned for the app tile and nowhere else**, which is what this
 * `.icns` carries and why nothing in the renderer may reach for it.
 */
const APP_ICON = join(__dirname, "AppIcon.icns");

/**
 * Wear the mark. **Runtime, not packaging** — `BrowserWindow`'s `icon` option
 * is ignored on macOS, and there is no packager in this workspace yet, so an
 * `electron-builder` `icon` key would be configuration nothing reads. This is
 * what makes the dock tile right for the app as it is actually run today; a
 * packaged bundle will take the same file through its own `Info.plist`.
 *
 * A failed read leaves Electron's own icon rather than stopping the app: a
 * window that will not open because of a picture is the wrong trade.
 */
function wearTheMark(): void {
  if (process.platform !== "darwin" || app.dock === undefined) return;
  const icon = nativeImage.createFromPath(APP_ICON);
  if (!icon.isEmpty()) app.dock.setIcon(icon);
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

let connection: FleetConnection | null = null;

function createWindow(): BrowserWindow {
  const window = new BrowserWindow({
    width: 1280,
    height: 800,
    // Every layout is designed for resize rather than for the size it was built
    // at, and the floor is where that stops being the layout's problem.
    minWidth: floor("--window-floor"),
    show: false,
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  window.on("ready-to-show", () => window.show());
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

/** Every window sees the same state, because there is one connection behind it. */
function publish(state: BridgeState): void {
  published = state;
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send(CHANNELS.changed, state);
  }
  // **Here rather than on chosen events**, because what decides a notification
  // is the needs-you set changing and this is the one funnel every change to
  // the list passes through. `readAt` is what tells a publish that carries a
  // reading from one that carries only a connection state.
  attention.read(state.jobs, state.readAt);
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
    const banner = new Notification({ title: told.title, body: told.body });
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

  connection = new FleetConnection({
    home: process.env["HOME"],
    publish,
    now: () => Date.now(),
  });

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
    return state;
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
  ipcMain.handle(CHANNELS.proposeFromRequest, (_event, request: string) =>
    connection?.commands.proposeFromRequest(request),
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
  // Real deletion, and the only channel here where that is true. One call per
  // id, in main, because main is what holds the board these ids came off of —
  // the renderer decides which ids are terminal and sends the set once.
  ipcMain.handle(CHANNELS.clearTerminalJobs, (_event, jobIds: string[]) =>
    connection?.commands.clearTerminalJobs(jobIds),
  );
  // The disk rather than the record, and the one act here `armada clean` could
  // already do — but only with Fleet stopped, which is never when a person
  // wants the space back. The Job stays on the board afterwards.
  // The re-read afterwards is not bookkeeping: whether the row is still held is
  // Fleet's reading, and a checkout that would not go has to stay on the list.
  // Folding the receipt in instead would be Bridge deciding a worktree is gone.
  ipcMain.handle(CHANNELS.reclaimWorktree, async (_event, jobId: string) => {
    const outcome = await connection?.commands.reclaimWorktree(jobId);
    await connection?.rereadHeld();
    return outcome;
  });
  // The two acts that resume a step without redispatching. Which applies is
  // decided by whether the Job still holds a Drone; Fleet is the authority
  // and refuses the wrong one rather than Bridge picking silently.
  ipcMain.handle(CHANNELS.answerQuestion, (_event, jobId: string, questionId: string, chose: string) =>
    connection?.commands.answerQuestion(jobId, questionId, chose),
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
  // Saying a job failed in error. Its own channel beside the override rather
  // than a flag on it: the override moves the job past a verdict and this moves
  // nothing, and the two would otherwise be one press meaning either.
  ipcMain.handle(CHANNELS.fileReport, (_event, jobId: string, filing: FileReport) =>
    connection?.commands.fileReport(jobId, filing),
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
  // Which Job's transition history is unfolded. One HTTP read, kept current
  // while it is open, and dropped when the section closes — a history is its
  // own operation precisely so a Job opened does not pay for it.
  ipcMain.handle(CHANNELS.readHistory, (_event, jobId: string | null) =>
    connection?.readHistory(jobId),
  );
  // The two reads a review is made of. Two channels because they are two
  // operations: the claims are four lines a step and the diff is the patch,
  // and the patch is read only where somebody is looking at one.
  ipcMain.handle(CHANNELS.readEvidence, (_event, jobId: string | null) =>
    connection?.readEvidence(jobId),
  );
  ipcMain.handle(CHANNELS.readDiff, (_event, jobId: string | null) =>
    connection?.readDiff(jobId),
  );
  // The rest of one cut row, fetched by the person who opened it. Its own
  // channel and not part of `observeJob`: the socket is bounded on purpose, and
  // an argument big enough to need this is the payload that would evict the
  // rows somebody is reading. It answers rather than publishing, so nothing on
  // the board re-renders because one reader opened a row.
  ipcMain.handle(CHANNELS.readCall, (_event, jobId: string, callId: string) =>
    connection?.readCall(jobId, callId),
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
  // The three decisions on the work, and they stay three channels. Approving
  // takes it, requesting changes sends the drone back to the same step, and
  // rejecting is terminal and ends the drone — a single channel taking which
  // one as an argument would make that difference a flag.
  ipcMain.handle(CHANNELS.approveReview, (_event, jobId: string) =>
    connection?.commands.approveReview(jobId),
  );
  ipcMain.handle(CHANNELS.requestChanges, (_event, jobId: string, note: string) =>
    connection?.commands.requestChanges(jobId, note),
  );
  ipcMain.handle(CHANNELS.rejectWork, (_event, jobId: string) =>
    connection?.commands.rejectWork(jobId),
  );
  // The one channel that reaches the OS, and the only one carrying no Fleet
  // request at all. **The path is built here** from the Job and the repository
  // its Manifest was read from; what crosses is a Job id and one of three
  // words, so no string the renderer composed reaches `shell.openPath`.
  ipcMain.handle(CHANNELS.openArtifact, (_event, jobId: string, what: Artifact) =>
    openArtifact(published, jobId, what),
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
