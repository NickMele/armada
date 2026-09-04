// The event vocabulary, as TypeScript sees it. `crates/ipc/src/event.rs`.
//
// **Split out of `protocol.ts`, not written apart from it.** That file reached
// the 900 lines the gate refuses, and this is the one seam inside it that is
// already a seam on the Rust side: a message pushed over the socket, rather
// than a shape answered to a request. `protocol.ts` re-exports every name here,
// so nothing that imported one had to change.
//
// The header rules there hold here: these are hand-written, they drift the day
// a field moves, and every closed set is left as `string`.

import type {
  JobForgotten,
  JobList,
  JobSummary,
  JudgeInFlight,
  Reason,
  Settled,
} from "./protocol";
import type { ManifestReading } from "./reading";
import type { ProposalInFlight } from "./proposing";
import type { QuestionInFlight } from "./waiting";
import type { ProtocolVersion } from "./version";

/** One message from Fleet to a connected client. `crates/ipc/src/event.rs`. */
export type StreamMessage =
  | ({ message: "resync" } & Resync)
  | ({ message: "event" } & Delivered)
  | ({ message: "missed" } & Missed);

export type Resync = {
  protocol_version: ProtocolVersion;
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
  | ({ kind: "job.step_advanced" } & JobStepAdvanced)
  | ({ kind: "job.files_changed" } & JobFilesChanged)
  | ({ kind: "job.judging" } & JobJudging)
  | ({ kind: "job.asking" } & JobAsking)
  | ({ kind: "job.forgotten" } & JobForgotten)
  | ({ kind: "job.landed" } & JobLanded)
  | ({ kind: "proposal.moved" } & ProposalMoved)
  | ({ kind: "manifest.reread" } & ManifestReading);

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

/**
 * Somebody merged, or closed, the pull request a job opened. Since protocol 6.6.
 *
 * **Not a state change and not a step move.** The job was finished and recorded
 * when this happened; what moved is a pull request on a forge, outside Armada
 * entirely. It carries the row whole, with `landed` filled in, so a list
 * replaces the row rather than re-reading it.
 *
 * **Once per job, ever.** Fleet writes the answer down and never asks again, so
 * a client may treat it as final.
 */
export type JobLanded = {
  job: JobSummary;
  /** The address, so a client can say which pull request without the detail. */
  pull_request: string;
  settled: Settled;
  /**
   * When Fleet **read** this, not when the merge happened. The two differ by up
   * to one sweep, and the forge is where the exact instant lives.
   */
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

/**
 * What the working Drone has changed in its worktree, as of one reading.
 * `crates/ipc/src/event.rs`.
 *
 * **The whole footprint, not a delta.** A client replaces the list it holds
 * rather than folding this into one, so a file that stopped being changed — a
 * revert, a checkout — leaves the view by not being in the next reading.
 *
 * It names no `JobSummary`, unlike the kinds that move a row: nothing on the
 * Board changes when a file does, and this is read by a detail view somebody
 * opened on one Job.
 */
export type JobFilesChanged = {
  job_id: string;
  /** Which step's Drone did it. The footprint is the Job's whole work. */
  step_id: string;
  drone_id: string;
  /**
   * Whether the step has a declared plan for `outside_plan` to mean anything.
   * **False is "there is no plan", not "nothing drifted"**, and a surface that
   * drew the two the same way would report every unscoped step as on plan.
   */
  plan_declared: boolean;
  /** Every file, in the order the reading found them. Empty is a real answer. */
  files: ChangedFile[];
  actor: string;
  at: string;
};

/**
 * A Judge call went out on a step, or the one that was out came back.
 * `crates/ipc/src/event.rs`.
 *
 * **Two messages per call and never a third.** The one going out carries
 * `judging`; the one coming back carries nothing, and that absence is the
 * message rather than the stream going quiet. Elapsed is subtracted from
 * `since` here, so a call that takes the whole two-minute budget costs the
 * channel two messages rather than one a second.
 *
 * It names no `JobSummary`: nothing on the Board's row changes when a call goes
 * out, and this is read by a detail view somebody has open on one Job — the
 * same terms as `job.files_changed`.
 */
export type JobJudging = {
  job_id: string;
  step_id: string;
  /** The call that went out, or absent because it came back. */
  judging?: JudgeInFlight;
  actor: string;
  at: string;
};

/**
 * A drone asked a person a question, or the one that was out was answered.
 * `crates/ipc/src/event.rs`.
 *
 * **Two messages per question and never a third.** The one going out carries
 * `asking`; the one coming back carries nothing, and that absence is the
 * message rather than the stream going quiet. `job.judging`'s shape exactly.
 *
 * **The actor differs between the two, which no other kind does.** Going out it
 * is `drone` and coming back it is `human` — the two ends of the act. Fleet
 * caused neither.
 *
 * It names no `JobSummary`: nothing on the board's row changes when a question
 * goes out, and this is read by a detail view somebody has open on one job.
 */
export type JobAsking = {
  job_id: string;
  step_id: string;
  /** The question that went out, or absent because it was answered. */
  asking?: QuestionInFlight;
  actor: string;
  at: string;
};

/**
 * A proposal went out, got somewhere, or came back. `crates/ipc/src/event.rs`.
 *
 * **The one kind here that names no Job**, and that is what it is for: a
 * proposal is the interval before any Job exists. `job.created` is what says
 * the Jobs arrived, and it is a different message.
 *
 * **More than two messages per call, unlike `job.judging`.** That one settles
 * for two because a surface can subtract an elapsed count for itself, which is
 * right for a gate nobody watches. Here somebody is being asked whether to keep
 * waiting, and the answer turns on whether the call is moving. Fleet publishes
 * none of these while nothing is subscribed, and throttles the token estimate
 * to one a second.
 */
export type ProposalMoved = {
  proposal_id: string;
  /**
   * The caller's own token, echoed. **On the envelope as well as inside
   * `proposing`** — it is what a client filters on, and the coming-back message
   * carries no `proposing` to read it from.
   */
  client_ref?: string;
  /**
   * The call while it is out, or **absent because it came back** — however it
   * came back: with Jobs, with a refusal, or stopped. What it produced arrives
   * as `job.created`, one per Job.
   */
  proposing?: ProposalInFlight;
  actor: string;
  at: string;
};

/**
 * One file in the Drone's footprint. **A name and a kind, never bytes** — what
 * changed inside a file is the patch, which is read only when a Judge fires and
 * is deliberately not on this seam.
 */
export type ChangedFile = {
  /** Repository-relative, exactly as git spells it. */
  path: string;
  /** `added`, `modified`, `deleted`, `renamed`, `copied`, `type_changed`,
   * `conflicted`, `unreadable`. Left as `string` like every other closed set. */
  change: string;
  /**
   * Not covered by the plan the step declared. **A mark, not a judgement** —
   * it restates a comparison already made and decides nothing. Always false
   * where the step declared no plan, which is what `plan_declared` is for.
   */
  outside_plan?: boolean;
};

// `manifest.reread` carries `ManifestReading` itself rather than a payload
// wrapping it, which is what `job.forgotten` does with `JobForgotten`: the
// answer to `get_manifest_reading` and this event's body are the same fact.
//
// It is the second kind here that names no Job, and unlike `proposal.moved` it
// names no Drone and no step either — a Manifest is Fleet's own, so nothing on
// the Board moves when it arrives. It carries no `actor` and no `at` on the
// envelope for that reason: `at` would be a second copy of the reading's own,
// and nobody pressed anything in Armada at all.
