// Overview's readings no other surface holds: Fleet's health, and drift for every repository in the
// scope — each one served on All, or the one picked.
//
// **Held while a surface wants them, `holding.ts`'s shape.** Health is a pull by Doctor's own terms,
// and drift on All is one read per repository, so nothing pays for either while Overview is closed.
// Read again when what they answer about may have moved: a listing or a pick (`repositories.ts`),
// a `manifest.reread` (`arrivals.ts`), and Refresh — which lists again, so it arrives as the first.
//
// **The last good reading stays up through a re-read.** Only opening draws the placeholders; a
// tile blanking on every save would read as a Fleet that stopped answering.

import type { ManifestDrift, ManifestDriftRead } from "@armada/protocol";
import type { DriftsRead, FleetHealth, HealthRead, RepositoryDrift } from "@armada/screens/src/overview-reads";
import type { BridgeState } from "../shared/bridge";
import type { Picked } from "./picked";
import { ask, NOT_SET_UP } from "./request";
import type { Answer } from "./request";

export type OverviewWiring = {
  publish: (change: Partial<BridgeState>) => void;
  picked: Picked;
  port: () => number | null;
};

const NOT_CONNECTED = { ok: false, why: "not_connected" } as const;

export class OverviewReads {
  private readonly wiring: OverviewWiring;
  private open = false;
  /** Only the newest read publishes, so a pick that moved mid-read never draws the old scope. */
  private asked = 0;

  constructor(wiring: OverviewWiring) {
    this.wiring = wiring;
  }

  /** Read them, or drop what was read. */
  async watch(want: boolean): Promise<void> {
    this.open = want;
    if (!want) {
      this.wiring.publish({ health: { state: "none" }, drifts: { state: "none" } });
      return;
    }
    const reading = { state: "reading" } as const;
    const repositories = this.scope().map(({ root }) => ({ root, drift: reading }));
    this.wiring.publish({ health: reading, drifts: { state: "held", repositories } });
    await this.again(this.wiring.port());
  }

  /** Read again, where a surface has them open. Nothing open is no read. */
  async again(port: number | null): Promise<void> {
    if (!this.open) return;
    this.asked += 1;
    const asked = this.asked;
    const scope = this.scope();
    if (port === null) {
      const failed = { state: "failed", outcome: NOT_CONNECTED } as const;
      const repositories = scope.map(({ root }) => ({ root, drift: failed }));
      this.wiring.publish({ health: failed, drifts: { state: "held", repositories } });
      return;
    }
    const [health, repositories] = await Promise.all([
      ask(port, "GET", "/health"),
      Promise.all(
        scope.map(async ({ root, path }): Promise<RepositoryDrift> => ({
          root,
          drift: path === null ? { state: "failed", outcome: NOT_SET_UP } : driftOf(await ask(port, "GET", path)),
        })),
      ),
    ]);
    // The surface closed, or a newer read went out, while this was in flight.
    if (!this.open || this.asked !== asked) return;
    const drifts: DriftsRead = { state: "held", repositories };
    this.wiring.publish({ health: healthOf(health), drifts });
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.open = false;
  }

  /** Every repository in the scope, with its drift route or `null` where it has no Manifest. */
  private scope(): { root: string; path: string | null }[] {
    return this.wiring.picked.each("/manifest/drift").map(({ repository, path }) => ({ root: repository.root, path }));
  }
}

function healthOf(answer: Answer): HealthRead {
  return answer.ok === true
    ? { state: "read", health: answer.body as FleetHealth }
    : { state: "failed", outcome: answer.outcome };
}

function driftOf(answer: Answer): ManifestDriftRead {
  return answer.ok === true
    ? { state: "read", drift: answer.body as ManifestDrift }
    : { state: "failed", outcome: answer.outcome };
}
