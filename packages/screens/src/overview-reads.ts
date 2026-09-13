// Overview's two reads, as the app holds them — `GET /health`, and drift for each repository in
// the scope. No React, so main imports these shapes.

import type { ManifestDriftRead, Outcome } from "@armada/protocol";

/**
 * `crates/ipc/src/health.rs`, typed by hand: `packages/protocol` has no copy of it yet, and this
 * moves there when it does. `outcome` is Doctor's pass, warn or fail, left a string as the wire
 * leaves it.
 */
export type Probe = { module: string; outcome: string; detail: string };
/** Probes Fleet cannot run, grouped by who owns them. */
export type Unprobed = { owner: string; because: string };
export type FleetHealth = { probes: Probe[]; not_probed: Unprobed[] };

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
