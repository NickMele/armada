// What a filed report is made of. `crates/ipc/src/report.rs`.
//
// **Split out of `protocol.ts`, not authored apart from it.** That file is the
// one import for the wire vocabulary and still is — it re-exports every type
// here — but it reached the 900 lines the gate refuses, and the cut follows a
// seam `crates/ipc` already draws rather than one invented for the line count.
// `events.ts` was cut the same way for the same reason.
//
// Every closed set is left as `string`, for the reason `protocol.ts` gives.

/**
 * What a person sends to say a Job failed in error. `crates/ipc/src/report.rs`.
 *
 * **`said` is the whole reason this exists.** Everything Fleet attaches around
 * it is already served by three other routes; the sentence is the one thing
 * that does not exist anywhere until somebody types it, and Fleet answers 422
 * without it.
 *
 * `claim` is left as `string` like every other closed set here — but it is the
 * one Bridge *writes* rather than renders, so the three values it may hold are
 * named where the picker offers them, in `renderer/src/Report.tsx`.
 */
export type FileReport = {
  claim: string;
  said: string;
  /**
   * The step the report is about. **Sendable without `criterion_id`**, which is
   * what a report about a step the gate judged nothing on looks like — an
   * undecided gate records no verdict, so there is none to name.
   */
  step_id?: string;
  /** Sent with `step_id` and never without it: a criterion id is unique inside a step. */
  criterion_id?: string;
};

/**
 * One filed report, as it reads afterwards. `crates/ipc/src/report.rs`.
 *
 * **`record` is the Job's own evidence rendered at filing time**, not a join to
 * rows that are still there: `armada clean` forgets a Job and takes every row
 * beneath it, and the report stays whole. `job_id` may therefore name a Job
 * that no longer exists, which is deliberate.
 */
export type Report = {
  id: string;
  filed_at: string;
  /** `human`. The column exists so the day Fleet files its own, it is a value. */
  origin: string;
  claim: string;
  job_id: string;
  job_title: string;
  step_id?: string;
  criterion_id?: string;
  /** The person's own words. The finding. */
  said: string;
  /** The record, as the body of an issue. The evidence. */
  record: string;
};

/** Every report filed, and the counts they are read beside. */
export type ReportList = {
  /** Newest first, bodies included. */
  reports: Report[];
  calibration: Calibration;
};

/**
 * What is known about whether the Judge has been right.
 *
 * **Four counts and not a rate.** A rate's denominator would count every Job
 * nobody read, and an unread Job is not a pass — so the gap between what the
 * Judge refused and what a person disputed is left visible rather than divided
 * away.
 */
export type Calibration = {
  refusals_recorded: number;
  refusals_disputed: number;
  /** Not the other half of the same number: a wrong pass is refused by nothing. */
  passes_disputed: number;
  reports_filed: number;
};
