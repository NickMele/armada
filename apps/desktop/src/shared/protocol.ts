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
  /**
   * The Job this one replaces. A redispatch mints a new Job rather than
   * reopening the old one, so without this a board reads every second failure
   * as a first one.
   */
  redispatched_from?: string;
  /**
   * When the Job was created. On the row rather than only on the detail,
   * because elapsed is what answers "is this stuck" without opening it, and
   * reading it per row would be one request per row.
   */
  created_at: string;
  /** Absent until a worktree exists. A Job at the approval gate has none. */
  branch?: string;
};

/**
 * One Job, whole. The answer to `GET /jobs/:job_id`. `crates/ipc/src/detail.rs`.
 *
 * **Every optional field is omitted, never null.** Absent and empty are
 * different sentences on screen, and `write_targets` is the one that shows why:
 * absent is scope undetermined, present and empty is determined to write
 * nothing.
 *
 * Evidence, per-step Check results, the log file and spend are not here and are
 * not invented. Nothing serves them.
 */
export type JobDetail = {
  /** The board row, unchanged. A field added to the row reaches here for free. */
  job: JobSummary;
  /** What a whole-Job elapsed is measured from. Creation is not a transition. */
  created_at: string;
  /** Absent until a worktree exists. A Job at the gate has no branch. */
  branch?: string;
  /** One entry per step of the frozen WorkflowDef, in order. */
  steps: StepDetail[];
  acceptance_criteria: Criterion[];
  /** Context the Job was given. Absent where none was, rather than `""`. */
  facts?: string;
  /** Absent is scope undetermined; present and empty is writing nothing. */
  write_targets?: string[];
  subject?: Subject;
  /** The DAG edges this Job sits on. Empty until something writes one. */
  dependencies: Dependency[];
};

/** One step: which, where in the order, and where it got to. */
export type StepDetail = {
  step_id: string;
  /** Position in the frozen WorkflowDef, so a rail draws past and future. */
  ordinal: number;
  /** `job_steps.state`, served rather than inferred from the Job's status. */
  state: string;
  /**
   * The Checks this step declares, in the workflow's order.
   *
   * **Empty is "this step is ungated"; absent is "Fleet cannot say."** Those
   * are two different sentences on screen and the rail says each of them in
   * words — the key being missing means the Job named a workflow this Fleet
   * does not hold, which is not the same as a step that gates on nothing.
   */
  checks?: DeclaredCheck[];
  /** What each declared Check did. Empty until the gate has run them. */
  check_runs: CheckRun[];
  /** Absent until a gate has ruled on the step. */
  last_verdict?: Verdict;
  /** Entered, then moved on entering `running`. To `updated_at` is how long. */
  entered_at: string;
  updated_at: string;
};

/** One Check a step declares. `crates/ipc/src/checks.rs`. */
export type DeclaredCheck = {
  /** `manifest_check` or `diff_nonempty`, as the WorkflowDef schema spells it. */
  kind: string;
  /** The Manifest Check's name. Absent on `diff_nonempty`, which names none. */
  name?: string;
  /** The exit code the step expects, where there is a command to return one. */
  expect_exit_code?: number;
};

/**
 * One declared Check, as the gate found it.
 *
 * **`produced` is absent on a pass because a pass measured nothing** — the
 * outcome is the whole sentence. The other four each say something different
 * about why a step did not advance, and none of them is `failed`.
 */
export type CheckRun = {
  /** The Manifest Check's name, or the built-in's kind. Joins to `checks`. */
  name: string;
  /** `check-outcomes.toml`: `passed`, `failed`, `signalled`, `timed_out`, `never_ran`. */
  outcome: string;
  /** What the Check was measured against. Absent on a pass. */
  expected?: string;
  /** The exit code, the signal, the budget it outran, or what is not installed. */
  produced?: string;
  /**
   * Where the Check's stdout and stderr were written, relative to the
   * repository root. **A reference, never the content**, and absent where there
   * is no file — a built-in assertion runs no command, and a Check that never
   * started printed nothing.
   */
  output_path?: string;
};

/** The last ruling against a step. `failed` carries its trigger; the rest do not. */
export type Verdict = {
  named: string;
  trigger?: string;
};

/** One acceptance criterion, with the id a Judge citation references. */
export type Criterion = {
  criterion_id: string;
  text: string;
  source: string;
};

/** What a Job is about. Neither sequencing nor provenance. */
export type Subject = { kind: string; reference: string };

/** One DAG edge, sequencing peer Jobs. */
export type Dependency = {
  direction: string;
  peer: string;
};

/**
 * What a redispatch did. **Two Jobs, because a redispatch is two acts** — the
 * failed one is killed and a replacement is minted carrying
 * `redispatched_from`. Nothing here reopens anything.
 */
export type Redispatched = {
  /** The Job that failed, now `killed`. Its worktree is as its Drone left it. */
  replaced: JobSummary;
  /** The replacement, at the approval gate. What the caller opens next. */
  dispatched: JobSummary;
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
  | ({ kind: "job.state_changed" } & JobStateChanged)
  | ({ kind: "job.step_advanced" } & JobStepAdvanced);

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

/**
 * A step of the frozen WorkflowDef moved. **The Job did not.**
 *
 * `from` and `to` are `job_steps.state`; `status` is the status the move
 * happened *beneath* and is unchanged by this event. A client that folded it as
 * a status change would draw a transition that never happened.
 *
 * The whole row travels, because `current_step_id` has already moved — which is
 * the reload this event exists to stop.
 */
export type JobStepAdvanced = {
  /** The Job as it now stands. Replaces the row whole; never patched into it. */
  job: JobSummary;
  step_id: string;
  from: string;
  to: string;
  /** The status the step moved beneath. Not a status change. */
  status: string;
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
  /**
   * The steps, in order. Order is the semantics.
   *
   * **Objects since protocol 3, not step ids.** A field whose type changed is a
   * major bump, and the reason it changed is that a composer offering a
   * workflow could say how many steps it had and not whether any of them gates.
   */
  steps: WorkflowStep[];
  manifest_id: string;
};

/** One step of a workflow, as the definition declares it. */
export type WorkflowStep = {
  step_id: string;
  /** Always a list, never absent. Empty is the ungated step. */
  checks: DeclaredCheck[];
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

// ------------------------------------------------------------ one Job's turns
//
// `GET /jobs/:job_id/observe`, the one query whose transport is a socket.
// Read-only: nothing here has a request half, and nothing here reaches a Drone.

/** One message on a Job's Observe socket. `crates/ipc/src/turn.rs`. */
export type TurnMessage =
  | ({ message: "opened" } & Opened)
  | ({ message: "row"; ts: string } & Saw)
  | ({ message: "missed" } & Missed)
  | ({ message: "closed" } & Closed);

/** The first message on every connection, before any row. */
export type Opened = {
  protocol_version: number;
  job_id: string;
  /** Whether a Drone was writing when this opened. `false` is ordinary. */
  live: boolean;
  /** Older rows the bounded backfill left out. Never a silent truncation. */
  skipped: number;
};

/** Nothing more is coming, and why. A socket that simply stops says nothing. */
export type Closed = {
  /** `drone_ended` or `nothing_writing`. */
  because: string;
};

/**
 * One row of a Drone's transcript, as a viewer is shown it.
 *
 * The tag is `event` and not `kind`, because `unrecognised` already carries a
 * `kind`. Three of the file's kinds never arrive here — `quota_moved`, `ended`
 * and the sink's own `missed` — so no case is written for them.
 */
export type Saw =
  | { event: "started"; session: string; model: string; mcp_servers: number }
  /**
   * The Drone reached for a tool.
   *
   * **`detail` is not on the wire yet, and the name here is a placeholder.**
   * `Saw::Called` carries a tool and an opaque call id, so a pane reads
   * `Bash · toolu_01Haa…` twenty-two times over — measured on one transcript.
   * What the call did is being added on Fleet's side; this is the field the
   * renderer reads when it lands, and nothing fakes it from the tool name in
   * the meantime. Reconcile the spelling against `crates/ipc/src/turn.rs`.
   *
   * It is bounded and may be cut: a `Write` argument is a whole file.
   */
  | {
      event: "called";
      tool: string;
      call: string;
      detail?: string;
      /** Whether `detail` was cut short. Never rendered as the whole value. */
      truncated?: boolean;
    }
  | { event: "answered"; call: string; failed: boolean }
  | { event: "said"; text: string }
  | { event: "refused"; tool: string; call: string; because: string }
  | { event: "unrecognised"; kind: string }
  | { event: "unreadable"; line: string; why: string };
