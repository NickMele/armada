// Overview, as the window mounts it: the tile band over the lists, out of `App.tsx`, which is at
// the length the gate refuses. #921 — nothing mounted either half before this.
//
// `.armada-screen__overview` is the one child `.armada-screen__mounted` gets — the surface's own
// padding and gap, so neither tile nor panel sits against the window's edge. `Boundary` renders its
// children straight through when nothing has thrown.

import type { RepositorySummary } from "@armada/protocol";
import { OverviewLists, OverviewTiles } from "@armada/screens";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";

export function Overview({
  state,
  now,
  live,
  repositories,
  disconnected,
  selected,
  onOpen,
  onKill,
  onOpenSettings,
  onOpenQueued,
  onOpenManifest,
  onCopied,
  onCursor,
}: {
  state: BridgeState;
  now: number;
  live: boolean;
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The connection's own statement, where Fleet cannot be reached. */
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  onOpenSettings: () => void;
  onOpenQueued: () => void;
  onOpenManifest: () => void;
  onCopied: (value: string) => void;
  /** Where the cursor is, reported up — `OverviewLists`' own state, mirrored. #1075. */
  onCursor?: (jobId: string | null) => void;
}) {
  const guarded = { bridge: state.bridge, onCopied };
  return (
    <Boundary region="the overview" {...guarded}>
      <div className="armada-screen__overview">
        <OverviewTiles
          connection={state.connection}
          now={now}
          readAt={state.readAt}
          health={state.health}
          capacity={state.capacity}
          jobs={state.jobs}
          repositories={repositories}
          picked={state.repository}
          drifts={state.drifts}
          onOpenSettings={onOpenSettings}
          onOpenQueued={onOpenQueued}
          onOpenManifest={onOpenManifest}
        />
        <OverviewLists
          jobs={state.jobs}
          stale={!live}
          now={now}
          workflows={state.holds.workflows}
          repositories={repositories}
          picked={state.repository}
          disconnected={disconnected}
          selected={selected}
          onOpen={onOpen}
          onKill={onKill}
          onCopied={onCopied}
          onCursor={onCursor}
        />
      </div>
    </Boundary>
  );
}
