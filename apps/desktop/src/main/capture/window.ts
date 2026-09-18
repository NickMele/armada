// The capture window — #1294, `docs/practices/capture-window.md`.
//
// **Two views in one frame.** The page runs in a view with no preload, in a
// persistent session of its own; Bridge's bar runs above it in a view with
// Bridge's preload and Bridge's CSP. While capture is armed the bar's view
// grows over the whole frame, so the outline is drawn where the page cannot
// read it, restyle it or forge it, and a press never reaches the app.
//
// Bridge's own window is untouched: it still loads one file and refuses every
// navigation — `bridge.md`, *Security posture*.

import { BaseWindow, session, shell, WebContentsView, type Session } from "electron";
import { join } from "node:path";

import type { Outcome, StagedFrame, StudioCapture } from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureHeld,
  CaptureRefusedWhat,
  CaptureWheel,
  CaptureWindowState,
} from "../../shared/capture-window";
import { CHANNELS } from "../../shared/bridge";
import { isPinned, offerable, onOrigin, partitionFor, type Pinned } from "./address";
import { aimed, bounded, chainOf, rectOf } from "./bounds";
import { askSource } from "./layer";

/** What the window needs from main: a Note lands, and a frame is staged. */
export type CaptureBoard = {
  capture: (studioId: string, said: string, capture: StudioCapture, frame: StagedFrame | null) => Promise<Outcome>;
  stage: (png: Buffer, width: number, height: number) => Promise<StagedFrame | null>;
};

/** `⌥⌘C`, as the action registry spells it. The renderer's own reader is `capture/Layer.tsx`. */
function isCaptureBinding(input: Electron.Input): boolean {
  return input.type === "keyDown" && input.code === "KeyC" && input.meta && input.alt && !input.control && !input.shift;
}

/**
 * Every partition already given its handlers. **Kept because a partition is
 * per repository and two windows share one**, so the download refusal would
 * otherwise be added twice and the second window's close would take both.
 */
const held = new Set<string>();

/**
 * The session this window's page runs in: persistent, one per repository, and
 * nothing of Bridge's in it.
 *
 * **Every permission is denied without asking.** A page that wants the camera,
 * the clipboard, a notification or the screen gets no dialog and no capability
 * — this window exists to look at a page, not to host one.
 */
function sessionFor(manifestId: string | null): Session {
  const name = partitionFor(manifestId);
  const part = session.fromPartition(name);
  if (held.has(name)) return part;
  held.add(name);
  part.setPermissionRequestHandler((_contents, _permission, decide) => decide(false));
  part.setPermissionCheckHandler(() => false);
  part.setDisplayMediaRequestHandler(() => {}, { useSystemPicker: false });
  part.on("will-download", (event) => event.preventDefault());
  return part;
}

export class CaptureWindow {
  private readonly window: BaseWindow;
  private readonly page: WebContentsView;
  private readonly bar: WebContentsView;
  private readonly board: CaptureBoard;
  private readonly pin: Pinned;
  private readonly studio: { id: string; name: string | null };
  /** How tall the bar is drawn, in CSS pixels, from the tokens the title row is built of. */
  private readonly barHeight: number;

  private armed = false;
  private serving = true;
  private framesRefused = 0;
  private refusedAddress: CaptureWindowState["refused"];
  /** The capture a press is holding, which never leaves this process until it lands. */
  private holding: StudioCapture | null = null;

  constructor(pin: Pinned, studio: { id: string; name: string | null }, board: CaptureBoard, barHeight: number) {
    this.pin = pin;
    this.studio = studio;
    this.board = board;
    this.barHeight = barHeight;

    this.window = new BaseWindow({
      width: 1280,
      height: 900,
      // The OS frame and its title, where Bridge's own window is frameless and
      // draws its own: two windows that are not the same kind of thing do not
      // have the same chrome.
      title: `${pin.name} — ${pin.origin}`,
      show: false,
    });

    this.page = new WebContentsView({
      webPreferences: {
        session: sessionFor(pin.manifestId),
        // **No preload.** The only surface that cannot be widened by accident
        // is an absent one — `capture-window.md`, *What the page can reach into*.
        sandbox: true,
        contextIsolation: true,
        nodeIntegration: false,
        nodeIntegrationInSubFrames: false,
        webviewTag: false,
        spellcheck: false,
        // A dialog the page draws would be a dialog that looks like Armada's.
        disableDialogs: true,
        // Neither loosened. A page's own CSP is what governs it, and Bridge
        // rewrites no header to make an awkward app render.
        webSecurity: true,
        allowRunningInsecureContent: false,
      },
    });

    this.bar = new WebContentsView({
      webPreferences: {
        preload: join(__dirname, "../preload/index.js"),
        sandbox: true,
        contextIsolation: true,
        nodeIntegration: false,
      },
    });
    this.bar.setBackgroundColor("#00000000");

    this.window.contentView.addChildView(this.page);
    this.window.contentView.addChildView(this.bar);
    this.window.on("resize", () => this.layout());
    this.layout();

    this.holdThePage();
    void this.bar.webContents.loadFile(join(__dirname, "../renderer/capture.html"));
    void this.page.webContents.loadURL(pin.url);
    this.window.show();
  }

  /** Whether this window is still on screen. */
  get open(): boolean {
    return !this.window.isDestroyed();
  }

  get runId(): string {
    return this.pin.run;
  }

  raise(): void {
    if (this.window.isDestroyed()) return;
    if (this.window.isMinimized()) this.window.restore();
    this.window.focus();
  }

  close(): void {
    if (!this.window.isDestroyed()) this.window.close();
  }

  /** Whether an IPC call came from this window's own bar and no other document. */
  owns(contents: Electron.WebContents): boolean {
    return !this.bar.webContents.isDestroyed() && contents.id === this.bar.webContents.id;
  }

  /**
   * The Run stopped serving. **Capture ends here and the window loads nothing
   * further** — a loopback port is not an identity, and anything on the machine
   * may bind it once the server exits.
   */
  ended(): void {
    if (!this.serving) return;
    this.serving = false;
    this.armed = false;
    this.holding = null;
    this.tell();
  }

  state(): CaptureWindowState {
    return {
      served: { run: this.pin.run, name: this.pin.name, address: this.pin.origin },
      studio: this.studio,
      serving: this.serving,
      armed: this.armed,
      framesRefused: this.framesRefused,
      ...(this.refusedAddress === undefined ? {} : { refused: this.refusedAddress }),
    };
  }

  arm(on: boolean): CaptureWindowState {
    this.armed = on && this.serving;
    if (!this.armed) this.holding = null;
    this.tell();
    return this.state();
  }

  /** What is under the pointer, asked of the page and bounded here. */
  async aim(x: number, y: number): Promise<CaptureAimed | null> {
    if (!this.armed) return null;
    return aimed(await this.ask(x, y, false), this.viewport());
  }

  /**
   * Take the capture under the pointer and hold it. **The bar gets the box and
   * one line**: the markup, the styles and the selector stop here.
   */
  async hold(x: number, y: number): Promise<CaptureHeld | null> {
    if (!this.armed || !this.serving) return null;
    const answer = await this.ask(x, y, true);
    const capture = bounded(answer, this.viewport(), {
      run: this.pin.run,
      name: this.pin.name,
      address: this.pin.origin,
    });
    if (capture === null) return null;
    this.holding = capture;
    return { box: rectOf(capture.bounds, this.viewport()), chain: chainOf(capture) };
  }

  release(): void {
    this.holding = null;
  }

  /**
   * Put the held Note on the Studio this window opened on. **The frame is
   * main's**, taken of the page's own contents cropped to the element's box.
   */
  async save(said: string): Promise<Outcome> {
    const capture = this.holding;
    if (capture === null) return { ok: false, why: "nothing_held" };
    // The run ended while the note was being written: the capture would name a
    // port that is no longer the Run's.
    if (!this.serving) return { ok: false, why: "run_ended" };
    const outcome = await this.board.capture(this.studio.id, said, capture, await this.frame(capture));
    if (outcome.ok) {
      this.holding = null;
      this.armed = false;
      this.tell();
    }
    return outcome;
  }

  /** Reload the pinned origin. There is no Back, no Forward and no history. */
  reload(): void {
    if (!this.serving || this.page.webContents.isDestroyed()) return;
    void this.page.webContents.loadURL(this.pin.url);
  }

  /**
   * Hand the last refused address to whatever browses the web here. **`http:`
   * and `https:` only**, and never followed in this window: an app a person is
   * building redirects to an auth provider, and the honest answer is the
   * browser they already use.
   */
  async followRefused(): Promise<void> {
    const refused = this.refusedAddress;
    if (refused === undefined || !refused.offerable) return;
    await shell.openExternal(refused.address);
  }

  /** A wheel the bar took while armed, so the page still scrolls under the outline. */
  scroll(wheel: CaptureWheel): void {
    if (!this.armed || this.page.webContents.isDestroyed()) return;
    this.page.webContents.sendInputEvent({
      type: "mouseWheel",
      x: Math.round(wheel.x),
      y: Math.round(wheel.y),
      deltaX: Math.round(wheel.deltaX),
      deltaY: Math.round(wheel.deltaY),
      canScroll: true,
    } as Electron.MouseWheelInputEvent);
  }

  /** The page's viewport, which every box is clamped to. */
  private viewport(): { width: number; height: number } {
    const bounds = this.page.getBounds();
    return { width: Math.max(1, bounds.width), height: Math.max(1, bounds.height) };
  }

  /** One ask, answered on the promise main created. A page that throws answers nothing. */
  private async ask(x: number, y: number, take: boolean): Promise<unknown> {
    if (this.page.webContents.isDestroyed()) return null;
    try {
      return await this.page.webContents.executeJavaScript(askSource(x, y, take), false);
    } catch {
      return null;
    }
  }

  /** A PNG of the page cropped to what the Note points at, staged for Fleet to keep. */
  private async frame(capture: StudioCapture): Promise<StagedFrame | null> {
    try {
      const box = rectOf(capture.bounds, this.viewport());
      if (box.width === 0 || box.height === 0) return null;
      const image = await this.page.webContents.capturePage(box);
      const png = image.toPNG();
      if (png.byteLength === 0) return null;
      const size = image.getSize();
      return await this.board.stage(png, size.width, size.height);
    } catch {
      return null;
    }
  }

  /**
   * The bar over the whole frame while armed, and a strip at the top while not.
   *
   * **Two rows when there is something to say** — a refusal, a count of frames,
   * or the run having ended — which is the same condition the bar's own
   * `hasASecondRow` draws on, so the page starts exactly where the bar ends.
   */
  private layout(): void {
    if (this.window.isDestroyed()) return;
    const { width, height } = this.window.getContentBounds();
    const said = !this.serving || this.refusedAddress !== undefined || this.framesRefused > 0;
    const strip = this.barHeight * (said ? 2 : 1);
    this.page.setBounds({ x: 0, y: strip, width, height: Math.max(0, height - strip) });
    this.bar.setBounds({ x: 0, y: 0, width, height: this.armed ? height : strip });
  }

  private tell(): void {
    this.layout();
    if (this.bar.webContents.isDestroyed()) return;
    this.bar.webContents.send(CHANNELS.captureWindowChanged, this.state());
  }

  private refuse(address: string, what: CaptureRefusedWhat): void {
    this.refusedAddress = { address, what, offerable: offerable(address) };
    this.tell();
  }

  /**
   * Every way the page could move, refused unless it stays on the origin this
   * window opened on — and refused outright once the Run has ended.
   *
   * **A subframe is counted rather than named.** A page draws many, and a bar
   * naming each would be noise where a count says the same thing.
   */
  private holdThePage(): void {
    const contents = this.page.webContents;
    const allowed = (address: string): boolean => this.serving && onOrigin(this.pin.origin, address);

    contents.setWindowOpenHandler(({ url }) => {
      this.refuse(url, "window");
      return { action: "deny" };
    });
    contents.on("will-navigate", (event, url) => {
      if (allowed(url)) return;
      event.preventDefault();
      this.refuse(url, "navigation");
    });
    contents.on("will-redirect", (event, url) => {
      if (allowed(url)) return;
      event.preventDefault();
      this.refuse(url, "redirect");
    });
    contents.on("will-frame-navigate", (event) => {
      if (event.isMainFrame || allowed(event.url)) return;
      event.preventDefault();
      this.framesRefused += 1;
      this.tell();
    });
    // A certificate error is not overridden: Electron's own default refuses,
    // and nothing here listens for one.
    contents.on("before-input-event", (event, input) => {
      if (!isCaptureBinding(input)) return;
      event.preventDefault();
      this.arm(!this.armed);
    });
    this.bar.webContents.on("before-input-event", (event, input) => {
      if (!isCaptureBinding(input)) return;
      event.preventDefault();
      this.arm(!this.armed);
    });
    // The bar goes nowhere either: it is a Bridge document and Bridge's own
    // window's rule holds over every one of them.
    this.bar.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
    this.bar.webContents.on("will-navigate", (event) => event.preventDefault());
    this.bar.webContents.on("did-finish-load", () => this.tell());
  }
}

export { isCaptureBinding, isPinned };
