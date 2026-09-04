// What one Job holds on this machine, and what came of asking whether it is
// working. `crates/ipc/src/resources.rs`.
//
// # Not the budget
//
// `JobSpend` answers the model axis — cost, turns, wall clock, drones. On the
// Job that hung on 4 Sep 2026 all four read zero, which was true and told
// nobody anything: a Job between phases and a Job hung are the same four zeros.
// This is the axis that was got by hand with a process listing, a `du` and a
// look at a log file's mtime.
//
// # Every figure carries when it was taken
//
// A process can exit between the sample and the render. `read_at` is on the
// reading rather than beside it, so a surface cannot draw the figures without
// the instant they belong to.
//
// # Absent is never empty
//
// `worktree` absent is a Job with no checkout; `bytes` absent is a walk that
// did not finish inside its bound. Neither is zero, and a surface that drew
// them as zero would report a Job that never ran as one that wrote nothing.
//
// Hand-written like `journal.ts`, for the same reason: the codegen that would
// emit the DTOs from the Rust source does not exist yet.

import type { NotedField } from "./journal";

/**
 * What Fleet recorded for this Job's Drone, and what is at that pid now.
 *
 * **The loud values are `gone` and `replaced`** — Fleet believing something is
 * running that is not. `none` is ordinary: a Job that has not been dispatched,
 * or one between Drones. `unreadable` is the probe itself failing and must not
 * be drawn as `gone`, which would decide on no evidence.
 */
export type Held = "none" | "running" | "gone" | "replaced" | "unreadable";

/**
 * One process this Job holds.
 *
 * **Named by its command and never by its arguments.** The argument vector
 * carries absolute paths, a repository layout and whatever a Check was invoked
 * with; what a person needs is what it is.
 */
export type JobProcess = {
  pid: number;
  /** The executable's own name — `node`, `cargo`, `git`. */
  command: string;
  /** Share of one core. **Not capped at 100**: a process across four is real. */
  cpu_percent: number;
  memory_bytes: number;
  /** How long it has been running, in `ps`'s spelling. Rendered, never parsed. */
  running_for: string;
  /** The process Fleet wrote down. At most one row carries it. */
  recorded: boolean;
};

/** The Job's checkout, and what it has taken. */
export type WorktreeOnDisk = {
  path: string;
  branch: string;
  /** Absent is a walk that did not finish inside its bound, never zero. */
  bytes?: number;
};

/** `GET /jobs/:job_id/resources` — what one Job holds, at one instant. */
export type JobResources = {
  job_id: string;
  /** Every figure here is as of this. A panel drawing them without it lies. */
  read_at: string;
  held: Held;
  /**
   * The recorded process and everything descended from it, that one first.
   *
   * **Empty is loud.** `held` of `running` with nothing here is a process that
   * answered a liveness probe and holds nothing.
   */
  processes: JobProcess[];
  worktree?: WorktreeOnDisk;
  /** When anything was last written to the Job's own log. */
  wrote_last_at?: string;
};

/**
 * What one look came to, and what the whole examination came to.
 *
 * **`working` reads as "this is as it should be"**, not as "a processor is
 * busy". A Job that is over is over and a Job waiting for a person is waiting
 * for a person; neither holds a process and neither is a fault.
 *
 * **`cannot_tell` is never rounded up.** One look that could not separate
 * working from not keeps the whole answer off `working`, because "everything
 * looks fine" on a plainly hung Job spends a person's suspicion and returns
 * nothing.
 */
export type Finding = "working" | "not_working" | "cannot_tell";

/** Which question one look asked. Five, and every examination asks all five. */
export type Asked = "process" | "worktree" | "writing" | "span" | "silence";

/** One question asked of the Job, and what was found. */
export type Look = {
  asked: Asked;
  found: Finding;
  /** One line, in Fleet's own words. Never carries an interpolated value. */
  said: string;
  /** What the line opens to. Absent is a look with nothing to show. */
  fields?: NotedField[];
};

/** `POST /jobs/:job_id/examine` — what Fleet found when somebody asked. */
export type JobExamined = {
  job_id: string;
  looked_at: string;
  found: Finding;
  looks: Look[];
  /** The reading the looks were drawn from, so one instant is on the screen. */
  resources: JobResources;
};
