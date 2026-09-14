// What Fleet serves, shared, and which of it each window's own rail picked, per window.
// Beside `connection.ts`, which is at the length the gate refuses.

import type { BridgeState, PickedView } from "../shared/bridge";
import type { Holdings, ManifestReading, RepositoryList } from "@armada/protocol";
import { PickedByWindow, type Picked } from "./picked";
import type { RehearsalConnection } from "./rehearsal";
import { Locating } from "./locating";
import { holdingsOf, manifestReadingOf, repositoriesOf } from "./request";

export type RepositoryWiring = {
  /** The one pick still shared across every window — Verify and `holdingsOf`'s `leftOut` read
   * this until `checkout-runs.ts` goes per window too. */
  picked: Picked;
  /** Every window's own pick, apart from the shared one above and from each other. */
  pickedByWindow: PickedByWindow;
  publish: (change: Partial<BridgeState>) => void;
  /** One window's own `repository`, `manifestReading`, `health` and `drifts`, overlaid onto what it alone receives. */
  publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
  /** Every window open right now, so a listing that moved reconciles every one of their picks. */
  windowIds: () => readonly number[];
  holds: () => Holdings;
  rehearsal: RehearsalConnection;
  /** Every open window's own Overview read again — `connection.ts`'s `windowFacades`. */
  overviewAgain: (port: number) => Promise<void>;
  port: () => number | null;
};

export class RepositoryReads {
  /** The still-shared pick — see `RepositoryWiring.picked`. */
  readonly picked: Picked;
  readonly pickedByWindow: PickedByWindow;
  /** Locate: a repository added or cloned, then listed and picked here. */
  readonly locating: Locating;
  private readonly wiring: RepositoryWiring;

  constructor(wiring: RepositoryWiring) {
    this.wiring = wiring;
    this.picked = wiring.picked;
    this.pickedByWindow = wiring.pickedByWindow;
    this.locating = new Locating({
      port: wiring.port,
      list: (port) => this.readHoldings(port),
      // On the state main publishes to every window; each renderer decides whether to announce it.
      landed: (repository) => wiring.publish({ located: { repository, at: Date.now() } }),
    });
  }

  /**
   * What a proposal may name, and every repository served. **Where the pick moved** — a first
   * listing, or a Manifest a root Write put down — what is scoped to it is read again, and
   * awaited so Setup's Verify redraws on this repository's sheet.
   */
  async readHoldings(port: number, moved = false): Promise<void> {
    await this.listed(await repositoriesOf(port), port, moved);
  }

  /**
   * A listing, read or carried whole by `repositories.changed`. **Shared**, because main
   * publishes one catalogue to every window; each window's own pick is reconciled against it
   * apart from every other window's.
   */
  async listed(listed: RepositoryList | null, port: number, moved = false): Promise<void> {
    const legacyShifted = listed !== null && this.picked.hold(listed.repositories);
    // Every open window's own pick, reconciled against the new listing apart from every other's —
    // **synchronously, before anything below awaits**, so a pick pressed the instant this listing
    // is published never races a window's own `Picked` still holding the old one.
    const shifted = new Map(
      this.wiring.windowIds().map((windowId) => [
        windowId,
        listed !== null && this.pickedByWindow.of(windowId).hold(listed.repositories),
      ]),
    );
    for (const windowId of shifted.keys()) {
      this.wiring.publishToWindow(windowId, { repository: this.pickedByWindow.of(windowId).picked });
    }
    const held = this.wiring.holds();
    const kept = listed === null ? held : { ...held, repositories: listed.repositories };
    this.wiring.publish({ holds: await holdingsOf(port, kept, this.picked) });
    if (moved || legacyShifted) {
      await Promise.all([this.wiring.rehearsal.onRepositoryMoved(port), this.wiring.overviewAgain(port)]);
    } else {
      await this.wiring.overviewAgain(port);
    }
    await Promise.all(
      [...shifted.entries()].map(([windowId, one]) => (moved || one ? this.readManifestOf(windowId, port) : null)),
    );
  }

  /** One window's own pick, `null` for All. A root Fleet does not list moves nothing. */
  async pick(windowId: number, root: string | null): Promise<void> {
    const picked = this.pickedByWindow.of(windowId);
    if (!picked.pick(root)) return;
    this.wiring.publishToWindow(windowId, { repository: picked.picked });
    // The still-shared pick moves with whichever window picked last — `RepositoryWiring.picked`.
    this.picked.pick(root);
    const port = this.wiring.port();
    if (port !== null) await this.readHoldings(port, true);
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
