// A form's edits to `armada.yml`, on their way to Fleet, and the file they
// left. `crates/ipc/src/amending.rs`.
//
// **Edits, not text.** `SaveManifestFile` carries a whole file because a person
// typed it; a form knows which key it changed and says so, and Fleet splices
// exactly that key into the file. Every line the form did not touch stays byte
// for byte; a removed entry takes the comment block directly above it.
//
// **What a form produces always loads.** Fleet refuses edits whose result would
// not parse — a 422 under `fleet.manifest_edit_refused`, carrying `faults` as
// `[key, fault]` pairs — where `SaveManifestFile` writes work in progress.
//
// The header rules in `protocol.ts` hold here: these are hand-written, they
// drift the day a field moves, and every closed set is left as `string`.

/** Edits to the Manifest, and the text they were made against. */
export type EditManifest = {
  /**
   * The `ManifestFile.text` the form was drawn from, whole. Fleet refuses a
   * file that moved — `fleet.manifest_moved_under_the_edit`, as for a save.
   */
  read: string;
  /** Applied in order, each to what the one before it left. */
  edits: ManifestEdit[];
};

/**
 * One edit a form makes. **`null` on a `set_` removes the key**, and the key
 * is required. Clearing a list removes its key.
 */
export type ManifestEdit =
  | { edit: "add_check"; name: string; check: CheckDraft }
  | { edit: "remove_check"; name: string }
  | { edit: "set_check_run"; name: string; run: string }
  | { edit: "set_check_requires"; name: string; requires: string[] }
  | { edit: "set_check_when"; name: string; when: string[] }
  | { edit: "set_check_narrow"; name: string; narrow: NarrowingDraft | null }
  | { edit: "add_command"; name: string; command: CommandDraft }
  | { edit: "remove_command"; name: string }
  | { edit: "set_command_run"; name: string; run: string | null }
  /** `false` removes a written `true`; absent already means `false`. */
  | { edit: "set_command_destructive"; name: string; destructive: boolean }
  | { edit: "set_command_serve"; name: string; serve: string | null }
  | { edit: "set_command_ready"; name: string; ready: string | null }
  | { edit: "set_command_links"; name: string; links: LinkDraft[] }
  | { edit: "add_port"; name: string; port: PortDraft }
  | { edit: "remove_port"; name: string }
  | { edit: "set_port_container"; name: string; container: number | null }
  | { edit: "set_port_env"; name: string; env: string | null }
  /** The word as the file writes it. `null` means `never`. */
  | { edit: "set_auto_merge"; auto_merge: string | null }
  /** The word as the file writes it. `null` means `human_always`. */
  | { edit: "set_review_gate"; review_gate: string | null }
  /** `null` defers to what Fleet runs with. */
  | { edit: "set_cost_cap_micros_per_job"; cost_cap_micros_per_job: number | null }
  /** `null` defers to what Fleet runs with. */
  | { edit: "set_turn_cap_per_job"; turn_cap_per_job: number | null };

/** A Check a form declares. */
export type CheckDraft = {
  run: string;
  requires?: string[];
  when?: string[];
  narrow?: NarrowingDraft;
};

/** `checks.<name>.narrow`, whole. */
export type NarrowingDraft = {
  run: string;
  each: string;
  from?: string[];
  under?: string;
  except?: string[];
};

/** A Command a form declares. `serve` makes it a server. */
export type CommandDraft = {
  run?: string;
  destructive?: boolean;
  serve?: string;
  ready?: string;
  links?: LinkDraft[];
};

/** One address a server offers. */
export type LinkDraft = {
  url: string;
  name?: string;
};

/** A port a form declares. Neither field is a port Armada places. */
export type PortDraft = {
  container?: number;
  env?: string;
};

/**
 * What the edits left on disk. **The text comes back**: it is the next edit's
 * `read`, and reading the file again would race whatever lands next.
 */
export type ManifestEdited = {
  path: string;
  at: string;
  text: string;
  /** What `text` loads as. Absent from an older Fleet; the form reads the file again. */
  declared?: ManifestDeclared;
};

/**
 * What a form draws: every key an edit can reach, as the file declares it, in
 * the drafts an edit sends — so a value read is a value sent.
 */
export type ManifestDeclared = {
  /** In the order the file writes them. */
  checks: { name: string; check: CheckDraft }[];
  /** Commands and servers together, sorted. `serve` tells them apart. */
  commands: { name: string; command: CommandDraft }[];
  /** Sorted. */
  ports: { name: string; port: PortDraft }[];
  auto_merge: PolicyWords;
  review_gate: PolicyWords;
  /** Absent where the file defers to what Fleet runs with. */
  cost_cap_micros_per_job?: number;
  turn_cap_per_job?: number;
};

/** A policy's value and every word it takes — the registry is Fleet's. */
export type PolicyWords = {
  /** In force: the default where the file says nothing. */
  written: string;
  offered: string[];
};

/**
 * `GET /manifest/spend` — the costliest and the longest Job against this
 * Manifest, each summed across its Drones. A cap is set per Job, so the
 * highest is what one below it would have stopped.
 */
export type ManifestSpend = {
  /** Jobs that have spent anything. Zero leaves nothing to warn against. */
  jobs: number;
  /** A floor where a Drone named no price. */
  most_cost_micros: number;
  most_turns: number;
};
