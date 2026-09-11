// A person's run of one Manifest entry in a Job's worktree — the run sheet's
// wire. `crates/ipc/src/rehearsal.rs`. Since protocol 10.6.
//
// **A rehearsal, never a verdict.** Nothing here is a Check row or Evidence,
// and nothing here moves a Job, so no field maps onto a status colour.

import type { ChangedFile } from "./events";
import type { ProtocolVersion } from "./version";

/** `GET /jobs/:job_id/run_sheet` — what can be run in this Job's worktree. */
export type RunSheet = {
  job_id: string;
  /** The Commands `setup.requires` names, in its order. */
  setup: RunEntry[];
  /** Every Check, in the order the Manifest declares them. */
  checks: RunEntry[];
  /** Every Command `setup.requires` does not name. */
  commands: RunEntry[];
  /** When the Manifest was last changed by a commit at or before the Job was created. */
  manifest_edited_at?: string;
  worktree_on_disk: boolean;
  /** The worktree's own `armada.yml` declares something other than these lists. */
  worktree_differs: boolean;
  /** Why the worktree's own `armada.yml` could not be read, where it could not. */
  worktree_unreadable?: string;
  /** A Drone is working in the tree: a run shares it, and Undo is refused. */
  drone_working: boolean;
  /** The run in flight on this Job, if one is. */
  running?: RunUnderway;
};

export type RunEntry = {
  name: string;
  /** The `run` line, exactly as declared. */
  run: string;
  /** It declares `narrow`. */
  narrows: boolean;
  /**
   * What a narrowed run resolves to against what the Job changed. Absent where
   * it declares no `narrow`, or where nothing the Job changed feeds it.
   */
  narrow_run?: string;
  /** The Commands that run first, in order. */
  requires: string[];
  expect_exit_code: number;
  destructive: boolean;
  /** From what the Job froze. `false` is read from the Manifest Fleet holds now. */
  frozen: boolean;
};

/** `POST /jobs/:job_id/start_run`'s body. A name, never a command line. */
export type StartRun = {
  name: string;
  narrowed?: boolean;
  /** Run the worktree's own `armada.yml` rather than what the Job froze. */
  worktree_version?: boolean;
};

/** A run that has started and not finished. `start_run`'s answer. */
export type RunUnderway = {
  id: string;
  job_id: string;
  name: string;
  command: string;
  narrowed: boolean;
  /** Elapsed time is counted from here; nothing ticks on the wire. */
  started_at: string;
};

/** One finished run. `run.finished`'s payload and `list_runs`'s row. */
export type RunRecord = {
  id: string;
  job_id: string;
  name: string;
  command: string;
  narrowed: boolean;
  worktree_version: boolean;
  frozen: boolean;
  required: string[];
  started_at: string;
  ended_at: string;
  duration_ms: number;
  /** Absent where there was no code: a signal, a budget, a spawn that failed. */
  exit_code?: number;
  expect_exit_code: number;
  /** How it ended, in a sentence. Unhued: a rehearsal. */
  ended: string;
  stopped: boolean;
  changed: ChangedFile[];
  /** Why the change could not be read. Undo is then unavailable. */
  changed_unreadable?: string;
  /** A Drone was working during the run, so `changed` may hold its edits too. */
  shared_with_drone: boolean;
  /** Where the tree before the run is kept. Absent: nothing to undo from. */
  snapshot?: string;
  undone_at?: string;
  /** The log, relative to `ManifestSummary.records_root`. */
  log: string;
};

/** `GET /jobs/:job_id/runs` — newest first, and what would not read. */
export type RunList = {
  job_id: string;
  runs: RunRecord[];
  unreadable: { id: string; why: string }[];
};

/** `GET /jobs/:job_id/runs/:run_id/output` — `CheckOutput`'s window. */
export type RunOutput = {
  id: string;
  name: string;
  path: string;
  lines: string[];
  from_line: number;
  total_lines: number;
  bytes: number;
  whole: boolean;
};

/** `stop_run`'s and `undo_run`'s body. */
export type NamedRun = { id: string };

/**
 * One message on a run's socket, `GET /jobs/:job_id/runs/:run_id/observe`.
 * `observe_job`'s shape: the log so far, then new lines, a count where the
 * bound dropped some, and why it stopped. Output never rides `/events`.
 */
export type RunMessage =
  | ({ message: "opened" } & RunOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "missed" } & { dropped: number })
  | ({ message: "closed" } & { because: string });

export type RunOpened = {
  protocol_version: ProtocolVersion;
  job_id: string;
  id: string;
  name: string;
  path: string;
  /** The run was still going when this opened. `false`: the history is all of it. */
  live: boolean;
  /** Older lines the opening read left out, because the window is bounded. */
  skipped: number;
};
