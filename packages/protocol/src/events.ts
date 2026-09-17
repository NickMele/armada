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

import type { JudgeInFlight, Settled } from "./detail";
import type { LineCount } from "./footprint";
import type { JobForgotten, JobList, JobSummary, Reason } from "./protocol";
import type { ManifestReading } from "./reading";
import type { ProposalInFlight } from "./proposing";
import type { CheckoutRunRecord, RunRecord } from "./rehearsal";
import type { ServerState } from "./servers";
import type { RepositoryList } from "./setup";
import type { Studio, StudioDeleted, StudioHelmActed } from "./studio";
import type { ChecksUnderway } from "./underway";
import type { QuestionInFlight } from "./waiting";
import type { CommandInFlight } from "./commanding";
import type { ProtocolVersion } from "./version";
import type { JobPlanChanged } from "./work-plan";

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
  | ({ kind: "drone.spawned" } & DroneSpawned)
  | ({ kind: "drone.exited" } & DroneExited)
  | ({ kind: "job.files_changed" } & JobFilesChanged)
  | ({ kind: "job.judging" } & JobJudging)
  | ({ kind: "job.checking" } & JobChecking)
  /** A Drone's own mid-step run of the Checks, never the gate's. Since 13.45. */
  | ({ kind: "job.dry_run" } & JobDryRun)
  /** A Drone handed in its report, before the gate starts. Since 13.3. */
  | ({ kind: "evidence.submitted" } & EvidenceSubmitted)
  | ({ kind: "job.asking" } & JobAsking)
  | ({ kind: "job.command_waiting" } & JobCommandWaiting)
  | ({ kind: "job.forgotten" } & JobForgotten)
  | ({ kind: "job.landed" } & JobLanded)
  | ({ kind: "job.remarks_changed" } & JobRemarksChanged)
  /** A Job's plan was recorded or a task changed. Since 13.21. */
  | ({ kind: "job.plan_changed" } & JobPlanChanged)
  | ({ kind: "proposal.moved" } & ProposalMoved)
  | ({ kind: "manifest.reread" } & ManifestReading)
  /** The repositories Fleet serves changed; the list now, whole, as `list_repositories` answers. */
  | ({ kind: "repositories.changed" } & RepositoryList)
  | ({ kind: "run.finished" } & RunRecord)
  /** A run in the main checkout ended. Its own kind: the record names no Job. Since 11.9. */
  | ({ kind: "checkout_run.finished" } & CheckoutRunRecord)
  | ({ kind: "server.starting" } & ServerState)
  | ({ kind: "server.serving" } & ServerState)
  | ({ kind: "server.exited" } & ServerState)
  /** A Studio after a write to it, whole. Since 14.6. */
  | ({ kind: "studio.changed" } & Studio)
  /** A Studio a person deleted. Since 14.6. */
  | ({ kind: "studio.deleted" } & StudioDeleted)
  /** Helm took one act on a Studio, beside its `studio.changed`. Since 14.7. */
  | ({ kind: "studio.helm_acted" } & StudioHelmActed);

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

/**
 * The forge's answer about one job's pull request comments changed since the
 * last sweep found it open: a comment was added, edited or removed, or a
 * review landed. `crates/ipc/src/event.rs`. Since protocol 10.9.
 *
 * **Carries the id and nothing else.** Fleet's own read is deliberately
 * narrow and does not bring in line comments; `get_remarks` already answers
 * what changed and brings those too, so a copy of its answer riding this
 * event would be a second shape to keep in step with the first.
 *
 * **Never on the first sweep that reads a given pull request.** A first read
 * has nothing to compare against, so this never fires for every open pull
 * request the moment Fleet starts.
 */
export type JobRemarksChanged = {
  job_id: string;
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
 * A Drone started against a worktree. `crates/ipc/src/event.rs`.
 *
 * **`job.step_advanced`'s shape, and for its reason.** `assigned_drone` is a
 * field of the row, so the summary travels whole and a list replaces the row
 * rather than asking for it again.
 */
export type DroneSpawned = {
  /** The Job as it now stands, with `assigned_drone` set. */
  job: JobSummary;
  step_id: string;
  drone_id: string;
  /** What a person checks out to watch it. Absent where the worktree names none. */
  branch?: string;
  actor: string;
  at: string;
};

/**
 * A Drone is gone, however it went. `crates/ipc/src/event.rs`.
 *
 * **It does not say why**, and a client must not read one into it: what an
 * ending meant is the Job's own `job.state_changed`, and an outcome restated
 * here would be a second statement of it.
 *
 * **It is what says a Drone's run has been paid for.** Fleet writes the spend
 * row before publishing this, on every path a Drone ends — so a detail re-read
 * on this kind is the first one that can see the whole figure. See
 * `crates/fleet/src/allowance.rs`.
 */
export type DroneExited = {
  /** The Job as it now stands, with `assigned_drone` gone. */
  job: JobSummary;
  step_id: string;
  drone_id: string;
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
 * One of a step's Checks started or finished, or the gate's ruling on them was
 * written down. `crates/ipc/src/event.rs`. Since 10.3.
 *
 * **`job.judging`'s shape, one tier along.** One message per start and per
 * finish, each carrying the whole set; elapsed is counted here from
 * `started_at`. The last message carries nothing and arrives after
 * `check_runs` holds the same results, so a re-read on it never finds them in
 * neither place.
 */
export type JobChecking = {
  job_id: string;
  step_id: string;
  /** The step's Checks as they stand, or absent because the ruling is written. */
  checking?: ChecksUnderway;
  actor: string;
  at: string;
};

/**
 * One of the Checks a Drone asked for mid-step started or finished, or its run
 * is no longer shown. `crates/ipc/src/event.rs`. Since 13.45.
 *
 * **`job.checking`'s shape, as a kind of its own**, so a Drone's run is never
 * drawn as the gate's.
 */
export type JobDryRun = {
  job_id: string;
  step_id: string;
  /** The run's Checks as they stand, or absent once it is no longer shown. */
  dry_run?: ChecksUnderway;
  actor: string;
  at: string;
};

/**
 * A Drone handed in its report. **The gate has not started.**
 * `crates/ipc/src/event.rs`. Since 13.3.
 *
 * The moment between the Drone submitting and `job.checking` that nothing else
 * carried — a surface inferred it from the Drone's process exiting and then the
 * Checks beginning, and neither is the same fact. `#813`.
 *
 * **A pointer, never a payload.** What was claimed, what shows it and what it
 * left alone are on `GET /jobs/:job_id/evidence`, fetched by whoever opens the
 * Job. One message per submission, and a step's second is refused while its
 * first waits for the gate.
 */
export type EvidenceSubmitted = {
  job_id: string;
  /** The step the submission is against, off the working slot. */
  step_id: string;
  /** What the frozen step asked the work product to be. Fleet's word, not the Drone's. */
  evidence_type: string;
  /** Always `drone`. */
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
 * A drone is held on a command waiting for a person, or the one it was held on
 * was answered. `crates/ipc/src/event.rs`. Since protocol 10.7.
 *
 * `job.asking`'s shape exactly: two messages per command, the one going out
 * carrying `waiting` and the one coming back carrying nothing. Only a job at
 * `ask_me` produces it — under the default nothing waits.
 */
export type JobCommandWaiting = {
  job_id: string;
  step_id: string;
  /** The command the drone is held on, or absent because it was answered. */
  waiting?: CommandInFlight;
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
  /**
   * What the file gained and lost, as of Fleet's last counted reading. Since
   * protocol 14.10, and only on `job.files_changed`. **Absent is not zero**: a
   * file nothing counted, or one that arrived after the last count, which
   * Fleet takes once the Drone's calls settle and at most every ten seconds.
   */
  lines?: LineCount;
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
