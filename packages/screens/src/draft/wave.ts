// A Job that dispatched a wave of Jobs, and which of them waits on which.
// Draft, for `crates/ipc/src/detail.rs`.
//
// Source of truth today: `JobSummary.dispatched_by`, the parent's id on every
// row it dispatched (protocol 14.2). That is a set with no order in it.
//
// **The order is the one field missing, and the workflow already asks for
// it**: `epic.json`'s plan Judge wants "an edge wherever one of them has to
// wait for another". It is recorded in `plan.md` and lost before the Board.
// Every other fact a wave draws is served — each Job's status, its delivery,
// its held command and its Judge question.
//
// **No kind name.** A Job is a Job (#1530, 22 Sep).

import type { JobDetail, JobSummary, Settled } from "@armada/protocol";

/**
 * One Job of the wave.
 *
 * **`waits_on` points at what this Job waits for**, never at what waits for
 * it. Drawn from it, the Job waited on comes first and the one waiting sits
 * behind it — the reverse reads as a roll-up feeding the work it rolls up.
 */
export type WaveJobView = {
  /** The Job's id, which is what a press opens. */
  job: string;
  title: string;
  /** A registered `job_status`. Never a word this file invents. */
  status: string;
  /** What a person calls it. Absent where the Board row has not arrived. */
  handle?: string;
  /**
   * Which pass of the loop dispatched it, counted from one. A loop return
   * replaces `plan.md` whole, so an earlier pass is history rather than a
   * second live split — `WaveView.rounds` carries that difference.
   */
  round: number;
  /** The Jobs this one waits on, by id. Empty is a Job that may start at once. */
  waits_on: readonly string[];
  /** Where its pull request settled, where it has. `Settled` on the wire. */
  landed?: Settled;
};

/** One pass of plan, dispatch and roll up, and what its split was for. */
export type WaveRoundView = {
  round: number;
  /** What this pass split the work into, one line. */
  says: string;
  /**
   * Whether this is the plan the Job is running now. Every earlier round is
   * history and says so, rather than reading as a second live split.
   */
  live: boolean;
};

/** A Job and the wave it dispatched. */
export type WaveView = {
  /** The parent Job's id. */
  job: string;
  title: string;
  /** Every pass of the loop, oldest first. */
  rounds: readonly WaveRoundView[];
  /** Every Job the wave dispatched, in the order the plan listed them. */
  jobs: readonly WaveJobView[];
};

/**
 * The wave a Job dispatched, from the Board's own rows.
 *
 * **A Job with no such row dispatched no wave**, and `undefined` is that
 * answer — never an empty graph, which reads as Jobs that failed to load.
 *
 * **`waits_on` comes back empty and that is honest.** Nothing on the wire
 * records the order, so every Job reads as one that may start at once and the
 * graph draws one column. The mock's own moment fills it until Fleet does.
 */
export function waveOf(detail: JobDetail, board: readonly JobSummary[]): WaveView | undefined {
  const jobs = board
    .filter((row) => row.dispatched_by === detail.job.id)
    .map(
      (row): WaveJobView => ({
        job: row.id,
        title: row.title,
        status: row.status,
        handle: row.handle,
        round: 1,
        waits_on: [],
        ...(row.landed === undefined ? {} : { landed: row.landed }),
      }),
    );
  if (jobs.length === 0) return undefined;
  return {
    job: detail.job.id,
    title: detail.job.title,
    rounds: [{ round: 1, says: detail.job.title, live: true }],
    jobs,
  };
}
