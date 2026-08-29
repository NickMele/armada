// What the three processes agree on: the state main publishes, the operations
// the renderer may initiate, and the channel names those two travel over.
//
// Types, the channel constants, and the empty state both ends start from.
// Nothing here runs in more than one process — the preload is a wire and not an
// import path, and this file is the shape of what crosses it.

import type {
  FileReport,
  JobDetail,
  JobFilesChanged,
  JobSummary,
  Report,
  UnreadableJob,
  WireError,
} from "./protocol";
import type { ManifestSummary, ModelChoices, WorkflowSummary } from "./setup";
import type { Artifact, Opened } from "./artifacts";
import type { Recorded } from "./history";
import type { Submitted, Work } from "./work";
import type { Saw } from "./turn";
import { connects, skew } from "./version";
import type { ProtocolVersion, Skew } from "./version";
import { PROTOCOL_VERSION } from "./generated/protocol-version";

/** Fleet, as its runtime file names it. Loopback plus `port` is the address. */
export type FleetIdentity = {
  /** Both numbers. Which one differs from Bridge's decides what happens. */
  protocolVersion: ProtocolVersion;
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
  /**
   * Fleet speaks a protocol Bridge will not connect over. Two readings, and
   * they need different sentences: `incompatible` is a different protocol,
   * `fleet_behind` is the same protocol missing additions Bridge now expects.
   */
  | {
      state: "version_skew";
      fleet: FleetIdentity;
      why: Extract<Skew, "fleet_behind" | "incompatible">;
      speaks: ProtocolVersion;
      expected: ProtocolVersion;
    }
  /**
   * Connected, and `skew` says whether there is a caveat on it.
   *
   * `fleet_ahead` is **not a failure** and must not render as one: everything
   * Bridge draws is current and correct, and the only fact is that Fleet knows
   * things this Bridge was built too early to ask about. Decided here, once, so
   * no surface re-derives it.
   */
  | {
      state: "connected";
      fleet: FleetIdentity;
      cursor: number;
      skew: Extract<Skew, "same" | "fleet_ahead">;
    };

/**
 * A live connection, saying which of the two readings it is.
 *
 * Here rather than at the call site so nothing can publish `connected` without
 * deciding whether Fleet is ahead — and narrowed rather than cast, because the
 * two readings that refuse never reach a socket.
 */
export function connectedTo(fleet: FleetIdentity, cursor: number): Connection {
  const reading = skew({ fleet: fleet.protocolVersion, bridge: PROTOCOL_VERSION });
  return { state: "connected", fleet, cursor, skew: connects(reading) ? reading : "same" };
}

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
  /**
   * What the open Job's Drone has changed in its worktree.
   *
   * **Only the open Job's, and only while the event arrives.**
   * `job.files_changed` is published for every Job on the one stream; keeping
   * every Job's footprint would make the Board pay for a detail nobody has
   * open, which is the thing this read is meant to stay off.
   */
  footprint: Footprint;
  /**
   * One Job's transition history, where a surface asked for one.
   *
   * **Read when it is asked for, not on every open.** It is its own operation
   * for that reason: a detail is fetched to draw a summary and a history has no
   * bound — it grows for as long as the Job lives, and a retried step is a row
   * per attempt plus the moves around it.
   */
  history: History;
  /**
   * What one Job's Drones claimed, where a surface asked for it. The cheap half
   * of the pair, and still asked for rather than paid for on every open.
   */
  evidence: Evidence;
  /**
   * One Job's worktree against the branch it was cut from, where a surface asked
   * for it. **The expensive half, and the one place the patch bytes are spent.**
   * `crates/adapter-traits/src/work_product.rs` splits it off the file list
   * because the bytes are large and most steps ask no semantic question; this is
   * read on the act they were split for, never folded into `watched`, which is
   * re-read every time an event names the open Job.
   */
  diff: Diff;
};

/**
 * The last `job.files_changed` reading for the open Job.
 *
 * Two states and not four: nothing is fetched, so there is no reading and no
 * failed read. The event either arrived or it has not, and a surface says which
 * in its own words rather than sharing one sentence with a failed query.
 */
export type Footprint =
  | { state: "none" }
  | { state: "read"; jobId: string; reading: JobFilesChanged };

/**
 * One read of one route under one Job, in the four states every such read has.
 * `Read` is what the answer carries; the other three states carry no answer.
 *
 * **Four, because "nobody asked" and "the read failed" are different things to
 * draw** — a surface with one state for both says a Job has nothing where what
 * is true is that nothing was read.
 *
 * `main/reader.ts` is the only thing that moves a read through these, and the
 * reason this shape is named once rather than written out per read.
 */
export type JobRead<Read> =
  | { state: "none" }
  | { state: "reading"; jobId: string }
  | ({ state: "read"; jobId: string } & Read)
  | { state: "failed"; jobId: string; outcome: Outcome };

/**
 * `GET /jobs/:job_id/events` for one Job.
 *
 * **The rows are rendered, never replayed.** `crates/store/src/fold.rs` owns
 * the machine and is the only thing that may put an event back through
 * `Job::transition`; nothing on this side of the wire does.
 */
export type History = JobRead<{ moves: Recorded[] }>;

/**
 * `GET /jobs/:job_id/evidence` for one Job. **Empty is a real answer** rather
 * than `none`: no step submitted anything is a fact about the Job, not about
 * the read.
 */
export type Evidence = JobRead<{ steps: Submitted[] }>;

/**
 * `GET /jobs/:job_id/diff` for one Job.
 *
 * **`work` stays optional on `read`, keeping the wire's distinction.** Absent is
 * a Job with no worktree; present with an empty `files` is a Drone that changed
 * nothing. A shape that could not tell them apart would report a Job at the
 * approval gate as a Drone that wrote nothing.
 */
export type Diff = JobRead<{ work?: Work }>;

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
export type Turn = {
  ts: string;
  seq: number;
  /**
   * The `step_id` that was running when Fleet saw the row — beside `saw`, since
   * it is true of every kind. **Absent is not the first step**: it is a row
   * written before Fleet recorded the field, and nothing recovers which it was.
   */
  step?: string;
  saw: Saw;
};

/** `GET /jobs/:job_id` for the open Job. */
export type Watched = JobRead<{ detail: JobDetail }>;

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
   *
   * `report` rides along on the same terms: filing produces a record the caller
   * needs next, and it is not app state — nothing on the board changes and the
   * surface that asked is the only one that shows it, so keeping it here would
   * leave a filed report on screen after the dialog that filed it closed.
   */
  | { ok: true; jobId?: string; report?: Report }
  | { ok: false; why: "not_connected" }
  | { ok: false; why: "empty_brief" }
  | { ok: false; why: "empty_title" }
  | { ok: false; why: "no_workflow" }
  | { ok: false; why: "no_manifest" }
  | { ok: false; why: "already_approving" }
  | { ok: false; why: "already_redispatching" }
  | { ok: false; why: "already_killing" }
  | { ok: false; why: "already_redirecting" }
  | { ok: false; why: "already_restarting" }
  | { ok: false; why: "already_overruling" }
  | { ok: false; why: "already_reporting" }
  | { ok: false; why: "empty_instruction" }
  | { ok: false; why: "empty_reason" }
  | { ok: false; why: "empty_report" }
  | { ok: false; why: "already_deciding" }
  | { ok: false; why: "empty_note" }
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
  /**
   * Files staged through `stageAttachment` before this Job exists. Empty is
   * the ordinary case and is sent as such — unlike `model`, there is no
   * absent-vs-empty distinction here for Fleet to fill in.
   */
  attachments: { path: string; filename: string; mimeType: string }[];
};

/** The whole preload surface, and therefore everything the renderer can reach. */
export type BridgeApi = {
  protocolVersion: () => ProtocolVersion;
  state: () => Promise<BridgeState>;
  subscribe: (onState: (state: BridgeState) => void) => () => void;
  proposeJob: (draft: Draft) => Promise<Outcome>;
  /**
   * Write pasted or picked bytes to a staging file before a Job exists —
   * there is no Job id yet to key storage on; one is minted at `propose`
   * time. Returns the absolute path written, which the caller carries on
   * `Draft.attachments` until `proposeJob` sends it as a `staged_path`.
   */
  stageAttachment: (
    bytes: ArrayBuffer,
    filename: string,
    mimeType: string,
  ) => Promise<{ path: string }>;
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
   * Inject an instruction into the Drone that is there. **Legal only on an
   * escalated Job that still holds one** — Fleet refuses 409 where the Drone
   * is gone, naming `restartStep` as the act that applies. Nothing is
   * spawned; the Job comes back `running` with the same session.
   */
  redirectDrone: (jobId: string, instruction: string) => Promise<Outcome>;
  /**
   * Put a fresh Drone on the surviving worktree, at the step that stopped.
   * **Legal only where the Drone is gone** — Fleet refuses 409 where one is
   * still alive, or where the worktree itself is gone.
   */
  restartStep: (jobId: string) => Promise<Outcome>;
  /**
   * Overrule a machine that stopped the work, and let the Job go on.
   *
   * **Not an approval, and its own entry for that reason.** `approveReview`
   * answers a gate nothing objected to; this answers one that stopped the step,
   * and it says a machine was wrong rather than that the work was right. The
   * step advances still carrying `failed`, and the reason is written to the
   * Job's log where it stays.
   *
   * Legal on an escalated Job whose step stopped on `gate_failure` — the Judge
   * refusing a criterion — or on `evidence_suspect`, the gaming check calling
   * the evidence untrustworthy. Both are a machine's decision, which is what a
   * person may overrule. Fleet refuses 409 for `gate_undecided`, where nothing
   * weighed the work, and for a step that stopped on anything else; 422 for a
   * blank reason. Whether the Drone is still there decides only how the Job
   * carries on.
   */
  overrideVerdict: (jobId: string, reason: string) => Promise<Outcome>;
  /**
   * Read one Job whole and keep it current, or `null` to stop.
   *
   * The renderer says which Job is open; main does the reading and republishes
   * it whenever an event names that Job. One call per open, not one per event.
   */
  /**
   * Say that this Job failed in error, in your own words, and file the Job's
   * own record with it.
   *
   * **Not an act on the Job.** Nothing moves, nothing is spawned and nothing is
   * dispatched: a report is a record of what a person concluded, and an entry
   * that also moved the Job would make disagreeing with a verdict a way of
   * getting past one — which is `overrideVerdict`, a different act with a
   * different refusal.
   *
   * The sentence is required and blank is refused before the request is sent,
   * matching the 422 Fleet would give it. What comes back on the outcome is the
   * report, because the rendered record is what a person does the next thing
   * with — Armada does not file it anywhere, and says so.
   */
  fileReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
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
  /**
   * Read one Job's transition history, or `null` to stop.
   *
   * **Its own entry because it is its own operation.** A history is not on
   * `JobDetail`, so folding it into `watchJob` would make every Job opened pay
   * for a surface that is folded away by default. Read-only, like the two above
   * it: a recorded move is a fact, and nothing here can add one.
   */
  readHistory: (jobId: string | null) => Promise<void>;
  /**
   * Read what one Job's Drones claimed, or `null` to stop. Read-only, and its
   * own entry rather than folded into `readDiff`: they are two operations on
   * the Rust side because a surface wanting only the claims would otherwise
   * fetch a megabyte to read four lines.
   */
  readEvidence: (jobId: string | null) => Promise<void>;
  /**
   * Read one Job's worktree against its branch, or `null` to stop. Read-only.
   * **The one capability here that spends the patch bytes**, and deliberately
   * not reachable by opening a Job — the renderer calls it from the surface
   * that draws a diff, which is the act the bytes were separated for.
   */
  readDiff: (jobId: string | null) => Promise<void>;
  /**
   * Take the work. **The counterpart to `approveDispatch`, at the other end of
   * the Job.** On the workflow's last step Fleet commits and delivers before
   * recording the Job done. Legal only at `awaiting_review`, like the two below.
   */
  approveReview: (jobId: string) => Promise<Outcome>;
  /**
   * Send the work back with a note. **The Job comes back `running`**, same step,
   * same Drone — nothing is spawned and nothing done is thrown away.
   */
  requestChanges: (jobId: string, note: string) => Promise<Outcome>;
  /**
   * A verdict on the work, and the Job is over. **Terminal, and it ends the
   * Drone** — that is what separates it from `requestChanges`, and it is not
   * `killJob`, which clears the Board and carries no verdict at all. Three
   * entries and not one taking which: that would read as one act and perform
   * three, and the three differ by whether anything survives.
   */
  rejectWork: (jobId: string) => Promise<Outcome>;
  /**
   * Open one of a Job's artifacts in whatever the OS opens it with.
   *
   * **One entry taking which, where the kills and the decisions are three.**
   * Those are split because they differ in what survives them; these three
   * differ only in which file, they change nothing about the Job, and a Job id
   * plus a word from a closed set is deliberately less than a path — main
   * derives the path, so the renderer never holds the argument that matters.
   *
   * The branch is absent from `Artifact` and stays a copy: it is served rather
   * than derived, and it is not a path.
   */
  openArtifact: (jobId: string, what: Artifact) => Promise<Opened>;
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
  footprint: { state: "none" },
  history: { state: "none" },
  evidence: { state: "none" },
  diff: { state: "none" },
};

/** The channels the preload is allowed to name. There is no general `invoke`. */
export const CHANNELS = {
  state: "bridge:state",
  changed: "bridge:changed",
  proposeJob: "bridge:propose-job",
  stageAttachment: "bridge:stage-attachment",
  approveDispatch: "bridge:approve-dispatch",
  redispatchJob: "bridge:redispatch-job",
  killDrone: "bridge:kill-drone",
  killJob: "bridge:kill-job",
  redirectDrone: "bridge:redirect-drone",
  restartStep: "bridge:restart-step",
  overrideVerdict: "bridge:override-verdict",
  fileReport: "bridge:file-report",
  watchJob: "bridge:watch-job",
  observeJob: "bridge:observe-job",
  readHistory: "bridge:read-history",
  readEvidence: "bridge:read-evidence",
  readDiff: "bridge:read-diff",
  approveReview: "bridge:approve-review",
  requestChanges: "bridge:request-changes",
  rejectWork: "bridge:reject-work",
  openArtifact: "bridge:open-artifact",
} as const;
