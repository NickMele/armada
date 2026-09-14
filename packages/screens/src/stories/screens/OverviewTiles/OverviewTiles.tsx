import { connectedTo, PROTOCOL_VERSION } from "@armada/protocol";
import type { Connection, FleetCapacity, JobSummary, ManifestDriftRead, RepositorySummary } from "@armada/protocol";
import { OverviewTiles } from "../../../OverviewTiles";
import type { FleetHealth } from "@armada/protocol";
import type { DriftsRead, HealthRead } from "../../../overview-reads";
import { job, repository } from "../../../fixtures/build/base";

/** The moment every figure is read at, so a story never moves. */
export const NOW = Date.parse("2026-09-13T12:00:00Z");

export const FLEET = { protocolVersion: PROTOCOL_VERSION, pid: 4242, port: 7878, startedAt: "2026-09-13T09:00:00Z" };
export const CONNECTED: Connection = connectedTo(FLEET, 1);

export const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
export const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

/** Fleet's own four probes, as `GET /health` answers them, and the half it cannot run. */
export function health(over: Record<string, [string, string]> = {}): HealthRead {
  const probes: FleetHealth["probes"] = [
    { module: "Fleet", outcome: "pass", detail: "pid 4242 answering" },
    { module: "SQLite", outcome: "pass", detail: "armada.db opens and writes" },
    { module: "Manifest", outcome: "pass", detail: "2 of 2 parse" },
    { module: "System stats", outcome: "pass", detail: "memory and disk above the admission floor" },
  ].map((probe) => {
    const changed = over[probe.module];
    return changed === undefined ? probe : { ...probe, outcome: changed[0], detail: changed[1] };
  });
  const not_probed = [{ owner: "adapters", because: "Its probes are Doctor's, which is not built" }];
  return { state: "read", health: { probes, not_probed, helm_action_authority: "acting" } };
}

const line = (name: string, missing?: string) => ({
  section: "checks",
  name,
  key: "run",
  run: missing === undefined ? "cargo test" : missing,
  drift: missing === undefined ? { verdict: "current" as const, checked: 1 } : { verdict: "gone" as const, missing: [missing] },
  unfollowed: [],
});

/** A drift read with `gone` lines behind out of `lines`. */
export function drift(lines: number, gone: number): ManifestDriftRead {
  const declarations = Array.from({ length: lines }, (_, at) => line(`check-${at}`, at < gone ? `scripts/check-${at}.sh` : undefined));
  return { state: "read", drift: { path: "armada.yml", checkout: "/Users/user/armada", declarations } };
}

/** Three queued across both repositories, and one working. */
export const JOBS: JobSummary[] = [
  job("queued", { id: "01Q1", owner_manifest_id: "armada" }),
  job("queued", { id: "01Q2", owner_manifest_id: "armada" }),
  job("queued", { id: "01Q3", owner_manifest_id: "storefront" }),
  job("running", { id: "01R1", owner_manifest_id: "storefront" }),
];

const noop = () => {};

/**
 * Overview's band, drawn by the app's own `OverviewTiles` from readings in the shapes Fleet sends
 * and Bridge holds. Only the data is made up.
 */
export function OverviewTilesFrom({
  connection = CONNECTED,
  readAt = NOW,
  healthRead = health(),
  capacity = { bound: 2, occupied: 1 },
  jobs = JOBS,
  repositories = [ARMADA, STOREFRONT],
  picked = null,
  drifts = { state: "held", repositories: [{ root: ARMADA.root, drift: drift(8, 0) }, { root: STOREFRONT.root, drift: drift(6, 2) }] },
  onOpenSettings = noop,
  onOpenQueued = noop,
  onOpenManifest = noop,
}: {
  connection?: Connection;
  readAt?: number | null;
  healthRead?: HealthRead;
  capacity?: FleetCapacity | null;
  jobs?: readonly JobSummary[];
  repositories?: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked?: string | null;
  drifts?: DriftsRead;
  onOpenSettings?: () => void;
  onOpenQueued?: () => void;
  onOpenManifest?: () => void;
}) {
  return (
    <OverviewTiles
      connection={connection}
      now={NOW}
      readAt={readAt}
      health={healthRead}
      capacity={capacity}
      jobs={jobs}
      repositories={repositories}
      picked={picked}
      drifts={drifts}
      onOpenSettings={onOpenSettings}
      onOpenQueued={onOpenQueued}
      onOpenManifest={onOpenManifest}
    />
  );
}
