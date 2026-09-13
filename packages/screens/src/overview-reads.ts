// Overview's two reads, as the app holds them — `GET /health`, and drift for each repository in
// the scope. No React, so main imports these shapes.

import type { FleetHealth, ManifestDriftRead, Outcome } from "@armada/protocol";

/** `GET /health`, in `reads.ts`'s four states. */
export type HealthRead =
  | { state: "none" }
  | { state: "reading" }
  | { state: "read"; health: FleetHealth }
  | { state: "failed"; outcome: Outcome };

/** One repository's drift, by its root. A repository with no Manifest fails `not_set_up`. */
export type RepositoryDrift = { root: string; drift: ManifestDriftRead };

/**
 * Drift for every repository in the scope — each one Fleet serves on All, or the one picked.
 * `none` while no surface holds it open.
 */
export type DriftsRead = { state: "none" } | { state: "held"; repositories: RepositoryDrift[] };
