import { app, BrowserWindow, ipcMain } from "electron";
import { join } from "node:path";

import tokens from "@armada/tokens/tokens.json";
import { CHANNELS } from "../shared/bridge";
import type { BridgeState, Draft } from "../shared/bridge";
import { FleetConnection } from "./connection";

// Bridge's window, and the one connection under it.
//
// Bridge and Fleet have independent lifetimes: Jobs keep progressing with the
// window closed, and opening it reconnects to whatever Fleet is already running
// rather than starting one. Nothing here spawns Fleet.

/** The hard window floor, from the token that exists to be read here. */
function floor(name: string): number {
  const found = tokens.tokens.find((token) => token.name === name);
  return found === undefined ? 0 : Number.parseInt(found.value, 10);
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
  void window.loadFile(join(__dirname, "../renderer/index.html"));
  return window;
}

/** Every window sees the same state, because there is one connection behind it. */
function publish(state: BridgeState): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send(CHANNELS.changed, state);
  }
}

void app.whenReady().then(() => {
  connection = new FleetConnection({
    home: process.env["HOME"],
    publish,
    now: () => Date.now(),
  });

  // The renderer initiates exactly these three things. There is no
  // arbitrary-channel invoke, which is what keeps the surface readable.
  ipcMain.handle(CHANNELS.state, () => connection?.state());
  ipcMain.handle(CHANNELS.proposeJob, (_event, draft: Draft) => connection?.proposeJob(draft));
  ipcMain.handle(CHANNELS.approveDispatch, (_event, jobId: string) =>
    connection?.approveDispatch(jobId),
  );

  createWindow();
  connection.start();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on("window-all-closed", () => {
  // Fleet is a daemon and not a subprocess of this window. Closing Bridge drops
  // the connection and nothing else.
  connection?.stop();
  if (process.platform !== "darwin") app.quit();
});
