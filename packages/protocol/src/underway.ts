// A step's Checks while the gate is running them, and a running Check's log
// read as it grows. `crates/ipc/src/underway.rs`.
//
// **A view of the run, never a record of it.** What the gate rules on and what
// `check_runs` holds are written after the ruling, as they always were; this is
// gone the moment they are. Since protocol 10.3.
//
// **Three states and no word for any of them.** The fields say which: no
// `started_at` is waiting, a start and no `ran` is running, and a `ran` is
// finished. A word spelled beside them would be a second statement of the same
// fact.

import type { CheckRun } from "./protocol";
import type { ProtocolVersion } from "./version";

/**
 * A step's Checks, from the moment the gate starts them until its ruling is
 * written down — **not only while one is running.** A Check that finished
 * minutes before the Judge answered is a result somebody can read in those
 * minutes, and `check_runs` does not hold it until the ruling does.
 */
export type ChecksUnderway = {
  /** Which run of the step these belong to. Joins to `check_runs` by `attempt`. */
  attempt: number;
  /** Every declared Check, in the step's order, waiting ones included. */
  checks: CheckUnderway[];
};

/** One declared Check, as the gate has it right now. */
export type CheckUnderway = {
  /** The Check's name, or the built-in's kind. Joins to `CheckRun.name`. */
  name: string;
  /**
   * When it started. **Absent while it waits** for a slot or for the Commands
   * it requires. Elapsed time is counted from here; nothing ticks on the wire.
   */
  started_at?: string;
  /** How long it took, once finished. */
  took_ms?: number;
  /**
   * What it came to, once finished — the row the ruling will write. Its own
   * `output_path` is absent: the live log is `output_path` below.
   */
  ran?: CheckRun;
  /**
   * Where its log is being written as it runs. The last component is what
   * `observe_check_output` takes. Absent while it waits, and on a Check that
   * runs no command.
   */
  output_path?: string;
};

/** One message on a running Check's log socket. `observe_job_log`'s shape. */
export type OutputMessage =
  | ({ message: "opened" } & OutputOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "closed" } & { because: string });

export type OutputOpened = {
  protocol_version: ProtocolVersion;
  job_id: string;
  /** Whose log this is, off the answer rather than off the row pressed. */
  name: string;
  attempt: number;
  path: string;
  /** Older lines the first read left out, because the window is bounded. */
  skipped: number;
};

/**
 * The running Check's log a window is following, as main holds it.
 *
 * **One at a time**, for the journal socket's reason: a person reads one
 * Check's log, and opening another replaces the first.
 */
export type FollowedLog =
  | { state: "none" }
  | { state: "opening"; jobId: string; kept: string }
  | {
      state: "following";
      jobId: string;
      kept: string;
      name: string;
      attempt: number;
      path: string;
      /** The file's own line number of `lines[0]`, counted from one. */
      fromLine: number;
      /** The newest lines, oldest first, bounded. */
      lines: string[];
      /**
       * Why the stream ended — `finished` or `unreadable` as Fleet said it, or
       * `broke` where the connection went without a sentence — or absent while
       * it is still arriving.
       */
      ended?: string;
    }
  | { state: "failed"; jobId: string; kept: string; detail: string };
