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
//
// # A Job as a row, and no longer a Job read whole
//
// This file was restating two Rust modules at once: `job.rs`, which is what a
// board and a list carry, and `detail.rs`, which is the answer to
// `GET /jobs/:job_id`. It reached the 900 lines the gate refuses for the fifth
// time, and the seam was the one already drawn on the other side — so
// `detail.rs`'s half is `detail.ts` now, down to which of the two modules
// `Dependency` and `Settled` belong to. What is left is `job.rs`, the smaller
// neighbours no module of their own has yet earned, and the re-exports that
// keep this the one import for the wire vocabulary.
//
// `Settled` is imported back from `detail.ts` because a row carries `landed`
// too. Type-only both ways, so the cycle is erased before anything runs.

import type { Settled } from "./detail";
import type { ProtocolVersion } from "./version";

/** A Job, as a list row. `crates/ipc/src/job.rs`. */
export type JobSummary = {
  id: string;
  /** What the Job is called. The one field on a row a person actually reads. */
  title: string;
  status: string;
  reason?: Reason;
  /**
   * Why an approved Job has not started. Its own field because it is derived
   * from the board at read time rather than recorded by a transition, which is
   * what `reason` carries — absent on every status but `queued`, and absent on
   * a queued Job that nothing is holding.
   */
  queued_reason?: string;
  /**
   * Which act a person took to put this Job back in the queue — a key into
   * `RESUMPTION` in the generated vocabulary.
   *
   * **The other axis over `queued`, and the one that says somebody is
   * waiting.** `queued_reason` says what the Job is waiting for. Absent on a
   * Job approved and never run, which is how a Job *arrives* at `queued`
   * rather than *returns* to it, and absent on every other status.
   *
   * Without it, pressing restart while the bound is spent moves nothing on
   * screen and a correct system reads as a dropped press.
   */
  resumption?: string;
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
   * Whether this job's drone is waiting on an answer from a person. Since
   * protocol 5.7.
   *
   * **A flag, and deliberately not the question.** What was asked and what each
   * answer commits to are on the detail's `asking`, which is one job somebody
   * opened; this is a board drawn for every job at once, and a paragraph per row
   * to say one true-or-false is what the summary redacts `facts` to avoid.
   *
   * **Absent is false and both mean the same thing**, unlike every other
   * optional field here — fleet omits it when it is not set rather than sending
   * `false`, because a bool has no third reading for absence to carry.
   *
   * One of two fields on the row not read off the record — `landed` is the
   * other. It comes from the working slot, so it is false on every summary
   * built where no slot was in hand, which is every event publish — correct
   * rather than a gap, since `job.asking` says a question exists.
   */
  asking?: boolean;
  /**
   * What became of the pull request this job opened. Since protocol 6.6.
   *
   * **Absent is every job that has not opened one, and every one nobody has
   * merged yet** — one absence, because neither is news. It is the only
   * question anybody has about finished work: without it a terminal row says
   * the same thing whether the change is in `main` or has sat unread a week.
   */
  landed?: Settled;
  /**
   * When the Job was created. On the row rather than only on the detail,
   * because elapsed is what answers "is this stuck" without opening it, and
   * reading it per row would be one request per row.
   */
  created_at: string;
  /** Absent until a worktree exists. A Job at the approval gate has none. */
  branch?: string;
};

/** One Check a step declares. `crates/ipc/src/checks.rs`. */
export type DeclaredCheck = {
  /** `manifest_check` or `diff_nonempty`, as the WorkflowDef schema spells it. */
  kind: string;
  /** The Manifest Check's name. Absent on `diff_nonempty`, which names none. */
  name?: string;
  /** The command the Check resolved to, as the Job's frozen workflow holds it.
   * **Absent on `diff_nonempty`**, which runs nothing. Always the frozen
   * workflow's and never the live Manifest's — editing `armada.yml` mid-Job
   * must not change what a finished step says it ran. */
  run?: string;
  /** The exit code the step expects, where there is a command to return one. */
  expect_exit_code?: number;
  /**
   * Which paths this Check covers, as the Job's frozen workflow holds them.
   * A step that changes none of them does not run it.
   *
   * **Absent means always, and it is never `[]`.** Always and never are
   * opposite answers and Fleet sends no key at all for the first, so a client
   * has nothing to disambiguate. Draw it *before* the Check runs — that is the
   * only moment it says anything the `skipped` row will not say later.
   */
  when?: string[];
};

/**
 * One `judge_checks[]` entry a step declares, counted rather than quoted.
 * `crates/ipc/src/checks.rs`. **The declaration, never the answer** — what the
 * Judge said is `Judged`, one row per criterion. No question crosses: a
 * question is a prompt in a screenshot.
 */
export type DeclaredJudge = {
  /** How many yes/no questions this entry asks. Zero looks only for gaming. */
  criteria: number;
  /** How many judges answer each one. **Absent at one**, so present means a
   * panel — a client comparing against `1` would restate the domain's default. */
  panel_size?: number;
  /** Whether a second look asks whether the evidence was gamed. It does not
   * gate; what it found arrives as an escalation, not as a verdict. */
  gaming_check: boolean;
};

/**
 * One declared Check, as the gate found it.
 *
 * **`produced` is absent on a pass because a pass measured nothing** — the
 * outcome is the whole sentence. The other five each say something different
 * about why a step did not pass, and none of them is `failed`.
 *
 * **`skipped` is the one that did not stop the step.** The Check declares which
 * paths it covers and this step changed none of them, so it was not run — it is
 * not a pass and it is not a failure, and a surface that drew it as either
 * would be reporting a verification that never happened. `produced` names the
 * paths it covers.
 */
export type CheckRun = {
  /** Which run of the step produced it, counted from one. Since 7.0. Joins
   * to `StepDetail.attempts`. */
  attempt: number;
  /** The Manifest Check's name, or the built-in's kind. Joins to `checks`. */
  name: string;
  /** `check-outcomes.toml`: `passed`, `failed`, `signalled`, `timed_out`, `never_ran`, `skipped`. */
  outcome: string;
  /** What the Check was measured against. Absent on a pass, and on a skip. */
  expected?: string;
  /**
   * The exit code, the signal, the budget it outran, what is not installed —
   * or, on a skip, the paths the Check covers and this step did not touch.
   */
  produced?: string;
  /**
   * Where the Check's stdout and stderr were written, relative to the
   * repository root. **A reference, never the content**, and absent where there
   * is no file — a built-in assertion runs no command, and a Check that never
   * started printed nothing.
   *
   * **Main opens it; the renderer never composes it into a path.** See
   * `artifacts.ts`, which owns that rule for all three kept records.
   */
  output_path?: string;
};

// What the gates said about a step, re-exported so `protocol.ts` stays the one
// import for the wire vocabulary. They live in `judged.ts` because this file
// reached the 900 lines the gate refuses again; the cut is the one
// `crates/ipc/src/judged.rs` already draws — and it draws it for this reason,
// having been split off `detail.rs` on the same line.
export type { Citation, Flagged, Given, Judged, KeptDeliverable } from "./judged";

/** What a Job is about. Neither sequencing nor provenance. */
export type Subject = { kind: string; reference: string };

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

/**
 * How full the fleet is, and the one thing holding the next Drone back.
 * `crates/ipc/src/capacity.rs`.
 *
 * **Fleet-wide, and the answer no per-Job field could give.** A `queued` row's
 * `queued_reason` folds the concurrency bound, CPU, memory and disk into
 * `waiting_on_resources`, because that is the only label the registry grants
 * it. This says which of them it was.
 */
export type FleetCapacity = {
  /** How many Jobs Fleet may work at once. */
  bound: number;
  /**
   * How many it is working, from the roster admission counts against — never a
   * count of `running` rows. An escalated Job keeps its Drone alive and idle so
   * a redirect costs no respawn, and it keeps its place while it does.
   */
  occupied: number;
  /**
   * A key into `ADMISSION_HOLD` in the generated vocabulary. **Absent means
   * nothing is holding it** — not unknown, and not that Fleet failed to look,
   * because an unreadable machine admits rather than refuses.
   *
   * `string` and not a union, for the reason at the top of this file and one
   * more: this is the one set on the seam a newer Fleet may widen without a
   * protocol bump, so a union here would be a build that refuses a message it
   * is meant to survive.
   */
  held_by?: string;
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
  /**
   * Files staged before the Job existed. Fleet reads the bytes itself, at
   * `staged_path`, on the same machine — nothing here carries a payload.
   */
  attachments?: AttachmentRef[];
};

/**
 * One other Job claiming paths this one claims. `crates/ipc/src/overlap.rs`.
 *
 * One entry per Job rather than per path: a person decides about the other Job,
 * so the Job is the row and the paths are its detail.
 */
export type ScopeOverlap = {
  job_id: string;
  /** What the other Job is called. Carried, so a card names it rather than an id. */
  title: string;
  /** Where the other Job is. `running` and `awaiting_review` are weighed differently. */
  status: string;
  /** Never empty. The entry exists because a path is shared. */
  paths: SharedPath[];
};

/**
 * One place both Jobs claim, and who claimed it on each side. A step id is a
 * Drone's own declaration; absent is the Job's `write_targets`, which the
 * requester stated before anything ran.
 */
export type SharedPath = {
  /** The narrower of the two claims — where the collision is, not what contains it. */
  path: string;
  this_step?: string;
  other_step?: string;
};

export type ProposedCriterion = { text: string; source: string };

/**
 * One staged file, named to Fleet. `staged_path` is an absolute path on the
 * machine Fleet runs on — the same-machine assumption this seam already
 * makes (`docs/practices/protocol.md`). Fleet reads the bytes itself; Bridge
 * never sends a payload over this channel.
 */
export type AttachmentRef = { staged_path: string; filename: string; mime_type: string };

// Everything a filed report is made of, re-exported so `protocol.ts` stays the
// one import for the wire vocabulary. They live in `report.ts` because this
// file reached the 900 lines the gate refuses again; the cut is the seam
// `crates/ipc/src/report.rs` already draws, and it is the same remedy the event
// shapes took below.
export type { Calibration, FileReport, Report, ReportList } from "./report";

/**
 * What forgetting a Job leaves to say. `crates/ipc/src/job.rs`.
 *
 * **The id, and nothing else** — there is no row left for a summary to
 * describe. `forget_job`'s command response and `job.forgotten`'s event
 * payload are the same type on the Rust side, so this is used both ways here
 * too rather than declared twice for one fact.
 */
export type JobForgotten = { job_id: string };

// The other half of forgetting: the worktree and the branch, given back while
// Fleet runs. Its own file for the reason the two above have one — this file is
// at the gate's ceiling, and the cut is the seam `crates/ipc/src/reclaimed.rs`
// already draws.
export type { ReclaimedBranch, ReclaimedWorktree, WorktreeReclaimed } from "./reclaimed";

// Every event shape, re-exported so `protocol.ts` stays the one import for the
// wire vocabulary. They live in `events.ts` because this file reached the 900
// lines the gate refuses; the cut is the socket seam `crates/ipc/src/event.rs`
// already draws.
// Every run of one step. Its own file for the reason `crates/ipc/src/attempt.rs`
// has one: it is folded out of the job's log rather than read off a row, and
// this file was over the gate's ceiling.
export type { StepAttempt } from "./attempt";

export type {
  ChangedFile,
  Delivered,
  DroneExited,
  DroneSpawned,
  Event,
  JobAsking,
  JobCreated,
  JobFilesChanged,
  JobJudging,
  JobLanded,
  JobStateChanged,
  JobStepAdvanced,
  Missed,
  Resync,
  StreamMessage,
} from "./events";

// Fleet's own reading of its Manifest, re-exported so `protocol.ts` stays the
// one import for the wire vocabulary. It lives in `reading.ts` because that is
// the cut `crates/ipc/src/reading.rs` makes, and because this file has no room:
// it was at exactly 900 before `judged.ts` was taken out of it.
export type {
  ManifestFault,
  ManifestMoved,
  ManifestReading,
  ManifestRefused,
} from "./reading";

// What is outstanding on a live drone and what a person sends it back,
// re-exported so `protocol.ts` stays the one import for the wire vocabulary.
// Cut out for `events.ts`'s reason — this file reached 900 again — and the cut
// is the one `crates/ipc/src/waiting.rs` makes on the other side. See
// `waiting.ts`.
export type {
  AskedOption,
  ChosenAnswer,
  QuestionInFlight,
  RedirectInFlight,
  RedirectWaiting,
} from "./waiting";

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
