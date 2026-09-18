// Every capture window open, one per Run — #1294.
//
// **One per Run, and opening again raises the one that is open.** Two windows
// on one origin would be two things a person has to tell apart by their title.

import tokens from "@armada/tokens/tokens.json";

import type { BridgeState } from "../../shared/bridge";
import type { CaptureOpened } from "@armada/protocol";
import { isPinned, pinned } from "./address";
import { CaptureWindow, type CaptureBoard } from "./window";

/**
 * How tall the bar is drawn. **The title row's own composition**, which
 * `TitleBar.css` spells as the same three tokens — read here rather than
 * written as a number, so the bar and Bridge's own title row are one height.
 */
function barHeight(): number {
  const of = (name: string): number => {
    const found = tokens.tokens.find((token) => token.name === name);
    return found === undefined ? 0 : Number.parseInt(found.value, 10);
  };
  return of("--space-8") + of("--space-2") + of("--space-1");
}

export class CaptureWindows {
  private readonly board: CaptureBoard;
  private readonly open = new Map<string, CaptureWindow>();

  constructor(board: CaptureBoard) {
    this.board = board;
  }

  /**
   * Open the capture window on one server Run, or raise the one already on it.
   *
   * **The address is resolved off the live holder Fleet published**, never off
   * the string the renderer sent — `servers.ts`'s rule, which loading a link in
   * a window is strictly more than.
   */
  openOn(
    state: BridgeState,
    serverId: string,
    url: string,
    studio: { id: string; name: string | null } | null,
  ): CaptureOpened {
    if (studio === null) return { ok: false, why: "no_studio" };
    const standing = this.open.get(serverId);
    if (standing !== undefined && standing.open) {
      standing.raise();
      return { ok: true };
    }
    const pin = pinned(state, serverId, url);
    if (!isPinned(pin)) return pin;
    this.open.set(serverId, new CaptureWindow(pin, studio, this.board, barHeight()));
    return { ok: true };
  }

  /**
   * Fold what main published: a Run that has stopped serving ends capture in
   * its window, and the bar says the run ended.
   */
  changed(state: BridgeState): void {
    for (const [serverId, window] of this.open) {
      if (!window.open) {
        this.open.delete(serverId);
        continue;
      }
      const server = state.servers.servers.find((one) => one.id === serverId);
      if (server === undefined || server.phase === "exited") window.ended();
    }
  }

  /** The window an IPC call came from, or `null` where it came from anything else. */
  from(contents: Electron.WebContents): CaptureWindow | null {
    for (const window of this.open.values()) {
      if (window.open && window.owns(contents)) return window;
    }
    return null;
  }

  closeAll(): void {
    for (const window of this.open.values()) window.close();
    this.open.clear();
  }
}
