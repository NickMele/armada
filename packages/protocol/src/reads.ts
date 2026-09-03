// The shape of a read, and of what comes back from asking for one.
//
// **A read is not a channel.** These say what Bridge holds about a Job — read,
// reading, failed, and what arrived — which is the same shape whatever carried
// it. The channel names and the preload surface that carried it are Electron's
// and stay in `apps/desktop`.
//
// `JobRead` is the family: every per-Job read is one of these, so a screen that
// can draw one unread Job can draw them all.

import type {
  CallArguments,
  JobDetail,
  JobFilesChanged,
  JobSummary,
  ManifestSummary,
  ModelChoices,
  Recorded,
  Report,
  ReportList,
  Saw,
  Submitted,
  Voice,
  Work,
  WorkflowSummary,
  WireError,
  WorktreeReclaimed,
} from "./index";

/**
 * `GET /reports`, where a surface asked for it.
 *
 * **Its own shape rather than a `JobRead`**, because it is not a read of a Job:
 * there is no id to check an answer against, which is the whole of what that
 * type exists to do. The four states are the same four, for the same reason —
 * "nobody asked" and "the read failed" are different things to draw.
 */
export type Reports =
  | { state: "none" }
  | { state: "reading" }
  | { state: "read"; list: ReportList }
  | { state: "failed"; outcome: Outcome };

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
  /**
   * The socket could not be read. **The rows are kept for `ended`'s reason**:
   * a failure is a fact about the connection and not about the turns already
   * in hand, and one transient error used to empty a log that was full a
   * moment before.
   */
  | { state: "failed"; jobId: string; turns: Turns; detail: string };

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
  /**
   * Whose row this is — Armada, the Drone, or Fleet. Beside `saw` for `step`'s
   * reason: it is true of every kind. Never absent here, because main fills a
   * row the wire did not stamp with `drone`, which is what every such row is.
   */
  by: Voice;
  saw: Saw;
};

/**
 * What one call's arguments came back as.
 *
 * **Answered to the caller rather than published as state.** Every other read
 * here is held by main and republished as events arrive, because a Job that
 * moves has to redraw. A recorded argument never moves: it is fetched once, by
 * the person who opened one row, and it is theirs. Putting it in `BridgeState`
 * would make one reader's gesture part of what every surface re-renders on.
 *
 * A refusal is the row's own, never the screen's — `refused` on this route is
 * the Job standing and the call not being in its transcripts, which is a thing
 * to say inside the payload and not an error state for the Job.
 */
export type CallRead =
  | { ok: true; call: CallArguments }
  | { ok: false; outcome: Outcome };

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
   *
   * `reclaimed` rides on the same terms as `report`, and for the same reason.
   * Giving a worktree back changes no row — the Job stays exactly where it is
   * — so what came back is a receipt the surface that asked shows once. **It
   * is not derivable from `ok`**: a branch holding commits the base cannot
   * reach is kept on purpose, so a successful reclaim can honestly report that
   * half of it did not happen.
   */
  | { ok: true; jobId?: string; report?: Report; reclaimed?: WorktreeReclaimed }
  | { ok: false; why: "not_connected" }
  | { ok: false; why: "empty_brief" }
  | { ok: false; why: "empty_title" }
  | { ok: false; why: "no_workflow" }
  | { ok: false; why: "no_manifest" }
  | { ok: false; why: "already_approving" }
  | { ok: false; why: "already_redispatching" }
  | { ok: false; why: "already_killing" }
  | { ok: false; why: "already_forgetting" }
  | { ok: false; why: "already_reclaiming" }
  | { ok: false; why: "already_redirecting" }
  | { ok: false; why: "already_restarting" }
  | { ok: false; why: "already_overruling" }
  | { ok: false; why: "already_rereading" }
  | { ok: false; why: "already_reporting" }
  | { ok: false; why: "empty_instruction" }
  | { ok: false; why: "empty_reason" }
  | { ok: false; why: "empty_report" }
  | { ok: false; why: "already_deciding" }
  | { ok: false; why: "already_answering" }
  | { ok: false; why: "empty_note" }
  | { ok: false; why: "refused"; error: WireError }
  | { ok: false; why: "transport"; detail: string };

/**
 * What `proposeFromRequest` answered. **Not `Outcome`**, for two reasons that
 * are both about what a person does next.
 *
 * One request can be several Jobs, and `Outcome` carries one optional `jobId`
 * for the one act that mints a Job the caller did not name. A plan is every Job
 * the request became, and approving one of several accepts a plan whose members
 * each take their own approval in turn — a shape that carried a single id would
 * make that unrepresentable.
 *
 * And the two refusals are not the same thing. **The request being declined and
 * the call failing are different statuses because a person does different
 * things about them** — one is said again differently or hand-entered through
 * `proposeJob`, the other is simply asked again. `crates/ipc/operations.toml`
 * is where that division is stated and `crates/fleet/src/refusing.rs` is where
 * the two codes are declared apart so a client can honour it.
 *
 * Nothing here is a Job that is running. Every member is at
 * `awaiting_approval`, exactly as `proposeJob` answers for one.
 */
export type Proposed =
  /**
   * Every Job the request became, in dependency order, already folded onto the
   * board. Never empty on a success: Fleet answers 201 with at least one.
   */
  | { ok: true; jobs: JobSummary[] }
  /**
   * The request was read and no workflow fits. **No Job was created**, and
   * `request` is what was sent, carried back off the error envelope's own
   * field so what a person retypes is what they wrote, character for
   * character.
   *
   * Not a fault and never a default: the resolved definition is frozen into
   * the Job and becomes what the work is judged against, so a nearest fit
   * would be the standard rather than a guess anybody could correct.
   */
  | { ok: false; why: "unresolved"; request: string; outcome: Outcome }
  /**
   * The call could not be made — the network, the quota, the timeout, or no
   * answer at all. **This says nothing about the request**, which is why it is
   * not the arm above: rendering an outage as "nothing fits" tells a person
   * their request was refused when it was never read. Asking again is
   * reasonable, and `request` is what to ask with.
   */
  | { ok: false; why: "faulted"; request: string; outcome: Outcome }
  /**
   * Nothing was proposed and neither of the two above says why: Bridge's own
   * refusal before anything was sent, or a refusal of Fleet's that is neither
   * of the named two. The outcome carries the whole of it, and the surface
   * renders it the way it renders every other refusal.
   */
  | { ok: false; why: "refused"; outcome: Outcome };

/**
 * What `clearTerminalJobs` answered. **Not `Outcome`**: there is no bulk
 * `forget_job`, so this is one call per id and some can refuse while others
 * land — a status that moved between the press and the call, or an id Fleet
 * no longer holds. `cleared` is folded into the board as each call answers;
 * `failed` is what the caller has to say something about.
 */
export type ClearOutcome = {
  cleared: string[];
  failed: { jobId: string; outcome: Outcome }[];
};

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
