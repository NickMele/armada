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
} from "./protocol";
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
  | ({ kind: "job.forgotten" } & JobForgotten);

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
