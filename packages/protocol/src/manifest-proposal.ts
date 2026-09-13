// A possible `armada.yml` per workspace, built from Scan, and the edits that
// iterate it. `crates/ipc/src/manifest_proposal.rs`.
//
// **Every line says where it came from, and nothing here sets that.** An edit
// carries no provenance; Fleet moves a touched line to `edited_during_setup` or
// `added_during_setup`. A port a file declares is `read`; every script is
// `convention`. Not `proposing.ts`, which is a Job on its way to the gate.
//
// **Lines, not text**: the file comes with Write, from `config`'s one writer.
//
// The header rules in `protocol.ts` hold here: hand-written, and every closed
// set is left as `string`.

/** `GET /repository/proposals`. */
export type ManifestProposals = {
  checkout: string;
  /** The root first, then by path — Scan's order. */
  proposals: ManifestProposal[];
  /** What a Job here stops at. **Stated, never written.** */
  caps: StatedCaps;
};

export type StatedCaps = { cost_micros: number; turns: number };

export type ManifestProposal = {
  /** The workspace, `.` for the root — what an edit names. */
  dir: string;
  /** Where the file would be, relative to the checkout. */
  file: string;
  /** The header, not a row. */
  id: ProposedId;
  ports: ProposedPort[];
  /** In written order, which is the order the gate starts them in. */
  checks: ProposedCheck[];
  commands: ProposedCommand[];
  /** Absent where nothing runs first. */
  setup?: ProposedSetup;
  /** `auto_merge` and `review_gate`, always both. */
  policy: ProposedPolicy[];
};

/**
 * `source` is `read`, `convention`, `default`, `edited_during_setup` or
 * `added_during_setup`. **Mono for a file, sans for the rest**: `file` and
 * `key` are present on the first two, and `key` is absent on a `convention`
 * whose whole file is the evidence.
 */
export type Provenance = { source: string; file?: string; key?: string };

export type ProposedId = { value: string; provenance: Provenance };

/** `env` is absent for a compose service, which gets its port without one. */
export type ProposedPort = {
  name: string;
  container?: number;
  env?: string;
  provenance: Provenance;
};

export type ProposedCheck = {
  name: string;
  run: string;
  /** Command names, in the order they run. Absent where none. */
  requires?: string[];
  provenance: Provenance;
};

export type ProposedCommand = {
  name: string;
  run: string;
  /** Absent where false. */
  destructive?: boolean;
  provenance: Provenance;
};

export type ProposedSetup = { requires: string[]; provenance: Provenance };

/** `key` is `auto_merge` or `review_gate`. A `default` row is an absent key. */
export type ProposedPolicy = { key: string; value: string; provenance: Provenance };

/** `POST /repository/edit_proposal`. */
export type EditManifestProposal = { dir: string; edit: ProposalEdit };

/**
 * A put naming a line that exists replaces it; one naming nothing adds it.
 * `band` is `ports`, `checks` or `commands`.
 */
export type ProposalEdit =
  | { edit: "id"; id: string }
  | { edit: "port"; name: string; container?: number; env?: string }
  | { edit: "check"; name: string; run: string; requires?: string[] }
  | { edit: "command"; name: string; run: string; destructive?: boolean }
  | { edit: "setup"; requires: string[] }
  | { edit: "policy"; key: string; value?: string }
  | { edit: "move"; name: string }
  | { edit: "remove"; band: string; name: string };
