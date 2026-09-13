// Overview's band: five readings Fleet already serves, each opening the surface it belongs to.
// #919. Not routed yet — #921 puts Overview in the rail — so a story draws it.

import { OverviewTile, OverviewTileBand } from "@armada/components";
import type { Connection, FleetCapacity, JobSummary, RepositorySummary } from "@armada/protocol";
import { doctorReading, driftReading, dronesReading, fleetReading, queuedIn, queuedReading } from "./overview";
import type { DriftsRead, HealthRead } from "./overview-reads";

export type OverviewTilesProps = {
  connection: Connection;
  now: number;
  readAt: number | null;
  health: HealthRead;
  capacity: FleetCapacity | null;
  jobs: readonly JobSummary[];
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The picked root, `BridgeState.repository`'s terms: `null` is All repositories. */
  picked: string | null;
  drifts: DriftsRead;
  onOpenFleetSettings: () => void;
  onOpenQueued: () => void;
  onOpenManifest: () => void;
};

/** Fleet and Doctor open nothing until Doctor ships. */
export function OverviewTiles(props: OverviewTilesProps) {
  const picked = props.repositories.find((one) => one.root === props.picked) ?? null;
  const queued = queuedIn(props.jobs, picked);
  return (
    <OverviewTileBand>
      <OverviewTile {...fleetReading(props.connection, props.now, props.readAt)} />
      <OverviewTile {...doctorReading(props.health)} />
      <OverviewTile
        {...dronesReading(props.connection, props.capacity, queued.length)}
        opens="Fleet settings"
        onOpen={props.onOpenFleetSettings}
      />
      <OverviewTile
        {...queuedReading(queued, props.repositories, picked)}
        opens="Queued on the Board"
        onOpen={props.onOpenQueued}
      />
      <OverviewTile {...driftReading(props.drifts, picked)} opens="Manifest" onOpen={props.onOpenManifest} />
    </OverviewTileBand>
  );
}
