// The wire vocabulary, as TypeScript sees it.
//
// **These are hand-written, and that is a gap rather than a design.**
// `crates/ipc/src/lib.rs` says a codegen step emits TypeScript from the Rust
// DTOs and that both generated outputs are checked in, so a cross-language
// breaking change is a build failure. That codegen does not exist yet, so the
// shapes below are a second statement of the ones in `crates/ipc/src/` and they
// drift the day a field moves. Nothing else in Bridge restates them.
//
// Every closed set is left as `string`. The Rust side refuses an arriving
// spelling the registry does not have; a union spelled here would be a third
// copy of a roster that already has two, and an unknown status renders as
// itself rather than as a guess.

/** A Job, as a list row. `crates/ipc/src/job.rs`. */
export type JobSummary = {
  id: string;
  /** What the Job is called. The one field on a row a person actually reads. */
  title: string;
  status: string;
  reason?: Reason;
  workflow_id: string;
  owner_manifest_id: string;
  origin: string;
  urgency: string;
  atomic: boolean;
  model: string;
  /** Which step the Job is on. */
  current_step_id?: string;
  /** Presence, not state: absent is a Job no process is on. */
  assigned_drone?: string;
};

/** The reason a transition carried, where it stored one. */
export type Reason = {
  named?: string;
  criteria_owed?: string[];
};

/** A Job on disk that would not load. Never filtered away. */
export type UnreadableJob = {
  job_id?: string;
  fault: string;
};

/** Every Job, and every one that would not load. */
export type JobList = {
  jobs: JobSummary[];
  unreadable?: UnreadableJob[];
};

/** A Job drafted onto the approval gate. The request half of `propose_job`. */
export type ProposeJob = {
  /** Required. A proposal without one does not decode on the Rust side. */
  title: string;
  workflow_id: string;
  owner_manifest_id: string;
  /** One of the four top-level origins. `sub_dispatched` does not deserialise. */
  origin: string;
  urgency: string;
  atomic: boolean;
  /**
   * Optional, and absent is the ordinary case: Fleet fills it from
   * configuration. It used to be required, and the `""` that invited was
   * accepted, stored, drawn on the board and refused at spawn.
   */
  model?: string;
  acceptance_criteria?: ProposedCriterion[];
  subject?: { kind: string; reference: string };
  /** Context the Job needs to run. Append-only once the Job exists. */
  facts: string;
  /** Null is not empty: absent is scope not yet determined. */
  write_targets?: string[];
};

export type ProposedCriterion = { text: string; source: string };

/** One message from Fleet to a connected client. `crates/ipc/src/event.rs`. */
export type StreamMessage =
  | ({ message: "resync" } & Resync)
  | ({ message: "event" } & Delivered)
  | ({ message: "missed" } & Missed);

export type Resync = {
  protocol_version: number;
  cursor: number;
  jobs: JobList;
};

export type Delivered = {
  cursor: number;
  event: Event;
};

/** The bound was reached and the oldest were dropped. Always followed by a resync. */
export type Missed = { dropped: number };

export type Event =
  | ({ kind: "job.created" } & JobCreated)
  | ({ kind: "job.state_changed" } & JobStateChanged);

/**
 * A Job exists that did not before, carrying the row whole.
 *
 * **Not a state change.** A created Job has no status it moved from, so a
 * `job.state_changed` here would name a transition the edge table does not
 * have. The summary travels with it, so the list inserts the row rather than
 * re-reading everything to learn one.
 */
export type JobCreated = {
  job: JobSummary;
  actor: string;
  at: string;
};

export type JobStateChanged = {
  job_id: string;
  from: string;
  to: string;
  reason?: Reason;
  actor: string;
  at: string;
};

/** One workflow Fleet holds. `crates/ipc/src/setup.rs`. */
export type WorkflowSummary = {
  /** What a proposal's `workflow_id` must name. Anything else is refused. */
  id: string;
  /** What a person reads. `bug`, where the id is what a Job points at. */
  name: string;
  version: number;
  /** The steps, in order. Order is the semantics. */
  steps: string[];
  manifest_id: string;
};

/** One Manifest Fleet holds. */
export type ManifestSummary = {
  id: string;
  /**
   * The repository the Manifest was read from. **Not a name it declares** —
   * `armada.yml` has no key for one, so this is the directory, which is a fact
   * rather than an invention.
   */
  repository: string;
  path: string;
  version: number;
  checks: string[];
};

/** The models a Job may name, and the one it gets when it names none. */
export type ModelChoices = {
  models: string[];
  /** Always a member of `models`, so a picker selects it without a lookup. */
  default: string;
};

/** A failure, flattened for the wire. `docs/contracts/error-contract.md`. */
export type WireError = {
  /** Opaque to Bridge: looked up, never parsed. */
  code: string;
  /** What renders when the lookup misses. */
  message: string;
  run_id: string;
  fields: Record<string, unknown>;
  chain: string[];
  job_id?: string;
  drone_id?: string;
  step_id?: string;
};
