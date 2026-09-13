// Setup's wire half, as the app reads it — Scan, the proposals, one edit, Write.
// `docs/journeys/set-up-a-project-manifest.md`. No React, so main imports these shapes.

import type {
  ManifestProposal,
  ManifestProposals,
  Outcome,
  RepositoryScan,
} from "@armada/protocol";

/** `GET /repository/scan`. */
export type RepositoryScanRead = { ok: true; scan: RepositoryScan } | { ok: false; outcome: Outcome };

/** `GET /repository/proposals`. */
export type ManifestProposalsRead =
  | { ok: true; proposals: ManifestProposals }
  | { ok: false; outcome: Outcome };

/**
 * An edit's or a Write's answer. `appeared` is its own state because nothing broke: a file is
 * already at the path, and what a person does is read it rather than retry.
 */
export type ProposalAnswer =
  | { state: "took"; proposal: ManifestProposal }
  | { state: "refused"; saying: string; faults: { key: string; fault: string }[] }
  | { state: "appeared"; onDisk: string | null }
  | { state: "failed"; outcome: Outcome };
