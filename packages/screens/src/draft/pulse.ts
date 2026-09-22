// What a Job is holding on this machine, right now. Draft, for
// `crates/ipc/src/resources.rs`.
//
// Source of truth today: `JobResources` — `read_at`, `held`, `processes` and a
// single optional `worktree`. Two things change here, and both come from one
// Job running several Drones (#1532, 22 Sep): **a process gains an owner**, and
// **`worktree` becomes a list**, one per member.
//
// Today a Job holds one worktree and a process has no owner, so the derivation
// gives every process that one worktree as its owner and makes a list of one.
// A board built on this draws the real Fleet as a Job with one worktree, which
// is exactly what it is.

import type { JobProcess, JobResources } from "@armada/protocol";

/** One process, and which worktree it belongs to. */
export type PulseProcess = {
  pid: number;
  /** The executable's own name. **Never its arguments**, which carry paths. */
  command: string;
  /** Share of one core. Not capped at 100: a process across four is real. */
  cpu_percent: number;
  memory_bytes: number;
  /** `ps`'s own spelling. Rendered, never parsed. */
  running_for: string;
  /** The process Fleet wrote down. At most one row carries it. */
  recorded: boolean;
  /**
   * The worktree it belongs to, by branch. **`null` is a process Fleet cannot
   * place** — with one worktree per Job that never happens; with several it is
   * a real answer and must not read as "the first one".
   */
  owner: string | null;
};

/** One checkout the Job holds. */
export type PulseWorktree = {
  path: string;
  branch: string;
  /** Absent is a walk that did not finish inside its bound, never zero. */
  bytes?: number;
};

/** What one log is, who owns it, and whether anything is writing to it. */
export type PulseLog = {
  /** An opaque string, on `LedgerRow.kind`'s argument. */
  kind: string;
  /** The worktree or member it belongs to. `null` is the Job's own. */
  owner: string | null;
  /** What it weighs. **Absent is unmeasured, never zero.** */
  bytes?: number;
  /** Whether something is writing to it right now. */
  writing: boolean;
};

/** Everything a Job holds, at one instant. */
export type PulseView = {
  job: string;
  /** Every figure here is as of this. **A panel drawing them without it lies.** */
  read_at: string;
  /** Fleet's reading of its recorded Drone: `running`, `gone`, `replaced`, … */
  held: string;
  processes: PulseProcess[];
  worktrees: PulseWorktree[];
  logs: PulseLog[];
};

/**
 * Today's reading, as the new board draws it.
 *
 * `logs` is the one part with nothing under it: the wire says when the Job's
 * log was last written to (`wrote_last_at`) and neither what it weighs nor
 * whether anything is writing now. So one row is derived for the Job's own log,
 * with `writing: false` and no size — both of which are the honest answer
 * rather than a figure nothing measured.
 */
export function pulseViewOf(resources: JobResources): PulseView {
  const worktree = resources.worktree;
  const owner = worktree?.branch ?? null;
  const logs: PulseLog[] =
    resources.wrote_last_at === undefined
      ? []
      : [{ kind: "job", owner: null, writing: false }];

  return {
    job: resources.job_id,
    read_at: resources.read_at,
    held: resources.held,
    processes: resources.processes.map((process) => pulseProcessOf(process, owner)),
    worktrees: worktree === undefined ? [] : [worktree],
    logs,
  };
}

function pulseProcessOf(process: JobProcess, owner: string | null): PulseProcess {
  return {
    pid: process.pid,
    command: process.command,
    cpu_percent: process.cpu_percent,
    memory_bytes: process.memory_bytes,
    running_for: process.running_for,
    recorded: process.recorded,
    owner,
  };
}
