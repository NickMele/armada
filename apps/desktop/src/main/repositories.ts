// What Fleet serves, shared, and which of it each window's own rail picked, per window.
// Beside `connection.ts`, which is at the length the gate refuses.

import type { BridgeState, PickedView } from "../shared/bridge";
import type { Holdings, ManifestReading, RepositoryList } from "@armada/protocol";
import { PickedByWindow } from "./picked";
import type { RehearsalConnection } from "./rehearsal";
import { Locating } from "./locating";
import { holdingsOf, manifestReadingOf, repositoriesOf } from "./request";

export type RepositoryWiring = {
  pickedByWindow: PickedByWindow;
  publish: (change: Partial<BridgeState>) => void;
  /** One window's own `PickedView` fields, overlaid onto what it alone receives. */
  publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
  /** Every window open right now, so a listing that moved reconciles every one of their picks. */
  windowIds: () => readonly number[];
  holds: () => Holdings;
  rehearsal: RehearsalConnection;
  /** Every open window's own Overview read again — `connection.ts`'s `windowFacades`. */
  overviewAgain: (port: number) => Promise<void>;
  /** A window's own pick moved, including to All (`null`) — `main/helm.ts`'s own reconciling. */
  onPicked: (root: string | null) => void;
  port: () => number | null;
};

export class RepositoryReads {
  readonly pickedByWindow: PickedByWindow;
  /** Locate: a repository added or cloned, then listed and picked here. */
  readonly locating: Locating;
  private readonly wiring: RepositoryWiring;

  constructor(wiring: RepositoryWiring) {
    this.wiring = wiring;
    this.pickedByWindow = wiring.pickedByWindow;
    this.locating = new Locating({
      port: wiring.port,
      list: (port) => this.readHoldings(port),
      // On the state main publishes to every window; each renderer decides whether to announce it.
      landed: (repository) => wiring.publish({ located: { repository, at: Date.now() } }),
    });
  }

  /** What a proposal may name, and every repository served. `movedWindowId` is a window whose
   * own pick just moved, forcing its reads again even where nothing else shifted for it. */
  async readHoldings(port: number, movedWindowId: number | null = null): Promise<void> {
    await this.listed(await repositoriesOf(port), port, movedWindowId);
  }

  /**
   * A listing, read or carried whole by `repositories.changed`. **Shared**, because main
   * publishes one catalogue to every window; each window's own pick is reconciled against it
   * apart from every other window's.
   */
  async listed(listed: RepositoryList | null, port: number, movedWindowId: number | null = null): Promise<void> {
    const windowIds = this.wiring.windowIds();
    // Every open window's own pick, reconciled against the new listing apart from every other's —
    // **synchronously, before anything below awaits**, so a pick pressed the instant this listing
    // is published never races a window's own `Picked` still holding the old one.
    const shifted = new Map(
      windowIds.map((windowId) => [
        windowId,
        listed !== null && this.pickedByWindow.of(windowId).hold(listed.repositories),
      ]),
    );
    for (const windowId of shifted.keys()) {
      const root = this.pickedByWindow.of(windowId).picked;
      this.wiring.publishToWindow(windowId, { repository: root });
      this.wiring.onPicked(root);
    }
    const held = this.wiring.holds();
    const kept = listed === null ? held : { ...held, repositories: listed.repositories };
    // `leftOut` is scoped to a pick; every other field here is not, so any one window's answer
    // carries the shared ones and every window's own call carries its own `leftOut`.
    const holdingsPerWindow = await Promise.all(
      windowIds.map(async (windowId) => ({
        windowId,
        holdings: await holdingsOf(port, kept, this.pickedByWindow.of(windowId)),
      })),
    );
    this.wiring.publish({ holds: holdingsPerWindow[0]?.holdings ?? { ...kept, leftOut: [] } });
    for (const { windowId, holdings } of holdingsPerWindow) {
      this.wiring.publishToWindow(windowId, { leftOut: holdings.leftOut });
    }
    await Promise.all([
      this.wiring.overviewAgain(port),
      ...windowIds.map(async (windowId) => {
        if (windowId !== movedWindowId && !shifted.get(windowId)) return;
        await Promise.all([this.readManifestOf(windowId, port), this.wiring.rehearsal.onRepositoryMoved(windowId, port)]);
      }),
    ]);
  }

  /** One window's own pick, `null` for All. A root Fleet does not list moves nothing. */
  async pick(windowId: number, root: string | null): Promise<void> {
    const picked = this.pickedByWindow.of(windowId);
    if (!picked.pick(root)) return;
    this.wiring.publishToWindow(windowId, { repository: picked.picked });
    this.wiring.onPicked(picked.picked);
    const port = this.wiring.port();
    if (port !== null) await this.readHoldings(port, windowId);
  }

  /** One window's own reading of its picked Manifest. */
  async readManifestOf(windowId: number, port: number): Promise<void> {
    const picked = this.pickedByWindow.of(windowId);
    this.wiring.publishToWindow(windowId, { manifestReading: await manifestReadingOf(port, picked) });
  }

  /**
   * Every open window's own reading, refreshed together. **Once per connection**, on `resync` —
   * a window picked something in an earlier connection and Fleet may answer differently now.
   */
  async readManifestForEveryWindow(port: number): Promise<void> {
    await Promise.all(this.wiring.windowIds().map((windowId) => this.readManifestOf(windowId, port)));
  }

  /**
   * `manifest.reread`, told to every window whose own pick reads it — `Picked.reads`. The whole
   * reading rides on the event, so this costs no second read.
   */
  manifestReread(event: ManifestReading): void {
    for (const [windowId, picked] of this.pickedByWindow.all()) {
      if (picked.reads(event.path)) this.wiring.publishToWindow(windowId, { manifestReading: event });
    }
  }
}
