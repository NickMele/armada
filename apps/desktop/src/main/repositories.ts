// What Fleet serves, and which of it the rail picked: the holdings, the pick, and the
// Manifest reading scoped to it. Beside `connection.ts`, which is at the length the gate refuses.

import type { BridgeState } from "../shared/bridge";
import type { Holdings, RepositoryList } from "@armada/protocol";
import type { OverviewReads } from "./overview";
import type { Picked } from "./picked";
import type { RehearsalConnection } from "./rehearsal";
import { Locating } from "./locating";
import { holdingsOf, manifestReadingOf, repositoriesOf } from "./request";

export type RepositoryWiring = {
  picked: Picked;
  publish: (change: Partial<BridgeState>) => void;
  holds: () => Holdings;
  rehearsal: RehearsalConnection;
  /** Overview's reads, whose scope a listing or a pick moves. */
  overview: OverviewReads;
  port: () => number | null;
};

export class RepositoryReads {
  readonly picked: Picked;
  /** Locate: a repository added or cloned, then listed and picked here. */
  readonly locating: Locating;
  private readonly wiring: RepositoryWiring;

  constructor(wiring: RepositoryWiring) {
    this.wiring = wiring;
    this.picked = wiring.picked;
    this.locating = new Locating({ port: wiring.port, list: (port) => this.readHoldings(port) });
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
   * A listing, read or carried whole by `repositories.changed`. Every window follows it, because
   * main publishes one state to all of them; the pick stays where it is still served.
   */
  async listed(listed: RepositoryList | null, port: number, moved = false): Promise<void> {
    const shifted = listed !== null && this.picked.hold(listed.repositories);
    const held = this.wiring.holds();
    const kept = listed === null ? held : { ...held, repositories: listed.repositories };
    this.wiring.publish({ holds: await holdingsOf(port, kept, this.picked), repository: this.picked.picked });
    if (moved || shifted) await Promise.all([this.readManifest(port), this.wiring.rehearsal.onRepositoryMoved(port)]);
    // On All a repository added moves the scope without moving the pick, so Overview reads on every listing.
    await this.wiring.overview.again(port);
  }

  /** The rail's pick, `null` for All repositories. A root Fleet does not list moves nothing. */
  async pick(root: string | null): Promise<void> {
    if (!this.picked.pick(root)) return;
    this.wiring.publish({ repository: this.picked.picked });
    const port = this.wiring.port();
    if (port !== null) await this.readHoldings(port, true);
  }

  /**
   * What Fleet's last read of the picked Manifest came to. **A failed read publishes `null`**:
   * there is no reading to report, and keeping the previous one would be a refusal drawn
   * against a Fleet that was never asked.
   */
  async readManifest(port: number): Promise<void> {
    this.wiring.publish({ manifestReading: await manifestReadingOf(port, this.picked) });
  }
}
