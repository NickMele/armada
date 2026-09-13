// What Fleet can say about its own health, and what it cannot — `GET /health`.
// `crates/ipc/src/health.rs`.
//
// **Not Doctor.** Doctor's grid is modules probed from `adapters`, `config` and
// `store`, and it is not built — `docs/concepts/doctor.md`. This answers the
// rows Fleet itself holds, and names the rest as unprobed.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and they
// drift the day a field moves.

/** One module, asked. */
export type Probe = {
  /** The module name as Doctor's grid spells it — `Fleet`, `SQLite`, `Manifest`, `System stats`. */
  module: string;
  /** `pass`, `warn` or `fail`, Doctor's own three words. A string: nothing matches on it. */
  outcome: string;
  /** What the probe read, in one line. Never empty. */
  detail: string;
};

/**
 * A set of Doctor probes Fleet cannot run, grouped by who owns them rather than
 * listed by name.
 */
export type Unprobed = {
  /** Which crate or surface owns the probes. */
  owner: string;
  /** Why Fleet cannot run them. Read by a person, never matched on. */
  because: string;
};

/**
 * Every probe Fleet ran, and everything it could not run one for. **`not_probed`
 * is the honest half**: four passing rows with no mention of the modules nobody
 * asked read as a healthy machine.
 */
export type FleetHealth = {
  probes: Probe[];
  not_probed: Unprobed[];
};
