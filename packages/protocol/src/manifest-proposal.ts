// A possible `armada.yml` per workspace, built from Scan: `crates/ipc/src/manifest_proposal.rs`.
// Hand-written under `protocol.ts`'s rules, so every closed set is a `string`.

import type { ManifestSaved } from "./editing";
import type { ManifestRefused } from "./reading";

/** `GET /repository/proposals`. */
export type ManifestProposals = {
  checkout: string;
  /** Root first, then by path. */
  proposals: ManifestProposal[];
  /** Stated at Setup, never written. */
  caps: StatedCaps;
};

export type StatedCaps = { cost_micros: number; turns: number };

export type ManifestProposal = {
  /** `.` for the root. What an edit names. */
  dir: string;
  /** Where the file would be, relative to the checkout. */
  file: string;
  id: ProposedId;
  ports: ProposedPort[];
  /** In written order, which is the order the gate starts them in. */
  checks: ProposedCheck[];
  commands: ProposedCommand[];
  setup?: ProposedSetup;
  /** `auto_merge`, then `review_gate`. */
  policy: ProposedPolicy[];
  /** The file Write would put down. Absent where it is refused. */
  text?: string;
  /** Every fault that stops the file loading. */
  refused?: ManifestRefused;
  /** Absent until Write lands it; after that it takes no edits. */
  written?: ManifestSaved;
};

/**
 * `source` is `read`, `convention`, `default`, `edited_during_setup` or
 * `added_during_setup`. Only the first two carry `file`; mono for those, sans otherwise.
 */
export type Provenance = { source: string; file?: string; key?: string };

export type ProposedId = { value: string; provenance: Provenance };

/** `env` is absent for a compose service, which needs no variable. */
export type ProposedPort = {
  name: string;
  container?: number;
  env?: string;
  provenance: Provenance;
};

export type ProposedCheck = {
  name: string;
  run: string;
  /** Command names, in the order they run. */
  requires?: string[];
  provenance: Provenance;
};

export type ProposedCommand = {
  name: string;
  run: string;
  destructive?: boolean;
  provenance: Provenance;
};

export type ProposedSetup = { requires: string[]; provenance: Provenance };

/** A `default` row is an absent key. */
export type ProposedPolicy = { key: string; value: string; provenance: Provenance };

/** `POST /repository/edit_proposal`. */
export type EditManifestProposal = { dir: string; edit: ProposalEdit };

/** A put replaces the line it names or adds one. `band` is `ports`, `checks` or `commands`. */
export type ProposalEdit =
  | { edit: "id"; id: string }
  | { edit: "port"; name: string; container?: number; env?: string }
  | { edit: "check"; name: string; run: string; requires?: string[] }
  | { edit: "command"; name: string; run: string; destructive?: boolean }
  | { edit: "setup"; requires: string[] }
  | { edit: "policy"; key: string; value?: string }
  | { edit: "move"; name: string }
  | { edit: "remove"; band: string; name: string };

/** `POST /repository/write_proposal`. */
export type WriteManifestProposal = { dir: string };
