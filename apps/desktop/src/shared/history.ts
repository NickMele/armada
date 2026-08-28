// One Job's transition history, as TypeScript sees it.
// `crates/ipc/src/history.rs`.
//
// Its own file for the reason the Rust side gives it one: a history is its own
// operation and not a field on `JobDetail`. A detail is fetched on every open
// of a Job to draw a summary; a history has no bound — it grows for as long as
// the Job lives, and a retried step is a row per attempt plus the moves around
// it. A field on the detail would make the common read pay for the rare one.
//
// **Read and rendered, never replayed.** `crates/store/src/fold.rs` owns the
// machine, and a client that folded these would be a second one — agreeing with
// the first only until one of them changed.

import type { Reason } from "./protocol";

/** Every move one Job made, oldest first. */
export type JobHistory = {
  job_id: string;
  /**
   * Every recorded move, in `seq` order. **Empty is a real answer** — a Job
   * created and not yet moved has no events, because creation is not a
   * transition and no row describes it.
   */
  moves: Recorded[];
};

/**
 * One row of the log. **Both machines, in one order** — a status transition, a
 * step move and a Drone arriving are rows in the same table, which is what lets
 * a step move be ordered against the status transitions around it.
 */
export type Recorded = {
  /**
   * The key the log assigned. Monotonic, never reused, and **what orders the
   * list — never `at`**, which is injected rather than read from a clock, so
   * two moves inside one millisecond carry the same instant.
   */
  seq: number;
  /**
   * The status the Job stood in when this happened. For a status move it is the
   * status it **left**; for a step or Drone move it is the status it stayed in.
   */
  status: string;
  moved: Movement;
  /** Who caused it: `human`, `fleet` or `drone`. */
  actor: string;
  at: string;
};

/** What the row says moved. The three shapes the log admits, and no fourth. */
export type Movement =
  | ({ kind: "status" } & StatusMoved)
  | ({ kind: "step" } & StepMoved)
  | ({ kind: "drone" } & DroneMoved);

/** The Job's own machine moved. `Recorded.status` is where it left. */
export type StatusMoved = {
  to: string;
  /** Absent on the eight destinations that store none. */
  reason?: Reason;
};

/**
 * The inner machine moved, beneath a status that did not. It has no status
 * target for that reason: the row's `status` is both sides of it.
 */
export type StepMoved = {
  step_id: string;
  from: string;
  to: string;
  /** Why the step stopped, on the one move that stops it. A trigger spelling. */
  why?: string;
};

/**
 * A Drone arrived or left. **Presence, not a state pair** — `assigned_drone` is
 * a pointer that is set or null and has no states of its own.
 */
export type DroneMoved = {
  drone_id: string;
  /** `drone_spawned` or `drone_exited`. */
  presence: string;
};
