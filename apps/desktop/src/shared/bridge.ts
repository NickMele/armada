// What the three processes agree on: the state main publishes, the operations
// the renderer may initiate, and the channel names those two travel over.
//
// Types, the channel constants, and the empty state both ends start from.
// Nothing here runs in more than one process — the preload is a wire and not an
// import path, and this file is the shape of what crosses it.

import type {
  JobDetail,
  JobSummary,
  ManifestSummary,
  ModelChoices,
  Saw,
  UnreadableJob,
  WireError,
  WorkflowSummary,
} from "./protocol";

/** Fleet, as its runtime file names it. Loopback plus `port` is the address. */
export type FleetIdentity = {
  protocolVersion: number;
  pid: number;
  port: number;
  /** `ps -o lstart=` as it read when Fleet published the file. */
  startedAt: string;
};

/**
 * Why the runtime file does not describe a live Fleet.
 *
 * Three, not one. Bridge renders the first two as "Fleet is not running" and
 * says which under it, because the third — a pid something else now holds — is
 * the case a bare liveness check gets wrong, and the consequence is a socket
 * opened against a port an unrelated program owns.
 */
export type Absence =
  | { why: "no_runtime_file"; path: string }
  | { why: "pid_dead"; path: string; pid: number }
  | { why: "pid_held_by_another"; path: string; pid: number; wrote: string; holder: string };

/**
 * Why the runtime file could not be read at all. **None of these is "not
 * running"** — that is a fact about the world, and folding a failed read into
 * it tells a person Fleet is down on no evidence.
 */
export type RuntimeFault =
  | { why: "unreadable"; path: string; detail: string }
  | { why: "undecodable"; path: string; detail: string }
  | { why: "probe_failed"; path: string; pid: number; detail: string };

/** Where Bridge's one connection is. */
export type Connection =
  | { state: "reading" }
  | { state: "not_running"; absence: Absence }
  | { state: "runtime_file_refused"; fault: RuntimeFault }
  | { state: "connecting"; fleet: FleetIdentity }
  /** The pid checks out and the socket does not answer. A different thing to do. */
  | { state: "unreachable"; fleet: FleetIdentity; detail: string; sinceMs: number }
  | { state: "version_skew"; fleet: FleetIdentity; speaks: number; expected: number }
  | { state: "connected"; fleet: FleetIdentity; cursor: number };

/**
 * What a Bridge failure can point a person at.
 *
 * **No `run_id`, and none is minted.** The envelope makes `run_id` the one id an
 * emitter mints for itself, but nothing in Bridge writes a log line yet, so an
 * id minted here would join to nothing and would read on screen as though it
 * identified the failure. The only real one is the one a `WireError` carries,
 * and that names Fleet's run rather than any single failure.
 *
 * The path is a fact about the main process — the renderer cannot resolve a
 * home directory — so it is published rather than guessed at.
 */
export type BridgeIdentity = {
  /** The machine log. `null` where HOME is not set and no path resolves. */
  auditPath: string | null;
};

/** Everything the renderer draws, published by main and never assembled twice. */
export type BridgeState = {
  connection: Connection;
  /** Where a Bridge failure points. Never a Job's identity. */
  bridge: BridgeIdentity;
  jobs: JobSummary[];
  /** Rows the store refused. Shown, never merged into `jobs` as a placeholder. */
  unreadable: UnreadableJob[];
  /** Events Fleet dropped before Bridge saw them, since the window opened. */
  missed: number;
  /** When the Jobs above were last current, in epoch milliseconds. */
  readAt: number | null;
  /** Jobs with an approval in flight. What stops a second dispatch. */
  approving: string[];
  /**
   * What Fleet holds, and therefore what a proposal may name.
   *
   * Read over the one connection like everything else here. The composer used
   * to offer a text field for a pasted id, because nothing served these — and a
   * pasted id was accepted by Fleet unchecked. Both halves of that are fixed:
   * Fleet refuses an id it does not hold, and this is what the form offers so
   * nobody has to guess at one.
   */
  holds: Holdings;
  /**
   * The one Job read whole, where a detail is open.
   *
   * **Published, not fetched by the component that draws it.** The detail is
   * re-read whenever an event names its Job, which is what makes the rail move
   * without a reload — a renderer holding its own copy would go stale the
   * moment a step advanced.
   */
  watched: Watched;
  /**
   * The Job being watched turn by turn, where somebody opened one.
   *
   * **Its own socket, and its own piece of state.** Transcript rows arrive at
   * Drone speed, and putting them on the stream the Board is drawn from would
   * evict the state changes that draw it. Separate here for the same reason.
   */
  observed: Observed;
};

/**
 * One Job's turns, as `GET /jobs/:job_id/observe` answered.
 *
 * **Read-only, all the way down.** Nothing here has a command beside it and
 * nothing here can reach the Drone: observing changes nothing about the Job,
 * which is the whole difference between this and Pilot.
 */
export type Observed =
  | { state: "none" }
  | { state: "opening"; jobId: string }
  | { state: "watching"; jobId: string; turns: Turns }
  /** The socket ended. The rows are kept — a closed transcript is still a record. */
  | { state: "ended"; jobId: string; turns: Turns; because: string }
  | { state: "failed"; jobId: string; detail: string };

/** What one Observe connection has said so far. */
export type Turns = {
  /** Whether a Drone was writing when the socket opened. `false` is ordinary. */
  live: boolean;
  /** Older rows the bounded backfill left out, from `opened`. */
  skipped: number;
  /** Rows this viewer fell behind and lost. **Not the sink's losses.** */
  missed: number;
  rows: Turn[];
};

/**
 * One row, with the instant Fleet's line loop saw it.
 *
 * `Called` and `Answered` stay two rows on the wire and are joined in the
 * view: joining them in Fleet would mean holding a call open until its result
 * arrived, which is unbounded buffering in the path that must never block.
 */
export type Turn = { ts: string; seq: number; saw: Saw };

/**
 * `GET /jobs/:job_id` for the open Job. Four states, because "no detail on
 * screen" and "the read failed" are different things to draw.
 */
export type Watched =
  | { state: "none" }
  | { state: "reading"; jobId: string }
  | { state: "read"; jobId: string; detail: JobDetail }
  | { state: "failed"; jobId: string; outcome: Outcome };

/** The values a proposal may name. Empty until the connection answers. */
export type Holdings = {
  workflows: WorkflowSummary[];
  manifests: ManifestSummary[];
  /** `null` until read: an empty roster and an unread one are not the same. */
  models: ModelChoices | null;
};

/** What a command answered. A refusal names itself; it never renders as silence. */
export type Outcome =
  /**
   * `jobId` is the Job the caller should open next, where the act produced one
   * that is not the Job it was called on. Redispatch is the only one: it mints
   * a replacement, so the id that came back is not the id that went in.
   */
  | { ok: true; jobId?: string }
  | { ok: false; why: "not_connected" }
  | { ok: false; why: "empty_brief" }
  | { ok: false; why: "empty_title" }
  | { ok: false; why: "no_workflow" }
  | { ok: false; why: "no_manifest" }
  | { ok: false; why: "already_approving" }
  | { ok: false; why: "already_redispatching" }
  | { ok: false; why: "already_killing" }
  | { ok: false; why: "refused"; error: WireError }
  | { ok: false; why: "transport"; detail: string };

/** What the create form collects, before it becomes a `ProposeJob`. */
export type Draft = {
  /** What the Job is called. Refused empty before the Job is created. */
  title: string;
  workflowId: string;
  manifestId: string;
  origin: string;
  urgency: string;
  /**
   * Which model the Drone is spawned as. The picker starts on the configured
   * default, so the common path is one click and this is rarely empty — and
   * empty is still legal on the wire, where Fleet fills it in.
   */
  model: string;
  atomic: boolean;
  /** Free text. Refused empty before the Job is created. */
  brief: string;
};

/** The whole preload surface, and therefore everything the renderer can reach. */
export type BridgeApi = {
  protocolVersion: () => number;
  state: () => Promise<BridgeState>;
  subscribe: (onState: (state: BridgeState) => void) => () => void;
  proposeJob: (draft: Draft) => Promise<Outcome>;
  approveDispatch: (jobId: string) => Promise<Outcome>;
  /**
   * Kill the failed Job and mint its replacement. **Nothing resumes** — the
   * Job it is called on ends at `killed` and a new one is created carrying
   * `redispatched_from`, whose id comes back on the outcome.
   */
  redispatchJob: (jobId: string) => Promise<Outcome>;
  /** Kill the process. The Job survives, with its worktree held. */
  killDrone: (jobId: string) => Promise<Outcome>;
  /** End the Job at `killed`. Terminal, and nothing resumes it. */
  killJob: (jobId: string) => Promise<Outcome>;
  /**
   * Read one Job whole and keep it current, or `null` to stop.
   *
   * The renderer says which Job is open; main does the reading and republishes
   * it whenever an event names that Job. One call per open, not one per event.
   */
  watchJob: (jobId: string | null) => Promise<void>;
  /**
   * Watch one Job's turns, or `null` to stop.
   *
   * **Nothing is sent to the Drone and nothing can be.** This opens a socket
   * that only reads, closes it when the window closes it, and leaves the Job
   * exactly as it found it — a capability that could intervene would be Pilot,
   * which is a different act with a transition on the record.
   */
  observeJob: (jobId: string | null) => Promise<void>;
};

/**
 * What Bridge holds before anything has answered.
 *
 * **One statement, not two.** Main and the renderer each used to declare their
 * own, and the two drifted the first time a field was added — a renderer
 * missing a key main publishes reads as a field that is always absent.
 */
export const NOTHING_YET: BridgeState = {
  connection: { state: "reading" },
  // Main resolves the log path from the home it can see. Until it answers, the
  // renderer does not know it and does not name one.
  bridge: { auditPath: null },
  jobs: [],
  unreadable: [],
  missed: 0,
  readAt: null,
  approving: [],
  holds: { workflows: [], manifests: [], models: null },
  watched: { state: "none" },
  observed: { state: "none" },
};

/** The channels the preload is allowed to name. There is no general `invoke`. */
export const CHANNELS = {
  state: "bridge:state",
  changed: "bridge:changed",
  proposeJob: "bridge:propose-job",
  approveDispatch: "bridge:approve-dispatch",
  redispatchJob: "bridge:redispatch-job",
  killDrone: "bridge:kill-drone",
  killJob: "bridge:kill-job",
  watchJob: "bridge:watch-job",
  observeJob: "bridge:observe-job",
} as const;
